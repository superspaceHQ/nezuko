use anyhow::{Context, Error, Result};
use axum::{
    extract::{Path, Query},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use futures::{future::Either, stream, StreamExt};
use serde::Deserialize;
use std::time::Duration;
mod agent;
mod config;
mod db_client;
mod helpers;
mod parser;
mod search;

use config::Config;

use crate::agent::agent::Action;
use crate::agent::agent::Agent;

use crate::agent::exchange::Exchange;

use agent::llm_gateway;
use core::result::Result::Ok;

use regex_syntax::ast::print;

mod web_retrieve;

// derive debug and clone for configuration.
const TIMEOUT_SECS: u64 = 60;

#[derive(Deserialize)]
struct QueryParams {
    query: String,
    repo_name: String,
}

async fn retrieve_answer(Query(params): Query<QueryParams>) -> Result<impl IntoResponse> {
    let result = format!("Query: {}, Repo Name: {}", params.query, params.repo_name);

    println!("Query: {}, Repo Name: {}", params.query, params.repo_name);
    let q = &params.query;
    // let q: &str = "How are github app private keys handled?";

    let query = parser::parser::parse_nl(q)
        .context("parse error")?
        .into_semantic()
        .context("got a 'Grep' query")?
        .into_owned();

    // Ok(result)

    // println!("parsed query: {}", query);

    if params.query.is_empty() || params.repo_name.is_empty() {
        Err("Query or repo name is empty")
    } else {
        Ok(format!(
            "Successfully processed query {} in repo {}",
            params.query, params.repo_name
        ))
    }
    // println!("{:?}", query);

    // let query_target = query
    //     .target
    //     .as_ref()
    //     .context("query was empty")?
    //     .as_plain()
    //     .context("user query was not plain text")?
    //     .clone()
    //     .into_owned();
    // println!("{:?}", query_target);

    // let mut action = Action::Query(query_target);

    // let id = uuid::Uuid::new_v4();
    // create array of  exchanges.
    // let mut exchanges = vec![agent::exchange::Exchange::new(id, query.clone())];
    // exchanges.push(Exchange::new(id, query));

    // let configuration = Config::new().unwrap();

    // intialize new llm gateway.
    // let llm_gateway = llm_gateway::Client::new(&configuration.openai_url)
    //     .temperature(0.0)
    //     .bearer(configuration.openai_key.clone())
    //     .model(&configuration.openai_model.clone());

    // create new db client.
    // let db_client = db_client::DbConnect::new()
    //     .await
    //     .context("Initiazing database failed.")?;

    // create agent.

    // let (exchange_tx, exchange_rx) = tokio::sync::mpsc::channel(10);

    // let mut agent: Agent = Agent {
    //     db: db_client,
    //     exchange_tx,
    //     exchanges,
    //     llm_gateway,
    //     query_id: id,
    //     complete: false,
    // };
    // ... [ rest of the setup code ]

    // let mut exchange_stream = tokio_stream::wrappers::ReceiverStream::new(exchange_rx);

    // let exchange_handler = tokio::spawn(async move {
    //     while let exchange = exchange_stream.next().await {
    //         match exchange {
    //             Some(e) => {
    //                 //println!("{:?}", e.compressed());
    //             }
    //             None => {
    //                 eprintln!("No more messages or exchange channel was closed.");
    //                 break;
    //             }
    //         }
    //     }
    // });

    // // first action
    // println!("first action {:?}\n", action);

    // let mut i = 1;
    // 'outer: loop {
    //     // Now only focus on the step function inside this loop.
    //     match agent.step(action).await {
    //         Ok(next_action) => {
    //             match next_action {
    //                 Some(act) => {
    //                     action = act;
    //                 }
    //                 None => break,
    //             }

    //             // print the action
    //             i = i + 1;

    //             println!("Action number: {}, Action: {:?}", i, action);
    //         } // Err(e) => {
    //           //     // Explicitly create a response and specify its type
    //           //     // let error_response = (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into();
    //           //     // return Err(error_response.into_response());
    //           //     return Err(e);
    //           // }
    //     }

    //     // Optionally, you can add a small delay here to prevent the loop from being too tight.
    //     tokio::time::sleep(Duration::from_millis(500)).await;
    // }

    // agent.complete();

    // Await the spawned task to ensure it has completed.
    // Though it's not strictly necessary in this context since the task will end on its own when the stream ends.
    // let _ = exchange_handler.await;

    // ... [ rest of your code ]

    // Ok(Response::new("Data received".to_string()))
}

#[tokio::main]
async fn main() {
    // Create a new Axum router
    // let app = Router::new().route("/", get(hello_world));

    // // Define the address to listen on
    // let addr = SocketAddr::from(([127, 0, 0, 1], 3000));

    // // Run the server
    // println!("Server running on http://{}", addr);
    // Server::bind(&addr)
    //     .serve(app.into_make_service())
    //     .await
    //     .unwrap();
    println!("Hello, world!");
    web_retrieve::start().await.unwrap();
    println!("Server running on http://,");
}
