use std::sync::Arc;

use axum::Router;
use sqlx::PgPool;
use tokio::net::TcpListener;
use tracing::info;
use tracing_subscriber::{
    Layer, fmt::format::FmtSpan, layer::SubscriberExt, util::SubscriberInitExt,
};

use crate::{config::Config, routes};

#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
    pub config: Arc<Config>,
}

impl AppState {
    async fn new(config: Config) -> color_eyre::Result<Self> {
        let db = PgPool::connect(&config.database_url).await?;

        Ok(Self {
            db,
            config: Arc::new(config),
        })
    }
}

pub struct App;

impl App {
    pub async fn start() -> color_eyre::Result<()> {
        let layer = tracing_subscriber::fmt::layer()
            .with_span_events(FmtSpan::NEW)
            .boxed();

        tracing_subscriber::registry().with(layer).init();

        // Em produção as variáveis normalmente já vêm do ambiente, então um
        // .env ausente não deveria derrubar a aplicação.
        dotenvy::dotenv().ok();

        let config = Config::from_env()?;
        let port = config.port;

        let state = AppState::new(config).await?;

        let listener = TcpListener::bind(format!("0.0.0.0:{port}")).await?;
        let router = Router::new()
            .nest("/api", routes::api::router())
            .merge(routes::frontend::router())
            .with_state(state);

        info!("Starting service on port {port}");

        axum::serve(listener, router).await?;

        Ok(())
    }
}
