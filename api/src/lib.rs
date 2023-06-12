#![deny(clippy::disallowed_methods, clippy::suspicious, clippy::style)]
#![warn(clippy::pedantic, clippy::cargo)]
#![allow(clippy::module_name_repetitions)]

pub mod dataloaders;
pub mod db;
#[allow(clippy::pedantic)]
pub mod entities;
pub mod events;
pub mod handlers;
pub mod mutations;
pub mod objects;
pub mod queries;

use async_graphql::{
    dataloader::DataLoader,
    extensions::{ApolloTracing, Logger},
    EmptySubscription, Schema,
};
use dataloaders::{
    CustomerTreasuryLoader, CustomerTreasuryWalletLoader, CustomerWalletAddressesLoader,
    ProjectTreasuryLoader, TreasuryLoader, TreasuryWalletsLoader, WalletLoader,
};
use db::Connection;
use fireblocks::Client as FireblocksClient;
use hub_core::{
    anyhow::{Error, Result},
    clap,
    consumer::RecvError,
    credits::CreditsClient,
    prelude::*,
    producer::Producer,
    tokio,
    uuid::Uuid,
};
use mutations::Mutation;
use poem::{async_trait, FromRequest, Request, RequestBody};
use proto::{TreasuryEventKey, TreasuryEvents};
use queries::Query;

pub type AppSchema = Schema<Query, Mutation, EmptySubscription>;

#[allow(clippy::pedantic)]
pub mod proto {
    include!(concat!(env!("OUT_DIR"), "/organization.proto.rs"));
    include!(concat!(env!("OUT_DIR"), "/customer.proto.rs"));
    include!(concat!(env!("OUT_DIR"), "/nfts.proto.rs"));
    include!(concat!(env!("OUT_DIR"), "/solana_nfts.proto.rs"));
    include!(concat!(env!("OUT_DIR"), "/polygon_nfts.proto.rs"));
    include!(concat!(env!("OUT_DIR"), "/treasury.proto.rs"));
}

#[derive(Debug)]
pub enum Services {
    Organizations(proto::OrganizationEventKey, proto::OrganizationEvents),
    Customers(proto::CustomerEventKey, proto::CustomerEvents),
    Solana(proto::SolanaNftEventKey, proto::SolanaNftEvents),
    Polygon(proto::PolygonNftEventKey, proto::PolygonNftEvents),
}

impl hub_core::consumer::MessageGroup for Services {
    const REQUESTED_TOPICS: &'static [&'static str] = &[
        "hub-orgs",
        "hub-customers",
        "hub-nfts-solana",
        "hub-nfts-polygon",
    ];

    fn from_message<M: hub_core::consumer::Message>(msg: &M) -> Result<Self, RecvError> {
        let topic = msg.topic();
        let key = msg.key().ok_or(RecvError::MissingKey)?;
        let val = msg.payload().ok_or(RecvError::MissingPayload)?;
        info!(topic, ?key, ?val);

        match topic {
            "hub-orgs" => {
                let key = proto::OrganizationEventKey::decode(key)?;
                let val = proto::OrganizationEvents::decode(val)?;

                Ok(Services::Organizations(key, val))
            },
            "hub-customers" => {
                let key = proto::CustomerEventKey::decode(key)?;
                let val = proto::CustomerEvents::decode(val)?;

                Ok(Services::Customers(key, val))
            },
            "hub-nfts-solana" => {
                let key = proto::SolanaNftEventKey::decode(key)?;
                let val = proto::SolanaNftEvents::decode(val)?;

                Ok(Services::Solana(key, val))
            },
            "hub-nfts-polygon" => {
                let key = proto::PolygonNftEventKey::decode(key)?;
                let val = proto::PolygonNftEvents::decode(val)?;

                Ok(Services::Polygon(key, val))
            },
            t => Err(RecvError::BadTopic(t.into())),
        }
    }
}

impl hub_core::producer::Message for TreasuryEvents {
    type Key = TreasuryEventKey;
}

#[derive(Debug, Clone, Copy)]
pub struct UserID(Option<Uuid>);

impl TryFrom<&str> for UserID {
    type Error = Error;

    fn try_from(value: &str) -> Result<Self> {
        let id = Uuid::from_str(value)?;

        Ok(Self(Some(id)))
    }
}

