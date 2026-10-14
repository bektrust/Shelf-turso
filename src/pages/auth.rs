use crate::{auth::*, db::Db};
use serde::Deserialize;
use topcoat::{
    Result,
    context::{Cx, app_context},
    cookie::{Cookies, cookies},
    router::{content::Form, error::see_other, page},
    view::{View, view},
};

#[derive(Deserialize)]
pub struct Credentials {
    login: String,
    password: String,
}

/// Stores the session cookie in the response.
///
/// `Path=/` matters: a cookie without it is scoped by the browser to the
/// directory of the URL that set it, so a cookie set by a `POST` to
/// `/auth/login` would only ever be sent back to `/auth/*` and never to `/`.
fn store_session(cx: &Cx, sid: String) {
    cookies(cx).default_path("/").add(("session_id", sid));
}

#[page("/auth")]
pub async fn auth_page() -> Result<impl View> {
    Ok(view! {
        // Both panels are in the markup; `:target` decides which one shows, so
        // the switch needs no JavaScript and no round trip. With no fragment
        // (or with `#login`) the login panel shows, with `#register` the
        // register one does. Each rule stands alone: a browser without `:has()`
        // drops only the last one and shows both forms rather than neither.
        //
        // Each heading sits inside its own panel, so it follows the switch.
        <style>
            "
            .auth-panel { display: none; }

            #login { display: block; }
            #register:target { display: block; }
            .auth:has(#register:target) #login { display: none; }
            "
        </style>

        <div class="auth">
            <section id="login" class="auth-panel">
                <h1>"Login"</h1>

                <form method="post" action="/auth/login">
                    <input name="login" placeholder="login">
                    <input name="password" type="password" placeholder="password">
                    <button type="submit">"Login"</button>
                </form>

                <p><a href="#register">"Register"</a></p>
            </section>

            <section id="register" class="auth-panel">
                <h1>"Register"</h1>

                <form method="post" action="/auth/register">
                    <input name="login" placeholder="login">
                    <input name="password" type="password" placeholder="password">
                    <button type="submit">"Register"</button>
                </form>

                <p><a href="#login">"Login"</a></p>
            </section>
        </div>
    })
}

/// Where an account that exists but is not switched on yet ends up. The login
/// form sends users here, and so does the profile form once the letter is sent.
#[page("/auth/pending")]
pub async fn pending_page() -> Result<impl View> {
    Ok(view! {
        <h1>"Registration"</h1>
        <p>"Пользователь добавлен в базу, но еще не активирован. Ждите. Об активации Вам сообщат по электронной почте."</p>
        <p><a href="/auth">"Login"</a></p>
    })
}

/// A handler that answers a `POST` sends the browser on with `see_other`, a
/// `303`. The router's `redirect` would be a `307`, which repeats the `POST`
/// at the new URL, while the page there serves `GET`; a `303` is followed by
/// a `GET`, which is what a form submission needs.
#[page(POST "/auth/login")]
pub async fn login(cx: &Cx, Form(input): Form<Credentials>) -> Result<()> {
    let db = app_context::<Db>(cx);

    let account = match find_user(db, &input.login).await {
        Some(account) if verify_password(&account.password_hash, &input.password) => account,
        // Back to the form: no such login, or the password did not match.
        _ => return Err(see_other("/auth").into()),
    };

    // The login and the password are right, but the account is still switched
    // off: activation happens in Turso, by hand.
    if !account.enable {
        return Err(see_other("/auth/pending").into());
    }

    match create_session(db, account.id).await.ok() {
        Some(sid) => {
            store_session(cx, sid);
            Err(see_other("/").into())
        }
        None => Err(see_other("/auth").into()),
    }
}

#[page(POST "/auth/register")]
pub async fn register(cx: &Cx, Form(input): Form<Credentials>) -> Result<()> {
    let db = app_context::<Db>(cx);

    let user_id = match create_user(db, &input.login, &input.password).await {
        Ok(user_id) => user_id,
        // The login is taken, so the form comes back. `#register` is the
        // fragment the panel selector matches.
        Err(_) => return Err(see_other("/auth#register").into()),
    };

    // The session exists only so the profile form of the next step knows which
    // account it belongs to; `save_profile` closes it again. The account is
    // still switched off, so `/` keeps sending this user to the pending page.
    match create_session(db, user_id).await.ok() {
        Some(sid) => {
            store_session(cx, sid);
            Err(see_other("/auth/profile").into())
        }
        None => Err(see_other("/auth/pending").into()),
    }
}

#[page("/logout")]
pub async fn logout(cx: &Cx) -> Result<()> {
    let db = app_context::<Db>(cx);

    // The value is copied out of the jar before the await, so no borrow of it
    // has to live across one.
    let session_id = cookies(cx)
        .get("session_id")
        .map(|id| id.value().to_owned());
    if let Some(session_id) = session_id {
        delete_session(db, &session_id).await;
    }
    // The removal has to carry the same `Path` the cookie was set with, or the
    // browser will not match it and the cookie stays.
    cookies(cx).default_path("/").remove(("session_id", ""));

    Err(see_other("/auth").into())
}
