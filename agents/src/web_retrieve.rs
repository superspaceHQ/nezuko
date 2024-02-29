use crate::agent::agent::Agent;
use crate::db_client::DbConnect;
use crate::{
    agent::{self},
    db_client, retrieve_answer,
};
use axum::Extension;
use axum::{routing::get, Router};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio_stream::Stream;

use crate::config::Config;
use agent::llm_gateway;

/// Starts the Axum server for retrieval.
///
/// This function initializes the Axum router, defines routes, and binds the server to a specified
/// address. It then starts serving HTTP requests asynchronously.
///
/// # Returns
///
/// Returns a `Result` containing the `SocketAddr` where the server is listening if the server
/// starts successfully. If an error occurs during server startup or execution, it returns an
/// `anyhow::Error`.
pub struct AppState {
    pub db_client: DbConnect, // Assume DbClient is your database client type
}
pub async fn start() -> anyhow::Result<SocketAddr> {
    println!("coming here in the start function of retrieval!");

    // create new db client.
    let db_client = db_client::DbConnect::new().await.unwrap();
    // .context("Initiazing database failed.")?;

    let shared_state = Arc::new(AppState { db_client });

    let app = Router::new()
        .route("/", get(hello_world))
        .route("/retrieve", get(retrieve_answer))
    // .layer(Extension(shared_state));

    let addr = "127.0.0.1:3000".parse().unwrap();

    println!("Listening on {}", addr);

    // Bind the server to the specified address and start serving
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .map(|_| addr)
        .map_err(Into::into)
}

async fn hello_world() -> &'static str {
    "Hello, world!"
}
