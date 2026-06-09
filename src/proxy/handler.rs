//! Proxy handler for `/ys-proxy/{alias}/**` routes.

use axum::extract::{Path, Request, State};
use axum::response::Response;

use crate::api::state::AppState;

/// Handle all proxied requests.
pub async fn proxy_handler(
    Path(alias): Path<String>,
    State(_state): State<AppState>,
    _req: Request,
) -> Response<String> {
    // TODO: implement in task 1.4
    Response::builder()
        .status(501)
        .body(format!("Proxy not yet implemented for alias: {}", alias))
        .unwrap()
}
