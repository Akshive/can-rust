use std::net::SocketAddr;

use axum::{response::IntoResponse, routing::get, Router, http::{Request, HeaderMap}};
use miette::{Result, IntoDiagnostic};

const PROXY_FROM_DOMAIN: &str = "slow-server.akshive.test";
const PROXY_ORIGIN_DOMAIN: &str = "www.google.com";

#[tokio::main]
async fn main () -> Result<()>
{
    tracing_subscriber::fmt::init();

    let app: Router = Router::new().fallback(proxy_request);

    let addr = SocketAddr::from(([0, 0, 0, 0], 3001));

    tracing::debug!("listening on {}", addr);

    axum::Server::bind(&addr)
    .serve(app.into_make_service())
    .await
    .into_diagnostic()?;

    Ok(())
}

async fn proxy_request<Body>(
    host: axum::extract::Host, 
    headers: HeaderMap,
    method: axum::http::Method,
    request: Request<Body>) -> Result<impl IntoResponse, String>
{
    let uri = request.uri();
    
    let split = host.0.split(":").collect::<Vec<_>>();
    let host_name = split[0];

    if host_name != PROXY_FROM_DOMAIN
    {
        return Err(format!("Unsupported host {}", host_name));
    }

    let path = uri.path_and_query().map(|pq| pq.path()).unwrap_or("/");

    let client = reqwest::Client::new();

    dbg!(&method, &path);
    
    let response = client
    .request(method, format!("https://{PROXY_ORIGIN_DOMAIN}{path}"))
    .headers(headers)
    .send()
    .await
    .map_err(|_| "Request failed")?;

    Ok((
        response.status(),
        response.headers().clone(),
        response.bytes().await.into_diagnostic().map_err(|_| "Could not get bytes from header")?,
    ))
}
