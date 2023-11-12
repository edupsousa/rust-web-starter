use std::fmt;

use axum::{
    extract::State,
    response::{IntoResponse, Redirect},
    Form,
};
use axum_extra::extract::{
    cookie::{Cookie, SameSite},
    CookieJar,
};
use color_eyre::eyre::Result;
use jsonwebtoken::{encode, EncodingKey, Header};
use serde::{Deserialize, Serialize};
use sqlx::{Pool, Sqlite};
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
    
    html
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
    let cookie = create_user_jwt_cookie(&state.config.jwt_secret, &user).unwrap();
    
    (jar.add(cookie), Redirect::to("/chat")).into_response()
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

#[derive(Serialize)]
struct UserTokenClaims {
    sub: String,
    iat: usize,
    exp: usize,
    name: String,
}

impl UserTokenClaims {
    fn new(user: &User) -> Self {
        let now = chrono::Utc::now();
        let iat = now.timestamp() as usize;
        let exp = (now + chrono::Duration::minutes(60)).timestamp() as usize;
        return UserTokenClaims {
            sub: user.uid.clone(),
            exp,
            iat,
            name: user.username.clone(),
        };
    }
}

struct Token(String);

impl Token {
    fn build(claims: impl Serialize, jwt_secret: &str) -> Result<Self> {
        let token = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(jwt_secret.as_ref()),
        )?;
        
        Ok(Token(token))
    }
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Into<Cookie<'static>> for Token {
    fn into(self) -> Cookie<'static> {
        let cookie = Cookie::build("token", self.0)
            .path("/")
            .same_site(SameSite::Lax)
            .http_only(true)
            .finish();

        cookie
    }
}

fn create_user_jwt_cookie(jwt_secret: &str, user: &User) -> Result<Cookie<'static>> {
    let claims = UserTokenClaims::new(user);
    let token = Token::build(claims, jwt_secret)?;
    
    Ok(token.into())
}
