use lettre::{
    message::Mailbox, transport::smtp::authentication::Credentials, Message, SmtpTransport,
    Transport,
};

use crate::error::ApiError;

#[derive(Clone)]
pub struct EmailService {
    smtp_host: String,
    smtp_username: String,
    smtp_password: String,
    from_email_str: String,
    from_name: String,
    frontend_url: String,
}

impl EmailService {
    pub fn new(
        smtp_host: &str,
        smtp_username: &str,
        smtp_password: &str,
        from_email: &str,
        from_name: &str,
        frontend_url: &str,
    ) -> Result<Self, ApiError> {
        // Validate email format
        let _from_mailbox: Mailbox = format!("{} <{}>", from_name, from_email)
            .parse()
            .map_err(|e| ApiError::Email(format!("Invalid from email: {}", e)))?;

        Ok(Self {
            smtp_host: smtp_host.to_string(),
            smtp_username: smtp_username.to_string(),
            smtp_password: smtp_password.to_string(),
            from_email_str: from_email.to_string(),
            from_name: from_name.to_string(),
            frontend_url: frontend_url.to_string(),
        })
    }

    fn create_transport(&self) -> Result<SmtpTransport, ApiError> {
        let credentials = Credentials::new(self.smtp_username.clone(), self.smtp_password.clone());

        let transport = SmtpTransport::relay(&self.smtp_host)
            .map_err(|e| ApiError::Email(format!("Failed to create SMTP transport: {}", e)))?
            .credentials(credentials)
            .build();

        Ok(transport)
    }

    pub fn send_password_reset_email(
        &self,
        to_email: &str,
        username: &str,
        reset_token: &str,
    ) -> Result<(), ApiError> {
        let smtp_transport = self.create_transport()?;
        let from_email: Mailbox = format!("{} <{}>", self.from_name, self.from_email_str)
            .parse()
            .map_err(|e| ApiError::Validation(format!("Invalid from email: {}", e)))?;

        let reset_url = format!("{}/reset-password?token={}", self.frontend_url, reset_token);

        let body = format!(
            "Hi {},\n\nYou requested to reset your password for your Matcha Time account.\n\nReset your password by clicking this link:\n{}\n\nThis link will expire in 1 hour.\n\nIf you didn't request this, you can safely ignore this email.",
            username, reset_url
        );

        let email = Message::builder()
            .from(from_email)
            .to(to_email
                .parse()
                .map_err(|e| ApiError::Validation(format!("Invalid recipient email: {}", e)))?)
            .subject("Reset Your Matcha Time Password")
            .body(body)
            .map_err(|e| ApiError::Email(format!("Failed to build email: {}", e)))?;

        smtp_transport
            .send(&email)
            .map_err(|e| ApiError::Email(format!("Failed to send email: {}", e)))?;

        Ok(())
    }
}
