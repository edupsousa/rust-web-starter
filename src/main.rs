use std::net::SocketAddr;

use askama::Template;
use axum::{
    extract::State,
    response::IntoResponse,
    routing::{get, get_service, post},
    Form, Router, http::StatusCode,
};
use color_eyre::eyre::Result;
use serde::Deserialize;
use tower_http::services::{ServeDir, ServeFile};
use tracing::info;
use crate::db::ChatDB;

mod db;

#[derive(Clone)]
struct AppState {
    pub db: ChatDB,
}

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;
    tracing_subscriber::fmt::init();

    let db = ChatDB::build().await?;

    let state = AppState { db };

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

async fn send_message(
    State(state): State<AppState>,
    Form(send_message): Form<SendMessageForm>,
) -> impl IntoResponse {
    let message = send_message.new_message;
    state.db.push_message(&message).await.unwrap();
    MessageTemplate { message }
}

#[derive(Template)]
#[template(path = "messages.html")]
struct MessagesTemplate {
    messages: Vec<String>,
}

async fn list_messages(State(state): State<AppState>) -> impl IntoResponse {
    let response = match state.db.list_all_messages().await {
       Ok(messages) => MessagesTemplate { 
        messages: messages
            .into_iter()
            .map(|msg| msg.text)
            .collect()
        }.into_response(),
       Err(error) => (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()).into_response()
    };
    return response;
}