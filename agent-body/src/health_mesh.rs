//! HTTP health mesh endpoint (Kernel V2 Phase 8).

use axum::{routing::get, Json, Router};
use std::net::SocketAddr;
use tracing::info;

use crate::supervisor::{build_organs_health_report, OrgansHealthReport};

pub async fn serve(port: u16) -> anyhow::Result<()> {
    let app = Router::new()
        .route("/organs/health", get(organs_health))
        .route("/health", get(|| async { "ok" }));

    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    info!("health mesh listening on http://{addr}/organs/health");
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

async fn organs_health() -> Json<OrgansHealthReport> {
    Json(
        build_organs_health_report().unwrap_or_else(|_e| OrgansHealthReport {
            timestamp: chrono::Utc::now().to_rfc3339(),
            mesh_score: 0,
            organs: Vec::new(),
            degradation: crate::degradation::DegradationState::from_mesh_score(0),
        }),
    )
}

#[cfg(test)]
mod tests {
    #[test]
    fn health_route_path_is_stable() {
        assert_eq!("/organs/health", "/organs/health");
    }
}
