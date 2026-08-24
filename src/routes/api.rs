use axum::{
    Json, Router,
    extract::Path,
    routing::{get, patch},
};
use serde::{Deserialize, Serialize};

use crate::{
    app::AppState,
    auth::{admin::Admin, user::User},
    error::AppError,
    models::Asset,
    repository::Repository,
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/assets", get(list_assets).post(create_asset))
        .route("/assets/{id}", patch(update_asset))
        .route("/me", get(me))
}

#[tracing::instrument(skip_all)]
async fn list_assets(repository: Repository) -> Result<Json<Vec<Asset>>, AppError> {
    let assets = repository.list_assets().await?;
    Ok(Json(assets))
}

#[derive(Deserialize)]
struct CreateAssetRequest {
    name: String,
    unit_value: f64,
}

#[tracing::instrument(skip_all)]
async fn create_asset(
    _: Admin,
    repository: Repository,
    Json(request): Json<CreateAssetRequest>,
) -> Result<Json<Asset>, AppError> {
    let name = request.name.trim();
    if name.is_empty() {
        return Err(AppError::InvalidInput(
            "o nome do ativo não pode ser vazio".into(),
        ));
    }
    if !request.unit_value.is_finite() || request.unit_value < 0.0 {
        return Err(AppError::InvalidInput(
            "unit_value precisa ser um número válido e não-negativo".into(),
        ));
    }

    let new_asset = match repository
        .create_asset(name.to_string(), request.unit_value)
        .await
    {
        Ok(new_asset) => new_asset,
        Err(sqlx::Error::Database(db_err)) if db_err.is_unique_violation() => {
            return Err(AppError::InvalidInput(format!(
                "já existe um ativo chamado '{name}'"
            )));
        }
        Err(err) => return Err(AppError::Database(err)),
    };

    Ok(Json(new_asset))
}

#[derive(Deserialize)]
struct UpdateAssetRequest {
    name: Option<String>,
    unit_value: Option<f64>,
}

#[tracing::instrument(skip_all)]
async fn update_asset(
    _: Admin,
    Path(id): Path<i64>,
    repository: Repository,
    Json(request): Json<UpdateAssetRequest>,
) -> Result<Json<Asset>, AppError> {
    if let Some(name) = &request.name {
        if name.trim().is_empty() {
            return Err(AppError::InvalidInput(
                "o nome do ativo não pode ser vazio".into(),
            ));
        }
    }
    if let Some(unit_value) = request.unit_value {
        if !unit_value.is_finite() || unit_value < 0.0 {
            return Err(AppError::InvalidInput(
                "unit_value precisa ser um número válido e não-negativo".into(),
            ));
        }
    }

    match repository
        .update_asset(id, request.name, request.unit_value)
        .await?
    {
        Some(updated_asset) => Ok(Json(updated_asset)),
        None => Err(AppError::AssetDoesNotExist),
    }
}

#[derive(Serialize)]
struct MeResponse {
    id: i64,
    username: String,
}

/// Endpoint simples pra confirmar que o cookie/JWT está sendo aceito,
/// sem depender de nenhuma página HTML ainda.
#[tracing::instrument(skip_all)]
async fn me(user: User) -> Json<MeResponse> {
    Json(MeResponse {
        id: user.id(),
        username: user.username().clone(),
    })
}

#[cfg(test)]
mod tests {
    use sqlx::PgPool;

    use super::*;

    #[sqlx::test]
    async fn test_create_asset(db: PgPool) {
        let request = CreateAssetRequest {
            name: "Bitcoin".to_string(),
            unit_value: 10.0,
        };
        let Json(new_asset) = create_asset(Admin, db.into(), Json(request))
            .await
            .expect("success");

        assert_eq!(new_asset.id, 1);
        assert_eq!(new_asset.name, "Bitcoin");
        assert_eq!(new_asset.unit_value, 10.0);

        insta::assert_json_snapshot!(new_asset);
    }

    #[sqlx::test]
    async fn test_create_asset_rejects_empty_name(db: PgPool) {
        let request = CreateAssetRequest {
            name: "   ".to_string(),
            unit_value: 10.0,
        };

        let err = create_asset(Admin, db.into(), Json(request))
            .await
            .expect_err("nome vazio deveria ser rejeitado");

        assert!(matches!(err, AppError::InvalidInput(_)));
    }

    #[sqlx::test]
    async fn test_create_asset_rejects_negative_unit_value(db: PgPool) {
        let request = CreateAssetRequest {
            name: "Bitcoin".to_string(),
            unit_value: -1.0,
        };

        let err = create_asset(Admin, db.into(), Json(request))
            .await
            .expect_err("unit_value negativo deveria ser rejeitado");

        assert!(matches!(err, AppError::InvalidInput(_)));
    }

    #[sqlx::test(fixtures("bitcoin_asset"))]
    async fn test_create_asset_rejects_duplicate_name(db: PgPool) {
        let request = CreateAssetRequest {
            name: "Bitcoin".to_string(),
            unit_value: 99.0,
        };

        let err = create_asset(Admin, db.into(), Json(request))
            .await
            .expect_err("nome duplicado deveria ser rejeitado");

        assert!(matches!(err, AppError::InvalidInput(_)));
    }

    #[sqlx::test(fixtures("bitcoin_asset"))]
    async fn test_list_assets(db: PgPool) {
        let Json(assets) = list_assets(db.into()).await.expect("success");

        assert_eq!(assets.len(), 1);
        assert_eq!(assets[0].name, "Bitcoin");

        insta::assert_json_snapshot!(assets);
    }

    #[sqlx::test(fixtures("bitcoin_asset"))]
    async fn test_update_asset(db: PgPool) {
        let request = UpdateAssetRequest {
            name: Some("Ethereum".to_string()),
            unit_value: Some(20.0),
        };

        let Json(updated_asset) = update_asset(Admin, Path(1), db.into(), Json(request))
            .await
            .expect("success");

        assert_eq!(updated_asset.id, 1);
        assert_eq!(updated_asset.name, "Ethereum");
        assert_eq!(updated_asset.unit_value, 20.0);

        insta::assert_json_snapshot!(updated_asset);
    }

    #[sqlx::test(fixtures("bitcoin_asset"))]
    async fn test_update_asset_partial(db: PgPool) {
        let request = UpdateAssetRequest {
            name: None,
            unit_value: Some(15.5),
        };

        let Json(updated_asset) = update_asset(Admin, Path(1), db.into(), Json(request))
            .await
            .expect("success");

        assert_eq!(updated_asset.name, "Bitcoin");
        assert_eq!(updated_asset.unit_value, 15.5);
    }

    #[sqlx::test(fixtures("bitcoin_asset"))]
    async fn test_update_asset_not_found(db: PgPool) {
        let request = UpdateAssetRequest {
            name: Some("Ethereum".to_string()),
            unit_value: None,
        };

        let err = update_asset(Admin, Path(999), db.into(), Json(request))
            .await
            .expect_err("asset inexistente deveria retornar erro");

        assert!(matches!(err, AppError::AssetDoesNotExist));
    }
}
