use std::{net::SocketAddr, sync::Arc};

use crate::config::Config;
use axum::{
    extract::State,
    response::IntoResponse,
    routing::{get, get_service, post},
    Router,
};
use color_eyre::eyre::Result;
use dotenv::dotenv;
use sqlx::{migrate::MigrateDatabase, Pool, Sqlite, SqlitePool};
use template_service::TemplateService;
use tower_http::services::ServeDir;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod auth_feature;
mod chat_feature;
mod config;
mod template_service;

pub type DbState = Pool<Sqlite>;
pub type ExtractedState = State<AppState>;

#[derive(Clone)]
pub struct AppState {
    pub db: DbState,
    pub config: Arc<Config>,
    pub templates: Arc<TemplateService>,
}

impl AppState {
    pub async fn build() -> Result<Self> {
        let config = Config::init();
        let db = Self::build_db_pool(&config.database_url).await?;
        let templates = TemplateService::build()?;
        let state = AppState {
            db,
            config: Arc::new(config),
            templates: Arc::new(templates),
        };
        Ok(state)
    }

    async fn build_db_pool(db_url: &str) -> Result<DbState> {
        if !Sqlite::database_exists(db_url).await.unwrap_or(false) {
            info!("Creating database {}", db_url);
            Sqlite::create_database(db_url).await?;
        }
        info!("Connecting to database {}", db_url);
        let pool = SqlitePool::connect(db_url).await?;
        sqlx::migrate!("./migrations").run(&pool).await?;
        Ok(pool)
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
                "rust-web-starter=debug,tower_http=debug,axum::rejection=trace".into()
            }),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();
}
