use sqlx::types::chrono::Utc;
use uuid::Uuid;
use actix_web::{web, HttpResponse};
use sqlx::{PgConnection, PgPool};
use tracing::Instrument;
use crate::telemetry::init_subscriber;

#[derive(serde::Deserialize)]
pub struct FormData {
    email : String,
    name: String,
}

#[tracing::instrument(
    name = "Adding a new subscriber",
    skip(form, pool),
    fields(
        request_id = %Uuid::new_v4(),
        subscriber_email = %form.email,
        subscriber_name = %form.name
    )
)]
pub async fn subscribe(
    form: web::Form<FormData>,
    pool: web::Data<PgPool>,
) -> HttpResponse {
    let request_id = Uuid::new_v4();
    let request_span = tracing::info_span!(
        "request_id {} - Adding '{}' '{}' as a new subscriber.",
        %request_id,
        subscriber_email = %form.email,
        subscriber_name = %form.name
    );
    let _request_span_guard = request_span.enter();
    let query_span = tracing::info_span!(
        "Saving new subscriber details in the database"
    );
    match insert_subscriber(&pool, &form).await 
    {
        Ok(_) => {
            HttpResponse::Ok().finish()
        },
        Err(e) => {
            tracing::error!(
                "Failed to execute query : {:?}",
                e
            );
            HttpResponse::InternalServerError().finish()
        }
    }
    
}

#[tracing::instrument(
    name = "Saving new subscriber details in database",
    skip(form, pool)
)]
pub async fn insert_subscriber(
    pool: &PgPool,
    form: &FormData
) -> Result<(), sqlx::Error> {

        sqlx::query!(
            r#"
                INSERT INTO subscriptions (id, email, name, subscribed_at)
                VALUES ($1, $2, $3, $4)
            "#,
            Uuid::new_v4(),
            form.email,
            form.name,
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
