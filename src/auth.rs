use crate::db::Db;
use argon2::{
    Argon2,
    password_hash::{PasswordHasher, PasswordVerifier, phc::PasswordHash},
};
use time::{Duration, OffsetDateTime};
use uuid::Uuid;

pub async fn find_user(db: &Db, login: &str) -> Option<(i64, String)> {
    let conn = db.inner.connect().ok()?;
    let mut rows = conn
        .query(
            "SELECT id, password_hash FROM users WHERE login = ?",
            [login],
        )
        .await
        .ok()?;

    let row = rows.next().await.ok()??;
    Some((row.get(0).ok()?, row.get(1).ok()?))
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

    conn.execute(
        "INSERT INTO users (login, password_hash) VALUES (?, ?)",
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

pub async fn user_by_session(db: &Db, session_id: &str) -> Option<(i64, String)> {
    let conn = db.inner.connect().ok()?;
    let now = OffsetDateTime::now_utc().unix_timestamp();

    let mut rows = conn
        .query(
            "SELECT users.id, users.login
             FROM sessions
             JOIN users ON users.id = sessions.user_id
             WHERE sessions.id = ? AND sessions.expires_at > ?",
            (session_id, now),
        )
        .await
        .ok()?;

    let row = rows.next().await.ok()??;
    Some((row.get(0).ok()?, row.get(1).ok()?))
}

pub async fn delete_session(db: &Db, session_id: &str) {
    if let Ok(conn) = db.inner.connect() {
        let _ = conn
            .execute("DELETE FROM sessions WHERE id = ?", [session_id])
            .await;
    }
}