#[async_trait]
impl<'a> FromRequest<'a> for UserID {
    async fn from_request(req: &'a Request, _body: &mut RequestBody) -> poem::Result<Self> {
        let id = req
            .headers()
            .get("X-USER-ID")
            .and_then(|value| value.to_str().ok())
            .map_or(Ok(Self(None)), Self::try_from)?;

        Ok(id)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct OrganizationId(Option<Uuid>);

impl TryFrom<&str> for OrganizationId {
    type Error = Error;

    fn try_from(value: &str) -> Result<Self> {
        let id = Uuid::from_str(value)?;

        Ok(Self(Some(id)))
    }
}

#[async_trait]
impl<'a> FromRequest<'a> for OrganizationId {
    async fn from_request(req: &'a Request, _body: &mut RequestBody) -> poem::Result<Self> {
        let id = req
            .headers()
            .get("X-ORGANIZATION-ID")
            .and_then(|value| value.to_str().ok())
            .map_or(Ok(Self(None)), Self::try_from)?;

        Ok(id)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Balance(Option<u64>);

impl TryFrom<&str> for Balance {
    type Error = Error;

    fn try_from(value: &str) -> Result<Self> {
        let balance = value.parse()?;

        Ok(Self(Some(balance)))
    }
}

#[async_trait]
impl<'a> FromRequest<'a> for Balance {
    async fn from_request(req: &'a Request, _body: &mut RequestBody) -> poem::Result<Self> {
        let id = req
            .headers()
            .get("X-CREDIT-BALANCE")
            .and_then(|value| value.to_str().ok())
            .map_or(Ok(Self(None)), Self::try_from)?;

        Ok(id)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, strum::EnumIter, strum::AsRefStr)]
pub enum Actions {
    CreateWallet,
}

impl From<Actions> for hub_core::credits::Action {
    fn from(value: Actions) -> Self {
        match value {
            Actions::CreateWallet => hub_core::credits::Action::CreateWallet,
        }
    }
}

#[derive(Debug, clap::Args)]
#[command(version, author, about)]
pub struct Args {
    #[arg(short, long, env, default_value_t = 3007)]
    pub port: u16,

    #[command(flatten)]
    pub db: db::DbArgs,

    #[command(flatten)]
    pub fireblocks: fireblocks::FbArgs,
}

#[derive(Clone)]
pub struct AppState {
    pub schema: AppSchema,
    pub connection: Connection,
    pub fireblocks: FireblocksClient,
    pub producer: Producer<TreasuryEvents>,
    pub credits: CreditsClient<Actions>,
}

impl AppState {
    #[must_use]
    pub fn new(
        schema: AppSchema,
        connection: Connection,
        fireblocks: FireblocksClient,
        producer: Producer<TreasuryEvents>,
        credits: CreditsClient<Actions>,
    ) -> Self {
        Self {
            schema,
            connection,
            fireblocks,
            producer,
            credits,
        }
    }
}

pub struct AppContext {
    pub db: Connection,
    pub user_id: UserID,
    pub organization_id: OrganizationId,
    pub balance: Balance,
    pub customer_treasury_loader: DataLoader<CustomerTreasuryLoader>,
    pub project_treasury_loader: DataLoader<ProjectTreasuryLoader>,
    pub wallet_loader: DataLoader<WalletLoader>,
    pub treasury_wallets_loader: DataLoader<TreasuryWalletsLoader>,
    pub customer_treasury_wallet_loader: DataLoader<CustomerTreasuryWalletLoader>,
    pub treasury_loader: DataLoader<TreasuryLoader>,
    pub customer_wallet_addresses_loader: DataLoader<CustomerWalletAddressesLoader>,
}

impl AppContext {
    #[must_use]
    pub fn new(
        db: Connection,
        user_id: UserID,
        organization_id: OrganizationId,
        balance: Balance,
    ) -> Self {
        let customer_treasury_loader =
            DataLoader::new(CustomerTreasuryLoader::new(db.clone()), tokio::spawn);
        let project_treasury_loader =
            DataLoader::new(ProjectTreasuryLoader::new(db.clone()), tokio::spawn);
        let wallet_loader = DataLoader::new(WalletLoader::new(db.clone()), tokio::spawn);
        let treasury_wallets_loader =
            DataLoader::new(TreasuryWalletsLoader::new(db.clone()), tokio::spawn);
        let customer_treasury_wallet_loader =
            DataLoader::new(CustomerTreasuryWalletLoader::new(db.clone()), tokio::spawn);
        let treasury_loader = DataLoader::new(TreasuryLoader::new(db.clone()), tokio::spawn);
        let customer_wallet_addresses_loader =
            DataLoader::new(CustomerWalletAddressesLoader::new(db.clone()), tokio::spawn);

        Self {
            db,
            user_id,
            organization_id,
            balance,
            customer_treasury_loader,
            project_treasury_loader,
            wallet_loader,
            treasury_wallets_loader,
            customer_treasury_wallet_loader,
            treasury_loader,
            customer_wallet_addresses_loader,
        }
    }
}

/// Builds the GraphQL Schema, attaching the Database to the context
#[must_use]
pub fn build_schema() -> AppSchema {
    Schema::build(Query::default(), Mutation::default(), EmptySubscription)
        .extension(ApolloTracing)
        .extension(Logger)
        .enable_federation()
        .finish()
}
