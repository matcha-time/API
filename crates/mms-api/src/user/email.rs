use lettre::{
    message::{header::ContentType, Mailbox},
    transport::smtp::authentication::Credentials,
    Message, SmtpTransport, Transport,
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

        let html_body = format!(
            r#"
<!DOCTYPE html>
<html>
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
</head>
<body style="font-family: Arial, sans-serif; line-height: 1.6; color: #333; max-width: 600px; margin: 0 auto; padding: 20px;">
    <div style="background-color: #f8f9fa; padding: 20px; border-radius: 8px;">
        <h1 style="color: #2c3e50; margin-bottom: 20px;">Password Reset Request</h1>

        <p>Hi <strong>{}</strong>,</p>

        <p>We received a request to reset your password for your Matcha Time account. If you didn't make this request, you can safely ignore this email.</p>

        <p>To reset your password, click the button below:</p>

        <div style="text-align: center; margin: 30px 0;">
            <a href="{}"
               style="background-color: #007bff; color: white; padding: 12px 30px; text-decoration: none; border-radius: 5px; display: inline-block; font-weight: bold;">
                Reset Password
            </a>
        </div>

        <p>Or copy and paste this link into your browser:</p>
        <p style="background-color: #fff; padding: 10px; border-left: 4px solid #007bff; word-break: break-all;">
            <a href="{}" style="color: #007bff;">{}</a>
        </p>

        <p style="color: #666; font-size: 14px; margin-top: 30px;">
            <strong>Note:</strong> This link will expire in 1 hour for security reasons.
        </p>

        <hr style="border: none; border-top: 1px solid #ddd; margin: 30px 0;">

        <p style="color: #999; font-size: 12px;">
            If you didn't request a password reset, please ignore this email or contact support if you have concerns.
        </p>
    </div>
</body>
</html>
            "#,
            username, reset_url, reset_url, reset_url
        );

        let email = Message::builder()
            .from(from_email)
            .to(to_email
                .parse()
                .map_err(|e| ApiError::Validation(format!("Invalid recipient email: {}", e)))?)
            .subject("Reset Your Matcha Time Password")
            .header(ContentType::TEXT_HTML)
            .body(html_body)
            .map_err(|e| ApiError::Email(format!("Failed to build email: {}", e)))?;

        smtp_transport
            .send(&email)
            .map_err(|e| ApiError::Email(format!("Failed to send email: {}", e)))?;

        Ok(())
    }
}
