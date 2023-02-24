#[cfg(feature = "server")]
use self::{
    group_service::GroupService, notification::Notification, session_service::SessionService,
};
#[cfg(feature = "server")]
use crate::state::{ClientId, State};
#[cfg(feature = "server")]
use axum::async_trait;
#[cfg(feature = "server")]
use std::{collections::HashMap, sync::Arc};

pub mod group_service;
pub mod notification;
pub mod session_service;

pub const SUBROUTE_SEPARATOR: &str = "_";

#[cfg(feature = "server")]
type ServiceResponse = Result<Option<json_rpc2::Response>, json_rpc2::Error>;

/// Trait for async services that maybe handle a request.
#[async_trait]
#[cfg(feature = "server")]
pub trait Service: Send + Sync {
    /// Should reply with a response or a `None`, according to the message type
    /// (request or notification).
    ///
    /// If the method is not handled by the service it should
    /// return an error of type `MethodNotFound`.
    async fn handle(
        &self,
        request: &json_rpc2::Request,
        ctx: (Arc<State>, Arc<tokio::sync::Mutex<Vec<Notification>>>),
        client_id: ClientId,
    ) -> Result<Option<json_rpc2::Response>, json_rpc2::Error>;
}

/// Service handler in charge of routing communications to given
/// services, in case they match, otherwise returns not found.
#[cfg(feature = "server")]
pub struct ServiceHandler {
    /// Services that the server should invoke for every request.
    services: HashMap<String, Box<dyn Service>>,
}

#[cfg(feature = "server")]
impl ServiceHandler {
    /// Create a new ServiceHandler.
    pub fn new() -> Self {
        let mut services: HashMap<String, Box<dyn Service>> = HashMap::new();
        let group_service: Box<dyn Service> = Box::new(GroupService {});
        let session_service: Box<dyn Service> = Box::new(SessionService {});
        services.insert(group_service::ROUTE_PREFIX.into(), group_service);
        services.insert(session_service::ROUTE_PREFIX.into(), session_service);
        Self { services }
    }

    /// Infallible service handler, errors are automatically converted to responses.
    ///
    /// If a request was a notification (no id field) this will yield `None`.
    pub async fn serve(
        &self,
        request: &json_rpc2::Request,
        ctx: (Arc<State>, Arc<tokio::sync::Mutex<Vec<Notification>>>),
        client_id: ClientId,
    ) -> Option<json_rpc2::Response> {
        match self.handle(request, ctx, client_id).await {
            Ok(response) => response,
            Err(e) => Some((request, e).into()),
        }
    }

    /// Call services according to subroute.
    ///
    /// If no services match the incoming request this will
    /// return `Error::MethodNotFound`.
    pub async fn handle(
        &self,
        req: &json_rpc2::Request,
        ctx: (Arc<State>, Arc<tokio::sync::Mutex<Vec<Notification>>>),
        client_id: ClientId,
    ) -> Result<Option<json_rpc2::Response>, json_rpc2::Error> {
        let subroute = req.method().split(SUBROUTE_SEPARATOR);
        let subroute: Vec<&str> = subroute.collect();

        if subroute.len() < 2 {
            return Ok(Some(self.build_error_not_found_method(req)));
        }

        let subroute = subroute[0];
        if let Some(service) = self.services.get(subroute) {
            return service.handle(req, ctx, client_id).await;
        }

        Ok(Some(self.build_error_not_found_method(req)))
    }

    fn build_error_not_found_method(&self, req: &json_rpc2::Request) -> json_rpc2::Response {
        let err = json_rpc2::Error::MethodNotFound {
            name: req.method().to_string(),
            id: req.id().clone(),
        };
        (req, err).into()
    }
}

#[cfg(feature = "server")]
impl Default for ServiceHandler {
    fn default() -> Self {
        Self::new()
    }
}
