//! Server implementation

use crate::{config::ServerConfig, error::Result, proxy::ProxyService};
use hyper::{Body, Request, Response, Server};
use hyper::service::{make_service_fn, service_fn};
use std::convert::Infallible;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::oneshot;

/// Load balancer server
#[derive(Debug)]
pub struct LoadBalancerServer {
    config: ServerConfig,
    shutdown_tx: Option<oneshot::Sender<()>>,
}

impl LoadBalancerServer {
    /// Create new load balancer server
    pub async fn new(config: &ServerConfig) -> Result<Self> {
        Ok(Self {
            config: config.clone(),
            shutdown_tx: None,
        })
    }

    /// Start the server
    pub async fn start(&mut self, proxy_service: Arc<ProxyService>) -> Result<()> {
        let addr: SocketAddr = format!("{}:{}", self.config.listen_address, self.config.listen_port)
            .parse()
            .map_err(|e| crate::error::LoadBalancerError::Config(format!("Invalid address: {}", e)))?;

        let (shutdown_tx, shutdown_rx) = oneshot::channel();
        self.shutdown_tx = Some(shutdown_tx);

        let make_svc = make_service_fn(move |_conn| {
            let proxy_service = proxy_service.clone();
            async move {
                Ok::<_, Infallible>(service_fn(move |req: Request<Body>| {
                    let proxy_service = proxy_service.clone();
                    async move {
                        match proxy_service.handle_request(req).await {
                            Ok(response) => Ok::<Response<Body>, Infallible>(response),
                            Err(_) => Ok(Response::builder()
                                .status(500)
                                .body(Body::from("Internal Server Error"))
                                .unwrap()),
                        }
                    }
                }))
            }
        });

        let server = Server::bind(&addr)
            .serve(make_svc)
            .with_graceful_shutdown(async {
                shutdown_rx.await.ok();
            });

        tracing::info!("Load balancer server listening on {}", addr);

        if let Err(e) = server.await {
            return Err(crate::error::LoadBalancerError::Server(format!("Server error: {}", e)));
        }

        Ok(())
    }

    /// Stop the server
    pub async fn stop(&mut self) -> Result<()> {
        if let Some(shutdown_tx) = self.shutdown_tx.take() {
            let _ = shutdown_tx.send(());
        }
        Ok(())
    }
}
