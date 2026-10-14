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

/// A handler that answers a `POST` sends the browser on with `see_other`, a
/// `303`. The router's `redirect` would be a `307`, which repeats the `POST`
/// at the new URL, while the page there serves `GET`; a `303` is followed by
/// a `GET`, which is what a form submission needs.
#[page(POST "/auth/login")]
pub async fn login(cx: &Cx, Form(input): Form<Credentials>) -> Result<()> {
    let db = app_context::<Db>(cx);

    let verified = match find_user(db, &input.login).await {
        Some((user_id, hash)) if verify_password(&hash, &input.password) => Some(user_id),
        _ => None,
    };

    let session = match verified {
        Some(user_id) => create_session(db, user_id).await.ok(),
        None => None,
    };

    match session {
        Some(sid) => {
            store_session(cx, sid);
            Err(see_other("/").into())
        }
        // Back to the form: the login or the password did not match.
        None => Err(see_other("/auth").into()),
    }
}

#[page(POST "/auth/register")]
pub async fn register(cx: &Cx, Form(input): Form<Credentials>) -> Result<()> {
    let db = app_context::<Db>(cx);

    let session = match create_user(db, &input.login, &input.password).await {
        Ok(user_id) => create_session(db, user_id).await.ok(),
        // The login is taken, so the form comes back.
        Err(_) => None,
    };

    match session {
        Some(sid) => {
            store_session(cx, sid);
            Err(see_other("/").into())
        }
        // `#register` is the fragment the panel selector matches, so a failed
        // registration returns to the form the user filled in.
        None => Err(see_other("/auth#register").into()),
    }
}

#[page("/logout")]
pub async fn logout(cx: &Cx) -> Result<()> {
    let db = app_context::<Db>(cx);

    if let Some(session_id) = cookies(cx).get("session_id") {
        delete_session(db, session_id.value()).await;
    }
    // The removal has to carry the same `Path` the cookie was set with, or the
    // browser will not match it and the cookie stays.
    cookies(cx).default_path("/").remove(("session_id", ""));

    Err(see_other("/auth").into())
}
