use crate::{
    service::{notification::Notification, ServiceHandler},
    state::{ClientId, State},
};
use axum::extract::ws::{self, WebSocket};
use futures_util::{SinkExt, StreamExt};
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};
use tokio_stream::wrappers::UnboundedReceiverStream;

/// Server handler that manages websocket communications.
pub struct Server {
    /// Common shared state among the server.
    state: Arc<State>,
    /// Unique client id.
    client_id: ClientId,
    /// Service handler for json-rpc requests.
    service_handler: Arc<ServiceHandler>,
}

impl Server {
    /// Creates a new server object.
    pub fn new(state: Arc<State>, service_handler: Arc<ServiceHandler>) -> Self {
        let client_id = state.new_client_id();
        Self {
            state,
            client_id,
            service_handler,
        }
    }

    /// Handles incoming websocket connection, consuming
    /// self in the process, not allowing more than one
    /// connection to be processed with the same `Server` instance.
    ///
    /// # Implementation notes
    ///
    /// Heartbeat is implemented automatically by tokio-tungstenite
    /// so there's no need to implement manually.
    ///
    /// `register_client` must be called before handle_connection otherwise server will panic
    #[tracing::instrument(name = "Handling connection", skip_all, fields(client_id = self.client_id.to_string()))]
    pub async fn handle_connection(self, socket: WebSocket) {
        let (mut ws_tx, mut ws_rx) = socket.split();
        let (internal_tx, internal_rx) = mpsc::unbounded_channel::<String>();
        let mut internal_rx = UnboundedReceiverStream::new(internal_rx);

        // Save client
        self.state.add_client(self.client_id, internal_tx).await;

        let self_c = Arc::new(self);
        let self_cc = self_c.clone();
        let receive_task = tokio::spawn(async move {
            while let Some(Ok(msg)) = ws_rx.next().await {
                // Ignore messages that are not text
                if let ws::Message::Text(txt) = msg {
                    self_cc.handle_incoming_message(txt).await;
                }
            }
        });

        let send_task = tokio::spawn(async move {
            while let Some(msg) = internal_rx.next().await {
                if let Err(err) = ws_tx.send(ws::Message::Text(msg)).await {
                    tracing::error!(error = ?err, "Error while sending message to websocket");
                    break;
                }
            }
        });

        tokio::select! {
            _ = receive_task => tracing::info!("Closing connection due to rx channel closed"),
            _ = send_task => tracing::info!("Closing connection due to tx channel closed"),
        }

        // Perform any operation needed after connection closed
        self_c.state.drop_client(self_c.client_id).await;
    }

    /// Handle incoming text message.
    #[tracing::instrument(name = "Handling incoming message", skip_all, fields(client_id = self.client_id.to_string(), method))]
    async fn handle_incoming_message(&self, msg: String) {
        match json_rpc2::from_str(&msg) {
            Ok(req) => self.handle_rpc_request(&req).await,
            Err(err) => tracing::warn!(
                client_id = self.client_id.to_string(),
                message = msg,
                error = ?err,
                "Error decoding incoming message as json-rpc"
            ),
        }
    }

    /// Handle json-rpc request.
    async fn handle_rpc_request(&self, req: &json_rpc2::Request) {
        tracing::Span::current().record("method", req.method());

        let notifications = Arc::new(Mutex::new(vec![]));

        let res = self
            .service_handler
            .serve(
                req,
                (self.state.clone(), notifications.clone()),
                self.client_id,
            )
            .await;
        if let Some(res) = res {
            self.send_rpc_response(&res, &self.client_id).await;
        }
        for notification in notifications.lock().await.iter() {
            self.handle_rpc_notification(notification).await; // TODO: perhaps this could be parallelized?
        }
    }

    /// Handle json-rpc notifications.
    async fn handle_rpc_notification(&self, notification: &Notification) {
        match notification {
            Notification::Group {
                group_id,
                filter,
                method,
                message,
            } => {
                let groups = self.state.groups.read().await;
                let Some(group) = groups.get(group_id) else {
                    return tracing::warn!(
                        group_id = group_id.to_string(),
                        "Group not found while sending group notification"
                    );
                };
                let request = json_rpc2::Request::new(
                    None,
                    method.into(),
                    Some(message.clone()), //FIXME check message format
                );
                let filtered_clients = group
                    .clients
                    .iter()
                    .filter(|&client_id| !filter.iter().any(|c| c == client_id));
                for client_id in filtered_clients {
                    self.send_rpc_request(&request, client_id).await;
                }
            }
            Notification::Session {
                group_id,
                session_id,
                filter,
                method,
                message,
            } => {
                tracing::info!("Sending notification to session");
                let groups = self.state.groups.read().await;
                let Some(group) = groups.get(group_id) else {
                    return tracing::warn!(
                        group_id = group_id.to_string(),
                        "Group not found while sending session notification"
                    );
                };
                let Some(session) = group.get_session(session_id) else {
                    return tracing::warn!(
                        session_id = session_id.to_string(),
                        "Session not found while sending session notification"
                    )
                };

                let request = json_rpc2::Request::new(None, method.into(), Some(message.clone()));
                let filtered_clients = session
                    .party_signups
                    .iter()
                    .filter(|(_, &client_id)| !filter.iter().any(|&c| c == client_id))
                    .filter(|(_, &client_id)| client_id != self.client_id);
                for (_, client_id) in filtered_clients {
                    self.send_rpc_request(&request, client_id).await;
                }
            }
            Notification::Relay { method, messages } => {
                for (client_id, message) in messages {
                    let request =
                        json_rpc2::Request::new(None, method.into(), Some(message.clone()));
                    self.send_rpc_request(&request, client_id).await;
                }
            }
        }
    }

    /// Sends json-rpc response.
    async fn send_rpc_response(&self, res: &json_rpc2::Response, client_id: &ClientId) {
        tracing::debug!(client_id = client_id.to_string(), "Sending response");
        let Some(tx) = self.state.get_client(client_id).await else {
            return tracing::warn!(client_id = client_id.to_string(), "Client not found");
        };
        let message = serde_json::to_string(&res).unwrap(); // FIXME: unwrap
        tx.send(message).unwrap(); //FIXME: unwrap
    }

    /// Sends json-rpc request. This method is especially used for notifications.
    async fn send_rpc_request(&self, req: &json_rpc2::Request, client_id: &ClientId) {
        tracing::debug!(client_id = client_id.to_string(), "Sending request");
        let Some(tx) = self.state.get_client(client_id).await else {
            return tracing::warn!(client_id = client_id.to_string(), "Client not found");
        };
        let message = serde_json::to_string(&req).unwrap(); // FIXME: unwrap
        tx.send(message).unwrap(); //FIXME: unwrap
    }

    /// Returns client id.
    pub fn client_id(&self) -> ClientId {
        self.client_id
    }
}
