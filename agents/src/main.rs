use anyhow::{Context, Result};
use axum::{extract::Query, http::StatusCode, Extension, Json};
use futures::{future::Either, stream, StreamExt};
use serde::Deserialize;
use std::time::Duration;
use web_retrieve::AppState;

use core::result::Result::Ok;
use std::sync::Arc;

mod agent;
mod config;
mod db_client;
mod parser;
mod web_retrieve;

use crate::agent::agent::Action;
use agent::{agent::Agent, llm_gateway};

use config::Config;

#[derive(Deserialize)]
struct QueryParams {
    query: String,
    repo_name: String,
}

async fn retrieve_answer(
    Query(params): Query<QueryParams>,
    Extension(state): Extension<Arc<AppState>>,
) -> Result<Json<String>, (StatusCode, String)> {
    // Implement your logic here. For now, we're just echoing back the query.

    let response = format!("Query: {}, Repo Name: {}", params.query, params.repo_name);

    let configuration = Config::new().unwrap();

    let query: String = params.query.clone();

    // TODO :: SHANKAR ADD PARSER FOR QUERY WITH PROPER TYPE

    // let query = match parser::parser::parse_nl(&q) {
    //     Ok(parsed_query) => {
    //         // Handle successful parsing
    //         Ok(Json(response)) // Or use parsed_query as needed
    //     }
    //     Err(e) => {
    //         // Convert parsing error to your function's error type
    //         let error_message = format!("Error parsing query: {:?}", e);
    //         Err((StatusCode::BAD_REQUEST, error_message))
    //     }
    // };
    // let query = parser::parser::parse_nl(&q)
    //     .context("parse error")?
    //     .into_semantic()
    //     .context("got a 'Grep' query")?
    //     .into_owned();
    // let query_target = query
    //     .unwrap()
    //     .target
    //     .as_ref()
    //     .context("query was empty")?
    //     .as_plain()
    //     .context("user query was not plain text")?
    //     .clone()
    //     .into_owned();

    println!("{:?}", query);

    let mut action = Action::Query(query);

    let id = uuid::Uuid::new_v4();

    let mut exchanges = vec![agent::exchange::Exchange::new(id, params.query.clone())];

    // intialize new llm gateway.
    let llm_gateway = llm_gateway::Client::new(&configuration.openai_url)
        .temperature(0.0)
        .bearer(configuration.openai_key.clone())
        .model(&configuration.openai_model.clone());

    let (exchange_tx, exchange_rx) = tokio::sync::mpsc::channel(10);

    let mut agent: Agent = Agent {
        db: state.db_client,
        exchange_tx,
        exchanges,
        llm_gateway,
        query_id: id,
        complete: false,
    };

    let mut exchange_stream = tokio_stream::wrappers::ReceiverStream::new(exchange_rx);

    let exchange_handler = tokio::spawn(async move {
        while let exchange = exchange_stream.next().await {
            match exchange {
                Some(e) => {
                    //println!("{:?}", e.compressed());
                }
                None => {
                    eprintln!("No more messages or exchange channel was closed.");
                    break;
                }
            }
        }
    });

    // first action
    println!("first action {:?}\n", action);

    let mut i = 1;
    'outer: loop {
        // Now only focus on the step function inside this loop.
        match agent.step(action).await {
            Ok(next_action) => {
                match next_action {
                    Some(act) => {
                        action = act;
                    }
                    None => break,
                }

                // print the action
                i = i + 1;

                println!("Action number: {}, Action: {:?}", i, action);
            }
            Err(e) => {
                eprintln!("Error during processing: {}", e);
                break 'outer;
            }
        }

        // Optionally, you can add a small delay here to prevent the loop from being too tight.
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
    agent.complete();

    // Await the spawned task to ensure it has completed.
    // Though it's not strictly necessary in this context since the task will end on its own when the stream ends.
    let _ = exchange_handler.await;

    Ok(Json(response))
}

#[tokio::main]
async fn main() {
    println!("Hello, world!");
    web_retrieve::start().await.unwrap();
}
