//!

use holaplex_hub_treasuries::{
    build_schema,
    db::Connection,
    events,
    handlers::{graphql_handler, health, playground},
    AppState, Args, Services,
};
use hub_core::{
    anyhow::Context as AnyhowContext,
    prelude::*,
    tokio::{self},
};
use poem::{get, listener::TcpListener, middleware::AddData, post, EndpointExt, Route, Server};
use solana_client::rpc_client::RpcClient;

pub fn main() {
    let opts = hub_core::StartConfig {
        service_name: "hub-treasuries",
    };

    hub_core::run(opts, |common, args| {
        let Args {
            port,
            db,
            fireblocks,
            solana_endpoint,
        } = args;

        common.rt.block_on(async move {
            let connection = Connection::new(db)
                .await
                .context("failed to get database connection")?;

            let schema = build_schema();
            let fireblocks = fireblocks::Client::new(fireblocks)?;

            let rpc_client = RpcClient::new(solana_endpoint);

            let state = AppState::new(schema, connection.clone(), fireblocks.clone());

            let cons = common.consumer_cfg.build::<Services>().await?;

            tokio::spawn(async move {
                loop {
                    match cons.stream().next().await {
                        Some(Ok(msg)) => {
                            info!(?msg, "message received");

                            if let Err(e) = events::process(
                                msg,
                                connection.clone(),
                                fireblocks.clone(),
                                &rpc_client,
                            )
                            .await
                            {
                                warn!("failed to process message {:?}", e);
                            }
                        },
                        None => (),
                        Some(Err(e)) => {
                            warn!("failed to get message {:?}", e);
                        },
                    }
                }
            });

            Server::new(TcpListener::bind(format!("0.0.0.0:{port}")))
                .run(
                    Route::new()
                        .at("/graphql", post(graphql_handler).with(AddData::new(state)))
                        .at("/playground", get(playground))
                        .at("/health", get(health)),
                )
                .await
                .context("failed to graphql server")
        })
    });
}
