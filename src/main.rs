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

use deadpool_postgres::{Pool, Runtime};
use tokio_postgres::NoTls;

async fn graphiql() -> impl IntoResponse {
    response::Html(GraphiQLSource::build().endpoint("/graphql").finish())
}

#[derive(serde::Deserialize, serde::Serialize)]
struct Config {
    pub pg: deadpool_postgres::Config,
}

impl Config {
    pub fn from_env() -> Result<Self, config::ConfigError> {
        let cfg = config::Config::builder()
            .add_source(config::Environment::default().separator("__"))
            .build()?;
        cfg.try_deserialize()
    }
}

async fn create_pool() -> Pool {
    // NOTE at least PG__DBNAME is required
    let cfg = Config::from_env().expect("Environment was not enough to setup configuration");
    cfg.pg.create_pool(Some(Runtime::Tokio1), NoTls).expect("Could not create pool")
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
