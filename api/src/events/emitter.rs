use hub_core::{prelude::*, producer::Producer};

use crate::proto::{
    treasury_events::{signed_transaction_event, Event, SignedTransaction, SignedTransactionEvent},
    SolanaNftEventKey, TreasuryEventKey, TreasuryEvents,
};

#[derive(Clone)]
pub struct Emitter {
    producer: Producer<TreasuryEvents>,
}

impl Emitter {
    pub fn new(producer: Producer<TreasuryEvents>) -> Self {
        Self { producer }
    }

    pub async fn create_drop_signed(
        &self,
        key: TreasuryEventKey,
        signed_transaction: SignedTransaction,
    ) -> Result<()> {
        let event = TreasuryEvents {
            event: Some(Event::MessageSigned(SignedTransactionEvent {
                event: Some(signed_transaction_event::Event::CreateDrop(
                    signed_transaction,
                )),
            })),
        };

        self.producer.send(Some(&event), Some(&key)).await?;

        Ok(())
    }

    pub async fn update_drop_signed(
        &self,
        key: TreasuryEventKey,
        signed_transaction: SignedTransaction,
    ) -> Result<()> {
        let event = TreasuryEvents {
            event: Some(Event::MessageSigned(SignedTransactionEvent {
                event: Some(signed_transaction_event::Event::UpdateDrop(
                    signed_transaction,
                )),
            })),
        };

        self.producer.send(Some(&event), Some(&key)).await?;

        Ok(())
    }

    pub async fn mint_drop_signed(
        &self,
        key: TreasuryEventKey,
        signed_transaction: SignedTransaction,
    ) -> Result<()> {
        let event = TreasuryEvents {
            event: Some(Event::MessageSigned(SignedTransactionEvent {
                event: Some(signed_transaction_event::Event::MintDrop(
                    signed_transaction,
                )),
            })),
        };

        self.producer.send(Some(&event), Some(&key)).await?;

        Ok(())
    }

    pub async fn transfer_asset_signed(
        &self,
        key: TreasuryEventKey,
        signed_transaction: SignedTransaction,
    ) -> Result<()> {
        let event = TreasuryEvents {
            event: Some(Event::MessageSigned(SignedTransactionEvent {
                event: Some(signed_transaction_event::Event::TransferAsset(
                    signed_transaction,
                )),
            })),
        };

        self.producer.send(Some(&event), Some(&key)).await?;

        Ok(())
    }

    pub async fn retry_create_drop_signed(
        &self,
        key: TreasuryEventKey,
        signed_transaction: SignedTransaction,
    ) -> Result<()> {
        let event = TreasuryEvents {
            event: Some(Event::MessageSigned(SignedTransactionEvent {
                event: Some(signed_transaction_event::Event::RetryCreateDrop(
                    signed_transaction,
                )),
            })),
        };

        self.producer.send(Some(&event), Some(&key)).await?;

        Ok(())
    }

    pub async fn retry_mint_drop_signed(
        &self,
        key: TreasuryEventKey,
        signed_transaction: SignedTransaction,
    ) -> Result<()> {
        let event = TreasuryEvents {
            event: Some(Event::MessageSigned(SignedTransactionEvent {
                event: Some(signed_transaction_event::Event::RetryMintDrop(
                    signed_transaction,
                )),
            })),
        };

        self.producer.send(Some(&event), Some(&key)).await?;

        Ok(())
    }
}

impl From<SolanaNftEventKey> for TreasuryEventKey {
    fn from(key: SolanaNftEventKey) -> Self {
        Self {
            id: key.project_id,
            user_id: key.user_id,
        }
    }
}
