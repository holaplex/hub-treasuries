use fireblocks::Fireblocks;
use hex::FromHex;
use hub_core::prelude::*;
use sea_orm::{
    prelude::*, ColumnTrait, DatabaseConnection, EntityTrait, JoinType, QueryFilter, QuerySelect,
};

use crate::{
    db::Connection,
    entities::{sea_orm_active_enums::TxType, treasuries, wallets},
    proto::{
        treasury_events::{
            signed_transaction::Transaction, SignedTransaction, SolanaSignedTransaction,
        },
        SolanaNftEventKey, SolanaTransaction,
    },
};

#[async_trait]
pub trait TransactionSigner<K, P> {
    async fn create_drop(&self, key: K, payload: P) -> Result<SignedTransaction>;
    async fn mint_drop(&self, key: K, payload: P) -> Result<SignedTransaction>;
    async fn update_drop(&self, key: K, payload: P) -> Result<SignedTransaction>;
    async fn transfer_asset(&self, key: K, payload: P) -> Result<SignedTransaction>;
    async fn retry_create_drop(&self, key: K, payload: P) -> Result<SignedTransaction>;
    async fn retry_mint_drop(&self, key: K, payload: P) -> Result<SignedTransaction>;
}

pub struct Signer {
    fireblocks: Fireblocks,
    vault_id: String,
    db: Connection,
}

impl Signer {
    pub fn new(fireblocks: Fireblocks, db: Connection, vault_id: String) -> Self {
        Self {
            fireblocks,
            vault_id,
            db,
        }
    }

    async fn find_wallet_by_vault(
        conn: &DatabaseConnection,
        vault_id: String,
    ) -> Result<wallets::Model> {
        wallets::Entity::find()
            .join(JoinType::InnerJoin, wallets::Relation::Treasuries.def())
            .filter(treasuries::Column::VaultId.eq(vault_id))
            .filter(wallets::Column::AssetId.is_in(vec![
                wallets::AssetType::Solana,
                wallets::AssetType::SolanaTest,
            ]))
            .one(conn)
            .await?
            .context("wallet not found")
    }

    async fn sign_message(
        &self,
        note: String,
        serialized_message: Vec<u8>,
    ) -> Result<[u8; 64]> {
        let conn = self.db.get();

        let wallet = Self::find_wallet_by_vault(conn, self.vault_id.clone()).await?;

        let transaction = self
            .fireblocks
            .client()
            .create()
            .raw_transaction(
                wallet.asset_id.into(),
                self.vault_id.to_string(),
                serialized_message,
                note,
            )
            .await?;

        let transaction_details = self
            .fireblocks
            .client()
            .wait_on_transaction_completion(transaction.id)
            .await?;

        let full_sig = transaction_details
            .signed_messages
            .get(0)
            .context("failed to get signed message response")?
            .clone()
            .signature
            .full_sig;

        let signature_decoded = <[u8; 64]>::from_hex(full_sig)?;

        Ok(signature_decoded)
    }

    pub async fn sign_transaction(
        &self,
        tx_type: TxType,
        key: SolanaNftEventKey,
        mut payload: SolanaTransaction,
    ) -> Result<SignedTransaction> {
        let note = format!(
            "{:?} by {:?} for project {:?}",
            tx_type, key.user_id, key.project_id
        );

        let signature = self
            .sign_message(note, payload.serialized_message.clone())
            .await?;

        payload
            .signed_message_signatures
            .push(bs58::encode(signature).into_string());

        Ok(SignedTransaction {
            transaction: Some(Transaction::Solana(SolanaSignedTransaction {
                serialized_message: payload.serialized_message,
                signed_message_signatures: payload.signed_message_signatures,
            })),
        })
    }
}
