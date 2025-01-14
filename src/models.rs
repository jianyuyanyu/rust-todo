use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use time::OffsetDateTime;

mod timestamp_serializer {
    use serde::{Deserialize, Deserializer, Serializer};
    use time::OffsetDateTime;

    pub fn serialize<S>(date: &OffsetDateTime, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_i64(date.unix_timestamp())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<OffsetDateTime, D::Error>
    where
        D: Deserializer<'de>,
    {
        let timestamp = i64::deserialize(deserializer)?;
        OffsetDateTime::from_unix_timestamp(timestamp).map_err(serde::de::Error::custom)
    }
}

mod optional_timestamp_serializer {
    use serde::{Deserialize, Deserializer, Serializer};
    use time::OffsetDateTime;

    pub fn serialize<S>(date: &Option<OffsetDateTime>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match date {
            Some(date) => serializer.serialize_some(&date.unix_timestamp()),
            None => serializer.serialize_none(),
        }
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<OffsetDateTime>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let timestamp: Option<i64> = Option::deserialize(deserializer)?;
        match timestamp {
            Some(ts) => OffsetDateTime::from_unix_timestamp(ts)
                .map(Some)
                .map_err(serde::de::Error::custom),
            None => Ok(None),
        }
    }
}

#[derive(Deserialize)]
pub struct QueryParams {
    pub ids: Option<String>,
    pub key: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct User {
    pub id: i64,
    pub username: String,
    #[serde(skip_serializing)]
    pub password_hash: String,
    #[serde(with = "timestamp_serializer")]
    pub create_time: OffsetDateTime,
}

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Deserialize)]
pub struct RegisterRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub token: String,
    pub user: User,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: i64, // user id
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct PracticeAction {
    pub id: i64,
    pub user_id: i64, // Add user_id field
    pub name: String,
    #[serde(with = "timestamp_serializer")]
    pub create_time: OffsetDateTime,
    #[serde(with = "optional_timestamp_serializer")]
    pub last_finish_time: Option<OffsetDateTime>,
}

#[derive(FromRow, Debug, Serialize, Deserialize)]
pub struct CreateActionRequest {
    pub name: String,
}

#[derive(FromRow, Debug, Serialize, Deserialize)]
pub struct PracticeRecord {
    pub id: i64,
    pub action_id: i64,
    #[serde(with = "timestamp_serializer")]
    pub finish_time: OffsetDateTime,
    pub note: Option<String>,
}

#[derive(Debug, Serialize, FromRow, Deserialize)]
pub struct ActionWithStats {
    pub id: i64,
    pub user_id: i64, // Add user_id field
    pub name: String,
    #[serde(with = "timestamp_serializer")]
    pub create_time: OffsetDateTime,
    #[serde(with = "optional_timestamp_serializer")]
    pub last_finish_time: Option<OffsetDateTime>,
    pub total_finished: i64,
    pub finished_today: bool,
}
