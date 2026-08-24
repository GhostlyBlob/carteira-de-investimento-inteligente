use askama::Template;
use axum::{
    Form, Router,
    extract::State,
    response::{Html, IntoResponse, Redirect, Response},
    routing::{get, post},
};
use axum_extra::extract::{
    CookieJar,
    cookie::{Cookie, SameSite},
};
use serde::Deserialize;

use crate::{
    app::AppState,
    auth::user::{UnauthenticatedUser, User},
    error::AppError,
    models::Asset,
    repository::Repository,
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(index))
        .route("/login", get(login_page).post(login))
        .route("/logout", post(logout))
}

/// Ativo já formatado para exibição (preço em BRL). Fica só na camada de
/// apresentação — o `Asset` que vem do repository continua "cru".
struct AssetRow {
    name: String,
    unit_value_display: String,
}

impl From<Asset> for AssetRow {
    fn from(asset: Asset) -> Self {
        Self {
            name: asset.name,
            unit_value_display: format_brl(asset.unit_value),
        }
    }
}

/// Formata um número como moeda brasileira (ex.: 1234.5 -> "R$ 1.234,50"),
/// sem depender de nenhuma crate extra de formatação/locale.
fn format_brl(value: f64) -> String {
    let cents = (value * 100.0).round() as i64;
    let sign = if cents < 0 { "-" } else { "" };
    let cents = cents.unsigned_abs();
    let reais = cents / 100;
    let centavos = cents % 100;

    let digits = reais.to_string();
    let mut grouped = String::new();
    for (i, c) in digits.chars().rev().enumerate() {
        if i != 0 && i % 3 == 0 {
            grouped.push('.');
        }
        grouped.push(c);
    }
    let grouped: String = grouped.chars().rev().collect();

    format!("{sign}R$ {grouped},{centavos:02}")
}

#[derive(Template)]
#[template(path = "dashboard.html")]
struct DashboardPage {
    username: String,
    total_value_display: String,
    assets: Vec<AssetRow>,
}

async fn index(maybe_user: Option<User>, repository: Repository) -> Result<Response, AppError> {
    match maybe_user {
        Some(user) => {
            let assets = repository.list_assets().await?;

            // Soma simples do valor unitário de cada ativo do catálogo.
            let mut total_value = 0.0;
            for asset in &assets {
                total_value += asset.unit_value;
            }

            let page = DashboardPage {
                username: user.username().clone(),
                total_value_display: format_brl(total_value),
                assets: assets.into_iter().map(AssetRow::from).collect(),
            };
            Ok(Html(page.render()?).into_response())
        }
        None => Ok(Redirect::to("/login").into_response()),
    }
}

#[derive(Template)]
#[template(path = "login.html")]
struct LoginPage;

async fn login_page() -> Result<Html<String>, AppError> {
    let html = LoginPage.render()?;
    Ok(Html(html))
}

#[derive(Deserialize)]
struct LoginForm {
    username: String,
    password: String,
}

/// Um único formulário cuida de login e cadastro: se o usuário já existe,
/// autentica; se não existe, cadastra na hora e já autentica em seguida.
async fn login(
    repository: Repository,
    State(state): State<AppState>,
    jar: CookieJar,
    Form(request): Form<LoginForm>,
) -> Result<impl IntoResponse, AppError> {
    let username = request.username.trim();
    if username.is_empty() {
        return Err(AppError::InvalidInput(
            "o nome de usuário não pode ser vazio".into(),
        ));
    }
    if request.password.len() < 6 {
        return Err(AppError::InvalidInput(
            "a senha precisa ter ao menos 6 caracteres".into(),
        ));
    }

    let unauth_user = UnauthenticatedUser::new(username.to_string(), request.password);
    let user = match unauth_user.authenticate(&repository).await {
        Ok(user) => user,
        Err(AppError::UserDoesNotExist) => unauth_user.register(&repository).await?,
        Err(other_err) => return Err(other_err),
    };

    let token = user.auth_token(&state.config.jwt_encoding_key, state.config.jwt_expiry_hours)?;
    let cookie = Cookie::build(("token", token))
        .http_only(true)
        .same_site(SameSite::Lax)
        .path("/");

    Ok((jar.add(cookie), Redirect::to("/")))
}

async fn logout(jar: CookieJar) -> impl IntoResponse {
    let expired = Cookie::build(("token", ""))
        .http_only(true)
        .same_site(SameSite::Lax)
        .path("/");

    (jar.remove(expired), Redirect::to("/login"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_total_value_sum() {
        let values = [10.0, 20.5, 0.25];
        let mut total = 0.0;
        for v in values {
            total += v;
        }
        assert_eq!(format_brl(total), "R$ 30,75");
    }

    #[test]
    fn test_format_brl() {
        assert_eq!(format_brl(10.0), "R$ 10,00");
        assert_eq!(format_brl(1234.5), "R$ 1.234,50");
        assert_eq!(format_brl(100_000.0), "R$ 100.000,00");
        assert_eq!(format_brl(0.0), "R$ 0,00");
        assert_eq!(format_brl(999.99), "R$ 999,99");
    }
}
