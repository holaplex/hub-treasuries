#![allow(clippy::module_name_repetitions)]

use anyhow::{Context, Result};
use clap::{arg, Parser};
use jsonwebtoken::EncodingKey;
use log::info;
use reqwest::{Client as HttpClient, RequestBuilder, Url};
use serde::Serialize;

use crate::{
    objects::{
        transaction::{CreateTransaction, CreateTransactionResponse},
        vault::{
            CreateVault, CreateVaultAssetResponse, CreateVaultWallet, QueryVaultAccounts,
            VaultAccount, VaultAccountsPagedResponse, VaultAsset,
        },
    },
    signer::RequestSigner,
};

#[derive(Parser)]
pub struct Args {
    #[arg(long, env)]
    fireblocks_endpoint: String,
    #[arg(long, env)]
    fireblocks_api_key: String,
    #[arg(long, env)]
    secret_path: String,
}

pub struct FireblocksClient {
    http: HttpClient,
    request_signer: RequestSigner,
    base_url: Url,
    api_key: String,
}

impl FireblocksClient {
    /// Res
    ///
    /// # Errors
    /// This function fails if ...
    pub fn new() -> Result<Self> {
        let Args {
            fireblocks_endpoint,
            fireblocks_api_key,
            secret_path,
        } = Args::parse();

        let http = HttpClient::new();

        let encoding_key = Self::build_encoding_key(secret_path)?;

        let base_url =
            Url::parse(&fireblocks_endpoint).context("failed to parse fireblocks endpoint")?;

        Ok(Self {
            http,
            request_signer: RequestSigner::new(encoding_key, fireblocks_api_key.clone()),
            base_url,
            api_key: fireblocks_api_key,
        })
    }
    /// Res
    ///
    /// # Errors
    /// This function fails if ...

    fn build_encoding_key(secret_path: String) -> Result<EncodingKey> {
        let rsa = std::fs::read(secret_path).context("failed to read secret key")?;

        EncodingKey::from_rsa_pem(&rsa).context("failed to create encoding key")
    }
    /// Res
    ///
    /// # Errors
    /// This function fails if ...
    fn authenticate(
        &self,
        req: RequestBuilder,
        endpoint: String,
        body: impl Serialize,
    ) -> Result<RequestBuilder> {
        let token = self.request_signer.sign(endpoint, body)?;

        Ok(req.header("X-API-KEY", &self.api_key).bearer_auth(token))
    }
    /// Res
    ///
    /// # Errors
    /// This function fails if ...
    pub async fn get_vaults(
        &self,
        filters: QueryVaultAccounts,
    ) -> Result<VaultAccountsPagedResponse> {
        let endpoint = "/v1/vault/accounts_paged";
        let url = self.base_url.join(endpoint)?;

        let mut req = self.http.get(url);
        req = self.authenticate(req, endpoint.to_string(), filters)?;

        let response = req.send().await?.text().await?;

        info!("{:?}", response);

        Ok(serde_json::from_str(&response)?)
    }
    /// Res
    ///
    /// # Errors
    /// This function fails if ...
    pub async fn get_vault(&self, vault_id: String) -> Result<VaultAccount> {
        let endpoint = format!("/v1/vault/accounts/{vault_id}");
        let url = self.base_url.join(&endpoint)?;

        let mut req = self.http.get(url);
        req = self.authenticate(req, endpoint, ())?;

        let response = req.send().await?.text().await?;

        info!("{:?}", response);

        Ok(serde_json::from_str(&response)?)
    }
    /// Res
    ///
    /// # Errors
    /// This function fails if ...
    pub async fn create_vault(&self, params: CreateVault) -> Result<VaultAccount> {
        let endpoint = "/v1/vault/accounts".to_string();
        let url = self.base_url.join(&endpoint)?;

        let mut req = self.http.post(url);
        req = self.authenticate(req, endpoint, params)?;

        let response = req.send().await?.text().await?;

        info!("{:?}", response);

        Ok(serde_json::from_str(&response)?)
    }
    /// Res
    ///
    /// # Errors
    /// This function fails if ...
    pub async fn create_vault_wallet(
        &self,
        vault_account_id: String,
        asset_id: String,
        body: CreateVaultWallet,
    ) -> Result<CreateVaultAssetResponse> {
        let endpoint = format!("/v1/vault/accounts/{vault_account_id}/{asset_id}");
        let url = self.base_url.join(&endpoint)?;

        let mut req = self.http.post(url);
        req = self.authenticate(req, endpoint, body)?;

        let response = req.send().await?.text().await?;

        info!("{:?}", response);

        Ok(serde_json::from_str(&response)?)
    }

    /// Res
    ///
    /// # Errors
    /// This function fails if ...
    pub async fn vault_assets(&self) -> Result<Vec<VaultAsset>> {
        let endpoint = "/v1/vault/assets".to_string();
        let url = self.base_url.join(&endpoint)?;

        let mut req = self.http.get(url);
        req = self.authenticate(req, endpoint, ())?;

        let response = req.send().await?.text().await?;

        info!("{:?}", response);

        Ok(serde_json::from_str(&response)?)
    }

    /// Res
    ///
    /// # Errors
    /// This function fails if ...
    pub async fn create_transaction(
        &self,
        tx: CreateTransaction,
    ) -> Result<CreateTransactionResponse> {
        let endpoint = "/v1/transactions".to_string();
        let url = self.base_url.join(&endpoint)?;

        let mut req = self.http.post(url).json(&tx);
        req = self.authenticate(req, endpoint, tx)?;

        let response = req.send().await?.text().await?;

        info!("{:?}", response);

        Ok(serde_json::from_str(&response)?)
    }
}
