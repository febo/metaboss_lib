use anyhow::{anyhow, Result};
use mpl_token_metadata::{
    instruction::{builders::TransferBuilder, InstructionBuilder, TransferArgs},
    processor::AuthorizationData,
    state::{ProgrammableConfig, TokenStandard},
};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{
    signature::{Keypair, Signature},
    signer::Signer,
    transaction::Transaction,
};

use crate::{data::Asset, decode::ToPubkey};

pub enum TransferAssetArgs<'a, P: ToPubkey> {
    V1 {
        payer: Option<&'a Keypair>,
        authority: &'a Keypair,
        mint: P,
        source_owner: P,
        source_token: P,
        destination_owner: P,
        destination_token: P,
        amount: u64,
        authorization_data: Option<AuthorizationData>,
    },
}

pub async fn transfer_asset<P: ToPubkey>(
    client: &RpcClient,
    args: TransferAssetArgs<'_, P>,
) -> Result<Signature> {
    match args {
        TransferAssetArgs::V1 { .. } => transfer_asset_v1(client, args).await,
    }
}

async fn transfer_asset_v1<P: ToPubkey>(
    client: &RpcClient,
    args: TransferAssetArgs<'_, P>,
) -> Result<Signature> {
    let TransferAssetArgs::V1 {
        payer,
        authority,
        mint,
        source_owner,
        source_token,
        destination_owner,
        destination_token,
        amount,
        authorization_data,
    } = args;

    let mint = mint.to_pubkey()?;
    let source_owner = source_owner.to_pubkey()?;
    let source_token = source_token.to_pubkey()?;
    let destination_owner = destination_owner.to_pubkey()?;
    let destination_token = destination_token.to_pubkey()?;

    let mut asset = Asset::new(mint);
    let payer = payer.unwrap_or(authority);

    let transfer_args = TransferArgs::V1 {
        amount,
        authorization_data,
    };

    let mut transfer_builder = TransferBuilder::new();
    transfer_builder
        .payer(payer.pubkey())
        .authority(authority.pubkey())
        .token(source_token)
        .token_owner(source_owner)
        .destination(destination_token)
        .destination_owner(destination_owner)
        .mint(asset.mint)
        .metadata(asset.metadata);

    let md = asset.get_metadata(client).await?;

    if matches!(
        md.token_standard,
        Some(TokenStandard::ProgrammableNonFungible)
    ) {
        // Always need the token records for pNFTs.
        let source_token_record = asset.get_token_record(&source_token);
        let destination_token_record = asset.get_token_record(&destination_token);
        transfer_builder
            .owner_token_record(source_token_record)
            .destination_token_record(destination_token_record);

        // If the asset's metadata account has auth rules set, we need to pass the
        // account in.
        if let Some(ProgrammableConfig::V1 {
            rule_set: Some(auth_rules),
        }) = md.programmable_config
        {
            transfer_builder.authorization_rules(auth_rules);
        }
    }

    if matches!(
        md.token_standard,
        Some(
            TokenStandard::NonFungible
                | TokenStandard::NonFungibleEdition
                | TokenStandard::ProgrammableNonFungible
        )
    ) {
        asset.add_edition();
        transfer_builder.edition(asset.edition.unwrap());
    }

    let transfer_ix = transfer_builder
        .build(transfer_args)
        .map_err(|e| anyhow!(e.to_string()))?
        .instruction();

    let recent_blockhash = client.get_latest_blockhash().await?;
    let tx = Transaction::new_signed_with_payer(
        &[transfer_ix],
        Some(&payer.pubkey()),
        &[payer, authority],
        recent_blockhash,
    );

    let sig = client.send_and_confirm_transaction(&tx).await?;

    Ok(sig)
}
