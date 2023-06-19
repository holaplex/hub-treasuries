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

pub fn main() {
    let opts = hub_core::StartConfig {
        service_name: "hub-treasuries",
    };

    hub_core::run(opts, |common, args| {
        let Args {
            port,
            db,
            fireblocks,
        } = args;

        common.rt.block_on(async move {
            let connection = Connection::new(db)
                .await
                .context("failed to get database connection")?;

            let schema = build_schema();
            let fireblocks = fireblocks::Fireblocks::new(fireblocks)?;
            let producer = common.producer_cfg.build::<proto::TreasuryEvents>().await?;

            let event_processor =
                events::Processor::new(connection.clone(), producer.clone(), fireblocks.clone());

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
                        let event_processor = event_processor.clone();

                        match stream.next().await {
                            Some(Ok(msg)) => {
                                info!(?msg, "message received");

                                tokio::spawn(async move { event_processor.process(msg).await });
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
