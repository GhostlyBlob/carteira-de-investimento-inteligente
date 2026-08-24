use color_eyre::eyre::WrapErr;
use jsonwebtoken::{DecodingKey, EncodingKey};

/// Configuração da aplicação, carregada uma única vez a partir das
/// variáveis de ambiente (veja o `.env`). Isso evita segredos
/// (JWT, chave de admin) hardcoded no código-fonte.
pub struct Config {
    pub database_url: String,
    pub admin_api_key: String,
    pub jwt_encoding_key: EncodingKey,
    pub jwt_decoding_key: DecodingKey,
    pub jwt_expiry_hours: u64,
    pub port: u16,
}

impl Config {
    pub fn from_env() -> color_eyre::Result<Self> {
        let database_url =
            std::env::var("DATABASE_URL").wrap_err("DATABASE_URL não foi definida")?;

        let admin_api_key =
            std::env::var("ADMIN_API_KEY").wrap_err("ADMIN_API_KEY não foi definida")?;

        let jwt_secret = std::env::var("JWT_SECRET").wrap_err("JWT_SECRET não foi definida")?;

        let jwt_expiry_hours: u64 = std::env::var("JWT_EXPIRY_HOURS")
            .unwrap_or_else(|_| "24".to_string())
            .parse()
            .wrap_err("JWT_EXPIRY_HOURS precisa ser um número inteiro")?;

        let port: u16 = std::env::var("PORT")
            .unwrap_or_else(|_| "3000".to_string())
            .parse()
            .wrap_err("PORT precisa ser um número entre 0 e 65535")?;

        Ok(Self {
            database_url,
            admin_api_key,
            jwt_encoding_key: EncodingKey::from_secret(jwt_secret.as_bytes()),
            jwt_decoding_key: DecodingKey::from_secret(jwt_secret.as_bytes()),
            jwt_expiry_hours,
            port,
        })
    }
}
