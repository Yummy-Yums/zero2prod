use std::net::TcpListener;
use std::rc::Rc;
use actix_web::dev::Server;
use actix_web::{web, App, HttpServer};
use sqlx::{PgConnection, PgPool};
use crate::routes::subscribe;
use crate::routes::health_check;

pub fn run(
    listener: TcpListener,
    pg_pool: PgPool
) -> Result<Server, std::io::Error> {
    let connection = web::Data::new(pg_pool);
    let server = HttpServer::new(move || {
        App::new()
            .route("/health_check", web::get().to(health_check))
            .route("/subscriptions", web::post().to(subscribe))
            .app_data(connection.clone())
    })
        .listen(listener)?
        .run();

    Ok(server)
}