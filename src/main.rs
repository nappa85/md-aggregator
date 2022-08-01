use std::{convert::Infallible, env, net::SocketAddr};

use hyper::{
    service::{make_service_fn, service_fn},
    Body, Method, Request, Response, Server,
};

use tracing::error;

mod cache;
mod config;
mod repo;

async fn handle(req: Request<Body>) -> Result<Response<Body>, Infallible> {
    match req.method() {
        &Method::GET => {
            if let Some(owner) = req.uri().query().and_then(|query| {
                query
                    .split('&')
                    .find(|s| s.starts_with("owner="))
                    .map(|s| &s[6..])
            }) {
                // send single file, if available
                let url = req.uri().path();
                let mime = mime_guess::from_path(url).first_or_text_plain();
                let res = Response::builder().header("Content-Type", mime.as_ref());
                match cache::retrieve(&url[1..], owner).await {
                    Some(s) => Ok(res.body(Body::from(s)).unwrap()),
                    None => Ok(res.status(404).body(Body::empty()).unwrap()),
                }
            } else {
                // send entire template
                let cache = cache::get();
                Ok(Response::new(Body::from(cache.to_string())))
            }
        }
        _ => Ok(Response::builder().status(404).body(Body::empty()).unwrap()),
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
    let port = env::var("PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(80);
    let addr = SocketAddr::from(([0, 0, 0, 0], port));

    // And a MakeService to handle each connection...
    let make_service = make_service_fn(|_conn| async { Ok::<_, Infallible>(service_fn(handle)) });

    // Then bind and serve...
    let server = Server::bind(&addr).serve(make_service);

    // And run forever...
    if let Err(e) = server.await {
        error!("server error: {}", e);
    }
}
