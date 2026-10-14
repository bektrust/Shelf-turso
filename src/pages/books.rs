use super::session_user;
use crate::{books::*, db::Db};
use serde::Deserialize;
use topcoat::{
    Result,
    context::{Cx, app_context},
    router::{
        content::Form,
        error::{redirect, see_other},
        page, path_param,
    },
    view::{View, view},
};

path_param!(book_id: i64, error = not_found);

#[derive(Deserialize)]
pub struct BookForm {
    title: String,
    author: String,
    year: i64,
    /// A checkbox sends its value only while it is checked.
    finished: Option<String>,
}

#[page("/")]
pub async fn index(cx: &Cx) -> Result<impl View> {
    let db = app_context::<Db>(cx);

    let Some(user) = session_user(cx, db).await else {
        return Err(redirect("/auth").into());
    };

    // Registered but not switched on yet: the same page the login form sends
    // these users to.
    if !user.enable {
        return Err(redirect("/auth/pending").into());
    }

    let books = all(db).await;

    Ok(view! {
        <h1>"Books"</h1>
        <p>"Signed in as " (user.login) " — " <a href="/logout">"Logout"</a></p>

        <form method="post" action="/books/create">
            <input name="title" placeholder="title">
            <input name="author" placeholder="author">
            <input name="year" type="number" placeholder="year">
            <label>
                <input name="finished" type="checkbox">
                "finished"
            </label>
            <button type="submit">"Create"</button>
        </form>

        <ul>
            for book in books {
                <li>
                    (book.title) " — " (book.author) " (" (book.year) ") "
                    <a href=(format!("/books/{}", book.id))>"Edit"</a>
                    <form method="post" action=(format!("/books/{}/delete", book.id)) style="display:inline;">
                        <button type="submit">"Delete"</button>
                    </form>
                </li>
            }
        </ul>
    })
}

#[page("/books/{book_id}")]
pub async fn edit(cx: &Cx) -> Result<impl View> {
    let db = app_context::<Db>(cx);
    let id = path_param::<BookId>(cx)?;
    let book = find(db, *id)
        .await
        .ok_or_else(|| topcoat::Error::msg("book not found"))?;

    Ok(view! {
        <h1>"Edit book"</h1>
        <form method="post" action=(format!("/books/{}/update", book.id))>
            <input name="title" value=(book.title)>
            <input name="author" value=(book.author)>
            <input name="year" type="number" value=(book.year)>
            <label>
                <input name="finished" type="checkbox" checked=(book.finished)>
                "finished"
            </label>
            <button type="submit">"Save"</button>
        </form>
    })
}

/// The pages below answer a `POST`, so they send the browser on with
/// `see_other`, a `303` the browser follows with a `GET`; see
/// [`crate::pages::auth`] for why the router's `redirect` (a `307`, which
/// repeats the `POST`) is not what a form submission needs.
#[page(POST "/books/create")]
pub async fn create_book(cx: &Cx, Form(input): Form<BookForm>) -> Result<()> {
    let db = app_context::<Db>(cx);

    create(
        db,
        &input.title,
        &input.author,
        input.year,
        input.finished.is_some(),
    )
    .await;

    Err(see_other("/").into())
}

#[page(POST "/books/{book_id}/update")]
pub async fn update_book(cx: &Cx, Form(input): Form<BookForm>) -> Result<()> {
    let db = app_context::<Db>(cx);
    let id = path_param::<BookId>(cx)?;

    update(
        db,
        *id,
        &input.title,
        &input.author,
        input.year,
        input.finished.is_some(),
    )
    .await;

    Err(see_other("/").into())
}

#[page(POST "/books/{book_id}/delete")]
pub async fn delete_book(cx: &Cx) -> Result<()> {
    let db = app_context::<Db>(cx);
    let id = path_param::<BookId>(cx)?;

    delete(db, *id).await;

    Err(see_other("/").into())
}
