mod assets;
mod users;

use std::convert::Infallible;

use axum::extract::FromRequestParts;
use sqlx::PgPool;

use crate::app::AppState;

/// Camada de acesso a dados. Guarda só o pool de conexões; a lógica de
/// cada consulta fica organizada em `repository/assets.rs` e
/// `repository/users.rs`, ambos implementando métodos para este mesmo tipo.
pub struct Repository {
    db: PgPool,
}

impl FromRequestParts<AppState> for Repository {
    type Rejection = Infallible;

    async fn from_request_parts(
        _parts: &mut axum::http::request::Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        Ok(Self {
            db: state.db.clone(),
        })
    }
}

#[cfg(test)]
impl From<PgPool> for Repository {
    fn from(db: PgPool) -> Self {
        Self { db }
    }
}
