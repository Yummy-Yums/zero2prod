// use once_cell::sync::Lazy;
// use secrecy::ExposeSecret;
// use sqlx::{Connection, Executor, PgConnection, PgPool};
// use std::net::TcpListener;
// use sqlx::__rt::timeout;
// use uuid::Uuid;
// use zero2prod::configuration::{get_configuration, DatabaseSettings};
// use zero2prod::email_client::EmailClient;
// use zero2prod::startup::run;
// use zero2prod::telemetry::{get_subscriber, init_subscriber};
// 
// static TRACING: Lazy<()> = Lazy::new(|| {
//     let default_filter_level = "info".to_string();
//     let subscriber_name  = "test".to_string();
//     
//     if std::env::var("TEST_LOG").is_ok() {
//         let subscriber = get_subscriber(
//             subscriber_name, 
//             default_filter_level,
//             std::io::stdout,
//         );
// 
//         init_subscriber(subscriber);
//     } else {
//         let subscriber = get_subscriber(
//             subscriber_name,
//             default_filter_level,
//             std::io::sink,
//         );
// 
//         init_subscriber(subscriber);
//     }
// });
// 
// pub struct TestApp {
//     pub address: String,
//     pub db_pool: PgPool
// }
// 
// async fn spawn_app() -> TestApp {
//     
//     Lazy::force(&TRACING);
//     
//     let listener = TcpListener::bind("127.0.0.1:0")
//         .expect("failed to bind random port");
//     
//     let port = listener.local_addr().unwrap().port();
//     let address = format!("http://127.0.0.1:{}", port);
// 
//     let mut configuration = get_configuration().expect("failed to read configuration");
//     configuration.database.database_name = Uuid::new_v4().to_string();
//     let connection_pool = configure_database(&configuration.database).await;
//     
//     let sender_email = configuration.email_client.sender()
//         .expect("Invalid sender email address.");
//     let timeout = configuration.email_client.timeout();
//     let email_client = EmailClient::new(
//         configuration.email_client.base_url,
//         sender_email,
//         configuration.email_client.authorization_token,
//         timeout
//     );
// 
//     let server = run(listener, connection_pool.clone(), email_client)
//         .expect("Failed to bind address");
// 
//     let _ = tokio::spawn(server);
// 
//    TestApp {
//        address,
//        db_pool: connection_pool
//    }
// }
// 
// pub async fn configure_database(config: &DatabaseSettings) -> PgPool {
//     // Create database
//     let mut connection = PgConnection::connect(
//         &config.connection_string_without_db().expose_secret()
//     )
//         .await
//         .expect("Failed to connect to Postgres");
// 
//     connection
//         .execute(format!(r#"CREATE DATABASE "{}";"#, config.database_name).as_str())
//         .await
//         .expect("Failed to create database");
// 
//     let connection_pool = PgPool::connect(
//         &config.connection_string().expose_secret()
//     )
//     .await
//     .expect("Failed to connect to Postgres");
//     
//     sqlx::migrate!("./migrations")
//         .run(&connection_pool)
//         .await
//         .expect("Failed to migrate the database");
//     
//     connection_pool
// }
// 
// #[tokio::test]
// async fn health_check_works(){
//     let address = spawn_app().await.address;
//     let client = reqwest::Client::new();
//     
//     let response = client
//         .get(format!("{}/health_check", address))
//         .send()
//         .await
//         .expect("Failed to execute request.");
//     
//     assert!(response.status().is_success());
//     assert_eq!(Some(0), response.content_length());
// }
// 
// #[tokio::test]
// async fn subscribe_returns_a_200_for_valid_form_data(){
//     let app= spawn_app().await;
//     // let configuration: Settings  = get_configuration().expect("Failed to read configuration.");
//     // let connection_string: String = configuration.database.connection_string();
//     // let mut connection: PgConnection  = PgConnection::connect(&connection_string)
//     //     .await
//     //     .expect("Failed to connect to Postgres");
// 
//     let client = reqwest::Client::new();
// 
//     let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";
//     let response = client
//         .post(format!("{}/subscriptions", app.address))
//         .header("Content-type", "application/x-www-form-urlencoded")
//         .body(body)
//         .send()
//         .await
//         .expect("Failed to execute request.");
// 
//     assert_eq!(200, response.status().as_u16());
// 
//     let saved = sqlx::query!(r#"SELECT email, name FROM subscriptions"#,)
//         .fetch_one(&app.db_pool)
//         .await
//         .expect("Failed to fetch saved subscription.");
// 
//     assert_eq!(saved.email, "ursula_le_guin@gmail.com");
//     assert_eq!(saved.name, "le guin");
// }
// 
// #[tokio::test]
// // #[should_panic]
// async fn subscribe_returns_a_400_when_fields_are_present_but_empty(){
//     let app = spawn_app().await;
//     let client = reqwest::Client::new();
//     let test_cases = vec![
//         ("name=&email=ursula_le_guin%40gmail.com", "empty name"),
//         ("email=Ursula&email=", "empty email"),
//         ("name=Ursula&email=definitely-not-an-email", "invalid email"),
//     ];
// 
//     for (body, description) in test_cases {
// 
//         let response = client
//             .post(&format!("{}/subscriptions", app.address))
//             .header("Content-type", "application/x-www-form-urlencoded")
//             .body(body)
//             .send()
//             .await
//             .expect("Failed to execute request.");
// 
//         assert_eq!(
//             400,
//             response.status().as_u16(),
//             "The API did not return a 200 OK when the payload was {}",
//             description
//         );
//     }
// }
// 
// #[tokio::test]
// async fn subscribe_returns_a_400_when_data_is_missing(){
//     let app = spawn_app().await;
//     let client = reqwest::Client::new();
//     let test_cases = vec![
//         ("name=le%20guin", "missing the email"),
//         ("email=ursula_le_guin%40gmail.com", "missing the name"),
//         ("", "missing both name and email"),
//     ];
// 
//     for (invalid_body, error_message) in test_cases {
// 
//         let response = client
//             .post(format!("{}/subscriptions", app.address))
//             .header("Content-type", "application/x-www-form-urlencoded")
//             .body(invalid_body)
//             .send()
//             .await
//             .expect("Failed to execute request.");
// 
//         assert_eq!(
//             400,
//             response.status().as_u16(),
//             "The API did fail with 400 Bad Request when the payload was {}",
//             error_message
//         );
//     }
// 
<<<<<<< HEAD
// }
=======
// }
>>>>>>> master
