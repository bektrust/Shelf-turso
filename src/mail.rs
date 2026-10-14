//! The SMTP side of the profile form.
//!
//! The account the mail goes out from is named by the environment: `SMTP_HOST`
//! (default `smtp.gmail.com`), `SMTP_PORT` (default `587`), `SMTP_USER` and
//! `SMTP_PASSWORD`. See `.env.example`.

use lettre::{
    SmtpTransport, Transport,
    message::{Mailbox, Message},
    transport::smtp::authentication::Credentials,
};

/// The address the profile form is sent to.
const OFFICE: &str = "utsbishkek@gmail.com";

/// Sends `body` to [`OFFICE`] with the given `subject`.
pub async fn send_profile(subject: String, body: String) -> Result<(), String> {
    // SMTP is blocking and slow, so it runs off the runtime's worker threads
    // rather than holding one for the length of a conversation with the server.
    tokio::task::spawn_blocking(move || send_blocking(subject, body))
        .await
        .map_err(|error| format!("mail task: {error}"))?
}

fn send_blocking(subject: String, body: String) -> Result<(), String> {
    let user = setting("SMTP_USER")?;
    let password = setting("SMTP_PASSWORD")?;
    let host = std::env::var("SMTP_HOST").unwrap_or_else(|_| "smtp.gmail.com".to_owned());
    let port: u16 = std::env::var("SMTP_PORT")
        .ok()
        .and_then(|port| port.parse().ok())
        .unwrap_or(587);

    let message = Message::builder()
        .from(mailbox(&user)?)
        .to(mailbox(OFFICE)?)
        .subject(subject)
        .body(body)
        .map_err(|error| error.to_string())?;

    // 465 speaks TLS from the first byte; 587, the usual port, upgrades to it
    // with STARTTLS.
    let relay = if port == 465 {
        SmtpTransport::relay(&host)
    } else {
        SmtpTransport::starttls_relay(&host)
    }
    .map_err(|error| error.to_string())?;

    relay
        .port(port)
        .credentials(Credentials::new(user, password))
        .build()
        .send(&message)
        // The server's reply says nothing the caller can act on.
        .map(|_response| ())
        .map_err(|error| error.to_string())
}

fn mailbox(address: &str) -> Result<Mailbox, String> {
    address
        .parse()
        .map_err(|error| format!("{address}: {error}"))
}

/// The value of the environment variable `name`, or an error naming it and
/// where to put it.
fn setting(name: &str) -> Result<String, String> {
    std::env::var(name).map_err(|_| format!("{name} is not set — add it to .env (see .env.example)"))
}
