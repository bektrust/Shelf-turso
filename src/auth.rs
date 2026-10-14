use crate::db::Db;
use argon2::{
    Argon2,
    password_hash::{PasswordHasher, PasswordVerifier, phc::PasswordHash},
};
use time::{Duration, OffsetDateTime};
use uuid::Uuid;

/// The account a login attempt is checked against.
pub struct Account {
    pub id: i64,
    pub password_hash: String,
    /// Whether an operator has switched the account on in Turso.
    pub enable: bool,
}

/// The signed-in user behind a session cookie.
pub struct SessionUser {
    pub login: String,
    /// Whether an operator has switched the account on in Turso.
    pub enable: bool,
}

pub async fn find_user(db: &Db, login: &str) -> Option<Account> {
    let conn = db.inner.connect().ok()?;
    let mut rows = conn
        .query(
            "SELECT id, password_hash, enable FROM users WHERE login = ?",
            [login],
        )
        .await
        .ok()?;

    let row = rows.next().await.ok()??;
    Some(Account {
        id: row.get(0).ok()?,
        password_hash: row.get(1).ok()?,
        enable: enabled(&row, 2),
    })
}

// `enable` as a flag, whatever the column happens to hold.
//
// The value is taken as a [`libsql::Value`] rather than as an integer on
// purpose: `FromValue for i64` answers anything that is not an integer with
// `unreachable!()`, which panics the worker thread instead of failing the
// query. `enable` was added to the table by hand, so a `'true'` text, a
// float, or a `NULL` is all too possible; none of them should cost a request.
fn enabled(row: &libsql::Row, index: i32) -> bool {
    match row.get_value(index) {
        Ok(libsql::Value::Integer(number)) => number != 0,
        Ok(libsql::Value::Real(number)) => number != 0.0,
        Ok(libsql::Value::Text(text)) => matches!(
            text.trim().to_ascii_lowercase().as_str(),
            "1" | "true" | "t" | "yes" | "y"
        ),
        // `NULL`, a blob, or a read that failed: not activated.
        _ => false,
    }
}

pub fn hash_password(password: &str) -> String {
    Argon2::default()
        .hash_password(password.as_bytes())
        .unwrap()
        .to_string()
}

pub fn verify_password(hash: &str, password: &str) -> bool {
    let Ok(parsed) = PasswordHash::new(hash) else {
        return false;
    };
    Argon2::default()
        .verify_password(password.as_bytes(), &parsed)
        .is_ok()
}

pub async fn create_user(db: &Db, login: &str, password: &str) -> Result<i64, String> {
    let conn = db.inner.connect().map_err(|e| e.to_string())?;
    let hash = hash_password(password);

    // A new account starts switched off; an operator turns it on in Turso.
    conn.execute(
        "INSERT INTO users (login, password_hash, enable) VALUES (?, ?, 0)",
        (login, hash),
    )
    .await
    .map_err(|e| e.to_string())?;

    let mut rows = conn
        .query("SELECT id FROM users WHERE login = ?", [login])
        .await
        .map_err(|e| e.to_string())?;

    let row = rows
        .next()
        .await
        .map_err(|e| e.to_string())?
        .ok_or("no user")?;
    Ok(row.get(0).map_err(|e| e.to_string())?)
}

pub async fn create_session(db: &Db, user_id: i64) -> Result<String, String> {
    let conn = db.inner.connect().map_err(|e| e.to_string())?;
    let sid = Uuid::new_v4().to_string();
    let expires_at = (OffsetDateTime::now_utc() + Duration::days(7)).unix_timestamp();

    conn.execute(
        "INSERT INTO sessions (id, user_id, expires_at) VALUES (?, ?, ?)",
        (sid.clone(), user_id, expires_at),
    )
    .await
    .map_err(|e| e.to_string())?;

    Ok(sid)
}

pub async fn user_by_session(db: &Db, session_id: &str) -> Option<SessionUser> {
    let conn = db.inner.connect().ok()?;
    let now = OffsetDateTime::now_utc().unix_timestamp();

    let mut rows = conn
        .query(
            "SELECT users.login, users.enable
             FROM sessions
             JOIN users ON users.id = sessions.user_id
             WHERE sessions.id = ? AND sessions.expires_at > ?",
            (session_id, now),
        )
        .await
        .ok()?;

    let row = rows.next().await.ok()??;
    Some(SessionUser {
        login: row.get(0).ok()?,
        // See `enabled` for why this is not a plain integer read.
        enable: enabled(&row, 1),
    })
}

pub async fn delete_session(db: &Db, session_id: &str) {
    if let Ok(conn) = db.inner.connect() {
        let _ = conn
            .execute("DELETE FROM sessions WHERE id = ?", [session_id])
            .await;
    }
}
