#![deny(clippy::disallowed_methods, clippy::suspicious, clippy::style)]
#![warn(clippy::pedantic, clippy::cargo)]
#![allow(clippy::module_name_repetitions)]

pub mod api;
pub mod db;
pub mod entities;
pub mod events;
pub mod handlers;
use db::Connection;
use fireblocks::Client as FireblocksClient;
use hub_core::{clap, consumer::RecvError, prelude::*};
use solana_client::rpc_client::RpcClient;

#[derive(Debug, clap::Args)]
#[command(version, author, about)]
pub struct Args {
    #[arg(short, long, env, default_value_t = 3007)]
    pub port: u16,

    #[arg(short, long, env)]
    pub solana_endpoint: String,

    #[command(flatten)]
    pub db: db::DbArgs,

    #[command(flatten)]
    pub fireblocks: fireblocks::FbArgs,
}

#[derive(Clone)]
pub struct AppState {
    pub connection: Connection,
    pub fireblocks: FireblocksClient,
}

impl AppState {
    #[must_use]
    pub fn new(connection: Connection, fireblocks: FireblocksClient) -> Self {
        Self {
            connection,
            fireblocks,
        }
    }
}

mod proto {
    include!(concat!(env!("OUT_DIR"), "/organization.proto.rs"));
    include!(concat!(env!("OUT_DIR"), "/drops.proto.rs"));
}

#[derive(Debug)]
pub enum Services {
    Organizations(proto::OrganizationEventKey, proto::OrganizationEvents),
    Drops(proto::DropEventKey, proto::DropEvents),
}

impl hub_core::consumer::MessageGroup for Services {
    const REQUESTED_TOPICS: &'static [&'static str] = &["hub-orgs", "hub-drops"];

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
            "hub-drops" => {
                let key = proto::DropEventKey::decode(key)?;
                let val = proto::DropEvents::decode(val)?;

                Ok(Services::Drops(key, val))
            },
            t => Err(RecvError::BadTopic(t.into())),
        }
    }
}
