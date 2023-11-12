use axum::{extract::State, response::IntoResponse, Form};
use chrono::Utc;
use color_eyre::eyre::Result;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

use crate::{AppState, DbState, auth_feature::UserClaims};

#[derive(Serialize)]
pub struct ChatTemplate {
    name: String,
}

pub async fn get_chat_page(State(state): State<AppState>, token: UserClaims) -> impl IntoResponse {
    
    let context = ChatTemplate {
        name: token.name
    };
    state.templates.render("chat.html", &context).unwrap()
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
    
    state.templates.render("message.html", &context).unwrap()
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
    
    state.templates.render("messages.html", &context).unwrap()
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
    Ok(())
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
    Ok(messages_result)
}
