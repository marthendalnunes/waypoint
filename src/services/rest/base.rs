use std::net::SocketAddr;
use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;
use tracing::{error, info, warn};

use crate::app::{Service, ServiceContext, ServiceHandle};
use crate::core::data_context::{DataContext, DataContextBuilder};
use crate::services::mcp::WaypointMcpService;
use crate::services::rest::{McpResourceReader, ResourceReader, handlers};

/// REST service that integrates with the App lifecycle.
pub struct RestService {
    bind_address: String,
    port: u16,
    max_limit: usize,
    swagger_ui_enabled: bool,
}

impl Default for RestService {
    fn default() -> Self {
        Self {
            bind_address: "127.0.0.1".to_string(),
            port: 8081,
            max_limit: 100,
            swagger_ui_enabled: false,
        }
    }
}

impl RestService {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn configure(mut self, bind_address: String, port: u16) -> Self {
        self.bind_address = bind_address;
        self.port = port;
        self
    }

    pub fn with_max_limit(mut self, max_limit: usize) -> Self {
        self.max_limit = max_limit;
        self
    }

    pub fn with_swagger_ui_enabled(mut self, swagger_ui_enabled: bool) -> Self {
        self.swagger_ui_enabled = swagger_ui_enabled;
        self
    }
}

#[async_trait]
impl Service for RestService {
    fn name(&self) -> &str {
        "rest"
    }

    async fn start<'a>(&'a self, context: ServiceContext<'a>) -> crate::app::Result<ServiceHandle> {
        let socket_addr =
            format!("{}:{}", self.bind_address, self.port).parse::<SocketAddr>().map_err(|e| {
                crate::app::ServiceError::Initialization(format!("Invalid socket address: {}", e))
            })?;

        info!("Starting REST service on {}", socket_addr);

        let database = context.state.database.as_ref().ok_or_else(|| {
            crate::app::ServiceError::Initialization(
                "Database client not available. REST service requires database access.".to_string(),
            )
        })?;

        let hub_config = context.config.hub.clone();
        let mut hub = crate::hub::client::Hub::new(Arc::new(hub_config)).map_err(|e| {
            crate::app::ServiceError::Initialization(format!("Failed to create Hub client: {}", e))
        })?;

        if let Err(err) = hub.connect().await {
            warn!("Failed to connect to Hub: {}. Will retry automatically when needed.", err);
        }

        let hub_client = crate::hub::providers::FarcasterHubClient::new(Arc::new(Mutex::new(hub)));
        let db_client =
            crate::database::providers::PostgresDatabaseClient::new(Arc::clone(database));

        let data_context: DataContext<crate::database::providers::PostgresDatabaseClient, _> =
            DataContextBuilder::new().with_database(db_client).with_hub_client(hub_client).build();

        let waypoint_service = WaypointMcpService::new(data_context);
        let reader: Arc<dyn ResourceReader> = Arc::new(McpResourceReader::new(waypoint_service));
        let state = crate::services::rest::RestState::new(reader, self.max_limit);
        let router = handlers::router(self.swagger_ui_enabled).with_state(state);

        let cancellation_token = CancellationToken::new();
        let ct_for_shutdown = cancellation_token.clone();

        let server_handle = tokio::spawn(async move {
            match tokio::net::TcpListener::bind(socket_addr).await {
                Ok(listener) => {
                    info!(
                        "REST service started on {} and ready to accept connections",
                        socket_addr
                    );

                    let ct_shutdown = ct_for_shutdown.child_token();
                    let server = axum::serve(listener, router).with_graceful_shutdown(async move {
                        ct_shutdown.cancelled().await;
                        info!("REST service shutting down");
                    });

                    if let Err(e) = server.await {
                        error!("REST server shutdown with error: {}", e);
                    }
                },
                Err(e) => {
                    error!("Failed to bind REST service to {}: {}", socket_addr, e);
                },
            }
        });

        let (stop_tx, stop_rx) = tokio::sync::oneshot::channel();
        tokio::spawn(async move {
            let _ = stop_rx.await;
            cancellation_token.cancel();
        });

        Ok(ServiceHandle::new(stop_tx, server_handle))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::time::Duration;

    use crate::{
        app::{AppState, Service, ServiceContext},
        config::{Config, ServiceMode},
        database::client::Database,
        health::HealthServer,
        redis::client::Redis,
    };

    fn free_port() -> u16 {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind ephemeral port");
        listener.local_addr().expect("read local addr").port()
    }

    async fn wait_for_http_response(
        client: &reqwest::Client,
        url: &str,
        timeout: Duration,
    ) -> Option<reqwest::StatusCode> {
        let started = tokio::time::Instant::now();
        while started.elapsed() < timeout {
            if let Ok(response) = client.get(url).send().await {
                return Some(response.status());
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }

        None
    }

    fn service_context<'a>(config: &'a Config) -> ServiceContext<'a> {
        let state = Arc::new(AppState {
            hub: None,
            redis: Arc::new(Redis::empty()),
            database: Some(Arc::new(Database::empty())),
            mode: ServiceMode::Consumer,
        });

        ServiceContext::with_config(state, config)
    }

    #[tokio::test]
    async fn startup_smoke_health_and_rest_endpoint_are_reachable() {
        let rest_port = free_port();
        let health_port = free_port();

        let mut config = Config::default();
        config.hub.url = "http://127.0.0.1:9".to_string();

        let rest_service =
            RestService::new().configure("127.0.0.1".to_string(), rest_port).with_max_limit(100);
        let rest_handle = rest_service.start(service_context(&config)).await.unwrap();

        let mut health_server = HealthServer::new(health_port);
        let mut health_runner = health_server.clone();
        let redis = Arc::new(Redis::empty());
        let health_task = tokio::spawn(async move {
            health_runner.run(None, redis, None, ServiceMode::Consumer).await.unwrap();
        });

        let client = reqwest::Client::builder().timeout(Duration::from_secs(2)).build().unwrap();
        let health_url = format!("http://127.0.0.1:{}/health", health_port);
        let rest_url = format!("http://127.0.0.1:{}/api/v1/users/1", rest_port);

        let health_status = wait_for_http_response(&client, &health_url, Duration::from_secs(5))
            .await
            .expect("health endpoint should become reachable");
        assert_eq!(health_status, reqwest::StatusCode::OK);

        let rest_status = wait_for_http_response(&client, &rest_url, Duration::from_secs(5))
            .await
            .expect("rest endpoint should become reachable");
        assert_ne!(rest_status, reqwest::StatusCode::NOT_FOUND);

        rest_handle.stop().await;
        health_server.shutdown().await;
        let _ = health_task.await;
    }

    #[tokio::test]
    async fn rest_service_shutdown_smoke_test() {
        let port = free_port();
        let mut config = Config::default();
        config.hub.url = "http://127.0.0.1:9".to_string();

        let rest_service = RestService::new().configure("127.0.0.1".to_string(), port);
        let handle = rest_service.start(service_context(&config)).await.unwrap();

        let client = reqwest::Client::builder().timeout(Duration::from_secs(2)).build().unwrap();
        let url = format!("http://127.0.0.1:{}/api/v1/users/1", port);

        let status = wait_for_http_response(&client, &url, Duration::from_secs(5))
            .await
            .expect("rest endpoint should become reachable before shutdown");
        assert_ne!(status, reqwest::StatusCode::NOT_FOUND);

        handle.stop().await;

        let mut connection_stopped = false;
        for _ in 0..20 {
            if client.get(&url).send().await.is_err() {
                connection_stopped = true;
                break;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }

        assert!(connection_stopped, "REST endpoint should stop accepting connections");
    }
}
