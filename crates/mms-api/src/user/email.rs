use lettre::{
    Message, SmtpTransport, Transport, message::Mailbox,
    transport::smtp::authentication::Credentials,
};
use sqlx::types::Uuid;

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
            .map_err(|e| ApiError::Email(format!("Invalid from email: {e}")))?;

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
            .map_err(|e| ApiError::Email(format!("Failed to create SMTP transport: {e}")))?
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
            .map_err(|e| ApiError::Validation(format!("Invalid from email: {e}")))?;

        let reset_url = format!("{}/reset-password?token={}", self.frontend_url, reset_token);

        let body = format!(
            "Hi {},\n\nYou requested to reset your password for your Matcha Time account.\n\nReset your password by clicking this link:\n{}\n\nThis link will expire in 1 hour.\n\nIf you didn't request this, you can safely ignore this email.",
            username, reset_url
        );

        let email = Message::builder()
            .from(from_email)
            .to(to_email
                .parse()
                .map_err(|e| ApiError::Validation(format!("Invalid recipient email: {e}")))?)
            .subject("Reset Your Matcha Time Password")
            .body(body)
            .map_err(|e| ApiError::Email(format!("Failed to build email: {e}")))?;

        smtp_transport
            .send(&email)
            .map_err(|e| ApiError::Email(format!("Failed to send email: {e}")))?;

        Ok(())
    }

    pub fn send_verification_email(
        &self,
        to_email: &str,
        username: &str,
        verification_token: &str,
    ) -> Result<(), ApiError> {
        let smtp_transport = self.create_transport()?;
        let from_email: Mailbox = format!("{} <{}>", self.from_name, self.from_email_str)
            .parse()
            .map_err(|e| ApiError::Validation(format!("Invalid from email: {e}")))?;

        let verification_url = format!(
            "{}/verify-email?token={}",
            self.frontend_url, verification_token
        );

        let body = format!(
            "Hi {},\n\nWelcome to Matcha Time! Please verify your email address to complete your registration.\n\nVerify your email by clicking this link:\n{}\n\nThis link will expire in 24 hours.\n\nIf you didn't create this account, you can safely ignore this email.",
            username, verification_url
        );

        let email = Message::builder()
            .from(from_email)
            .to(to_email
                .parse()
                .map_err(|e| ApiError::Validation(format!("Invalid recipient email: {e}")))?)
            .subject("Verify Your Matcha Time Email")
            .body(body)
            .map_err(|e| ApiError::Email(format!("Failed to build email: {e}")))?;

        smtp_transport
            .send(&email)
            .map_err(|e| ApiError::Email(format!("Failed to send email: {e}")))?;

        Ok(())
    }

    pub fn send_password_changed_email(
        &self,
        to_email: &str,
        username: &str,
    ) -> Result<(), ApiError> {
        let smtp_transport = self.create_transport()?;
        let from_email: Mailbox = format!("{} <{}>", self.from_name, self.from_email_str)
            .parse()
            .map_err(|e| ApiError::Validation(format!("Invalid from email: {e}")))?;

        let body = format!(
            "Hi {},\n\nYour Matcha Time password has been successfully changed.\n\nIf you did not make this change, please contact support immediately and secure your account.\n\nFor security, you can request a password reset at:\n{}/reset-password\n\nBest regards,\nMatcha Time Team",
            username, self.frontend_url
        );

        let email = Message::builder()
            .from(from_email)
            .to(to_email
                .parse()
                .map_err(|e| ApiError::Validation(format!("Invalid recipient email: {e}")))?)
            .subject("Your Matcha Time Password Has Been Changed")
            .body(body)
            .map_err(|e| ApiError::Email(format!("Failed to build email: {e}")))?;

        smtp_transport
            .send(&email)
            .map_err(|e| ApiError::Email(format!("Failed to send email: {e}")))?;

        Ok(())
    }
}

/// Helper function to send verification email if email service is available
/// Logs errors but doesn't fail - useful for registration and resend flows
pub fn send_verification_email_if_available(
    email_service: &Option<EmailService>,
    user_id: Uuid,
    email: &str,
    username: &str,
    verification_token: &str,
) {
    if let Some(email_service) = email_service {
        if let Err(e) = email_service.send_verification_email(email, username, verification_token) {
            tracing::error!(error = %e, user_id = %user_id, "Failed to send verification email");
            // Don't fail the request, user can resend later
        }
    } else {
        tracing::info!(
            user_id = %user_id,
            token = %verification_token,
            "Email service not configured - verification token generated"
        );
    }
}
