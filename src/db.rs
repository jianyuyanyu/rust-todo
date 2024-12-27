use sqlx::PgPool;
use time::OffsetDateTime;

use crate::models::{ActionWithStats, PracticeAction, PracticeRecord, User};

pub async fn init_db(db_url: &str) -> Result<PgPool, sqlx::Error> {
    let pool = PgPool::connect(db_url).await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS users (
            id BIGSERIAL PRIMARY KEY,
            username TEXT NOT NULL UNIQUE,
            password_hash TEXT NOT NULL,
            create_time TIMESTAMPTZ NOT NULL
        )
        "#,
    )
    .execute(&pool)
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS practice_action (
            id BIGSERIAL PRIMARY KEY,
            user_id BIGINT NOT NULL REFERENCES users(id),
            name TEXT NOT NULL,
            create_time TIMESTAMPTZ NOT NULL,
            last_finish_time TIMESTAMPTZ
        )
        "#,
    )
    .execute(&pool)
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS practice_record (
            id BIGSERIAL PRIMARY KEY,
            action_id BIGINT NOT NULL REFERENCES practice_action(id),
            finish_time TIMESTAMPTZ NOT NULL,
            note TEXT
        )
        "#,
    )
    .execute(&pool)
    .await?;

    Ok(pool)
}

pub async fn create_user(
    pool: &PgPool,
    username: &str,
    password_hash: &str,
) -> Result<User, sqlx::Error> {
    let now = OffsetDateTime::now_utc();

    let user = sqlx::query_as::<_, User>(
        r#"
        INSERT INTO users (username, password_hash, create_time)
        VALUES ($1, $2, $3)
        RETURNING id, username, password_hash, create_time
        "#,
    )
    .bind(username)
    .bind(password_hash)
    .bind(now)
    .fetch_one(pool)
    .await?;

    Ok(user)
}

pub async fn get_user_by_username(
    pool: &PgPool,
    username: &str,
) -> Result<Option<User>, sqlx::Error> {
    let user = sqlx::query_as::<_, User>(
        r#"
        SELECT id, username, password_hash, create_time
        FROM users
        WHERE username = $1
        "#,
    )
    .bind(username)
    .fetch_optional(pool)
    .await?;

    Ok(user)
}

pub async fn create_practice_action(
    pool: &PgPool,
    user_id: i64,
    name: String,
) -> Result<PracticeAction, sqlx::Error> {
    let now = OffsetDateTime::now_utc();
    println!("now: {}, uid: {} name {}", now, user_id, name);

    let action = sqlx::query_as::<_, PracticeAction>(
        r#"
        INSERT INTO practice_action (user_id, name, create_time)
        VALUES ($1, $2, $3)
        RETURNING id, user_id, name, create_time, last_finish_time
        "#,
    )
    .bind(user_id)
    .bind(name)
    .bind(now)
    .fetch_one(pool)
    .await?;
    println!("action created: {:?}", action);

    Ok(action)
}

pub async fn get_practice_action(
    pool: &PgPool,
    user_id: i64,
    id: i64,
) -> Result<Option<PracticeAction>, sqlx::Error> {
    let action = sqlx::query_as::<_, PracticeAction>(
        r#"
        SELECT id, user_id, name, create_time, last_finish_time
        FROM practice_action 
        WHERE id = $1 AND user_id = $2
        "#,
    )
    .bind(id)
    .bind(user_id)
    .fetch_optional(pool)
    .await?;

    Ok(action)
}

pub async fn list_actions_with_stats(
    pool: &PgPool,
    user_id: i64,
) -> Result<Vec<ActionWithStats>, sqlx::Error> {
    let actions = sqlx::query_as::<_, ActionWithStats>(
        r#"
        WITH today_completions AS (
            SELECT action_id, true as completed
            FROM practice_record
            WHERE DATE(finish_time) = CURRENT_DATE
        ),
        completion_counts AS (
            SELECT action_id, COUNT(*) as total_count
            FROM practice_record
            GROUP BY action_id
        )
        SELECT 
            a.id as id,
            a.user_id as user_id,
            a.name as name,
            a.create_time as create_time,
            a.last_finish_time as last_finish_time,
            COALESCE(cc.total_count, 0) as total_finished,
            COALESCE(tc.completed, false) as finished_today
        FROM practice_action a
        LEFT JOIN completion_counts cc ON a.id = cc.action_id
        LEFT JOIN today_completions tc ON a.id = tc.action_id
        WHERE a.user_id = $1
        ORDER BY finished_today ASC, last_finish_time DESC NULLS LAST, create_time DESC
        "#,
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;

    Ok(actions)
}

pub async fn get_practice_records(
    pool: &PgPool,
    user_id: i64,
    action_id: i64,
) -> Result<Vec<PracticeRecord>, sqlx::Error> {
    let records = sqlx::query_as::<_, PracticeRecord>(
        r#"
        SELECT r.id, r.action_id, r.finish_time, r.note
        FROM practice_record r
        JOIN practice_action a ON r.action_id = a.id
        WHERE r.action_id = $1 AND a.user_id = $2
        ORDER BY r.finish_time DESC
        "#,
    )
    .bind(action_id)
    .bind(user_id)
    .fetch_all(pool)
    .await?;

    Ok(records)
}

pub async fn can_finish_today(
    pool: &PgPool,
    user_id: i64,
    action_id: i64,
) -> Result<bool, sqlx::Error> {
    let today = OffsetDateTime::now_utc().date();

    let count: Option<i64> = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)
        FROM practice_record r
        JOIN practice_action a ON r.action_id = a.id
        WHERE r.action_id = $1 
        AND a.user_id = $2
        AND DATE(r.finish_time) = DATE($3)
        "#,
    )
    .bind(action_id)
    .bind(user_id)
    .bind(today)
    .fetch_one(pool)
    .await?;

    Ok(count.unwrap_or(0) == 0)
}

pub async fn create_practice_record(
    pool: &PgPool,
    user_id: i64,
    action_id: i64,
    note: Option<String>,
) -> Result<PracticeRecord, sqlx::Error> {
    let now = OffsetDateTime::now_utc();

    // Verify user owns the action
    let action_exists: Option<bool> = sqlx::query_scalar(
        r#"
        SELECT EXISTS(
            SELECT 1 FROM practice_action
            WHERE id = $1 AND user_id = $2
        )
        "#,
    )
    .bind(action_id)
    .bind(user_id)
    .fetch_one(pool)
    .await?;

    if !action_exists.unwrap_or(false) {
        return Err(sqlx::Error::RowNotFound);
    }

    // Update last_finish_time
    sqlx::query(
        r#"
        UPDATE practice_action 
        SET last_finish_time = $1
        WHERE id = $2 AND user_id = $3
        "#,
    )
    .bind(now)
    .bind(action_id)
    .bind(user_id)
    .execute(pool)
    .await?;

    // Create record
    let record = sqlx::query_as::<_, PracticeRecord>(
        r#"
        INSERT INTO practice_record (action_id, finish_time, note)
        VALUES ($1, $2, $3)
        RETURNING id, action_id, finish_time, note
        "#,
    )
    .bind(action_id)
    .bind(now)
    .bind(note)
    .fetch_one(pool)
    .await?;

    Ok(record)
}
