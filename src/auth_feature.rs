use axum::{
    extract::State,
    response::{IntoResponse, Redirect},
    Form,
};
use axum_extra::extract::CookieJar;
use serde::{Deserialize, Serialize};
use sqlx::{Pool, Sqlite};
use validator::Validate;

use crate::{jwt_auth::create_token_cookie, AppState};

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
    jar: CookieJar,
    Form(form): Form<LoginData>,
) -> impl IntoResponse {
    let validation = form.validate();
    if validation.is_err() {
        let context = LoginTemplateData { error: true };
        let html = state.templates.render("login.html", &context).unwrap();
        return html.into_response();
    }
    let user = get_authenticated_user(&state.db, &form.username, &form.password).await;
    if user.is_none() {
        let context = LoginTemplateData { error: true };
        let html = state.templates.render("login.html", &context).unwrap();
        return html.into_response();
    }
    let user = user.unwrap();
    let cookie = create_token_cookie(&state.config.jwt_secret, &user.uid).unwrap();
    return (jar.add(cookie), Redirect::to("/chat")).into_response();
}

#[derive(Deserialize, sqlx::FromRow)]
struct User {
    uid: String,
    username: String,
    password: String,
}

async fn get_authenticated_user(db: &Pool<Sqlite>, username: &str, password: &str) -> Option<User> {
    let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE username = ?;")
        .bind(username)
        .fetch_optional(db)
        .await;

    match user {
        Ok(Some(user)) => {
            if user.password == password {
                return Some(user);
            } else {
                return None;
            }
        }
        _ => return None,
    };
}
