use askama_axum::IntoResponse;
use axum::{extract::State, Form};
use serde::{Deserialize, Serialize};
use validator::Validate;

use crate::AppState;

#[derive(Deserialize, Validate)]
pub struct LoginData {
    #[validate(length(min = 3))]
    username: String,
    #[validate(length(min = 8))]
    password: String,
}

#[derive(Serialize)]
pub struct LoginTemplateData {
    pub error: bool,
}

pub async fn get_login_page(State(state): State<AppState>) -> impl IntoResponse {
    let context = LoginTemplateData { error: false };
    let html = state.templates.render("login.html", &context).unwrap();
    return html;
}

pub async fn post_login(
    State(state): State<AppState>,
    Form(form): Form<LoginData>,
) -> impl IntoResponse {
    let validation = form.validate();
    let context = match validation {
        Ok(_) => LoginTemplateData { error: false },
        Err(_) => LoginTemplateData { error: true },
    };
    let html = state.templates.render("login.html", &context).unwrap();
    return html;
}
