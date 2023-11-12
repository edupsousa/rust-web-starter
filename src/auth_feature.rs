use std::fmt;

use axum::{
    async_trait,
    extract::{FromRef, FromRequestParts, State},
    http::{request::Parts, StatusCode},
    response::{IntoResponse, Redirect, Response},
    Form, Json, RequestPartsExt,
};
use axum_extra::extract::{
    cookie::{Cookie, SameSite},
    CookieJar,
};
use color_eyre::eyre::Result;
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::{Pool, Sqlite};
use validator::Validate;

use crate::AppState;

pub async fn get_login(State(state): State<AppState>) -> impl IntoResponse {
    let context = LoginTemplate { error: false };

    state.templates.render("login.html", &context).unwrap()
}

pub async fn post_login(
    State(state): State<AppState>,
    jar: CookieJar,
    Form(form): Form<LoginForm>,
) -> impl IntoResponse {
    let validation = form.validate();
    if validation.is_err() {
        let context = LoginTemplate { error: true };
        let html = state.templates.render("login.html", &context).unwrap();
        return html.into_response();
    }
    let user = get_user_if_authenticated(&state.db, &form.username, &form.password).await;
    if user.is_none() {
        let context = LoginTemplate { error: true };
        let html = state.templates.render("login.html", &context).unwrap();
        return html.into_response();
    }
    let user = user.unwrap();
    let claims = UserClaims::new(&user);
    let token = UserToken::build(claims, &state.config.jwt_secret).unwrap();
    
    (jar.add(token.into()), Redirect::to("/chat")).into_response()
}

#[derive(Deserialize, Validate)]
pub struct LoginForm {
    #[validate(length(min = 3))]
    username: String,
    #[validate(length(min = 8))]
    password: String,
}

#[derive(Serialize)]
pub struct LoginTemplate {
    pub error: bool,
}

#[derive(Deserialize, sqlx::FromRow)]
struct User {
    uid: String,
    username: String,
    password: String,
}

async fn get_user_if_authenticated(db: &Pool<Sqlite>, username: &str, password: &str) -> Option<User> {
    let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE username = ?;")
        .bind(username)
        .fetch_optional(db)
        .await;

    match user {
        Ok(Some(user)) => {
            if user.password == password {
                Some(user)
            } else {
                None
            }
        }
        _ => None,
    }
}

#[derive(Serialize, Deserialize)]
pub struct UserClaims {
    iat: usize,
    exp: usize,
    pub sub: String,
    pub name: String,
}

impl UserClaims {
    fn new(user: &User) -> Self {
        let now = chrono::Utc::now();
        let iat = now.timestamp() as usize;
        let exp = (now + chrono::Duration::minutes(60)).timestamp() as usize;
        UserClaims {
            sub: user.uid.clone(),
            exp,
            iat,
            name: user.username.clone(),
        }
    }
}

struct UserToken(String);

impl UserToken {
    fn build(claims: impl Serialize, jwt_secret: &str) -> Result<Self> {
        let token = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(jwt_secret.as_ref()),
        )?;

        Ok(UserToken(token))
    }
}

impl fmt::Display for UserToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<UserToken> for Cookie<'static> {
    fn from(val: UserToken) -> Self {
        let cookie = Cookie::build("token", val.0)
            .path("/")
            .same_site(SameSite::Lax)
            .http_only(true)
            .finish();

        cookie
    }
}

#[async_trait]
impl<S> FromRequestParts<S> for UserClaims
where
    S: Send + Sync,
    AppState: FromRef<S>,
{
    type Rejection = AuthError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let cookie_jar = parts
            .extract_with_state::<CookieJar, _>(state)
            .await
            .map_err(|_| AuthError::MissingToken)?;

        let cookie = cookie_jar.get("token").ok_or(AuthError::MissingToken)?;

        // Get the jwt secret from the app state
        let state = AppState::from_ref(state);
        let key = &DecodingKey::from_secret(state.config.jwt_secret.as_ref());

        // Decode the user data
        let token_data = decode::<UserClaims>(cookie.value(), key, &Validation::default())
            .map_err(|_| AuthError::InvalidToken)?;

        Ok(token_data.claims)
    }
}

pub enum AuthError {
    InvalidToken,
    MissingToken,
}

impl IntoResponse for AuthError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            AuthError::InvalidToken => (StatusCode::BAD_REQUEST, "Invalid token"),
            AuthError::MissingToken => (StatusCode::UNAUTHORIZED, "Missing token."),
        };
        let body = Json(json!({
            "error": error_message,
        }));
        (status, body).into_response()
    }
}
