use std::net::SocketAddr;

use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{Html, Json},
    routing::get,
    Router,
};
use serde::{Deserialize, Serialize};
use sqlx::MySqlPool;

#[derive(Debug, Deserialize)]
struct RegisterForm {
    email: String,
    endpoint_id: String,
    display_name: Option<String>,
}

#[derive(Debug, Serialize)]
struct User {
    id: i64,
    email: String,
    endpoint_id: String,
    display_name: String,
}

#[derive(Debug, Deserialize)]
struct ValidateQuery {
    email: String,
    endpoint_id: String,
}

#[derive(Debug, Serialize)]
struct ValidateResult {
    valid: bool,
}

#[derive(Debug, Serialize)]
struct Status {
    email: String,
    endpoint_id: String,
    display_name: String,
    online: bool,
    last_event: Option<String>,
    authorized: bool,
    at: Option<String>,
}

async fn index() -> Html<&'static str> {
    Html(include_str!("../static/index.html"))
}

async fn style() -> ([(&'static str, &'static str); 1], &'static str) {
    (
        [("content-type", "text/css")],
        include_str!("../static/style.css"),
    )
}

async fn app_js() -> ([(&'static str, &'static str); 1], &'static str) {
    (
        [("content-type", "text/javascript")],
        include_str!("../static/app.js"),
    )
}

async fn list_users(State(pool): State<MySqlPool>) -> Result<Json<Vec<User>>, StatusCode> {
    let rows: Vec<User> = sqlx::query_as(
        "SELECT id, email, endpoint_id, display_name FROM users ORDER BY id DESC LIMIT 200",
    )
    .fetch_all(&pool)
    .await
    .map(|v: Vec<(i64, String, String, String)>| {
        v.into_iter()
            .map(|(id, email, endpoint_id, display_name)| User {
                id,
                email,
                endpoint_id,
                display_name,
            })
            .collect()
    })
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(rows))
}

async fn register(
    State(pool): State<MySqlPool>,
    Query(form): Query<RegisterForm>,
) -> Result<Json<User>, (StatusCode, String)> {
    let email = form.email.trim().to_lowercase();
    let endpoint_id = form.endpoint_id.trim().to_string();
    let display_name = form.display_name.unwrap_or_default().trim().to_string();
    if email.is_empty() || endpoint_id.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            "email and endpoint_id required".into(),
        ));
    }
    let r = sqlx::query("INSERT INTO users (email, endpoint_id, display_name) VALUES (?, ?, ?)")
        .bind(&email)
        .bind(&endpoint_id)
        .bind(&display_name)
        .execute(&pool)
        .await
        .map_err(|e| (StatusCode::CONFLICT, format!("insert failed: {e}")))?;
    Ok(Json(User {
        id: r.last_insert_id() as i64,
        email,
        endpoint_id,
        display_name,
    }))
}

async fn validate(
    State(pool): State<MySqlPool>,
    Query(q): Query<ValidateQuery>,
) -> Result<Json<ValidateResult>, StatusCode> {
    let row: Option<(i64,)> =
        sqlx::query_as("SELECT 1 FROM users WHERE email = ? AND endpoint_id = ? LIMIT 1")
            .bind(q.email.trim().to_lowercase())
            .bind(q.endpoint_id.trim())
            .fetch_optional(&pool)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(ValidateResult {
        valid: row.is_some(),
    }))
}

/// One row per enrolled user with their latest presence state.
/// online = last event is a login (no logout since). Purely from
/// presence_log, so the page shows what the console has seen.
async fn status(State(pool): State<MySqlPool>) -> Result<Json<Vec<Status>>, StatusCode> {
    let users: Vec<(String, String, String)> =
        sqlx::query_as("SELECT email, endpoint_id, display_name FROM users ORDER BY id DESC LIMIT 200")
            .fetch_all(&pool)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let mut out = Vec::with_capacity(users.len());
    for (email, endpoint_id, display_name) in users {
        let row: Option<(String, i8, String)> = sqlx::query_as(
            "SELECT event, authorized, DATE_FORMAT(created_at, '%Y-%m-%d %H:%i:%s')
             FROM presence_log WHERE email = ? ORDER BY id DESC LIMIT 1",
        )
        .bind(&email)
        .fetch_optional(&pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        let (last_event, authorized, at) = match row {
            Some((e, a, t)) => (Some(e.clone()), a != 0, Some(t)),
            None => (None, false, None),
        };
        let online = last_event.as_deref() == Some("login");
        out.push(Status {
            email,
            endpoint_id,
            display_name,
            online,
            last_event,
            authorized,
            at,
        });
    }
    Ok(Json(out))
}

pub async fn is_valid(pool: &MySqlPool, email: &str, endpoint_id: &str) -> bool {
    sqlx::query("SELECT 1 FROM users WHERE email = ? AND endpoint_id = ? LIMIT 1")
        .bind(email.trim().to_lowercase())
        .bind(endpoint_id.trim())
        .fetch_optional(pool)
        .await
        .ok()
        .flatten()
        .is_some()
}

/// Append one row to presence_log.
pub async fn log_presence(
    pool: &MySqlPool,
    email: &str,
    endpoint_id: &str,
    event: &str,
    authorized: bool,
) {
    let _ = sqlx::query(
        "INSERT INTO presence_log (email, endpoint_id, event, authorized) VALUES (?, ?, ?, ?)",
    )
    .bind(email)
    .bind(endpoint_id)
    .bind(event)
    .bind(authorized as i8)
    .execute(pool)
    .await;
}

fn schema_table(table: &str) -> &'static str {
    const SCHEMA: &str = include_str!("../../schema.sql");
    const MARKER: &str = "CREATE TABLE IF NOT EXISTS presence_log";
    match table {
        "users" => {
            let end = SCHEMA.find(MARKER).unwrap_or(SCHEMA.len());
            &SCHEMA[..end]
        }
        _ => {
            let start = SCHEMA.find(MARKER).unwrap_or(0);
            &SCHEMA[start..]
        }
    }
}

pub async fn serve(pool: MySqlPool, listen: SocketAddr) -> anyhow::Result<()> {
    for table in ["users", "presence_log"] {
        let ddl = schema_table(table);
        sqlx::query(ddl).execute(&pool).await?;
    }

    let app = Router::new()
        .route("/", get(index))
        .route("/style.css", get(style))
        .route("/app.js", get(app_js))
        .route("/users", get(list_users))
        .route("/status", get(status))
        .route("/register", get(register))
        .route("/validate", get(validate))
        .with_state(pool);

    tracing::info!("HR registry on http://{listen}");
    let listener = tokio::net::TcpListener::bind(listen).await?;
    axum::serve(listener, app).await?;
    Ok(())
}
