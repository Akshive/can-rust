use std::net::SocketAddr;

use axum::{response::IntoResponse, routing::get, Router};
use miette::{Result, IntoDiagnostic};

#[tokio::main]
async fn main () -> Result<()>
{
    tracing_subscriber::fmt::init();

    let app: Router = Router::new().route("/", get(root));

    let addr = SocketAddr::from(([0, 0, 0, 0], 3001));

    tracing::debug!("listening on {}", addr);

    axum::Server::bind(&addr)
    .serve(app.into_make_service())
    .await
    .into_diagnostic()?;

    Ok(())
}

async fn root () -> impl IntoResponse 
{
    "Hello, World!"
}
