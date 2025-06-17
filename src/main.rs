use actix_web::{HttpRequest, Responder};
use sqlx::{Connection, PgPool};
use std::net::TcpListener;
use secrecy::ExposeSecret;
use zero2prod::configuration::get_configuration;
use zero2prod::startup::run;
use tracing_log::LogTracer;
use zero2prod::telemetry::get_subscriber;

async fn greet(req: HttpRequest) -> impl Responder {
    println!("{:?}", req);
    let name = req
        .match_info()
        .get("name")
        .unwrap_or("Worl");
    
    format!("Hello {}!", &name)
}

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    let subscriber = get_subscriber(
        "zero2prod".into(),
        "info".into(),
        std::io::stdout,
    );
    LogTracer::init().expect("Failed to set logger");
    
    let configuration = get_configuration().expect("Failed to read configuration.");
    let connection_pool = PgPool::connect(
            &configuration.database.connection_string().expose_secret()
        )   
        .await
        .expect("Failed to connect to Postgres.");
    let address = format!("127.0.0.1:{}", configuration.application_port);
    let listener = TcpListener::bind(address)?;
    run(listener, connection_pool)?.await
}