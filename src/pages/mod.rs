pub mod auth;
pub mod books;
pub mod profile;

use crate::{
    auth::{SessionUser, user_by_session},
    db::Db,
};
use topcoat::{
    context::Cx,
    cookie::{Cookies, cookies},
};

/// The user the request's session cookie belongs to, if any.
///
/// Every page that needs to know who is asking goes through here, so the
/// cookie name lives in one place.
pub async fn session_user(cx: &Cx, db: &Db) -> Option<SessionUser> {
    let session_id = cookies(cx).get("session_id")?;
    user_by_session(db, session_id.value()).await
}
