//!

use holaplex_hub_treasuries::{
    api::TreasuryApi, db::Connection, events, handlers::health, AppState, Args, Services,
};
use hub_core::{
    anyhow::Context as AnyhowContext,
    prelude::*,
    tokio::{self},
};
use poem::{get, listener::TcpListener, middleware::AddData, EndpointExt, Route, Server};
use poem_openapi::OpenApiService;

pub fn main() {
    let opts = hub_core::StartConfig {
        service_name: "hub-orgs",
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

            let fireblocks = fireblocks::Client::new(fireblocks)?;
            let api_service = OpenApiService::new(TreasuryApi, "HubTreasury", "0.1.0")
                .server(format!("http://localhost:{port}/v1"));
            let ui = api_service.swagger_ui();
            let spec = api_service.spec_endpoint();
            let state = AppState::new(connection.clone(), fireblocks.clone());

            let cons = common.consumer_cfg.build::<Services>().await?;

            tokio::spawn(async move {
                loop {
                    match cons.stream().next().await {
                        Some(Ok(msg)) => {
                            info!(?msg, "message received");

                            if let Err(e) =
                                events::process(msg, connection.clone(), fireblocks.clone()).await
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
                        .nest("/v1", api_service.with(AddData::new(state)))
                        .nest("/", ui)
                        .at("/spec", spec)
                        .at("/health", get(health)),
                )
                .await
                .context("failed to build rest api server")
        })
    });
}
