#![allow(missing_debug_implementations)]

use hub_core::{
    anyhow::{Context as _, Result},
    serde_json::{self},
    thiserror,
    tokio::time,
};
use jsonwebtoken::EncodingKey;
use reqwest::{Client as HttpClient, RequestBuilder, Url};
use serde::Serialize;

use crate::{
    objects::{
        transaction::{
            CreateTransaction, CreateTransactionResponse, ExtraParameters, RawMessageData,
            TransactionDetails, TransactionOperation, TransactionStatus, TransferPeerPath,
            UnsignedMessage,
        },
        vault::{
            CreateVault, CreateVaultAssetResponse, CreateVaultWallet, QueryVaultAccounts,
            VaultAccount, VaultAccountsPagedResponse, VaultAsset,
        },
    },
    signer::RequestSigner,
    FbArgs,
};

/// Represents a Fireblocks API client.
/// It contains an HTTP client, a request signer, the base URL for the Fireblocks endpoint, and an API key.
#[derive(Clone)]
pub struct Client {
    http: HttpClient,
    request_signer: RequestSigner,
    base_url: Url,
    api_key: String,
}

#[derive(Debug, Clone, Copy, thiserror::Error)]
pub enum Error {
    #[error("failed to sign transaction")]
    Transaction(TransactionStatus),
}

impl Client {
    /// Constructs a new Fireblocks API client.
    ///
    /// # Arguments
    ///
    /// * `args` - Fireblocks API client arguments containing endpoint, API key, and secret path.
    ///
    /// # Errors
    ///
    /// This function can fail if:
    ///
    /// * Failed to parse the Fireblocks endpoint URL.
    /// * Failed to read the secret key file.
    /// * Failed to create an encoding key from the secret key.
    pub fn new(args: FbArgs) -> Result<Self> {
        let FbArgs {
            fireblocks_endpoint,
            fireblocks_api_key,
            fireblocks_secret_path,
            ..
        } = args;

        let http = HttpClient::new();

        let encoding_key = Self::build_encoding_key(fireblocks_secret_path)?;

        let base_url =
            Url::parse(&fireblocks_endpoint).context("failed to parse fireblocks endpoint")?;

        Ok(Self {
            http,
            request_signer: RequestSigner::new(encoding_key, fireblocks_api_key.clone()),
            base_url,
            api_key: fireblocks_api_key,
        })
    }

    /// Builds an encoding key from the provided secret key file path.
    ///
    /// # Arguments
    ///
    /// * `secret_path` - Path to the secret key file.
    ///
    /// # Errors
    ///
    /// This function can fail if:
    ///
    /// * Failed to read the secret key file.
    /// * Failed to create an encoding key from the secret key.
    fn build_encoding_key(secret_path: String) -> Result<EncodingKey> {
        let rsa = std::fs::read(secret_path).context("failed to read secret key")?;

        EncodingKey::from_rsa_pem(&rsa).context("failed to create encoding key")
    }

    /// Authenticates the request by signing it and adding necessary headers.
    ///
    /// # Arguments
    ///
    /// * `req` - Request builder for the HTTP request.
    /// * `endpoint` - API endpoint path.
    /// * `body` - Request body for serialization.
    ///
    /// # Errors
    ///
    /// This function can fail if:
    ///
    /// * Failed to sign the request.
    ///
    /// # Returns
    ///
    /// A `RequestBuilder` with added authentication headers.

    fn authenticate(
        &self,
        req: RequestBuilder,
        endpoint: String,
        body: impl Serialize,
    ) -> Result<RequestBuilder> {
        let token = self.request_signer.sign(endpoint, body)?;

        Ok(req.header("X-API-KEY", &self.api_key).bearer_auth(token))
    }

    #[must_use]
    pub fn read(&self) -> ReadRequestBuilder {
        ReadRequestBuilder(self.clone())
    }

    #[must_use]
    pub fn create(&self) -> CreateRequestBuilder {
        CreateRequestBuilder(self.clone())
    }

    /// Waits for a transaction to reach the "COMPLETED" status by periodically checking the transaction details.
    ///
    /// # Arguments
    ///
    /// * `id` - Transaction ID.
    ///
    /// # Errors
    ///
    /// This function can fail if:
    ///
    /// * The GET request fails.
    /// * Failed to deserialize the transaction details.
    /// * The transaction status is not one of the expected states.
    ///
    /// # Returns
    ///
    /// Transaction details when the transaction is completed.
    pub async fn wait_on_transaction_completion(&self, id: String) -> Result<TransactionDetails> {
        let mut interval = time::interval(time::Duration::from_millis(250));

        loop {
            let tx_details = self.read().transaction(id.clone()).await?;
            let status = tx_details.clone().status;

            match status {
                TransactionStatus::SUBMITTED
                | TransactionStatus::QUEUED
                | TransactionStatus::BROADCASTING
                | TransactionStatus::CONFIRMING
                | TransactionStatus::PENDING_SIGNATURE => {
                    interval.tick().await;

                    continue;
                },
                TransactionStatus::COMPLETED => {
                    break Ok(tx_details);
                },
                _ => return Err(Error::Transaction(status).into()),
            }
        }
    }
}

