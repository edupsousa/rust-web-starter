use std::{net::SocketAddr, sync::{Arc, Mutex}};

use askama::Template;
use axum::{
    response::IntoResponse,
    routing::{get_service, post, get},
    Form, Router, extract::State,
};
use color_eyre::eyre::Result;
use serde::Deserialize;
use tower_http::services::{ServeDir, ServeFile};
use tracing::info;

#[derive(Clone)]
struct AppState {
    pub db: ChatDB,
}

#[derive(Clone)]
struct ChatDB {
    items: Arc<Mutex<Vec<String>>>,
}

impl ChatDB {
    fn new() -> Self {
        Self {
            items: Arc::new(Mutex::new(Vec::new())),
        }
    }

    fn push_message(&self, contents: String) {
        let mut items = self.items.lock().unwrap();
        items.push(contents);
    }

    fn list_all_messages(&self) -> Vec<String> {
        self.items.lock().unwrap().clone()
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;
    tracing_subscriber::fmt::init();

    let state = AppState { db: ChatDB::new() };

    let app = Router::new()
        .route("/", get_service(ServeFile::new("static/index.html")))
        .route("/message", post(send_message))
        .route("/messages", get(list_messages))
        .nest_service("/static", get_service(ServeDir::new("static")))
        .with_state(state);

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    info!("listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await?;

    Ok(())
}

#[derive(Template)]
#[template(path = "message.html")]
struct MessageTemplate {
    message: String,
}

#[derive(Deserialize)]
struct SendMessageForm {
    new_message: String,
}

async fn send_message(State(state): State<AppState>, Form(send_message): Form<SendMessageForm>) -> impl IntoResponse {
    let message = send_message.new_message;
    state.db.push_message(message.clone());
    MessageTemplate {
        message,
    }
}

#[derive(Template)]
#[template(path = "messages.html")]
struct MessagesTemplate {
    messages: Vec<String>
}

async fn list_messages(State(state): State<AppState>) -> impl IntoResponse {
    MessagesTemplate {
        messages: state.db.list_all_messages()
    }
}