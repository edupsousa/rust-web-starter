use axum::{
    extract::State,
    http::{header, Request, StatusCode},
    middleware::Next,
    response::IntoResponse,
    Json,
};

use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use color_eyre::eyre::Result;
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Serialize, Deserialize};
use uuid::Uuid;

use crate::AppState;

#[derive(Debug, Serialize, Deserialize)]
pub struct TokenClaims {
    pub sub: String,
    pub iat: usize,
    pub exp: usize,
}
#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub status: &'static str,
    pub message: String,
}

pub async fn auth<B>(
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
        let (user_id, cookie) = anonymous_login(data);
        req.extensions_mut().insert(user_id.to_string());
        let mut res = next.run(req).await;
        res.headers_mut().insert(header::SET_COOKIE, cookie.to_string().parse().unwrap());
        return Ok(res);
    }
}

fn anonymous_login(data: AppState) -> (Uuid, Cookie<'static>) {
    let user_id = Uuid::new_v4();

    let now = chrono::Utc::now();
    let iat = now.timestamp() as usize;
    let exp = (now + chrono::Duration::minutes(60)).timestamp() as usize;
    let claims: TokenClaims = TokenClaims {
        sub: user_id.to_string(),
        exp,
        iat,
    };

    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(data.config.jwt_secret.as_ref()),
    )
    .unwrap();

    let cookie = Cookie::build("token", token.to_owned())
        .path("/")
        .same_site(SameSite::Lax)
        .http_only(true)
        .finish();

    return (user_id, cookie);
}

pub fn create_token_cookie(jwt_secret: &str, uid: &str) -> Result<Cookie<'static>> {
    let now = chrono::Utc::now();
    let iat = now.timestamp() as usize;
    let exp = (now + chrono::Duration::minutes(60)).timestamp() as usize;
    let claims: TokenClaims = TokenClaims {
        sub: uid.to_string(),
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