#[derive(Clone)]
pub struct ReadRequestBuilder(Client);

impl ReadRequestBuilder {
    /// Sends a GET request to the specified path and deserializes the response body.
    ///
    /// # Arguments
    ///
    /// * `path` - API endpoint path.
    ///
    /// # Errors
    ///
    /// This function can fail if:
    ///
    /// * The URL parsing fails.
    /// * The HTTP request fails.
    /// * Failed to deserialize the response body.
    ///
    /// # Returns
    ///
    /// Deserialized response body of type `T`.
    pub async fn send<T: serde::de::DeserializeOwned>(
        &self,
        path: &str,
        body: impl Serialize,
    ) -> Result<T> {
        let client = &self.0;

        let url = client.base_url.join(path)?;

        let mut req = client.http.get(url);

        req = client.authenticate(req, path.to_owned(), body)?;

        let response = req.send().await?.text().await?;

        Ok(serde_json::from_str(&response)?)
    }

    /// Retrieves a paged response of vault accounts based on the provided filters.
    ///
    /// # Arguments
    ///
    /// * `filters` - Query filters for vault accounts.
    ///
    /// # Errors
    ///
    /// This function can fail if:
    ///
    /// * The POST request fails.
    /// * Failed to serialize the query filters.
    /// * Failed to deserialize the paged response.
    ///
    /// # Returns
    ///
    /// Paged response of vault accounts.
    pub async fn vaults(&self, filters: QueryVaultAccounts) -> Result<VaultAccountsPagedResponse> {
        let endpoint = "/v1/vault/accounts_paged";
        self.send(endpoint, filters).await
    }
    /// Retrieves the details of a specific transaction based on the transaction ID.
    ///
    /// # Arguments
    ///
    /// * `txid` - Transaction ID.
    ///
    /// # Errors
    ///
    /// This function can fail if:
    ///
    /// * The GET request fails.
    /// * Failed to deserialize the transaction details.
    ///
    /// # Returns
    ///
    /// Transaction details.

    pub async fn transaction(&self, txid: String) -> Result<TransactionDetails> {
        let endpoint = format!("/v1/transactions/{txid}");

        self.send(&endpoint, ()).await
    }

    /// Retrieves the details of a specific vault account based on the vault ID.
    ///
    /// # Arguments
    ///
    /// * `vault_id` - Vault account ID.
    ///
    /// # Errors
    ///
    /// This function can fail if:
    ///
    /// * The GET request fails.
    /// * Failed to deserialize the vault account details.
    ///
    /// # Returns
    ///
    /// Vault account details.
    pub async fn vault(&self, vault_id: String) -> Result<VaultAccount> {
        let endpoint = format!("/v1/vault/accounts/{vault_id}");
        self.send(&endpoint, ()).await
    }

    /// Retrieves a list of vault assets.
    ///
    /// # Errors
    ///
    /// This function can fail if:
    ///
    /// * The GET request fails.
    /// * Failed to deserialize the vault assets.
    ///
    /// # Returns
    ///
    /// List of vault assets.
    pub async fn vault_assets(&self) -> Result<Vec<VaultAsset>> {
        let endpoint = "/v1/vault/assets".to_string();
        self.send(&endpoint, ()).await
    }

    /// Retrieves a list of all transactions.
    ///
    /// # Errors
    ///
    /// This function can fail if:
    ///
    /// * The GET request fails.
    /// * Failed to deserialize the transactions.
    ///
    /// # Returns
    ///
    /// List of transactions.
    pub async fn transactions(&self) -> Result<Vec<TransactionDetails>> {
        let endpoint = "/v1/transactions".to_string();
        self.send(&endpoint, ()).await
    }
}

#[derive(Clone)]
pub struct CreateRequestBuilder(Client);

impl CreateRequestBuilder {
    /// Sends a POST request to the specified path with the provided body and deserializes the response body.
    ///
    /// # Arguments
    ///
    /// * `path` - API endpoint path.
    /// * `body` - Request body for serialization.
    ///
    /// # Errors
    ///
    /// This function can fail if:
    ///
    /// * The URL parsing fails.
    /// * The HTTP request fails.
    /// * Failed to serialize the request body.
    /// * Failed to deserialize the response body.
    ///
    /// # Returns
    ///
    /// Deserialized response body of type `T`.
    pub async fn send<T: serde::de::DeserializeOwned>(
        &self,
        path: &str,
        body: impl Serialize,
    ) -> Result<T> {
        let client = &self.0;
        let url = client.base_url.join(path)?;
        let mut req = client.http.post(url).json(&body);

        req = client.authenticate(req, path.to_owned(), body)?;

        let response = req.send().await?.text().await?;

        Ok(serde_json::from_str(&response)?)
    }

