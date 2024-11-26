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

use deadpool_postgres::Runtime;
use tokio_postgres::NoTls;

async fn graphiql() -> impl IntoResponse {
    response::Html(GraphiQLSource::build().endpoint("/graphql").finish())
}

#[derive(serde::Deserialize, serde::Serialize)]
struct GraphQLConfig {
    pub host: String,
    pub port: u32,
}

#[derive(serde::Deserialize, serde::Serialize)]
struct Config {
    pub pg: deadpool_postgres::Config,
    pub graphql: GraphQLConfig,
}

pub struct AppData {
    pub pg_pool: deadpool_postgres::Pool,
}

impl Config {
    pub fn from_env() -> Result<Self, config::ConfigError> {
        let cfg = config::Config::builder()
            .add_source(config::Environment::default().separator("__"))
            .set_default("graphql.host", "127.0.0.1")?
            .set_default("graphql.port", 8000)?
            .build()?;
        cfg.try_deserialize()
    }
}

#[tokio::main]
async fn main() {
    let cfg = Config::from_env().expect("Environment was not enough to setup configuration");
    let pg_pool = cfg.pg.create_pool(Some(Runtime::Tokio1), NoTls).expect("Could not create pool");
    let context = AppData { pg_pool };
    let schema = Schema::build(QueryRoot, MutationRoot, EmptySubscription)
        .data(context)
        .finish();

    let app = Router::new().route("/graphql", get(graphiql).post_service(GraphQL::new(schema)));

    println!("GraphiQL IDE: http://{}:{}/graphql", cfg.graphql.host, cfg.graphql.port);

    axum::serve(TcpListener::bind(format!("{}:{}", cfg.graphql.host, cfg.graphql.port)).await.unwrap(), app)
        .await
        .unwrap();
}
