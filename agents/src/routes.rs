use warp::{http::Response, Filter};

use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
struct retreiveAnswer {
    query: String,
    repo: String,
}

pub fn agent_routes() -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    warp::path("fetch-answer")
        .and(warp::query::<retreiveAnswer>())
        .map(|params: retreiveAnswer| {
            let response = format!("Query: {}, Repo: {}", params.query, params.repo);
            Response::builder().body(response).unwrap()
        })
    //  warp::reply::json(&params))
    // .and(json_body())
    // .and_then(controller::fetch_answer)
}
