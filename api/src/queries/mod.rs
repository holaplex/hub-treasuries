#![allow(clippy::unused_async)]

mod customer;
mod project;
mod treasury;
mod wallet;

// // Add your other ones here to create a unified Query object
#[derive(async_graphql::MergedObject, Default)]
pub struct Query(
    customer::Query,
    project::Query,
    treasury::Query,
    wallet::Query,
);