    /// Creates a new vault account with the provided details.
    ///
    /// # Arguments
    ///
    /// * `body` - Request body for creating a vault account.
    ///
    /// # Errors
    ///
    /// This function can fail if:
    ///
    /// * The POST request fails.
    /// * Failed to serialize the request body.
    /// * Failed to deserialize the created vault account details.
    ///
    /// # Returns
    ///
    /// Created vault account details.
    pub async fn vault(&self, body: CreateVault) -> Result<VaultAccount> {
        let endpoint = "/v1/vault/accounts".to_string();
        self.send(&endpoint, body).await
    }

    /// Creates a new wallet within a vault account for the specified asset.
    ///
    /// # Arguments
    ///
    /// * `vault_account_id` - ID of the vault account.
    /// * `asset_id` - ID of the asset.
    /// * `body` - Request body for creating a vault wallet.
    ///
    /// # Errors
    ///
    /// This function can fail if:
    ///
    /// * The POST request fails.
    /// * Failed to serialize the request body.
    /// * Failed to deserialize the created vault wallet details.
    ///
    /// # Returns
    ///
    /// Created vault wallet details.
    pub async fn wallet(
        &self,
        vault_account_id: String,
        asset_id: String,
        body: CreateVaultWallet,
    ) -> Result<CreateVaultAssetResponse> {
        let endpoint = format!("/v1/vault/accounts/{vault_account_id}/{asset_id}");
        self.send(&endpoint, body).await
    }

    /// Creates a new transaction with the provided details.
    ///
    /// # Arguments
    ///
    /// * `tx` - Request body for creating a transaction.
    ///
    /// # Errors
    ///
    /// This function can fail if:
    ///
    /// * The POST request fails.
    /// * Failed to serialize the request body.
    /// * Failed to deserialize the created transaction details.
    ///
    /// # Returns
    ///
    /// Created transaction details.
    pub async fn transaction(&self, tx: CreateTransaction) -> Result<CreateTransactionResponse> {
        let endpoint = "/v1/transactions".to_string();
        self.send(&endpoint, tx).await
    }

    /// Creates and signs a raw message transaction with the provided asset ID, vault ID, message content, and note.
    ///
    /// # Arguments
    ///
    /// * `asset_id` - ID of the asset.
    /// * `vault_id` - ID of the vault.
    /// * `message` - Message content as a byte array.
    /// * `note` - Note for the transaction.
    ///
    /// # Errors
    ///
    /// This function can fail if:
    ///
    /// * The POST request fails.
    /// * Failed to serialize the request body.
    /// * Failed to deserialize the created transaction details.
    ///
    /// # Returns
    ///
    /// Created transaction details.
    pub async fn raw_transaction(
        &self,
        asset_id: String,
        vault_id: String,
        message: Vec<u8>,
        note: String,
    ) -> Result<CreateTransactionResponse> {
        let tx = CreateTransaction {
            asset_id,
            operation: TransactionOperation::RAW,
            source: TransferPeerPath {
                peer_type: "VAULT_ACCOUNT".to_string(),
                id: vault_id,
            },
            destination: None,
            destinations: None,
            treat_as_gross_amount: None,
            customer_ref_id: None,
            amount: "0".to_string(),
            extra_parameters: Some(ExtraParameters::RawMessageData(RawMessageData {
                messages: vec![UnsignedMessage {
                    content: hex::encode(&message),
                }],
            })),
            note: Some(note),
        };

        let endpoint = "/v1/transactions".to_string();
        self.send(&endpoint, tx).await
    }

    /// Creates a new wallet within a vault account for the specified asset.
    ///
    /// # Arguments
    ///
    /// * `vault_id` - ID of the vault account.
    /// * `asset_id` - ID of the asset.
    /// * `body` - Request body for creating a vault wallet.
    ///
    /// # Errors
    ///
    /// This function can fail if:
    ///
    /// * The POST request fails.
    /// * Failed to serialize the request body.
    /// * Failed to deserialize the created vault wallet details.
    ///
    /// # Returns
    ///
    /// Created vault wallet details.
    pub async fn create_wallet(
        &self,
        vault_id: String,
        asset_id: String,
        body: CreateVaultWallet,
    ) -> Result<CreateVaultAssetResponse> {
        let endpoint = format!("/v1/vault/accounts/{vault_id}/{asset_id}");
        self.send(&endpoint, body).await
    }
}
