use mpl_token_metadata::instruction::builders::UnverifyBuilder;

use super::*;

pub enum UnverifyCreatorArgs<'a, P1: ToPubkey> {
    V1 { authority: &'a Keypair, mint: P1 },
}

pub fn unverify_creator<P1>(client: &RpcClient, args: UnverifyCreatorArgs<P1>) -> Result<Signature>
where
    P1: ToPubkey,
{
    match args {
        UnverifyCreatorArgs::V1 { .. } => unverify_creator_v1(client, args),
    }
}

fn unverify_creator_v1<P1>(client: &RpcClient, args: UnverifyCreatorArgs<P1>) -> Result<Signature>
where
    P1: ToPubkey,
{
    let UnverifyCreatorArgs::V1 { authority, mint } = args;

    let mint = mint.to_pubkey()?;
    let asset = Asset::new(mint);

    let md = asset.get_metadata(client)?;

    let mut unverify_builder = UnverifyBuilder::new();
    unverify_builder
        .authority(authority.pubkey())
        .metadata(asset.metadata);

    if !matches!(
        md.token_standard,
        Some(TokenStandard::NonFungible | TokenStandard::ProgrammableNonFungible) | None
    ) {
        bail!("Only NFTs or pNFTs can have creators be verified");
    }

    let unverify_ix = unverify_builder
        .build(VerificationArgs::CreatorV1)
        .map_err(|e| anyhow!(e.to_string()))?
        .instruction();

    let recent_blockhash = client.get_latest_blockhash()?;
    let tx = Transaction::new_signed_with_payer(
        &[unverify_ix],
        Some(&authority.pubkey()),
        &[authority],
        recent_blockhash,
    );

    // Send tx with retries.
    let res = retry(
        Exponential::from_millis_with_factor(250, 2.0).take(3),
        || client.send_and_confirm_transaction(&tx),
    );

    Ok(res?)
}
