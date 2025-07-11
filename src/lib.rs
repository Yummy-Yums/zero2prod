pub mod configuration;
pub mod routes;
pub mod startup;
pub mod telemetry;
mod domain;
mod authentication;
pub mod email_client;
pub mod session_state;
pub mod utils;
pub mod idempotency;
pub mod issue_delivery_worker;

#[cfg(test)]
mod tests {
    use crate::routes::health_check;

    #[tokio::test]
    async fn health_check_succeeds(){
        let response = health_check().await;
        assert!(response.status().is_success())
    }
}