use askama::Template;
use askama_axum::IntoResponse;


#[derive(Template)]
#[template(path = "login.html")]
struct LoginTemplate;

pub async fn get_login_page() -> impl IntoResponse {
    LoginTemplate {}
}
