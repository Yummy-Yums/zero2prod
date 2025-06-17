pub mod configuration;
pub mod routes;
pub mod startup;
pub mod telemetry;

use actix_web::Responder;
use crate::routes::health_check;


#[cfg(test)]
mod tests {
    use crate::health_check;

    #[tokio::test]
    async fn health_check_succeeds(){
        let response = health_check().await;
        assert!(response.status().is_success())
    }
}