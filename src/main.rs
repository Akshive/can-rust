use std::{net::SocketAddr, collections::HashMap};

use axum::{response::IntoResponse, Router, http::{HeaderMap, Uri}};
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

async fn proxy_request(
    uri: Uri,
    host: axum::extract::Host, 
    headers: HeaderMap,
    method: axum::http::Method,
    bytes: axum::body::Bytes) -> Result<impl IntoResponse, String>
{  
    let split = host.0.split(":").collect::<Vec<_>>();
    let host_name = split[0];

    if host_name != PROXY_FROM_DOMAIN
    {
        return Err(format!("Unsupported host {}", host_name));
    }

    let path = uri.path_and_query().map(|pq| pq.path()).unwrap_or("/");

    let url_builder = axum::http::Uri::builder().scheme("http").authority(PROXY_ORIGIN_DOMAIN);
    let url = url_builder.path_and_query(path).build().map_err(|_| "Could not build url")?;

    let response = try_get_cached_response(method, url, headers, bytes).await?;

    Ok((
        response.status(),
        response.headers().clone(),
        response.into_body()
    ))
}

type CacheKey = (axum::http::Method, axum::http::Uri);

lazy_static::lazy_static! 
{
    static ref CACHE: std::sync::Mutex<HashMap<CacheKey, axum::http::Response<axum::body::Bytes>>>  = std::sync::Mutex::new(HashMap::new());
}

async fn try_get_cached_response(
    method: reqwest::Method,
    url: axum::http::Uri,
    headers: HeaderMap,
    body: axum::body::Bytes
) -> Result<axum::http::Response<axum::body::Bytes>, String>
{
    {
        let cache = CACHE.lock().unwrap();
        let cached_response = cache.get(&(method.clone(), url.clone()));

        if let Some(cached) = cached_response 
        {
            let mut response = axum::http::Response::builder().status(cached.status());
            for (key, value) in cached.headers().iter() 
            {
                response = response.header(key, value)
            }
            let response = response.body(cached.body().clone()).map_err(|_| "Could not build response")?;

            return Ok(response);
        }
    }

    let client = reqwest::Client::new();

    let origin_response = client
    .request(method, url.to_string())
    .body(body)
    .headers(headers)
    .send()
    .await
    .map_err(|_| "Request failed")?;

    let mut response = axum::http::Response::builder().status(origin_response.status());
    for (key, value) in origin_response.headers().iter() 
    {
        response = response.header(key, value)
    }
    let response = response.body(
        origin_response.bytes().await.map_err(|_| "Could not get bytes from origin response")?
    )
    .map_err(|_| "Could not build response")?;
    
    Ok(response)
}