use crate::domain::{NewSubscriber, SubscriberEmail, SubscriberName};
use crate::email_client::EmailClient;
use crate::startup::ApplicationBaseUrl;
use actix_web::http::StatusCode;
use actix_web::{web, HttpResponse, ResponseError};
use anyhow::Context;
use chrono::Utc;
use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};
use sqlx::{Executor, PgPool, Postgres, Transaction};
use std::convert::{TryFrom, TryInto};
use std::error::Error;
use std::fmt::{Display, Formatter};
use uuid::Uuid;

#[derive(serde::Deserialize)]
pub struct FormData {
    email : String,
    name: String,
}

impl TryFrom<FormData> for NewSubscriber {
    type Error = String;

    fn try_from(value: FormData) -> Result<Self, Self::Error> {
        let name: SubscriberName   = SubscriberName::parse(value.name)?;
        let email: SubscriberEmail = SubscriberEmail::parse(value.email)?;
        Ok(Self {email, name})
    }
}

#[derive(thiserror::Error)]
pub enum SubscriberError {
    #[error("{0}")]
    ValidationError(String),
    
    #[error("transparent")]
    UnexpectedError(#[source] anyhow::Error),
}

impl std::fmt::Debug for SubscriberError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

impl ResponseError for SubscriberError {
    fn status_code(&self) -> StatusCode {
        match self {
            SubscriberError::ValidationError(_) => StatusCode::BAD_REQUEST,
            SubscriberError::UnexpectedError(_) => 
                StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl From<anyhow::Error> for SubscriberError {
    fn from(e: anyhow::Error) -> Self {
        Self::UnexpectedError(e)
    }
}

#[tracing::instrument(
    name = "Adding a new subscriber",
    skip(form, pool, email_client, base_url),
    fields(
        subscriber_email = %form.email,
        subscriber_name = %form.name
    )
)]
pub async fn subscribe(
    form: web::Form<FormData>,
    pool: web::Data<PgPool>,
    email_client: web::Data<EmailClient>,
    base_url: web::Data<ApplicationBaseUrl>,
) -> Result<HttpResponse, SubscriberError> {
    let new_subscriber = form.0.try_into().map_err(SubscriberError::ValidationError)?;

    let mut transaction: Transaction<Postgres> = pool
        .begin()
        .await
        .context("Failed to acquire a Postgres connection from the pool")?;

    let subscriber_id: Uuid =  insert_subscriber(
        &mut transaction, 
        &new_subscriber
    )
    .await
    .context("Failed to insert new subscriber in the database.")?;

    let subscription_token: String= generate_subscription_token();
    store_token(&mut transaction, subscriber_id, &subscription_token)
        .await
        .context("Failed to store the confirmation token for a new subscriber.")?;

    transaction
        .commit()
        .await
        .context("Failed to commit SQL transaction to store a new subscriber")?;
    
    send_confirmation_email(
        &email_client, 
        new_subscriber, 
        &base_url.0,
        &subscription_token
    )
     .await
     .context("Failed to send a confirmation email. ")?;
    Ok(HttpResponse::Ok().finish())
}



pub struct StoreTokenError(sqlx::Error);

impl Display for StoreTokenError {
    fn fmt(&self, f:&mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "A database error was encountered while \
            trying to store a subscription token."
        )
    }
}

impl Error for StoreTokenError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&self.0)
    }
}

impl std::fmt::Debug for StoreTokenError {
    fn fmt(&self, f:&mut Formatter<'_>) -> std::fmt::Result  {
        error_chain_fmt(self, f)
    }
}

#[tracing::instrument(
    name = "Send a confirmation email to our new subscriber",
    skip(email_client, new_subscriber),
)]
pub async fn send_confirmation_email(
    email_client: &EmailClient,
    new_subscriber: NewSubscriber,
    base_url: &str,
    subscription_token: &str
) -> Result<(), reqwest::Error> {

    let confirmation_link = format!(
        "{}/subscriptions/confirm?subscription_token={}", 
        base_url,
        subscription_token
    );
    
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
            &new_subscriber.email,
            "Welcome!",
            &html_body,
            &plain_body
        )
        .await
}

#[tracing::instrument(
    name = "Saving new subscriber details in database",
    skip(new_subscriber, transaction)
)]
pub async fn insert_subscriber(
    transaction: &mut Transaction<'_, Postgres>,
    new_subscriber: &NewSubscriber,
) -> Result<Uuid, sqlx::Error> {
    
        let subscriber_id: Uuid = Uuid::new_v4();

        let query = sqlx::query!(
            r#"
                INSERT INTO subscriptions (id, email, name, subscribed_at, status)
                VALUES ($1, $2, $3, $4, 'pending_confirmation')
            "#,
            subscriber_id,
            new_subscriber.email.as_ref(),
            new_subscriber.name.as_ref(),
            Utc::now(),
        );
        transaction.execute(query)
            .await
            .map_err(|e| {
                tracing::error!("Failed to execute query: {:?}", e);
                e
            })?;
        Ok(subscriber_id)
}

#[tracing::instrument(
    name = "Store subscription token in the database",
    skip(subscription_token, transaction)
)]
pub async fn store_token(
    transaction: &mut Transaction<'_, Postgres>,
    subscriber_id: Uuid,
    subscription_token: &str,
) -> Result<(), StoreTokenError> {
    let query = sqlx::query!(
        r#"INSERT INTO subscription_tokens (subscription_token, subscriber_id)
        VALUES ($1, $2)"#,
        subscription_token,
        subscriber_id
    );
    transaction.execute(query)
        .await
        .map_err(|e| {
            tracing::error!("Failed to execute query: {:?}", e);
            StoreTokenError(e)
        })?;
    Ok(())
}

fn generate_subscription_token() -> String {
    let mut rng = thread_rng();
    std::iter::repeat_with(|| rng.sample(Alphanumeric))
        .map(char::from)
        .take(25)
        .collect()
}

pub fn error_chain_fmt(
    e: &impl Error,
    f: &mut Formatter<'_>,
) -> std::fmt::Result {

    writeln!(f, "{}\n", e)?;
    let mut current: Option<&dyn Error> = e.source();

    while let Some(cause) = current {
        writeln!(f, "Caused by:\n\t{}", cause)?;
        current = cause.source();
    }

    Ok(())
}