use askama::Template;
use askama_axum::IntoResponse;
use axum::{http::StatusCode, Form, extract::State};
use serde::Deserialize;

use crate::AppState;


#[derive(Template)]
#[template(path = "chat.html")]
struct IndexTemplate;

pub async fn get_chat_page() -> impl IntoResponse {
    IndexTemplate {}
}

#[derive(Template)]
#[template(path = "message.html")]
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
    MessageTemplate { message }
}

#[derive(Template)]
#[template(path = "messages.html")]
struct MessagesTemplate {
    messages: Vec<String>,
}

pub async fn get_list_messages(State(state): State<AppState>) -> impl IntoResponse {
    let response = match state.db.list_all_messages().await {
        Ok(messages) => MessagesTemplate {
            messages: messages.into_iter().map(|msg| msg.text).collect(),
        }
        .into_response(),
        Err(error) => (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()).into_response(),
    };
    return response;
}
