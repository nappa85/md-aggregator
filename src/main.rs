use std::{borrow::Cow, convert::Infallible, net::SocketAddr};

use hyper::{
    body::{aggregate, Buf},
    service::{make_service_fn, service_fn},
    Body, Method, Request, Response, Server,
};

use serde::Deserialize;

use tracing::error;

mod cache;
mod config;
mod repo;

#[derive(Deserialize)]
struct Owner<'a> {
    owner: Cow<'a, str>,
}

async fn retrieve(req: Request<Body>) -> Option<String> {
    let (head, body) = req.into_parts();
    let url = head.uri.path();
    let buf = aggregate(body)
        .await
        .map_err(|e| error!("Request body aggregate error: {}", e))
        .ok()?;
    let owner: Owner = serde_json::from_reader(buf.reader())
        .map_err(|e| error!("Request body deserialize error: {}", e))
        .ok()?;
    cache::retrieve(&url[1..], &owner.owner).await
}

async fn handle(req: Request<Body>) -> Result<Response<Body>, Infallible> {
    match req.method() {
        &Method::GET => {
            let cache = cache::get();
            Ok(Response::new(Body::from(cache.to_string())))
        }
        &Method::POST => {
            let res = Response::builder().header("Content-Type", "text/plain");
            match retrieve(req).await {
                Some(s) => Ok(res.body(Body::from(s)).unwrap()),
                None => Ok(res.status(404).body(Body::empty()).unwrap()),
            }
        }
        _ => Ok(Response::new(Body::empty())),
    }
}

#[tokio::main]
async fn main() {
    // Start loggin
    tracing_subscriber::fmt::init();

    // Parse config and abort on error
    config::Config::init().await.unwrap();

    // Init cache and abort on error
    cache::init().await.unwrap();

    // Construct our SocketAddr to listen on...
    let addr = SocketAddr::from(([0, 0, 0, 0], 8080));

    // And a MakeService to handle each connection...
    let make_service = make_service_fn(|_conn| async { Ok::<_, Infallible>(service_fn(handle)) });

    // Then bind and serve...
    let server = Server::bind(&addr).serve(make_service);

    // And run forever...
    if let Err(e) = server.await {
        error!("server error: {}", e);
    }
}
