//!

#![deny(
    clippy::disallowed_methods,
    clippy::suspicious,
    clippy::style,
    missing_debug_implementations,
    missing_copy_implementations
)]
#![warn(clippy::pedantic, clippy::cargo)]

use hub_core::{anyhow::Result, clap};
pub mod assets;
mod client;
pub mod objects;
mod signer;

use assets::Assets;
pub use client::{Client, Error};

#[derive(clap::Args, Clone, Debug)]
pub struct FbArgs {
    #[arg(long, env)]
    pub fireblocks_endpoint: String,
    #[arg(long, env)]
    pub fireblocks_api_key: String,
    #[arg(long, env)]
    pub fireblocks_secret_path: String,
    #[arg(long, env, default_value = "false")]
    pub fireblocks_test_mode: bool,
    #[arg(long, env, value_delimiter = ',')]
    pub fireblocks_supported_asset_ids: Vec<String>,
    #[arg(long, env)]
    pub fireblocks_treasury_vault_id: String,
    #[arg(long, env)]
    pub fireblocks_whitelisted_contract_wallet_id: String,
}

#[allow(missing_debug_implementations)]
#[derive(Clone)]
pub struct Fireblocks {
    client: Client,
    assets: Assets,
    treasury_vault: String,
}

impl Fireblocks {
    /// Creates a Fireblocks Client
    /// # Errors
    /// Returns an error if the client cannot be created
    pub fn new(args: FbArgs) -> Result<Self> {
        let client = Client::new(args.clone())?;
        let assets = Assets::new(args.clone());
        let treasury_vault = args.fireblocks_treasury_vault_id;

        Ok(Self {
            client,
            assets,
            treasury_vault,
        })
    }

    #[must_use]
    pub fn assets(&self) -> &Assets {
        &self.assets
    }

    #[must_use]
    pub fn client(&self) -> &Client {
        &self.client
    }

    #[must_use]
    pub fn treasury_vault(&self) -> String {
        self.treasury_vault.clone()
    }
}
