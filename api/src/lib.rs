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
use dataloaders::{CustomerTreasuryLoader, ProjectTreasuryLoader, WalletsLoader};
use db::Connection;
use fireblocks::Client as FireblocksClient;
use hub_core::{
    anyhow::{Error, Result},
    clap,
    consumer::RecvError,
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
    include!(concat!(env!("OUT_DIR"), "/treasury.proto.rs"));
}

#[derive(Debug)]
pub enum Services {
    Organizations(proto::OrganizationEventKey, proto::OrganizationEvents),
    Customers(proto::CustomerEventKey, proto::CustomerEvents),
    Nfts(proto::NftEventKey, proto::NftEvents),
}

impl hub_core::consumer::MessageGroup for Services {
    const REQUESTED_TOPICS: &'static [&'static str] = &["hub-orgs", "hub-customers", "hub-nfts"];

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
            "hub-nfts" => {
                let key = proto::NftEventKey::decode(key)?;
                let val = proto::NftEvents::decode(val)?;

                Ok(Services::Nfts(key, val))
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

#[derive(Debug, clap::Args)]
#[command(version, author, about)]
pub struct Args {
    #[arg(short, long, env, default_value_t = 3007)]
    pub port: u16,

    #[arg(short, long, env)]
    pub solana_endpoint: String,

    #[arg(short, long, env, value_delimiter = ',')]
    pub fireblocks_supported_asset_ids: Vec<String>,

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
}

impl AppState {
    #[must_use]
    pub fn new(
        schema: AppSchema,
        connection: Connection,
        fireblocks: FireblocksClient,
        producer: Producer<TreasuryEvents>,
    ) -> Self {
        Self {
            schema,
            connection,
            fireblocks,
            producer,
        }
    }
}

pub struct AppContext {
    pub db: Connection,
    pub user_id: UserID,
    pub customer_treasury_loader: DataLoader<CustomerTreasuryLoader>,
    pub project_treasury_loader: DataLoader<ProjectTreasuryLoader>,
    pub wallets_loader: DataLoader<WalletsLoader>,
}

impl AppContext {
    #[must_use]
    pub fn new(db: Connection, user_id: UserID) -> Self {
        let customer_treasury_loader =
            DataLoader::new(CustomerTreasuryLoader::new(db.clone()), tokio::spawn);
        let project_treasury_loader =
            DataLoader::new(ProjectTreasuryLoader::new(db.clone()), tokio::spawn);
        let wallets_loader = DataLoader::new(WalletsLoader::new(db.clone()), tokio::spawn);

        Self {
            db,
            user_id,
            customer_treasury_loader,
            project_treasury_loader,
            wallets_loader,
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
