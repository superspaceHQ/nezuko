use crate::{agent, db_client, retrieve_answer};
use axum::{routing::get, Router};
use std::net::SocketAddr;

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

pub async fn start() -> anyhow::Result<SocketAddr> {
    println!("coming here in the start function of retrieval!");

    let configuration = Config::new().unwrap();

    // intialize new llm gateway.
    let llm_gateway = llm_gateway::Client::new(&configuration.openai_url)
        .temperature(0.0)
        .bearer(configuration.openai_key.clone())
        .model(&configuration.openai_model.clone());

    // create new db client.
    // let db_client = db_client::DbConnect::new()
    //     .await
    //     .context("Initiazing database failed.")?;

    let app = Router::new()
        .route("/", get(hello_world))
        .route("/retrieve", get(retrieve_answer));

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
