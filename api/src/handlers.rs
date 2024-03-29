use async_graphql::http::{playground_source, GraphQLPlaygroundConfig};
use async_graphql_poem::{GraphQLRequest, GraphQLResponse};
use hub_core::{
    anyhow::Result,
    metrics::{Encoder, TextEncoder},
};
use poem::{
    handler,
    http::StatusCode,
    web::{Data, Html},
    IntoResponse,
};

use crate::{AppContext, AppState, Balance, Metrics, OrganizationId, UserID};

#[handler]
pub fn health() -> StatusCode {
    StatusCode::OK
}

#[handler]
pub fn playground() -> impl IntoResponse {
    Html(playground_source(GraphQLPlaygroundConfig::new("/graphql")))
}

#[handler]
pub async fn graphql_handler(
    Data(state): Data<&AppState>,
    user_id: UserID,
    organization_id: OrganizationId,
    balance: Balance,
    req: GraphQLRequest,
) -> Result<GraphQLResponse> {
    let context = AppContext::new(state.connection.clone(), user_id, organization_id, balance);

    Ok(state
        .schema
        .execute(
            req.0
                .data(context)
                .data(state.fireblocks.clone())
                .data(state.producer.clone())
                .data(state.credits.clone()),
        )
        .await
        .into())
}

#[handler]
pub fn metrics_handler(Data(metrics): Data<&Metrics>) -> Result<String> {
    let mut buffer = vec![];
    let encoder = TextEncoder::new();
    encoder.encode(&metrics.registry.gather(), &mut buffer)?;
    Ok(String::from_utf8_lossy(&buffer).into_owned())
}
