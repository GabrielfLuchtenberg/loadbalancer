use std::{
    env,
    str::FromStr,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};

use axum::{
    body::Body,
    extract::{Request, State},
    handler::Handler,
    http::{
        uri::{Authority, Scheme},
        StatusCode, Uri,
    },
    response::IntoResponse,
};
use hyper_util::{
    client::legacy::{connect::HttpConnector, Client},
    rt::TokioExecutor,
};
use tokio::net::TcpListener;

#[derive(Clone)]
struct AppState {
    load_balancer: Arc<dyn LoadBalancer + Send + Sync>,
    http_client: Client<HttpConnector, Body>,
}
trait LoadBalancer {
    fn next_server(&self, req: &Request) -> String;
}

struct RoundRobin {
    addrs: Vec<String>,
    req_counter: Arc<AtomicUsize>,
}

impl LoadBalancer for RoundRobin {
    fn next_server(&self, _req: &Request) -> String {
        let count = self.req_counter.fetch_add(1, Ordering::Relaxed);
        self.addrs[count % self.addrs.len()].clone()
    }
}

#[tokio::main]
async fn main() {
    let port = env::var("PORT")
        .ok()
        .and_then(|port| port.parse::<u16>().ok())
        .unwrap_or(9999);

    let addresses = env::var("ADDRESSES")
        .ok()
        .map(|addresses| {
            addresses
                .split(",")
                .map(|addr| addr.trim().to_owned())
                .collect::<Vec<String>>()
        })
        .unwrap_or(vec![
            String::from("0.0.0.0:9997"),
            String::from("0.0.0.0:9998"),
        ]);

    let listener = TcpListener::bind(("0.0.0.0", port)).await.unwrap();

    // not sure why, but http 2 only is not working
    let client = Client::builder(TokioExecutor::new()).build_http::<Body>();

    let round_robin = RoundRobin {
        addrs: addresses.clone(),
        req_counter: Arc::new(AtomicUsize::new(0)),
    };

    let app_state = AppState {
        load_balancer: Arc::new(round_robin),
        http_client: client,
    };

    let app = proxy.with_state(app_state);
    println!("HTTP lb ({}) ready {}", env!("CARGO_PKG_VERSION"), port);
    axum::serve(listener, app).await.unwrap();
}

async fn proxy(
    State(AppState {
        load_balancer,
        http_client,
    }): State<AppState>,
    mut req: Request,
) -> impl IntoResponse {
    let addr = load_balancer.next_server(&req);
    *req.uri_mut() = {
        let uri = req.uri();
        let mut parts = uri.clone().into_parts();
        parts.authority = Authority::from_str(addr.as_str()).ok();
        parts.scheme = Some(Scheme::HTTP);
        Uri::from_parts(parts).unwrap()
    };

    match http_client.request(req).await {
        Ok(res) => Ok(res),
        Err(e) => {
            println!("e: {}", e.to_string());
            let _t = e.to_string();
            Err(StatusCode::BAD_GATEWAY)
        }
    }
}
