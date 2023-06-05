use fireblocks::{
    objects::transaction::{
        CreateTransaction, ExtraParameters, RawMessageData, TransactionOperation, TransferPeerPath,
        UnsignedMessage,
    },
    Fireblocks,
};
use hex::FromHex;
use hub_core::{prelude::*, producer::Producer};
use sea_orm::{prelude::*, DatabaseConnection, JoinType, QuerySelect};

use crate::{
    db::Connection,
    entities::{sea_orm_active_enums::TxType, treasuries, wallets},
    proto::{
        treasury_events::{Event, SignedTransaction, TransactionStatus},
        SolanaNftEventKey, SolanaTransaction, TreasuryEventKey, TreasuryEvents,
    },
};

#[async_trait]
pub trait SolanaTransactionSigner {
    async fn create_drop(
        &self,
        key: SolanaNftEventKey,
        payload: SolanaTransaction,
    ) -> Result<SignedTransaction>;
    async fn mint_drop(
        &self,
        key: SolanaNftEventKey,
        project: SolanaTransaction,
    ) -> Result<SignedTransaction>;
    async fn update_drop(
        &self,
        key: SolanaNftEventKey,
        project: SolanaTransaction,
    ) -> Result<SignedTransaction>;
    async fn transfer_asset(
        &self,
        key: SolanaNftEventKey,
        project: SolanaTransaction,
    ) -> Result<SignedTransaction>;
    async fn retry_create_drop(
        &self,
        key: SolanaNftEventKey,
        project: SolanaTransaction,
    ) -> Result<SignedTransaction>;
    async fn retry_mint_drop(
        &self,
        key: SolanaNftEventKey,
        project: SolanaTransaction,
    ) -> Result<SignedTransaction>;
}

pub struct Solana {
    fireblocks: Fireblocks,
    db: Connection,
    producer: Producer<TreasuryEvents>,
}

impl Solana {
    pub fn new(fireblocks: Fireblocks, db: Connection, producer: Producer<TreasuryEvents>) -> Self {
        Self {
            fireblocks,
            db,
            producer,
        }
    }

    pub fn signer(&self, vault_id: String) -> Signer {
        Signer::new(self.fireblocks.clone(), self.db.clone(), vault_id)
    }

    pub fn event(&self) -> Emitter {
        Emitter::new(self.producer.clone())
    }
}

pub struct Emitter {
    producer: Producer<TreasuryEvents>,
}

impl Emitter {
    pub fn new(producer: Producer<TreasuryEvents>) -> Self {
        Self { producer }
    }

    pub async fn create_drop_signed(
        &self,
        key: SolanaNftEventKey,
        signed_transaction: SignedTransaction,
    ) -> Result<()> {
        let event = TreasuryEvents {
            event: Some(Event::SolanaCreateDropSigned(signed_transaction)),
        };

        let key = TreasuryEventKey {
            id: key.id,
            user_id: key.user_id,
        };

        self.producer.send(Some(&event), Some(&key)).await?;

        Ok(())
    }

    pub async fn update_drop_signed(
        &self,
        key: SolanaNftEventKey,
        signed_transaction: SignedTransaction,
    ) -> Result<()> {
        let event = TreasuryEvents {
            event: Some(Event::SolanaUpdateDropSigned(signed_transaction)),
        };

        let key = TreasuryEventKey {
            id: key.id,
            user_id: key.user_id,
        };

        self.producer.send(Some(&event), Some(&key)).await?;

        Ok(())
    }

    pub async fn mint_drop_signed(
        &self,
        key: SolanaNftEventKey,
        signed_transaction: SignedTransaction,
    ) -> Result<()> {
        let event = TreasuryEvents {
            event: Some(Event::SolanaMintDropSigned(signed_transaction)),
        };

        let key = TreasuryEventKey {
            id: key.id,
            user_id: key.user_id,
        };

        self.producer.send(Some(&event), Some(&key)).await?;

        Ok(())
    }

    pub async fn transfer_asset_signed(
        &self,
        key: SolanaNftEventKey,
        signed_transaction: SignedTransaction,
    ) -> Result<()> {
        let event = TreasuryEvents {
            event: Some(Event::SolanaTransferAssetSigned(signed_transaction)),
        };

        let key = TreasuryEventKey {
            id: key.id,
            user_id: key.user_id,
        };

        self.producer.send(Some(&event), Some(&key)).await?;

        Ok(())
    }

    pub async fn retry_create_drop_signed(
        &self,
        key: SolanaNftEventKey,
        signed_transaction: SignedTransaction,
    ) -> Result<()> {
        let event = TreasuryEvents {
            event: Some(Event::SolanaRetryCreateDropSigned(signed_transaction)),
        };

        let key = TreasuryEventKey {
            id: key.id,
            user_id: key.user_id,
        };

        self.producer.send(Some(&event), Some(&key)).await?;

        Ok(())
    }

