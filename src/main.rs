#[cfg(feature = "server")]
use axum::extract::State as AxumState;
#[cfg(feature = "server")]
use axum::extract::WebSocketUpgrade;
#[cfg(feature = "server")]
use axum::response::IntoResponse;
#[cfg(feature = "server")]
use axum::routing::get;
#[cfg(feature = "server")]
use axum::Router;
#[cfg(feature = "server")]
use mpc_manager::server::Server;
#[cfg(feature = "server")]
use mpc_manager::service::ServiceHandler;
#[cfg(feature = "server")]
use mpc_manager::state::State;
#[cfg(feature = "server")]
use mpc_manager::telemetry::{get_subscriber, init_subscriber};
#[cfg(feature = "server")]
use std::net::SocketAddr;
#[cfg(feature = "server")]
use std::sync::Arc;
#[cfg(feature = "server")]
use tower_http::trace::{DefaultMakeSpan, TraceLayer};

#[cfg(feature = "server")]
async fn ws_handler(
    ws: WebSocketUpgrade,
    AxumState(app_state): AxumState<Arc<AppState>>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| {
        let state = app_state.state.clone();
        let service_handler = app_state.service_handler.clone();
        let server = Server::new(state, service_handler);
        server.handle_connection(socket)
    })
}

#[cfg(feature = "server")]
struct AppState {
    state: Arc<State>,
    service_handler: Arc<ServiceHandler>,
}

#[tokio::main]
#[cfg(feature = "server")]
async fn main() {
    let subscriber = get_subscriber("mpc-manager".into(), "info".into(), std::io::stdout);
    init_subscriber(subscriber);

    let state = Arc::new(State::new());
    let service_handler = Arc::new(ServiceHandler::new());
    let app_state = Arc::new(AppState {
        state,
        service_handler,
    });

    let app = Router::new()
        .route("/", get(ws_handler))
        .with_state(app_state)
        .layer(TraceLayer::new_for_http().make_span_with(DefaultMakeSpan::default()));

    let addr = SocketAddr::from(([127, 0, 0, 1], 8080));
    tracing::info!("Listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service_with_connect_info::<SocketAddr>())
        .await
        .unwrap();
}
