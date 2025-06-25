use sqlx::types::chrono::Utc;
use uuid::Uuid;
use actix_web::{web, HttpResponse};
use sqlx::{PgConnection, PgPool};
use crate::telemetry::init_subscriber;
use crate::domain::{NewSubscriber, SubscriberName, SubscriberEmail};
use crate::email_client::EmailClient;
use unicode_segmentation::UnicodeSegmentation;

#[derive(serde::Deserialize)]
pub struct FormData {
    email : String,
    name: String,
}

#[tracing::instrument(
    name = "Adding a new subscriber",
    skip(form, pool, email_client),
    fields(
        subscriber_email = %form.email,
        subscriber_name = %form.name
    )
)]
pub async fn subscribe(
    form: web::Form<FormData>,
    pool: web::Data<PgPool>,
    email_client: web::Data<EmailClient>,
) -> HttpResponse {
    let new_subscriber = match form.0.try_into() {
        Ok(subscriber) => subscriber,
        Err(_) => return HttpResponse::BadRequest().finish(),
    };
    
    if insert_subscriber(&pool, &new_subscriber).await.is_err() {
        return HttpResponse::InternalServerError().finish();
    }
    
    if send_confirmation_email(&email_client, new_subscriber).await.is_err()
    {
        HttpResponse::InternalServerError().finish();
    }
    
    HttpResponse::Ok().finish()
}

#[tracing::instrument(
    name = "Send a confirmation email to our new subscriber",
    skip(email_client, new_subscriber),
)]
pub async fn send_confirmation_email(
    email_client: &EmailClient,
    new_subscriber: NewSubscriber,
) -> Result<(), reqwest::Error> {

    let confirmation_link =
        "https://there-is-no-such-domain.com/subscriptions/confirm";
    
    let plain_body = format!(
        "Welcome to our newsletter! \n Visit {} to confirm your subscription.",
        confirmation_link
    );
    
    let html_body = format!(
        "Welcome to our newsletter!<br />\
        Click <a href=\"{}\">here</a> to confirm your subscription.",
        confirmation_link
    );
    
    email_client
        .send_email(
            new_subscriber.email,
            "Welcome!",
            &html_body,
            &plain_body
        )
        .await
}

#[tracing::instrument(
    name = "Saving new subscriber details in database",
    skip(new_subscriber, pool)
)]
pub async fn insert_subscriber(
    pool: &PgPool,
    new_subscriber: &NewSubscriber,
) -> Result<(), sqlx::Error> {

        sqlx::query!(
            r#"
                INSERT INTO subscriptions (id, email, name, subscribed_at, status)
                VALUES ($1, $2, $3, $4, 'pending_confirmation')
            "#,
            Uuid::new_v4(),
            new_subscriber.email.as_ref(),
            new_subscriber.name.as_ref(),
            Utc::now(),
        )
        .execute(pool)
            .await
            .map_err(|e| {
                tracing::error!("Failed to execute query: {:?}", e);
                e
            })?;
        Ok(())
}

impl TryFrom<FormData> for NewSubscriber {
    type Error = String;

    fn try_from(value: FormData) -> Result<Self, Self::Error> {
        let name = SubscriberName::parse(value.name)?;
        let email = SubscriberEmail::parse(value.email)?;
        
        Ok(Self {email, name})
    }
}