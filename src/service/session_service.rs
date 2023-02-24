use crate::state::{
    group::{Group, GroupId},
    session::{Session, SessionId, SessionKind, SessionPartyNumber},
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use strum::{Display, EnumString};

#[cfg(feature = "server")]
use super::{notification::Notification, Service, ServiceResponse};
#[cfg(feature = "server")]
use crate::state::{ClientId, State};
#[cfg(feature = "server")]
use json_rpc2::{Error, Request};
#[cfg(feature = "server")]
use std::str::FromStr;
#[cfg(feature = "server")]
use std::sync::Arc;
#[cfg(feature = "server")]
use tokio::sync::Mutex;

pub const ROUTE_PREFIX: &str = "session";

#[derive(Debug, Display, EnumString)]
pub enum SessionMethod {
    #[strum(serialize = "session_create")]
    SessionCreate,
    #[strum(serialize = "session_signup")]
    SessionSignup,
    #[strum(serialize = "session_login")]
    SessionLogin,
    #[strum(serialize = "session_message")]
    SessionMessage,
}

#[derive(Debug, Display, EnumString)]
pub enum SessionEvent {
    #[strum(serialize = "session_created")]
    SessionCreated,
    #[strum(serialize = "session_ready")]
    SessionReady,
    #[strum(serialize = "session_message")]
    SessionMessage,
}

#[derive(Deserialize, Serialize)]
pub struct SessionCreateRequest {
    #[serde(rename = "groupId")]
    pub group_id: GroupId,
    pub kind: SessionKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<Value>,
}

#[derive(Serialize)]
pub struct SessionCreateResponse {
    session: Session,
}

#[derive(Deserialize, Serialize)]
pub struct SessionCreatedNotification {
    group: Group,
    session: Session,
}

#[derive(Deserialize, Serialize)]
pub struct SessionSignupRequest {
    #[serde(rename = "groupId")]
    pub group_id: GroupId,
    #[serde(rename = "sessionId")]
    pub session_id: SessionId,
}

#[derive(Serialize)]
pub struct SessionSignupResponse {
    session: Session,
    #[serde(rename = "partyNumber")]
    party_number: SessionPartyNumber,
}

#[derive(Deserialize, Serialize)]
pub struct SessionLoginRequest {
    #[serde(rename = "groupId")]
    pub group_id: GroupId,
    #[serde(rename = "sessionId")]
    pub session_id: SessionId,
    #[serde(rename = "partyNumber")]
    pub party_number: SessionPartyNumber,
}

#[derive(Serialize)]
pub struct SessionLoginResponse {
    session: Session,
}

#[derive(Deserialize, Serialize)]
pub struct SessionReadyNotification {
    group: Group,
    session: Session,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct SessionMessageRequest<T: Serialize = Value> {
    #[serde(rename = "groupId")]
    pub group_id: GroupId,
    #[serde(rename = "sessionId")]
    pub session_id: SessionId,
    pub receiver: Option<SessionPartyNumber>,
    pub message: T,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SessionMessageNotification<T: Serialize = Value> {
    #[serde(rename = "groupId")]
    pub group_id: GroupId,
    #[serde(rename = "sessionId")]
    pub session_id: SessionId,
    pub sender: SessionPartyNumber,
    pub message: T,
}

#[derive(Debug)]
#[cfg(feature = "server")]
pub struct SessionService;

#[axum::async_trait]
#[cfg(feature = "server")]
impl Service for SessionService {
    async fn handle(
        &self,
        req: &Request,
        ctx: (Arc<State>, Arc<Mutex<Vec<Notification>>>),
        client_id: ClientId,
    ) -> ServiceResponse {
        let method = SessionMethod::from_str(req.method()).map_err(|_| {
            json_rpc2::Error::MethodNotFound {
                name: req.method().to_string(),
                id: req.id().clone(),
            }
        })?;
        let response = match method {
            SessionMethod::SessionCreate => self.session_create(req, ctx, client_id).await?,
            SessionMethod::SessionSignup => self.session_signup(req, ctx, client_id).await?,
            SessionMethod::SessionLogin => self.session_login(req, ctx, client_id).await?,
            SessionMethod::SessionMessage => self.session_message(req, ctx, client_id).await?,
        };
        Ok(response)
    }
}

#[cfg(feature = "server")]
impl SessionService {
    async fn session_create(
        &self,
        req: &Request,
        ctx: (Arc<State>, Arc<Mutex<Vec<Notification>>>),
        client_id: ClientId,
    ) -> ServiceResponse {
        let params: SessionCreateRequest = req.deserialize()?;
        tracing::info!(
            group_id = params.group_id.to_string(),
            "Creating a new session"
        );
        let (state, notifications) = ctx;
        let (group, session) = state
            .add_session(params.group_id, params.kind, params.value)
            .await
            .map_err(|e| Error::InvalidParams {
                id: req.id().clone(),
                data: e.to_string(),
            })?;

        let res = serde_json::to_value(SessionCreateResponse {
            session: session.clone(),
        })
        .map_err(|e| Error::from(Box::from(e)))?;
        let notification = serde_json::to_value(SessionCreatedNotification { group, session })
            .map_err(|e| Error::from(Box::from(e)))?;

        notifications.lock().await.push(Notification::Group {
            group_id: params.group_id,
            filter: vec![client_id],
            method: SessionEvent::SessionCreated.to_string(),
            message: notification.clone(),
        });
        Ok(Some((req, res).into()))
    }

    async fn session_signup(
        &self,
        req: &Request,
        ctx: (Arc<State>, Arc<Mutex<Vec<Notification>>>),
        client_id: ClientId,
    ) -> ServiceResponse {
        let params: SessionSignupRequest = req.deserialize()?;
        tracing::info!(
            group_id = params.group_id.to_string(),
            session_id = params.session_id.to_string(),
            "Signing up client to a session"
        );
        let (state, notifications) = ctx;

        let (group, session, party_number, threshold) = state
            .signup_session(client_id, params.group_id, params.session_id)
            .await
            .map_err(|e| Error::InvalidParams {
                id: req.id().clone(),
                data: e.to_string(),
            })?;

        let res = serde_json::to_value(SessionSignupResponse {
            session: session.clone(),
            party_number,
        })
        .map_err(|e| Error::from(Box::from(e)))?;

        if threshold {
            let notification = serde_json::to_value(SessionReadyNotification { group, session })
                .map_err(|e| Error::from(Box::from(e)))?;
            notifications.lock().await.push(Notification::Group {
                group_id: params.group_id,
                filter: vec![],
                method: SessionEvent::SessionReady.to_string(),
                message: notification,
            });
        }
        Ok(Some((req, res).into()))
    }
    async fn session_login(
        &self,
        req: &Request,
        ctx: (Arc<State>, Arc<Mutex<Vec<Notification>>>),
        client_id: ClientId,
    ) -> ServiceResponse {
        let params: SessionLoginRequest = req.deserialize()?;
        tracing::info!(
            group_id = params.group_id.to_string(),
            session_id = params.session_id.to_string(),
            "Loggin in client to a session"
        );
        let (state, notifications) = ctx;
        let (group, session, threshold) = state
            .login_session(
                client_id,
                params.group_id,
                params.session_id,
                params.party_number,
            )
            .await
            .map_err(|e| Error::InvalidParams {
                id: req.id().clone(),
                data: e.to_string(),
            })?;
        let res = serde_json::to_value(SessionLoginResponse {
            session: session.clone(),
        })
        .map_err(|e| Error::from(Box::from(e)))?;
        if threshold {
            let notification = serde_json::to_value(SessionReadyNotification { group, session })
                .map_err(|e| Error::from(Box::from(e)))?;
            notifications.lock().await.push(Notification::Group {
                group_id: params.group_id,
                filter: vec![],
                method: SessionEvent::SessionReady.to_string(),
                message: notification,
            });
        }
        Ok(Some((req, res).into()))
    }
    async fn session_message(
        &self,
        req: &Request,
        ctx: (Arc<State>, Arc<Mutex<Vec<Notification>>>),
        client_id: ClientId,
    ) -> ServiceResponse {
        let params: SessionMessageRequest = req.deserialize()?;
        tracing::info!(
            group_id = params.group_id.to_string(),
            session_id = params.session_id.to_string(),
            "Sending message to session"
        );
        let (state, notifications) = ctx;

        let self_party_number = state
            .get_party_number_from_client_id(params.group_id, params.session_id, client_id)
            .await
            .map_err(|e| Error::InvalidParams {
                id: req.id().clone(),
                data: e.to_string(),
            })?;
        state
            .validate_group_and_session(params.group_id, params.session_id)
            .await
            .map_err(|e| Error::InvalidParams {
                id: req.id().clone(),
                data: e.to_string(),
            })?;

        let res = serde_json::to_value(SessionMessageNotification {
            group_id: params.group_id,
            session_id: params.session_id,
            message: params.message,
            sender: self_party_number,
        })
        .map_err(|e| Error::from(Box::from(e)))?;

        let mut notifications = notifications.lock().await;
        match params.receiver {
            Some(party_number) => {
                let receiver_client_id = state
                    .get_client_id_from_party_number(
                        params.group_id,
                        params.session_id,
                        party_number,
                    )
                    .await
                    .map_err(|e| Error::InvalidParams {
                        id: req.id().clone(),
                        data: e.to_string(),
                    })?;
                notifications.push(Notification::Relay {
                    method: SessionEvent::SessionMessage.to_string(),
                    messages: vec![(receiver_client_id, res)],
                })
            }
            None => notifications.push(Notification::Session {
                method: SessionEvent::SessionMessage.to_string(),
                group_id: params.group_id,
                session_id: params.session_id,
                filter: vec![],
                message: res,
            }),
        };

        Ok(None)
    }
}
