use axum::{
    extract::State,
    response::{IntoResponse, Redirect},
    Form, http::{Request, StatusCode, header}, middleware::Next, Json,
};
use axum_extra::extract::{CookieJar, cookie::{Cookie, SameSite}};
use color_eyre::eyre::Result;
use jsonwebtoken::{encode, Header, EncodingKey, decode, DecodingKey, Validation};
use serde::{Deserialize, Serialize};
use sqlx::{Pool, Sqlite};
use validator::Validate;

use crate::{AppState};

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
    let cookie = create_user_jwt_cookie(&state.config.jwt_secret, &user).unwrap();
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

#[derive(Debug, Serialize, Deserialize)]
struct TokenClaims {
    sub: String,
    iat: usize,
    exp: usize,
}

fn create_user_jwt_cookie(jwt_secret: &str, user: &User) -> Result<Cookie<'static>> {
    let now = chrono::Utc::now();
    let iat = now.timestamp() as usize;
    let exp = (now + chrono::Duration::minutes(60)).timestamp() as usize;
    let claims: TokenClaims = TokenClaims {
        sub: user.uid.clone(),
        exp,
        iat,
    };

    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(jwt_secret.as_ref()),
    )?;

    let cookie = Cookie::build("token", token.to_owned())
        .path("/")
        .same_site(SameSite::Lax)
        .http_only(true)
        .finish();

    return Ok(cookie);
}

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub status: &'static str,
    pub message: String,
}

#[allow(dead_code)]
async fn auth<B>(
    cookie_jar: CookieJar,
    State(data): State<AppState>,
    mut req: Request<B>,
    next: Next<B>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    let token = cookie_jar
        .get("token")
        .map(|cookie| cookie.value().to_string())
        .or_else(|| {
            req.headers()
                .get(header::AUTHORIZATION)
                .and_then(|auth_header| auth_header.to_str().ok())
                .and_then(|auth_value| {
                    if auth_value.starts_with("Bearer ") {
                        Some(auth_value[7..].to_owned())
                    } else {
                        None
                    }
                })
        });
    
    if let Some(token) = token {
        let claims = decode::<TokenClaims>(
            &token,
            &DecodingKey::from_secret(data.config.jwt_secret.as_ref()),
            &Validation::default(),
        )
        .map_err(|_| {
            let json_error = ErrorResponse {
                status: "fail",
                message: "Invalid token".to_string(),
            };
            (StatusCode::UNAUTHORIZED, Json(json_error))
        })?
        .claims;
    
        let user_id = uuid::Uuid::parse_str(&claims.sub).map_err(|_| {
            let json_error = ErrorResponse {
                status: "fail",
                message: "Invalid token".to_string(),
            };
            (StatusCode::UNAUTHORIZED, Json(json_error))
        })?;
        req.extensions_mut().insert(user_id.to_string());
        return Ok(next.run(req).await);
    } else {
        let json_error = ErrorResponse {
            status: "fail",
            message: "Token not set".to_string(),
        };
        return Err((StatusCode::UNAUTHORIZED, Json(json_error)));
    }
}