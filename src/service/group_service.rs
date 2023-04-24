//! # Group service
//!
//! This module contains the group service that handles incoming requests
//! for group management.

use crate::state::{
    group::{Group, GroupId},
    parameters::Parameters,
};
use serde::{Deserialize, Serialize};
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
use tokio::sync::Mutex;

/// Prefix for group routes.
pub const ROUTE_PREFIX: &str = "group";

/// Available group methods.
#[derive(Debug, Display, EnumString)]
pub enum GroupMethod {
    #[strum(serialize = "group_create")]
    GroupCreate,
    #[strum(serialize = "group_join")]
    GroupJoin,
}

/// Group create request.
#[derive(Deserialize, Serialize)]
pub struct GroupCreateRequest {
    pub parameters: Parameters,
}

/// Group create response.
#[derive(Deserialize, Serialize)]
pub struct GroupCreateResponse {
    pub group: Group,
}

/// Group join request.
#[derive(Deserialize, Serialize)]
pub struct GroupJoinRequest {
    #[serde(rename = "groupId")]
    pub group_id: GroupId,
}

/// Group join response.
#[derive(Deserialize, Serialize)]
pub struct GroupJoinResponse {
    pub group: Group,
}

/// Group service that handles incoming requests and maps
/// them to the corresponding methods.
#[cfg(feature = "server")]
pub struct GroupService;

#[axum::async_trait]
#[cfg(feature = "server")]
impl Service for GroupService {
    async fn handle(
        &self,
        req: &Request,
        ctx: (
            std::sync::Arc<State>,
            std::sync::Arc<Mutex<Vec<Notification>>>,
        ),
        client_id: ClientId,
    ) -> ServiceResponse {
        let method =
            GroupMethod::from_str(req.method()).map_err(|_| json_rpc2::Error::MethodNotFound {
                name: req.method().to_string(),
                id: req.id().clone(),
            })?;
        let response = match method {
            GroupMethod::GroupCreate => self.group_create(req, ctx, client_id).await?,
            GroupMethod::GroupJoin => self.group_join(req, ctx, client_id).await?,
        };
        Ok(response)
    }
}

#[cfg(feature = "server")]
impl GroupService {
    async fn group_create(
        &self,
        req: &Request,
        ctx: (
            std::sync::Arc<State>,
            std::sync::Arc<Mutex<Vec<Notification>>>,
        ),
        client_id: ClientId,
    ) -> ServiceResponse {
        tracing::info!("Creating a new group");
        let params: GroupCreateRequest = req.deserialize()?;
        let (state, _) = ctx;
        params
            .parameters
            .validate()
            .map_err(|e| Error::InvalidParams {
                id: req.id().clone(),
                data: e.to_string(),
            })?;

        let group = state.add_group(params.parameters).await;
        state
            .join_group(group.id, client_id)
            .await
            .map_err(|e| Error::from(Box::from(e)))?;
        tracing::info!(group_id = group.id().to_string(), "Group created");
        let res = serde_json::to_value(GroupCreateResponse { group })
            .map_err(|e| Error::from(Box::from(e)))?;
        Ok(Some((req, res).into()))
    }

    async fn group_join(
        &self,
        req: &Request,
        ctx: (
            std::sync::Arc<State>,
            std::sync::Arc<Mutex<Vec<Notification>>>,
        ),
        client_id: ClientId,
    ) -> ServiceResponse {
        let params: GroupJoinRequest = req.deserialize()?;
        tracing::info!(
            group_id = params.group_id.to_string(),
            "Joining client to group"
        );
        let (state, _) = ctx;
        let group = state
            .join_group(params.group_id, client_id)
            .await
            .map_err(|e| Error::InvalidParams {
                id: req.id().clone(),
                data: e.to_string(),
            })?;
        let res = serde_json::to_value(GroupJoinResponse { group })
            .map_err(|e| Error::from(Box::from(e)))?;
        Ok(Some((req, res).into()))
    }
}
