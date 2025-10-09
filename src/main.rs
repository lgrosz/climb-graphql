mod schema;
mod s3_client_manager;

use crate::schema::QueryRoot;
use async_graphql::{http::GraphiQLSource, Context, EmptySubscription, Schema};
use async_graphql_axum::GraphQL;
use axum::{
    response::{self, IntoResponse},
    routing::get,
    Router,
};
use deadpool::managed::PoolError;
use schema::MutationRoot;
use std::{collections::HashMap, fmt::Display};
use tokio::net::TcpListener;
use tokio::signal;

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
    pub pg: Option<deadpool_postgres::Config>,
    pub graphql: GraphQLConfig,
    pub images: Option<s3_client_manager::S3Config>,
}

pub struct AppData {
    pub pg_pool: Option<deadpool_postgres::Pool>,
    pub s3_pools: HashMap<String, s3_client_manager::Pool>,
}

pub enum AppDataDbError {
    NoPool,
    PoolError(PoolError<tokio_postgres::Error>),
}

impl Display for AppDataDbError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AppDataDbError::NoPool => write!(f, "No pool"),
            AppDataDbError::PoolError(e) => write!(f, "Pool error: {}", e),
        }
    }
}

pub enum AppDataError {
    NoAppData,
    Db(AppDataDbError),
}

impl Display for AppDataError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AppDataError::NoAppData => write!(f, "No app data"),
            AppDataError::Db(e) => write!(f, "DB Error: {}", e),
        }
    }
}

pub trait WithAppData {
    fn app_data(&self) -> Result<&AppData, AppDataError>;
    fn db_client(&self) -> impl std::future::Future<Output = Result<deadpool_postgres::Client, AppDataError>>;
}

impl Config {
    pub fn from_env() -> Result<Self, config::ConfigError> {
        let cfg = config::Config::builder()
            .add_source(config::Environment::default().separator("__"))
            .set_default("graphql.host", "127.0.0.1")?
            .set_default("graphql.port", 4000)?
            .set_default("images.host", "127.0.0.1")?
            .set_default("images.port", 80)?
            .set_default("images.region", "")?
            .set_default("images.bucket", "images")?
            .set_default("images.style", "subdomain")?
            .build()?;
        cfg.try_deserialize()
    }
}

impl WithAppData for Context<'_> {
    /// Access to app data directly. Prefer other helpers.
    fn app_data(&self) -> Result<&AppData, AppDataError> {
        self.data::<AppData>()
            .map_err(|_| AppDataError::NoAppData)
    }

    /// Helper to get a db client.
    async fn db_client(&self) -> Result<deadpool_postgres::Client, AppDataError> {
        let data = self.app_data()?;

        let client = match &data.pg_pool {
            Some(pool) => pool.get().await.map_err(|e| AppDataError::Db(AppDataDbError::PoolError(e)))?,
            None => return Err(AppDataError::Db(AppDataDbError::NoPool)),
        };

        Ok(client)
    }
}

#[tokio::main]
async fn main() {
    let cfg = Config::from_env().expect("Environment was not enough to setup configuration");

    let pg_pool = match &cfg.pg {
        Some(pg_config) => {
            match pg_config.create_pool(Some(Runtime::Tokio1), NoTls) {
                Ok(pool) => Some(pool),
                Err(err) => {
                    eprintln!("Warning: Could not create PostgreSQL pool: {}", err);
                    None
                }
            }
        }
        None => {
            eprintln!("Warning: PostgreSQL configuration is missing");
            None
        }
    };

    let mut s3_pools = HashMap::new();
    if let Some(s3_config) = &cfg.images {
        match s3_config.create_pool() {
            Ok(pool) => {
                s3_pools.insert(s3_config.bucket.clone(), pool);
            }
            Err(err) => {
                eprintln!("Warning: Could not create S3 pool: {}", err);
            }
        }
    } else {
        eprintln!("Warning: S3 configuration is missing");
    }

    let context = AppData { pg_pool, s3_pools };
    let schema = Schema::build(QueryRoot, MutationRoot, EmptySubscription)
        .data(context)
        .finish();

    let app = Router::new().route("/graphql", get(graphiql).post_service(GraphQL::new(schema)));

    println!("GraphiQL IDE: http://{}:{}/graphql", cfg.graphql.host, cfg.graphql.port);

    axum::serve(TcpListener::bind(format!("{}:{}", cfg.graphql.host, cfg.graphql.port)).await.unwrap(), app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .unwrap();
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}
