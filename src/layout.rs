use topcoat::{
    Result,
    router::{Slot, layout},
    view::{View, view},
};

#[layout("/")]
pub async fn root_layout(slot: Slot<'_>) -> Result<impl View> {
    Ok(view! {
        <!DOCTYPE html>
        <html>
            <head>
                <meta charset="utf-8" />
                <title>"Shelf"</title>
                topcoat::dev::script()
            </head>
            <body>(slot)</body>
        </html>
    })
}
