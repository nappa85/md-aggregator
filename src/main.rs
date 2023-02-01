use std::{convert::Infallible, env, net::SocketAddr};

use hyper::{
    service::{make_service_fn, service_fn},
    Body, Method, Request, Response, Server,
};

use tracing::{error, info};

mod cache;
mod config;
mod repo;

async fn handle(req: Request<Body>) -> Result<Response<Body>, Infallible> {
    match req.method() {
        &Method::GET => {
            if let Some((Ok(owner), Ok(url))) = req.uri().query().and_then(|query| {
                query.split('&').find(|s| s.starts_with("owner=")).map(|s| {
                    (
                        urlencoding::decode(&s[6..]),
                        urlencoding::decode(req.uri().path()),
                    )
                })
            }) {
                // send single file, if available
                let mime = mime_guess::from_path(url.as_ref()).first_or_text_plain();
                let res = Response::builder().header("Content-Type", mime.as_ref());
                match cache::retrieve(&url[1..], &owner).await {
                    Some(s) => Ok(res.body(Body::from(s)).unwrap()),
                    None => {
                        info!("Path {url} not found");
                        Ok(res.status(404).body(Body::empty()).unwrap())
                    }
                }
            } else {
                // serve static files
                if let Some((content_type, body)) = cache::static_cache(req.uri().path()) {
                    Ok(Response::builder()
                        .header("Content-Type", content_type)
                        .body(Body::from(body))
                        .unwrap())
                } else {
                    // send entire template
                    let cache = cache::get();
                    Ok(Response::new(Body::from(cache.to_string())))
                }
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
    config::init();

    // Init cache and abort on error
    cache::init().await.unwrap();

    // Construct our SocketAddr to listen on...
    let port = env::var("PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(8080);
    let addr = SocketAddr::from(([0, 0, 0, 0], port));

    // And a MakeService to handle each connection...
    let make_service = make_service_fn(|_conn| async { Ok::<_, Infallible>(service_fn(handle)) });

    // Then bind and serve...
    let server = Server::bind(&addr).serve(make_service);

    info!("Listening on {addr}");

    // And run forever...
    if let Err(e) = server.await {
        error!("server error: {e}");
    }
}
