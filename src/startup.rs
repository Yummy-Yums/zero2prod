use crate::routes::health_check;
use crate::routes::subscribe;
use actix_web::dev::Server;
use actix_web::{web, App, HttpServer};
use sqlx::PgPool;
use std::net::TcpListener;
use tracing_actix_web::TracingLogger;

pub fn run(
    listener: TcpListener,
    pg_pool: PgPool
) -> Result<Server, std::io::Error> {
    let db_pool = web::Data::new(pg_pool);
    let server = HttpServer::new(move || {
        App::new()
            .wrap(TracingLogger :: default())
            .route("/health_check", web::get().to(health_check))
            .route("/subscriptions", web::post().to(subscribe))
            .app_data(db_pool.clone())
    })
    .listen(listener)?
    .run();

    Ok(server)
}