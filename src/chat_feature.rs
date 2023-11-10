use axum::{extract::State, response::IntoResponse, Form};
use chrono::Utc;
use color_eyre::eyre::Result;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

use crate::{AppState, DbState};

pub async fn get_chat_page(State(state): State<AppState>) -> impl IntoResponse {
    let html = state.templates.render_empty_context("chat.html").unwrap();
    return html;
}

#[derive(Serialize)]
struct MessageTemplate {
    message: String,
}

#[derive(Deserialize)]
pub struct SendMessageForm {
    new_message: String,
}

pub async fn post_send_message(
    State(state): State<AppState>,
    Form(send_message): Form<SendMessageForm>,
) -> impl IntoResponse {
    let message = send_message.new_message;
    push_message(&state.db, &message).await.unwrap();
    let context = MessageTemplate { message };
    let html = state.templates.render("message.html", &context).unwrap();
    return html;
}

#[derive(Serialize)]
struct MessagesTemplate {
    messages: Vec<String>,
}

pub async fn get_list_messages(State(state): State<AppState>) -> impl IntoResponse {
    let messages = list_all_messages(&state.db).await.unwrap();
    let context = MessagesTemplate {
        messages: messages.into_iter().map(|msg| msg.text).collect(),
    };
    let html = state.templates.render("messages.html", &context).unwrap();
    return html;
}

pub async fn push_message(db: &DbState, text: &str) -> Result<()> {
    let id: String = Uuid::new_v4().into();
    let create_time = Utc::now().timestamp();
    let _result = sqlx::query("INSERT INTO messages (id, create_time, text) VALUES (?, ?, ?);")
        .bind(id)
        .bind(create_time)
        .bind(text)
        .execute(db)
        .await?;
    return Ok(());
}

#[derive(Debug, Clone, FromRow)]
pub struct Message {
    pub id: String,
    pub text: String,
    pub create_time: i64,
}

pub async fn list_all_messages(db: &DbState) -> Result<Vec<Message>> {
    let messages_result =
        sqlx::query_as::<_, Message>("SELECT * FROM messages ORDER BY create_time;")
            .fetch_all(db)
            .await?;
    return Ok(messages_result);
}
