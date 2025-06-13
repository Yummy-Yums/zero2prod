use std::net::TcpListener;
use actix_web::{web, App, HttpRequest, HttpResponse, HttpServer, Responder};
use zero2prod::{run};

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
    let listener = TcpListener::bind("127.0.0.1:0").expect("failed to bind");
    println!("Listening on port {:?}", listener.local_addr().unwrap().port());

    run(listener)?.await
}