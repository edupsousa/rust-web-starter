use askama_axum::IntoResponse;
use axum::{Form, extract::State};
use serde::{Deserialize, Serialize};

use crate::AppState;

pub async fn get_chat_page( State(state): State<AppState> ) -> impl IntoResponse {
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
    state.db.push_message(&message).await.unwrap();
    let context = MessageTemplate { message };
    let html = state.templates.render("message.html", &context).unwrap();
    return html;
}

#[derive(Serialize)]
struct MessagesTemplate {
    messages: Vec<String>,
}

pub async fn get_list_messages(State(state): State<AppState>) -> impl IntoResponse {
    let messages = state.db.list_all_messages().await.unwrap();
    let context = MessagesTemplate {
        messages: messages.into_iter().map(|msg| msg.text).collect(),
    };
    let html = state.templates.render("messages.html", &context).unwrap();
    return html;
}
