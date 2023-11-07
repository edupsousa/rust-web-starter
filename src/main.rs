use std::net::SocketAddr;

use crate::{config::Config, db::ChatDB, jwt_auth::auth};
use axum::{
    middleware,
    routing::{get, get_service, post},
    Router,
};
use color_eyre::eyre::Result;
use dotenv::dotenv;
use tower_http::services::ServeDir;
use tracing::info;

mod auth_feature;
mod chat_feature;
mod config;
mod db;
mod jwt_auth;

#[derive(Clone)]
pub struct AppState {
    pub db: ChatDB,
    pub env: Config,
}

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;
    tracing_subscriber::fmt::init();
    dotenv().unwrap();

    let config = Config::init();
    let db = ChatDB::build(&config.database_url).await?;
    let state = AppState { db, env: config };

    let app = Router::new()
        .route("/", get(chat_feature::get_chat_page))
        .route("/login", get(auth_feature::get_login_page))
        .route("/message", post(chat_feature::post_send_message))
        .route("/messages", get(chat_feature::get_list_messages))
        .nest_service("/assets", get_service(ServeDir::new("assets")))
        .route_layer(middleware::from_fn_with_state(state.clone(), auth))
        .with_state(state);

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    info!("listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await?;

    Ok(())
}

