use std::{fmt, sync::Arc};

use async_graphql::{self, Context, Enum, Object, Result};
use fireblocks::{
    client::FireblocksClient,
    objects::{
        transaction::{
            CreateTransaction, CreateTransactionResponse, ExtraParameters, RawMessageData,
            TransactionOperation, TransferPeerPath, UnsignedMessage,
        },
        vault::{QueryVaultAccounts, VaultAccount, VaultAccountsPagedResponse, VaultAsset},
    },
};
use hex::FromHex;
use log::debug;
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};
use solana_client::rpc_client::RpcClient;
use solana_program::message::Message;
use solana_sdk::{signature::Signature, transaction::Transaction};
use uuid::Uuid;

use crate::models::project_treasuries;

#[derive(Enum, Debug, Copy, Clone, Eq, PartialEq)]
pub enum OrderBy {
    #[graphql(name = "Ascending")]
    Asc,
    #[graphql(name = "Descending")]
    Desc,
}

impl fmt::Display for OrderBy {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

impl Default for OrderBy {
    fn default() -> Self {
        Self::Desc
    }
}

#[derive(Default)]
pub struct Query;

#[Object(name = "VaultQuery")]
impl Query {
    /// Res
    ///
    /// # Errors
    /// This function fails if ...
    async fn vaults(
        &self,
        ctx: &Context<'_>,
        asset_id: Option<u64>,
        limit: Option<u64>,
        order_by: Option<OrderBy>,
    ) -> Result<VaultAccountsPagedResponse> {
        let fireblocks = &**ctx.data::<Arc<FireblocksClient>>()?;

        let vaults = fireblocks
            .get_vaults(QueryVaultAccounts {
                name_prefix: None,
                name_suffix: None,
                min_amount_threshold: None,
                asset_id,
                order_by: order_by.unwrap_or_default().to_string(),
                limit: limit.unwrap_or(500),
                before: None,
                after: None,
                max_bip44_address_index_used: 966,
                max_bip44_change_address_index_used: 20,
            })
            .await?;

        Ok(vaults)
    }

    async fn vault(&self, ctx: &Context<'_>, vault_id: String) -> Result<VaultAccount> {
        let fireblocks = &**ctx.data::<Arc<FireblocksClient>>()?;

        let vault = fireblocks.get_vault(vault_id).await?;

        Ok(vault)
    }

    async fn vault_assets(&self, ctx: &Context<'_>) -> Result<Vec<VaultAsset>> {
        let fireblocks = &**ctx.data::<Arc<FireblocksClient>>()?;

        let vault = fireblocks.vault_assets().await?;

        Ok(vault)
    }

    async fn project_treasuries(
        &self,
        ctx: &Context<'_>,
        project_id: Uuid,
    ) -> Result<Vec<project_treasuries::Model>> {
        let db = ctx.data::<DatabaseConnection>()?;
        let t = project_treasuries::Entity::find()
            .filter(project_treasuries::Column::ProjectId.eq(project_id))
            .all(db)
            .await?;

        Ok(t)
    }

    async fn tx(&self, ctx: &Context<'_>) -> Result<CreateTransactionResponse> {
        let fireblocks = &**ctx.data::<Arc<FireblocksClient>>()?;

        let from = "AWN3jYp1u5DE9DpCiVdHdd9rc4NL5XVYLGmzKJXwFhaP".parse()?;
        let to: solana_program::pubkey::Pubkey =
            "Et8ALhvkqfLekymLYpnuSqKxZkbqHQcQFEeW9LS8MVGu".parse()?;
        let ins = solana_program::system_instruction::transfer(&from, &to, 1000000000);

        let url = "https://api.devnet.solana.com".to_string();

        let rpc_client = RpcClient::new(url);
        let blockhash = rpc_client.get_latest_blockhash()?;

        let message = Message::new_with_blockhash(&[ins], Some(&from), &blockhash);
        let hashed_data = hex::encode(message.serialize());

        let tx = CreateTransaction {
            asset_id: "SOL_TEST".to_string(),
            operation: TransactionOperation::RAW,
            source: TransferPeerPath {
                peer_type: "VAULT_ACCOUNT".to_string(),
                id: "6".to_string(),
            },
            destination: None,
            destinations: None,
            treat_as_gross_amount: None,
            customer_ref_id: None,
            amount: "0".to_string(),
            extra_parameters: Some(ExtraParameters::RawMessageData(RawMessageData {
                messages: vec![UnsignedMessage {
                    content: hashed_data,
                }],
            })),
            note: Some("solana transfer instruction ".to_string()),
        };

        let transaction = fireblocks.create_transaction(tx).await?;

        let mut tx_details = fireblocks.transaction(transaction.id.clone()).await?;

        while tx_details.signed_messages.len() == 0 {
            tx_details = fireblocks.transaction(transaction.id.clone()).await?;
        }

        debug!("{:?}", tx_details.signed_messages);

        let full_sig = tx_details.clone().signed_messages[0]
            .clone()
            .signature
            .full_sig;

        // SIGNATURE LEN = 64 bytes

        let signature_decoded = <[u8; 64]>::from_hex(full_sig)?;

        let signature = Signature::new(&signature_decoded);

        debug!("signature {:?}", signature);

        let signed_transaction = Transaction {
            signatures: vec![signature],
            message,
        };

        let res = rpc_client.send_transaction(&signed_transaction);

        debug!("rpc transaction sent response {:?}", res);

        Ok(transaction)
    }
}
