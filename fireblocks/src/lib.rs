pub mod client;
pub mod objects;
pub mod signer;

use anyhow::{Context, Result};
use client::FireblocksClient;

pub fn build() -> Result<FireblocksClient> {
    let client = FireblocksClient::new().context("failed to construct client")?;

    Ok(client)
}
