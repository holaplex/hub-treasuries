#![deny(clippy::disallowed_methods, clippy::suspicious, clippy::style)]
#![warn(clippy::pedantic, clippy::cargo)]
#![allow(clippy::module_name_repetitions)]

pub mod api;
pub mod db;
pub mod entities;
pub mod handlers;
use db::Connection;
use fireblocks::Client as FireblocksClient;
use hub_core::{clap, prelude::*};

#[derive(Debug, clap::Args)]
#[command(version, author, about)]
pub struct Args {
    #[arg(short, long, env, default_value_t = 3002)]
    pub port: u16,

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
