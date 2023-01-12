//!

#![deny(
    clippy::pedantic,
    clippy::match_wildcard_for_single_variants,
    clippy::redundant_closure_for_method_calls
)]
#![warn(
    clippy::perf,
    clippy::complexity,
    clippy::style,
    clippy::suspicious,
    clippy::correctness,
    clippy::module_name_repetitions,
    clippy::similar_names,
    clippy::if_not_else,
    clippy::must_use_candidate,
    clippy::missing_errors_doc,
    clippy::option_if_let_else,
    clippy::match_same_arms,
    clippy::default_trait_access,
    clippy::map_flatten,
    clippy::map_unwrap_or,
    clippy::explicit_iter_loop,
    clippy::too_many_lines,
    clippy::cast_sign_loss,
    clippy::unused_self,
    clippy::cast_lossless,
    clippy::cast_possible_truncation,
    clippy::use_self,
    clippy::needless_borrow,
    clippy::redundant_pub_crate,
    clippy::useless_let_if_seq,
    // missing_docs,
    clippy::upper_case_acronyms
)]
#![forbid(unsafe_code)]
#![allow(clippy::unused_async)]

mod db;
#[allow(clippy::pedantic)]
mod models;
mod mutations;
mod queries;

mod prelude {
    pub use std::time::Duration;

    pub use anyhow::{anyhow, bail, Context, Result};
    pub use chrono::{DateTime, Utc};
    pub use clap::Parser;
    pub use log::debug;
}

use std::{str::FromStr, sync::Arc};

use anyhow::{Context as AnyhowContext, Result};
use async_graphql::{
    extensions::{ApolloTracing, Logger},
    http::{playground_source, GraphQLPlaygroundConfig},
    EmptySubscription, Schema,
};
use async_graphql_poem::{GraphQLRequest, GraphQLResponse};
use db::Connection;
use fireblocks::client::FireblocksClient;
use mutations::Mutation;
use poem::{
    async_trait, get, handler,
    listener::TcpListener,
    post,
    web::{Data, Html},
    EndpointExt, FromRequest, IntoResponse, Request, RequestBody, Route, Server,
};
use prelude::*;
use queries::Query;
use sea_orm::DatabaseConnection;

#[derive(Debug, Parser)]
pub struct Args {
    #[clap(short, long, env, default_value = "127.0.0.1")]
    server_address: String,
    #[clap(short, long, env, default_value = "3003")]
    port: u16,
}

#[derive(Debug)]
pub struct UserID(Option<uuid::Uuid>);

impl TryFrom<&str> for UserID {
    type Error = anyhow::Error;
    fn try_from(value: &str) -> Result<Self> {
        let id = uuid::Uuid::from_str(value)?;

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

type AppSchema = Schema<Query, Mutation, EmptySubscription>;

#[handler]
async fn playground() -> impl IntoResponse {
    Html(playground_source(GraphQLPlaygroundConfig::new("/graphql")))
}

#[handler]
async fn graphql_handler(
    Data(schema): Data<&AppSchema>,
    user_id: UserID,
    req: GraphQLRequest,
) -> GraphQLResponse {
    schema.execute(req.0.data(user_id)).await.into()
}

pub struct Context {
    db: Arc<DatabaseConnection>,
    fireblocks: FireblocksClient,
}

impl Context {
    async fn new() -> Result<Self> {
        let db = Connection::new()
            .await
            .context("failed to get database connection")?
            .get();

        let fireblocks = fireblocks::build()?;

        Ok(Self { db, fireblocks })
    }
}
/// Builds the GraphQL Schema, attaching the Database to the context
/// # Errors
/// This function fails if ...
pub async fn build_schema(ctx: Context) -> Result<AppSchema> {
    let schema = Schema::build(Query::default(), Mutation::default(), EmptySubscription)
        .extension(ApolloTracing)
        .extension(Logger)
        .data(ctx.db)
        .data(ctx.fireblocks)
        .enable_federation()
        .finish();

    Ok(schema)
}

#[tokio::main]
pub async fn main() -> Result<()> {
    if cfg!(debug_assertions) {
        dotenv::dotenv().ok();
    }

    let Args {
        server_address,
        port,
    } = Args::parse();

    env_logger::builder()
        .filter_level(if cfg!(debug_assertions) {
            log::LevelFilter::Debug
        } else {
            log::LevelFilter::Info
        })
        .parse_default_env()
        .init();

    let app_context = Context::new()
        .await
        .context("failed to build app context")?;
    let schema = build_schema(app_context)
        .await
        .context("failed to build schema")?;

    Server::new(TcpListener::bind(format!("{server_address}:{port}")))
        .run(
            Route::new()
                .at("/graphql", post(graphql_handler))
                .at("/playground", get(playground))
                .data(schema),
        )
        .await
        .context("failed to build graphql server")
}