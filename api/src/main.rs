//!

use holaplex_hub_treasuries::{
    build_schema,
    db::Connection,
    events,
    handlers::{graphql_handler, health, playground},
    proto, Actions, AppState, Args, Services,
};
use hub_core::{anyhow::Context as AnyhowContext, prelude::*, tokio};
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
                cons.consume(
                    |b| {
                        b.with_jitter()
                            .with_min_delay(Duration::from_millis(500))
                            .with_max_delay(Duration::from_secs(90))
                    },
                    |e| async move { event_processor.process(e).await },
                )
                .await
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
