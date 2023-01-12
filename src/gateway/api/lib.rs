//!
//!
//!
use std::path::PathBuf;
use std::str::FromStr;
use std::{env, net::SocketAddr};

use anyhow::Result;
use axum::body::Body;
use axum::extract::{Path, Query};
use axum::http::{Response, StatusCode};
use axum::routing::{get, post};
use axum::{Router, Server};
use hyper::Request;
use serde::{Deserialize, Serialize};
use tower::ServiceBuilder;
use tower_cookies::CookieManagerLayer;

use crate::git::protocol::HttpProtocol;

#[tokio::main]
pub(crate) async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env::set_var("RUST_LOG", "info");
    tracing_subscriber::fmt::init();

    dotenvy::dotenv().ok();
    let _db_url = env::var("DATABASE_URL").expect("DATABASE_URL is not set in .env file");
    let host = env::var("HOST").expect("HOST is not set in .env file");
    let port = env::var("PORT").expect("PORT is not set in .env file");
    let server_url = format!("{}:{}", host, port);
    let app = Router::new()
        .route("/:repo/info/refs", get(git_info_refs))
        .route("/:repo/git-upload-pack", post(git_upload_pack))
        .route("/:repo/git-receive-pack", post(git_receive_pack))
        .route("/:repo/decode", post(decode_packfile))
        .layer(
            ServiceBuilder::new().layer(CookieManagerLayer::new()),
            // .layer(Extension(data_source)),
        );

    let addr = SocketAddr::from_str(&server_url).unwrap();
    Server::bind(&addr).serve(app.into_make_service()).await?;

    Ok(())
}

#[derive(Deserialize, Debug)]
struct ServiceName {
    pub service: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct Params {
    pub repo: String,
}

//
async fn git_info_refs(
    Query(service): Query<ServiceName>,
    Path(params): Path<Params>,
) -> Result<Response<Body>, (StatusCode, String)> {
    let mut work_dir =
        PathBuf::from(env::var("WORK_DIR").expect("WORK_DIR is not set in .env file"));
    work_dir = work_dir
        .join(params.repo.replace(".git", ""))
        .join(".git");
    let http_protocol = HttpProtocol::default();

    let service_name = service.service;
    if service_name == "git-upload-pack" || service_name == "git-receive-pack" {
        http_protocol.git_info_refs(work_dir, service_name).await
    } else {
        return Err((
            StatusCode::FORBIDDEN,
            String::from("Operation not supported"),
        ));
    }
}

//
async fn git_upload_pack(
    Path(params): Path<Params>,
    req: Request<Body>,
) -> Result<Response<Body>, (StatusCode, String)> {
    tracing::info!("{:?}", params.repo);
    let http_protocol = HttpProtocol::default();
    let mut work_dir =
        PathBuf::from(env::var("WORK_DIR").expect("WORK_DIR is not set in .env file"));
    work_dir = work_dir.join(params.repo.replace(".git", ""));
    http_protocol.git_upload_pack(work_dir, req).await
}

//
async fn git_receive_pack(
    Path(_params): Path<Params>,
    req: Request<Body>,
) -> Result<Response<Body>, (StatusCode, String)> {
    tracing::info!("req: {:?}", req);

    let work_dir = PathBuf::from("~/").join("Downloads/crates.io-index");
    let http_protocol = HttpProtocol::default();

    http_protocol.git_receive_pack(work_dir, req).await
}

/// try to unpack all object from pack file
async fn decode_packfile() {
    let http_protocol = HttpProtocol::default();

    http_protocol.decode_packfile().await
}
