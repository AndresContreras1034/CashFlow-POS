//! Envío por SMTP (pensado para Gmail: smtp.gmail.com, 465 TLS o 587 STARTTLS).

use std::time::Duration;

use lettre::message::{header::ContentType, Attachment, Mailbox, MultiPart, SinglePart};
use lettre::transport::smtp::authentication::Credentials;
use lettre::{Address, AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};

/// Gmail admite 25 MB, pero el adjunto crece ~33 % al codificarse.
pub const MAX_ATTACHMENT_BYTES: u64 = 18 * 1024 * 1024;

pub type Transport = AsyncSmtpTransport<Tokio1Executor>;

pub struct SmtpConfig {
    pub host: String,
    pub port: u16,
    pub starttls: bool,
    pub username: String,
    pub password: String,
}

pub fn transport(cfg: &SmtpConfig) -> Result<Transport, String> {
    let builder = if cfg.starttls {
        Transport::starttls_relay(&cfg.host)
    } else {
        Transport::relay(&cfg.host)
    }
    .map_err(|e| format!("Servidor SMTP no válido: {e}"))?;

    Ok(builder
        .port(cfg.port)
        .credentials(Credentials::new(cfg.username.clone(), cfg.password.clone()))
        .timeout(Some(Duration::from_secs(60)))
        .build())
}

pub fn mailbox(name: Option<&str>, address: &str) -> Result<Mailbox, String> {
    let address: Address = address
        .trim()
        .parse()
        .map_err(|_| format!("Correo no válido: {}", address.trim()))?;
    let name = name.map(str::trim).filter(|n| !n.is_empty()).map(str::to_string);
    Ok(Mailbox::new(name, address))
}

fn friendly(error: lettre::transport::smtp::Error) -> String {
    let text = error.to_string();
    let lower = text.to_lowercase();
    if text.contains("535") || lower.contains("username and password not accepted") {
        "Gmail rechazó el usuario o la contraseña. Usa una «contraseña de aplicación» \
         (requiere verificación en dos pasos), no la contraseña normal de la cuenta."
            .to_string()
    } else {
        format!("No se pudo enviar el correo: {text}")
    }
}

pub async fn send(
    transport: &Transport,
    from: &Mailbox,
    to: &str,
    subject: &str,
    body: &str,
    attachment: Option<(&str, &[u8])>,
) -> Result<(), String> {
    let builder = Message::builder()
        .from(from.clone())
        .to(mailbox(None, to)?)
        .subject(subject);

    let message = match attachment {
        Some((name, bytes)) => {
            let content_type = ContentType::parse("application/octet-stream")
                .map_err(|e| format!("Tipo de adjunto inválido: {e}"))?;
            builder.multipart(
                MultiPart::mixed()
                    .singlepart(SinglePart::plain(body.to_string()))
                    .singlepart(Attachment::new(name.to_string()).body(bytes.to_vec(), content_type)),
            )
        }
        None => builder.body(body.to_string()),
    }
    .map_err(|e| format!("No se pudo construir el correo: {e}"))?;

    transport.send(message).await.map(|_| ()).map_err(friendly)
}
