use crate::db::Db;

#[derive(Clone)]
pub struct Book {
    pub id: i64,
    pub title: String,
    pub author: String,
    pub year: i64,
    pub finished: bool,
}

pub async fn all(db: &Db) -> Vec<Book> {
    let conn = match db.inner.connect() {
        Ok(c) => c,
        Err(_) => return vec![],
    };

    let mut rows = match conn
        .query(
            "SELECT id, title, author, year, finished FROM books ORDER BY id DESC",
            (),
        )
        .await
    {
        Ok(r) => r,
        Err(_) => return vec![],
    };

    let mut items = Vec::new();
    while let Ok(Some(row)) = rows.next().await {
        items.push(Book {
            id: number(&row, 0, 0),
            title: row.get(1).unwrap_or_default(),
            author: row.get(2).unwrap_or_default(),
            year: number(&row, 3, 0),
            finished: number(&row, 4, 0) != 0,
        });
    }
    items
}

pub async fn find(db: &Db, id: i64) -> Option<Book> {
    let conn = db.inner.connect().ok()?;
    let mut rows = conn
        .query(
            "SELECT id, title, author, year, finished FROM books WHERE id = ?",
            [id],
        )
        .await
        .ok()?;

    let row = rows.next().await.ok()??;
    Some(Book {
        id: number(&row, 0, 0),
        title: row.get(1).ok()?,
        author: row.get(2).ok()?,
        year: number(&row, 3, 0),
        finished: number(&row, 4, 0) != 0,
    })
}

/// The integer at `index`, whatever the column happens to hold.
///
/// `row.get::<i64>` is not an option here: libsql answers a value that is not
/// an integer with `unreachable!()`, which panics the worker thread instead of
/// failing the query (`libsql-0.10.0/src/rows.rs:191`). A column declared
/// `TEXT` stores numbers as text, so that panic is easy to reach from a table
/// whose columns were typed by hand.
fn number(row: &libsql::Row, index: i32, default: i64) -> i64 {
    match row.get_value(index) {
        Ok(libsql::Value::Integer(value)) => value,
        Ok(libsql::Value::Real(value)) => value as i64,
        Ok(libsql::Value::Text(text)) => text.trim().parse().unwrap_or(default),
        _ => default,
    }
}

pub async fn create(db: &Db, title: &str, author: &str, year: i64, finished: bool) {
    if let Ok(conn) = db.inner.connect() {
        let _ = conn
            .execute(
                "INSERT INTO books (title, author, year, finished) VALUES (?, ?, ?, ?)",
                (title, author, year, if finished { 1 } else { 0 }),
            )
            .await;
    }
}

pub async fn update(db: &Db, id: i64, title: &str, author: &str, year: i64, finished: bool) {
    if let Ok(conn) = db.inner.connect() {
        let _ = conn
            .execute(
                "UPDATE books SET title = ?, author = ?, year = ?, finished = ? WHERE id = ?",
                (title, author, year, if finished { 1 } else { 0 }, id),
            )
            .await;
    }
}

pub async fn delete(db: &Db, id: i64) {
    if let Ok(conn) = db.inner.connect() {
        let _ = conn.execute("DELETE FROM books WHERE id = ?", [id]).await;
    }
}
