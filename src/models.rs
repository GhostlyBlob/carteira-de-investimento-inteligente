use serde::Serialize;

#[derive(Debug, Serialize, Clone)]
pub struct Asset {
    pub id: i64,
    pub name: String,
    pub unit_value: f64,
}

#[derive(Debug)]
pub struct UserRecord {
    pub id: i64,
    pub username: String,
    pub password_hash: String,
}
