use std::convert::Infallible;

use axum::extract::FromRequestParts;
use axum_extra::extract::CookieJar;
use chrono::{Duration, Utc};
use jsonwebtoken::{DecodingKey, EncodingKey, Header, decode, encode};
use password_auth::VerifyError;
use serde::{Deserialize, Serialize};

use crate::{app::AppState, error::AppError, repository::Repository};

pub struct UnauthenticatedUser {
    username: String,
    password: String,
}

impl UnauthenticatedUser {
    pub fn new(username: String, password: String) -> Self {
        Self { username, password }
    }

    pub async fn authenticate(&self, repository: &Repository) -> Result<User, AppError> {
        let user_record = match repository.get_user_by_name(&self.username).await? {
            Some(user_record) => user_record,
            None => return Err(AppError::UserDoesNotExist),
        };

        match password_auth::verify_password(&self.password, &user_record.password_hash) {
            Ok(()) => Ok(User::new(user_record.id, user_record.username)),
            Err(VerifyError::PasswordInvalid) => Err(AppError::InvalidCredentials),
            Err(VerifyError::Parse(err)) => panic!("Hashing algorithm failed: {err}"),
        }
    }

    pub async fn register(self, repository: &Repository) -> Result<User, AppError> {
        let password_hash = password_auth::generate_hash(self.password);
        let user_record = match repository.add_user(&self.username, &password_hash).await {
            Ok(user_record) => user_record,
            Err(sqlx::Error::Database(db_err)) if db_err.is_unique_violation() => {
                return Err(AppError::UsernameTaken);
            }
            Err(err) => return Err(AppError::Database(err)),
        };

        Ok(User::new(user_record.id, user_record.username))
    }
}

pub struct User {
    id: i64,
    username: String,
}

impl User {
    fn new(id: i64, username: String) -> Self {
        Self { id, username }
    }

    pub const fn username(&self) -> &String {
        &self.username
    }

    pub const fn id(&self) -> i64 {
        self.id
    }

    /// Gera o JWT de sessão, assinado com a chave e válido pelo tempo
    /// definidos em `Config` (variáveis `JWT_SECRET` / `JWT_EXPIRY_HOURS`).
    pub fn auth_token(
        self,
        encoding_key: &EncodingKey,
        expiry_hours: u64,
    ) -> Result<String, AppError> {
        let claims = UserClaims::from(self);
        let exp = Utc::now() + Duration::hours(expiry_hours as i64);
        let token = encode(
            &Header::default(),
            &Claims {
                sub: claims.id.to_string(),
                exp: exp.timestamp() as usize,
                data: claims,
            },
            encoding_key,
        )?;
        Ok(token)
    }

    pub fn from_auth_token(token: &str, decoding_key: &DecodingKey) -> Result<Self, AppError> {
        let token_data = decode::<Claims>(
            token,
            decoding_key,
            &jsonwebtoken::Validation::default(),
        )?;
        let claims = token_data.claims.data;
        Ok(Self::new(claims.id, claims.username))
    }
}

impl FromRequestParts<AppState> for User {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let jar = CookieJar::from_headers(&parts.headers);

        let token = match jar.get("token") {
            Some(token) => token.value(),
            None => return Err(AppError::MissingAuthorization),
        };

        User::from_auth_token(token, &state.config.jwt_decoding_key)
    }
}

impl FromRequestParts<AppState> for Option<User> {
    type Rejection = Infallible;

    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        Ok(User::from_request_parts(parts, state).await.ok())
    }
}

#[derive(Serialize, Deserialize)]
struct Claims {
    sub: String,
    exp: usize,
    #[serde(flatten)]
    data: UserClaims,
}

#[derive(Serialize, Deserialize)]
struct UserClaims {
    id: i64,
    username: String,
}

impl From<User> for UserClaims {
    fn from(User { id, username }: User) -> Self {
        Self { id, username }
    }
}
