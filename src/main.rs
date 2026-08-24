use crate::app::App;

pub mod app;
pub mod auth;
pub mod config;
pub mod error;
pub mod models;
pub mod repository;
pub mod routes;

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    App::start().await
}
