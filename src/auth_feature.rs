use std::fmt;

use axum::{
    extract::{State, FromRequestParts, FromRef},
    response::{IntoResponse, Redirect, Response},
    Form, async_trait, http::{request::Parts, StatusCode}, Json,
    RequestPartsExt
};
use axum_extra::extract::{
    cookie::{Cookie, SameSite},
    CookieJar,
};
use color_eyre::eyre::Result;
use jsonwebtoken::{encode, EncodingKey, Header, decode, Validation, DecodingKey};
use serde::{Deserialize, Serialize};
use serde_json::json;
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
    
    
    state.templates.render("login.html", &context).unwrap()
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
                Some(user)
            } else {
                None
            }
        }
        _ => None,
    }
}

#[derive(Serialize, Deserialize)]
pub struct UserTokenClaims {
    iat: usize,
    exp: usize,
    pub sub: String,
    pub name: String,
}

impl UserTokenClaims {
    fn new(user: &User) -> Self {
        let now = chrono::Utc::now();
        let iat = now.timestamp() as usize;
        let exp = (now + chrono::Duration::minutes(60)).timestamp() as usize;
        UserTokenClaims {
            sub: user.uid.clone(),
            exp,
            iat,
            name: user.username.clone(),
        }
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

impl From<Token> for Cookie<'static> {
    fn from(val: Token) -> Self {
        let cookie = Cookie::build("token", val.0)
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

#[async_trait]
impl<S> FromRequestParts<S> for UserTokenClaims 
where
    S: Send + Sync,
    AppState: FromRef<S>,
{
    type Rejection = AuthError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let cookie_jar = parts.extract_with_state::<CookieJar, _>(state).await.map_err(|_| AuthError::MissingToken)?;
        
        let cookie = cookie_jar
            .get("token")
            .ok_or(AuthError::MissingToken)?;

        // Get the jwt secret from the app state
        let state = AppState::from_ref(state);
        let key = &DecodingKey::from_secret(state.config.jwt_secret.as_ref());
        
        // Decode the user data
        let token_data = decode::<UserTokenClaims>(cookie.value(), key, &Validation::default())
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
            AuthError::MissingToken => (StatusCode::UNAUTHORIZED, "Missing token.")
        };
        let body = Json(json!({
            "error": error_message,
        }));
        (status, body).into_response()
    }
}
