//!

use holaplex_hub_treasuries::{
    build_schema,
    db::Connection,
    events,
    handlers::{graphql_handler, health, playground},
    proto, Actions, AppState, Args, Services,
};
use hub_core::{
    anyhow::Context as AnyhowContext,
    prelude::*,
    tokio::{self, task},
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
            solana_endpoint,
            fireblocks_supported_asset_ids,
            db,
            fireblocks,
        } = args;

        common.rt.block_on(async move {
            let connection = Connection::new(db)
                .await
                .context("failed to get database connection")?;

            let schema = build_schema();
            let fireblocks = fireblocks::Client::new(fireblocks)?;
            let rpc_client = Arc::new(RpcClient::new(solana_endpoint));
            let producer = common.producer_cfg.build::<proto::TreasuryEvents>().await?;
            let credits = common.credits_cfg.build::<Actions>().await?;
            let state = AppState::new(
                schema,
                connection.clone(),
                fireblocks.clone(),
                producer.clone(),
                credits,
            );

            let cons = common.consumer_cfg.build::<Services>().await?;

            tokio::spawn(async move {
                {
                    let mut stream = cons.stream();
                    loop {
                        let fireblocks = fireblocks.clone();
                        let connection = connection.clone();
                        let rpc_client = rpc_client.clone();
                        let fireblocks_supported_asset_ids = fireblocks_supported_asset_ids.clone();
                        let producer = producer.clone();

                        match stream.next().await {
                            Some(Ok(msg)) => {
                                info!(?msg, "message received");

                                tokio::spawn(async move {
                                    {
                                        events::process(
                                            msg,
                                            connection.clone(),
                                            fireblocks.clone(),
                                            fireblocks_supported_asset_ids,
                                            &rpc_client,
                                            producer,
                                        )
                                        .await
                                    }
                                });
                                task::yield_now().await;
                            },
                            None => (),
                            Some(Err(e)) => {
                                warn!("failed to get message {:?}", e);
                            },
                        }
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
                .context("failed to build graphql server")
        })
    });
}
