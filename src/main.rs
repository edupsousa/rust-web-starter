use std::net::SocketAddr;

use crate::{config::Config, db::ChatDB};
use axum::{
    extract::State,
    response::IntoResponse,
    routing::{get, get_service, post},
    Router,
};
use color_eyre::eyre::Result;
use dotenv::dotenv;
use template_service::TemplateService;
use tower_http::services::ServeDir;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod auth_feature;
mod chat_feature;
mod config;
mod db;
mod template_service;

#[derive(Clone)]
pub struct AppState {
    pub db: ChatDB,
    pub config: Config,
    pub templates: TemplateService,
}

impl AppState {
    pub async fn build() -> Result<Self> {
        let config = Config::init();
        let db = ChatDB::build(&config.database_url).await?;
        let templates = TemplateService::build()?;
        let state = AppState {
            db,
            config,
            templates,
        };
        Ok(state)
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    init()?;

    let state = AppState::build().await?;

    let app = Router::new()
        .route("/", get(get_index_page))
        .route("/login", get(auth_feature::get_login_page))
        .route("/login", post(auth_feature::post_login))
        .route("/chat", get(chat_feature::get_chat_page))
        .route("/message", post(chat_feature::post_send_message))
        .route("/messages", get(chat_feature::get_list_messages))
        .nest_service("/assets", get_service(ServeDir::new("assets")))
        .nest_service("/scripts", get_service(ServeDir::new("scripts")))
        .with_state(state)
        .layer(tower_http::trace::TraceLayer::new_for_http());

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    info!("listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await?;

    Ok(())
}

pub async fn get_index_page(State(state): State<AppState>) -> impl IntoResponse {
    let html = state.templates.render_empty_context("index.html").unwrap();
    html.into_response()
}

fn init() -> Result<()> {
    color_eyre::install()?;
    dotenv()?;
    init_tracing();
    Ok(())
}

fn init_tracing() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                "rust-chatbot-api=debug,tower_http=debug,axum::rejection=trace".into()
            }),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();
}