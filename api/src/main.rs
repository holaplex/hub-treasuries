//!

use holaplex_hub_treasuries::{
    build_schema,
    db::Connection,
    events,
    handlers::{fireblocks_webhook_handler, graphql_handler, health, playground},
    proto, AppState, Args, Services,
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
            let rpc_client = RpcClient::new(solana_endpoint).get_inner_client().clone();
            let producer = common.producer_cfg.build::<proto::TreasuryEvents>().await?;

            let state = AppState::new(
                schema,
                connection.clone(),
                fireblocks.clone(),
                producer.clone(),
            );

            let cons = common.consumer_cfg.build::<Services>().await?;

            let connection = connection.clone();

            tokio::spawn({
                let connection = connection.clone();
                let producer = producer.clone();

                async move {
                    {
                        let mut stream = cons.stream();

                        match stream.next().await {
                            Some(Ok(msg)) => {
                                info!(?msg, "message received");

                                tokio::spawn(async move {
                                    let producer = producer.clone();

                                    {
                                        events::process(
                                            msg,
                                            connection.clone(),
                                            fireblocks.clone(),
                                            fireblocks_supported_asset_ids,
                                            producer.clone(),
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
                        .at("/health", get(health))
                        .at(
                            "/webhooks",
                            post(fireblocks_webhook_handler)
                                .with(AddData::new(connection))
                                .with(AddData::new(producer))
                                .with(AddData::new(rpc_client)),
                        ),
                )
                .await
                .context("failed to build graphql server")
        })
    });
}
