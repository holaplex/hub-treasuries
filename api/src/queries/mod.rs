mod customer;
mod project;

// // Add your other ones here to create a unified Query object
#[derive(async_graphql::MergedObject, Default)]
pub struct Query(customer::Query, project::Query);
