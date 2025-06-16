use std::fmt::format;
use std::net::TcpListener;
use actix_web::{web, App, HttpRequest, HttpResponse, HttpServer, Responder};
use zero2prod::configuration::get_configuration;
use zero2prod::startup::run;
use sqlx::{Connection, PgConnection, PgPool};

async fn greet(req: HttpRequest) -> impl Responder {
    println!("{:?}", req);
    let name = req
        .match_info()
        .get("name")
        .unwrap_or("Worl");
    
    format!("Hello {}!", &name)
}

// #[tokio::main]
// async fn main() -> Result<(), std::io::Error> {
//     let listener = TcpListener::bind("127.0.0.1:0").expect("failed to bind");
//     println!("Listening on port {:?}", listener.local_addr().unwrap().port());
// 
//     run(listener)?.await
// }

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    let configuration = get_configuration().expect("Failed to read configuration.");
    let connection_pool = PgPool::connect(
            &configuration.database.connection_string()
        )   
        .await
        .expect("Failed to connect to Postgres.");
    let address = format!("127.0.0.1:{}", configuration.application_port);
    let listener = TcpListener::bind(address)?;
    run(listener, connection_pool)?.await
}