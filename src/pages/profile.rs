use super::session_user;
use crate::{auth::delete_session, db::Db, mail};
use serde::Deserialize;
use topcoat::{
    Result,
    context::{Cx, app_context},
    cookie::{Cookies, cookies},
    router::{
        content::Form,
        error::{redirect, see_other},
        page,
    },
    view::{View, view},
};

/// The second half of the registration: who the new account belongs to.
#[derive(Deserialize)]
pub struct Profile {
    name: String,
    last_name: String,
    born_date: String,
    gender: String,
    company: String,
    email: String,
    phone: String,
}

#[page("/auth/profile")]
pub async fn profile_page(cx: &Cx) -> Result<impl View> {
    let db = app_context::<Db>(cx);

    let Some(user) = session_user(cx, db).await else {
        return Err(redirect("/auth").into());
    };

    Ok(view! {
        // The button stays greyed out and unclickable until every field holds
        // something: an empty `required` field matches `:invalid`. Each part
        // stands on its own — a browser without `:has()` drops the styling, but
        // `required` still holds the submit back.
        <style>
            "
            .profile:has(:invalid) button {
                opacity: .5;
                pointer-events: none;
            }
            "
        </style>

        <h1>"Registration"</h1>
        <p>"Account " (user.login)</p>

        <form class="profile" method="post" action="/auth/profile">
            <label>"Name" <input name="name" required=""></label>
            <label>"Last name" <input name="last_name" required=""></label>
            <label>"Born date" <input name="born_date" type="date" required=""></label>
            <label>
                "Gender"
                <select name="gender" required="">
                    <option value="">"choose"</option>
                    <option value="male">"male"</option>
                    <option value="female">"female"</option>
                    <option value="other">"other"</option>
                </select>
            </label>
            <label>"Company" <input name="company" required=""></label>
            <label>"E-mail" <input name="email" type="email" required=""></label>
            <label>"Phone" <input name="phone" type="tel" required=""></label>
            <button type="submit">"Submit"</button>
        </form>
    })
}

/// Sends the filled-in form to the office and hands the user over to the
/// "wait for activation" page.
///
/// A send that fails answers with a view, not with an error: the letter goes
/// out through the account named in `.env`, and the reason it did not is
/// exactly what has to be readable to fix those settings.
#[page(POST "/auth/profile")]
pub async fn save_profile(cx: &Cx, Form(input): Form<Profile>) -> Result<impl View> {
    let db = app_context::<Db>(cx);

    let Some(user) = session_user(cx, db).await else {
        return Err(see_other("/auth").into());
    };

    let subject = format!("Регистрация: {}", user.login);
    let body = format!(
        "Логин: {login}\n\n\
         Name: {name}\n\
         LastName: {last_name}\n\
         BornDate: {born_date}\n\
         Gender: {gender}\n\
         Company: {company}\n\
         Email: {email}\n\
         Phone: {phone}\n",
        login = user.login,
        name = input.name,
        last_name = input.last_name,
        born_date = input.born_date,
        gender = input.gender,
        company = input.company,
        email = input.email,
        phone = input.phone,
    );

    if let Err(error) = mail::send_profile(subject, body).await {
        // The session stays, so the form can be filled in and sent again.
        return Ok(view! {
            <h1>"Письмо не отправлено"</h1>
            <p>(error)</p>
            <p><a href="/auth/profile">"Заполнить заново"</a></p>
        });
    }

    // The letter is on its way, so the account is out of this user's hands:
    // close the session registration opened and wait for activation.
    let session_id = cookies(cx)
        .get("session_id")
        .map(|id| id.value().to_owned());
    if let Some(session_id) = session_id {
        delete_session(db, &session_id).await;
    }
    // The removal carries the same `Path` the cookie was set with.
    cookies(cx).default_path("/").remove(("session_id", ""));

    Err(see_other("/auth/pending").into())
}
