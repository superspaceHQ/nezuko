use crate::retrieve_answer;
use axum::{routing::get, Router};

// Function to initialize and start the server
pub async fn start() -> anyhow::Result<()> {
    // Create a new Axum router

    println!("coming here in the start function of retrieval!");
    let app = Router::new()
        .route("/", get(hello_world)) // Home route that returns "Hello, world!"
        .route("/retrieve", get(retrieve_answer))
        .route("/answer", get(semantic_retrieve)); // Route to handle 'answer' with `semantic_retrieve` function

    // Define the address to bind the server to
    let addr = "127.0.0.1:3000".parse().unwrap();

    println!("Listening on {}", addr);

    // Bind the server to the specified address and start serving
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();

    Ok(())
}

// Define the `hello_world` function to handle requests to the home route
async fn hello_world() -> &'static str {
    "Hello, world!"
}

// Define the `retrieve` function to handle requests to the '/retrieve' route
// async fn retrieve() -> &'static str {
//     "Retrieve function called"
// }

// Define the `semantic_retrieve` function to handle requests to the '/answer' route
async fn semantic_retrieve() -> &'static str {
    "Semantic Retrieve function called"
}
