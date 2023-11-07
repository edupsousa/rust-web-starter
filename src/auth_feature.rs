use askama::Template;
use askama_axum::IntoResponse;
use axum::{Form, http::StatusCode};
use validator::Validate;
use serde::Deserialize;

#[derive(Deserialize, Validate)]
pub struct LoginData {
    #[validate(length(min = 3))]
    username: String,
    #[validate(length(min = 8))]
    password: String,
}

#[derive(Template)]
#[template(path = "login.html")]
struct LoginTemplate;

pub async fn get_login_page() -> impl IntoResponse {
    LoginTemplate {}
}

pub async fn post_login(Form(form): Form<LoginData>) -> impl IntoResponse {
    let validation = form.validate();
    return match validation {
      Ok(_) => {
        println!("Login success");
        LoginTemplate {}.into_response()
      },
      Err(_) => {
        (StatusCode::BAD_REQUEST, LoginTemplate {}).into_response()
      }
    };
}