    pub async fn retry_mint_drop_signed(
        &self,
        key: SolanaNftEventKey,
        signed_transaction: SignedTransaction,
    ) -> Result<()> {
        let event = TreasuryEvents {
            event: Some(Event::SolanaRetryMintDropSigned(signed_transaction)),
        };

        let key = TreasuryEventKey {
            id: key.id,
            user_id: key.user_id,
        };

        self.producer.send(Some(&event), Some(&key)).await?;

        Ok(())
    }
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
            db,
            vault_id,
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
        note: Option<String>,
        serialized_message: Vec<u8>,
    ) -> Result<[u8; 64]> {
        let conn = self.db.get();

        let wallet = Self::find_wallet_by_vault(conn, self.vault_id.clone()).await?;

        let tx = CreateTransaction {
            asset_id: wallet.asset_id.into(),
            operation: TransactionOperation::RAW,
            source: TransferPeerPath {
                peer_type: "VAULT_ACCOUNT".to_string(),
                id: self.vault_id.to_string(),
            },
            destination: None,
            destinations: None,
            treat_as_gross_amount: None,
            customer_ref_id: None,
            amount: "0".to_string(),
            extra_parameters: Some(ExtraParameters::RawMessageData(RawMessageData {
                messages: vec![UnsignedMessage {
                    content: hex::encode(&serialized_message),
                }],
            })),
            note,
        };

        let transaction = self.fireblocks.client().create_transaction(tx).await?;

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
}

#[async_trait]
impl SolanaTransactionSigner for Signer {
    async fn create_drop(
        &self,
        key: SolanaNftEventKey,
        payload: SolanaTransaction,
    ) -> Result<SignedTransaction> {
        let note = Some(format!(
            "{:?} by {:?} for project {:?}",
            TxType::CreateDrop,
            key.user_id,
            key.project_id
        ));

        let signed_message = self
            .sign_message(note, payload.serialized_message.clone())
            .await?;

        let mut signatures = payload.signed_message_signatures.clone();
        signatures.push(hex::encode(signed_message));

        Ok(SignedTransaction {
            serialized_message: payload.serialized_message,
            signed_message_signatures: signatures,
            project_id: key.project_id,
        })
    }
    async fn update_drop(
        &self,
        key: SolanaNftEventKey,
        payload: SolanaTransaction,
    ) -> Result<SignedTransaction> {
        let note = Some(format!(
            "{:?} by {:?} for project {:?}",
            TxType::UpdateMetadata,
            key.user_id,
            key.project_id
        ));

        let signed_message = self
            .sign_message(note, payload.serialized_message.clone())
            .await?;

        let mut signatures = payload.signed_message_signatures.clone();
        signatures.push(hex::encode(signed_message));

        Ok(SignedTransaction {
            serialized_message: payload.serialized_message,
            signed_message_signatures: signatures,
            project_id: key.project_id,
        })
    }

    async fn mint_drop(
        &self,
        key: SolanaNftEventKey,
        payload: SolanaTransaction,
    ) -> Result<SignedTransaction> {
        let note = Some(format!(
            "{:?} by {:?} for project {:?}",
            TxType::MintEdition,
            key.user_id,
            key.project_id
        ));

        let signed_message = self
            .sign_message(note, payload.serialized_message.clone())
            .await?;

        let mut signatures = payload.signed_message_signatures.clone();
        signatures.push(hex::encode(signed_message));

        Ok(SignedTransaction {
            serialized_message: payload.serialized_message,
            signed_message_signatures: signatures,
            project_id: key.project_id,
        })
    }

    async fn transfer_asset(
        &self,
        key: SolanaNftEventKey,
        payload: SolanaTransaction,
    ) -> Result<SignedTransaction> {
        let note = Some(format!(
            "{:?} by {:?} for project {:?}",
            TxType::TransferMint,
            key.user_id,
            key.project_id
        ));

        let signed_message = self
            .sign_message(note, payload.serialized_message.clone())
            .await?;

        let mut signatures = payload.signed_message_signatures.clone();
        signatures.push(hex::encode(signed_message));

        Ok(SignedTransaction {
            serialized_message: payload.serialized_message,
            signed_message_signatures: signatures,
            project_id: key.project_id,
        })
    }

    async fn retry_create_drop(
        &self,
        key: SolanaNftEventKey,
        payload: SolanaTransaction,
    ) -> Result<SignedTransaction> {
        let note = Some(format!(
            "{:?} by {:?} for project {:?}",
            TxType::CreateDrop,
            key.user_id,
            key.project_id
        ));

        let signed_message = self
            .sign_message(note, payload.serialized_message.clone())
            .await?;

        let mut signatures = payload.signed_message_signatures.clone();
        signatures.push(hex::encode(signed_message));

        Ok(SignedTransaction {
            serialized_message: payload.serialized_message,
            signed_message_signatures: signatures,
            project_id: key.project_id,
        })
    }
    async fn retry_mint_drop(
        &self,
        key: SolanaNftEventKey,
        payload: SolanaTransaction,
    ) -> Result<SignedTransaction> {
        let note = Some(format!(
            "{:?} by {:?} for project {:?}",
            TxType::MintEdition,
            key.user_id,
            key.project_id
        ));

        let signed_message = self
            .sign_message(note, payload.serialized_message.clone())
            .await?;

        let mut signatures = payload.signed_message_signatures.clone();
        signatures.push(hex::encode(signed_message));

        Ok(SignedTransaction {
            serialized_message: payload.serialized_message,
            signed_message_signatures: signatures,
            project_id: key.project_id,
        })
    }
}
