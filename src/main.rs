mod auth;
mod books;
mod db;
mod layout;
mod mail;
mod pages;

use db::Db;
use topcoat::{
    Result,
    cookie::RouterBuilderCookieExt,
    router::{Router, RouterBuilderDiscoverExt},
};

#[tokio::main]
async fn main() -> Result<()> {
    let db = Db::connect().await;

    let router = Router::builder()
        .discover()
        .cookies()
        .app_context(db)
        .build();

    topcoat::start(router).await?;
    Ok(())
}
