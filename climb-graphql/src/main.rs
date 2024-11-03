mod schema;

use crate::schema::QueryRoot;
use async_graphql::{http::GraphiQLSource, EmptySubscription, Schema};
use async_graphql_axum::GraphQL;
use axum::{
    response::{self, IntoResponse},
    routing::get,
    Router,
};
use schema::MutationRoot;
use tokio::net::TcpListener;

use deadpool_postgres::{Config, ManagerConfig, Pool, RecyclingMethod, Runtime};
use tokio_postgres::NoTls;

async fn graphiql() -> impl IntoResponse {
    response::Html(GraphiQLSource::build().endpoint("/graphql").finish())
}

async fn create_pool() -> Pool {
    let mut cfg = Config::new();
    cfg.dbname = Some("climb-pg_test".to_string());
    cfg.host = Some("localhost".to_string());
    cfg.manager = Some(ManagerConfig {
        recycling_method: RecyclingMethod::Fast,
    });

    cfg.create_pool(Some(Runtime::Tokio1), NoTls).unwrap()
}

#[tokio::main]
async fn main() {
    let pool = create_pool().await;
    let schema = Schema::build(QueryRoot, MutationRoot, EmptySubscription)
        .data(pool)
        .finish();

    let app = Router::new().route("/graphql", get(graphiql).post_service(GraphQL::new(schema)));

    println!("GraphiQL IDE: http://localhost:8000/graphql");

    axum::serve(TcpListener::bind("127.0.0.1:8000").await.unwrap(), app)
        .await
        .unwrap();
}
