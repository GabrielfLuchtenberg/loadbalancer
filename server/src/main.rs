use axum::{extract::State, routing::get, Router};
use std::env;

#[derive(Clone)]
struct PortState {
    port: u16,
}

#[tokio::main]
async fn main() {
    let port = env::var("PORT")
        .ok()
        .and_then(|port| port.parse::<u16>().ok())
        .unwrap();
    let state = PortState { port };

    let app = Router::new().route("/t", get(t)).with_state(state);
    let listener = tokio::net::TcpListener::bind(("0.0.0.0", port))
        .await
        .unwrap();

    axum::serve(listener, app).await.unwrap();
}

async fn t(State(s): State<PortState>) -> String {
    print!("hi");
    format!("Hi from {}", &*s.port.to_string())
}
