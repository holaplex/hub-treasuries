pub mod client;
pub mod objects;
pub mod signer;

use anyhow::{Context, Result};
use client::FireBlocksClient;

pub fn build() -> Result<FireBlocksClient> {
    let client = FireBlocksClient::new().context("failed to construct client")?;

    Ok(client)
}
