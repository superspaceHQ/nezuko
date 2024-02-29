use anyhow::{Context, Result};
use axum::{extract::Query, http::StatusCode, Extension, Json};
use futures::{future::Either, stream, StreamExt};
use serde::Deserialize;
use std::time::Duration;
use tantivy::query;
use web_retrieve::AppState;

use core::result::Result::Ok;
use std::sync::Arc;

mod agent;
mod config;
mod db_client;
mod helpers;
mod parser;
mod search;
mod web_retrieve;

use crate::{agent::agent::Action, search::semantic};
use agent::{agent::Agent, llm_gateway};

use config::Config;

#[derive(Deserialize)]
struct QueryParams {
    query: String,
    repo_name: String,
}

async fn retrieve_answer(
    Query(params): Query<QueryParams>,
) -> Result<Json<String>, (StatusCode, String)> {
    // Implement your logic here. For now, we're just echoing back the query.

    let response = format!("Query: {}, Repo Name: {}", params.query, params.repo_name);

    let configuration = Config::new().unwrap();

    // Bind the owned string to a variable
    let query = params.query;
    // println!("owned_query {:?}", owned_query);

    // let semantic_query = semantic::SemanticQuery::new(query);

    // TODO :: SHANKAR ADD PARSER FOR QUERY WITH PROPER TYPE

    // let query = match parser::parser::parse_nl(&owned_query.query) {
    //     Ok(parsed_query) => {
    //         // Handle successful parsing
    //         parsed_query.into_semantic().context("got a 'Grep' query")
    //     }
    //     Err(e) => {
    //         // Convert parsing error to your function's error type
    //         panic!("Error parsing query: {:?}", e);
    //         // Err((StatusCode::BAD_REQUEST, error_message))
    //     }
    // };

    // let query_target: Result<String, String> = match query {
    //     Ok(q) => {
    //         match q.target.as_ref() {
    //             Some(target) => {
    //                 match target.as_plain() {
    //                     Some(plain) => Ok(plain.clone().into_owned()), // Assuming plain is a reference that needs to be owned
    //                     None => Err("The target is not in plain format.".to_string()),
    //                 }
    //             }
    //             None => Err("Target was not found in the query.".to_string()),
    //         }
    //     }
    //     Err(e) => Err(format!("Error parsing query: {:?}", e)),
    // };

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
    let new_query = query.clone();

    let mut action = Action::Query(query);

    let id = uuid::Uuid::new_v4();

    let mut exchanges = vec![agent::exchange::Exchange::new(id, new_query)];

    let db_client = db_client::DbConnect::new().await.unwrap();

    // intialize new llm gateway.
    let llm_gateway = llm_gateway::Client::new(&configuration.openai_url)
        .temperature(0.0)
        .bearer(configuration.openai_key.clone())
        .model(&configuration.openai_model.clone());

    let (exchange_tx, exchange_rx) = tokio::sync::mpsc::channel(10);

    let mut agent: Agent = Agent {
        db: db_client, // Clone the Arc here,
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

    // let db_client = db_client::DbConnect::new()
    //     .await
    //     .context("Initiazing database failed.");

    web_retrieve::start().await.unwrap();
}
