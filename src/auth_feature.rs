use askama::Template;
use askama_axum::IntoResponse;
use axum::Form;
use serde::Deserialize;
use validator::Validate;

#[derive(Deserialize, Validate)]
pub struct LoginData {
    #[validate(length(min = 3))]
    username: String,
    #[validate(length(min = 8))]
    password: String,
}

#[derive(Template)]
#[template(path = "login.html")]
pub struct LoginTemplate {
    pub error: Option<String>,
}

pub async fn get_login_page() -> impl IntoResponse {
    LoginTemplate { error: None }
}

pub async fn post_login(Form(form): Form<LoginData>) -> impl IntoResponse {
    let validation = form.validate();
    return match validation {
        Ok(_) => LoginTemplate { error: None },
        Err(_) => LoginTemplate {
            error: Some("Bad request!".to_string()),
        },
    };
}
