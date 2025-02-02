use anyhow::{anyhow, Result};
use mpl_token_metadata::state::{Creator, Data};
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

use crate::data::{NFTCreator, NFTData};

pub fn convert_local_to_remote_data(local: NFTData) -> Result<Data> {
    let creators = local
        .creators
        .ok_or_else(|| anyhow!("No creators specified in json file!"))?
        .iter()
        .map(convert_creator)
        .collect::<Result<Vec<Creator>>>()?;

    let data = Data {
        name: local.name,
        symbol: local.symbol,
        uri: local.uri,
        seller_fee_basis_points: local.seller_fee_basis_points,
        creators: Some(creators),
    };
    Ok(data)
}

fn convert_creator(c: &NFTCreator) -> Result<Creator> {
    Ok(Creator {
        address: Pubkey::from_str(&c.address)?,
        verified: c.verified,
        share: c.share,
    })
}
