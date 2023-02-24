use sqlx::PgPool;
use sqlx::{Connection, PgConnection};
use std::net::TcpListener;
use zero2prod::configuration::get_configuration;
use zero2prod::startup::run;

// #[tokio::main]
// async fn main() -> std::io::Result<()> {
//     let listener = TcpListener::bind("127.0.0.1:8000").expect("Failed to bind random port");
//     run(listener)?.await
// }

#[tokio::main]
async fn main() -> std::io::Result<()> {
    // Panic if we can't read configuration
    let configuration = get_configuration().expect("Failed to read configuration."); // We have removed the hard-coded `8000` - it's now coming from our settings!
    let connection = PgPool::connect(&configuration.database.connection_string())
        .await
        .expect("Failed to connect to Postgres.");
    let address = format!("127.0.0.1:{}", configuration.application_port);
    let listener = TcpListener::bind(address)?;
    run(listener, connection)?.await
}
