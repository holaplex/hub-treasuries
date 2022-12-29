use anyhow::{Context, Result};
use chrono::prelude::*;
use clap::{arg, Parser};
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use log::debug;
use reqwest::{Client as HttpClient, RequestBuilder, Url};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{
    objects::vault::{CreateVault, QueryVaultAccounts, VaultAccount, VaultAccountsPagedResponse},
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

pub struct FireBlocksClient {
    http: HttpClient,
    request_signer: RequestSigner,
    base_url: Url,
    api_key: String,
}

impl FireBlocksClient {
    pub(crate) fn new() -> Result<FireBlocksClient> {
        let Args {
            fireblocks_endpoint,
            fireblocks_api_key,
            secret_path,
        } = Args::parse();

        let http = HttpClient::new();

        let encoding_key = Self::build_encoding_key(secret_path)?;

        let base_url =
            Url::parse(&fireblocks_endpoint).context("failed to parse fireblocks endpoint")?;

        Ok(FireBlocksClient {
            http,
            request_signer: RequestSigner::new(encoding_key, fireblocks_api_key.clone()),
            base_url,
            api_key: fireblocks_api_key,
        })
    }

    fn build_encoding_key(secret_path: String) -> Result<EncodingKey> {
        let rsa = std::fs::read(secret_path).context("failed to read secret key")?;

        EncodingKey::from_rsa_pem(&rsa).context("failed to create encoding key")
    }

    fn authenticate(
        &self,
        req: RequestBuilder,
        endpoint: String,
        body: impl Serialize,
    ) -> Result<RequestBuilder> {
        let token = self.request_signer.sign(endpoint, body)?;

        Ok(req.header("X-API-KEY", &self.api_key).bearer_auth(token))
    }

    pub async fn get_vaults(
        &self,
        filters: QueryVaultAccounts,
    ) -> Result<VaultAccountsPagedResponse> {
        let endpoint = "/v1/vault/accounts_paged".to_string();

        let url = self.base_url.join(&endpoint)?;

        let mut req = self.http.get(url);

        req = self.authenticate(req, endpoint, filters)?;

        let response = req
            .send()
            .await?
            .json::<VaultAccountsPagedResponse>()
            .await?;

        Ok(response)
    }

    pub async fn get_vault(&self, vault_id: String) -> Result<VaultAccount> {
        let endpoint = format!("/v1/vault/accounts/{}", vault_id);

        let url = self.base_url.join(&endpoint)?;

        let mut req = self.http.get(url);

        req = self.authenticate(req, endpoint, ())?;

        let response = req.send().await?.json::<VaultAccount>().await?;

        Ok(response)
    }

    pub async fn create_vault(&self, params: CreateVault) -> Result<VaultAccount> {
        let endpoint = format!("/v1/vault/accounts");

        let url = self.base_url.join(&endpoint)?;

        let mut req = self.http.post(url);

        req = self.authenticate(req, endpoint, params)?;

        let response = req.send().await?.json::<VaultAccount>().await?;

        Ok(response)
    }
}
