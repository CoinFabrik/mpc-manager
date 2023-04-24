//! State module.
//!
//! This module contains the state of the server and the different types used
//! to represent it.

use uuid::Uuid;

#[cfg(feature = "server")]
use self::{
    group::{Group, GroupId},
    parameters::Parameters,
    session::{Session, SessionId, SessionKind, SessionPartyNumber, SessionValue},
};
#[cfg(feature = "server")]
use anyhow::Result;
#[cfg(feature = "server")]
use std::collections::HashMap;
#[cfg(feature = "server")]
use thiserror::Error;
#[cfg(feature = "server")]
use tokio::sync::{mpsc::UnboundedSender, RwLock};

pub mod group;
pub mod parameters;
pub mod session;

/// Unique ID of a client.
pub type ClientId = Uuid;

/// Error type for state operations.
#[derive(Debug, Error)]
#[cfg(feature = "server")]
pub enum StateError {
    /// Error generated when a group was not found.
    #[error("group `{0}` not found")]
    GroupNotFound(GroupId),
    /// Error generated when a session was not found.
    #[error("session `{0}` fro group `{1} not found")]
    SessionNotFound(SessionId, GroupId),
    /// Error generated when a group is already full.
    #[error("group `{0}` is full")]
    GroupIsFull(GroupId),
    /// Error generated when a party number was not found.
    #[error("party `{0}` not found")]
    PartyNotFound(SessionPartyNumber),
    /// Error generated when a client was not found.
    #[error("client id `{0}` not found")]
    ClientNotFound(ClientId),
}

/// Shared state of clients and db managed by the server.
#[derive(Debug, Default)]
#[cfg(feature = "server")]
pub struct State {
    /// Connected clients.
    clients: RwLock<HashMap<ClientId, UnboundedSender<String>>>,
    /// Collection of groups mapped by UUID.
    groups: RwLock<HashMap<GroupId, Group>>,
}

#[cfg(feature = "server")]
impl State {
    /// Return new state, should only be called once.
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns a new client id.
    pub fn new_client_id(&self) -> ClientId {
        Uuid::new_v4()
    }

    /// Adds a new client.
    pub async fn add_client(&self, id: ClientId, tx: UnboundedSender<String>) {
        self.clients.write().await.insert(id, tx);
    }

    /// Returns client data.
    pub async fn get_client(&self, id: &ClientId) -> Option<UnboundedSender<String>> {
        self.clients.read().await.get(id).cloned()
    }

    /// Drops a client, performing all necessary cleanup to preserve
    /// security.
    pub async fn drop_client(&self, id: ClientId) {
        // Remove client from groups and remove group if empty
        let mut groups = self.groups.write().await;
        let mut empty_groups: Vec<Uuid> = Vec::new();
        groups.iter_mut().for_each(|(group_id, group)| {
            group.drop_client(id);
            if group.is_empty() {
                empty_groups.push(*group_id);
            }
        });
        empty_groups.iter().for_each(|group_id| {
            tracing::info!(group_id = group_id.to_string(), "Removing empty group");
            groups.remove(group_id);
        });

        // TODO: remove from sessions?

        // Remove client
        self.clients.write().await.remove(&id);
    }

    /// Adds a new group to the state, returning a clone without
    /// sensitive information for logging purposes.
    pub async fn add_group(&self, params: Parameters) -> Group {
        let uuid = Uuid::new_v4();
        let group = Group::new(uuid, params);
        let group_c = group.clone();
        self.groups.write().await.insert(uuid, group);
        group_c
    }

    /// Joins a client to a group, returning a clone without
    /// sensitive information for logging purposes.
    pub async fn join_group(&self, group_id: GroupId, client_id: ClientId) -> Result<Group> {
        // Validate group exists and is not full
        let groups = self.groups.read().await;
        let group = groups
            .get(&group_id)
            .ok_or(StateError::GroupNotFound(group_id))?;
        if group.is_full() {
            return Err(StateError::GroupIsFull(group_id).into());
        }

        // Join group
        let mut groups = self.groups.write().await;
        let group = groups.get_mut(&group_id).unwrap(); // validation was done previously
        group.add_client(client_id)?;
        Ok(group.clone())
    }

    /// Adds a new session, returning a clone without sensitive information
    /// for logging purposes.
    pub async fn add_session(
        &self,
        group_id: GroupId,
        kind: SessionKind,
        value: SessionValue,
    ) -> Result<(Group, Session)> {
        // Validate group exists
        let groups = self.groups.read().await;
        groups
            .get(&group_id)
            .ok_or(StateError::GroupNotFound(group_id))?;

        // Add session
        let mut groups = self.groups.write().await;
        let group = groups.get_mut(&group_id).unwrap();
        let session = group.add_session(kind, value);
        Ok((group.clone(), session))
    }

    /// Registers a client to a given session and returns
    /// a session clone, session party number and a boolean
    /// indicating if the threshold has been reached.
    pub async fn signup_session(
        &self,
        client_id: ClientId,
        group_id: GroupId,
        session_id: SessionId,
    ) -> Result<(Group, Session, SessionPartyNumber, bool)> {
        // Validate group and session exist
        let groups = self.groups.read().await;
        let group = groups
            .get(&group_id)
            .ok_or(StateError::GroupNotFound(group_id))?;
        group
            .get_session(&session_id)
            .ok_or(StateError::SessionNotFound(session_id, group_id))?;

        // Signup session
        let mut groups = self.groups.write().await;
        let group = groups.get_mut(&group_id).unwrap();
        let session = group.get_session_mut(&session_id).unwrap();
        let party_index = session.signup(client_id)?;

        let parties = session.get_number_of_clients();
        let session_c = session.clone();
        let threshold = group.params.threshold_reached(session_c.kind, parties);
        Ok((group.clone(), session_c, party_index, threshold))
    }

    /// Logins a client witha given party number to a session and returns
    /// the session and a boolean indicating if the threshold has been reached.
    pub async fn login_session(
        &self,
        client_id: ClientId,
        group_id: GroupId,
        session_id: SessionId,
        party_number: SessionPartyNumber,
    ) -> Result<(Group, Session, bool)> {
        // Validate group and session exist
        let groups = self.groups.read().await;
        let group = groups
            .get(&group_id)
            .ok_or(StateError::GroupNotFound(group_id))?;
        group
            .get_session(&session_id)
            .ok_or(StateError::SessionNotFound(session_id, group_id))?;

        // Login session
        let mut groups = self.groups.write().await;
        let group = groups.get_mut(&group_id).unwrap();
        let session = group.get_session_mut(&session_id).unwrap();
        session.login(client_id, party_number)?;
        let session_c = session.clone();
        let parties = session.party_signups.len();
        let threshold = group.params.threshold_reached(session_c.kind, parties);
        Ok((group.clone(), session_c, threshold))
    }

    /// Returns client ids associated with a given group, if it exists.
    pub async fn get_client_ids_from_group(&self, group_id: &GroupId) -> Result<Vec<ClientId>> {
        let groups = self.groups.read().await;
        let group = groups
            .get(group_id)
            .ok_or(StateError::GroupNotFound(*group_id))?;
        let client_ids: Vec<ClientId> = group.clients().iter().copied().collect();
        Ok(client_ids)
    }

    /// Returns client ids associated with a given session, if it exists.
    pub async fn get_client_ids_from_session(
        &self,
        group_id: &GroupId,
        session_id: &SessionId,
    ) -> Result<Vec<ClientId>> {
        let groups = self.groups.read().await;
        let group = groups
            .get(group_id)
            .ok_or(StateError::GroupNotFound(*group_id))?;
        let session = group
            .get_session(session_id)
            .ok_or(StateError::SessionNotFound(*session_id, *group_id))?;
        let client_ids = session.get_all_client_ids();
        Ok(client_ids)
    }

    /// Returns client id associated with a given session and party number.
    pub async fn get_client_id_from_party_number(
        &self,
        group_id: GroupId,
        session_id: SessionId,
        party_number: SessionPartyNumber,
    ) -> Result<ClientId> {
        // Validate group, session and party number exist.
        let groups = self.groups.read().await;
        let group = groups
            .get(&group_id)
            .ok_or(StateError::GroupNotFound(group_id))?;
        let session = group
            .get_session(&session_id)
            .ok_or(StateError::SessionNotFound(session_id, group_id))?;

        // Get client id
        let client_id = session
            .get_client_id(party_number)
            .ok_or(StateError::PartyNotFound(party_number))?;
        Ok(client_id)
    }

    /// Returns the party number of a given client id.
    pub async fn get_party_number_from_client_id(
        &self,
        group_id: GroupId,
        session_id: SessionId,
        client_id: ClientId,
    ) -> Result<SessionPartyNumber> {
        let groups = self.groups.read().await;
        let group = groups
            .get(&group_id)
            .ok_or(StateError::GroupNotFound(group_id))?;
        let session = group
            .get_session(&session_id)
            .ok_or(StateError::SessionNotFound(session_id, group_id))?;
        let party_number = session
            .get_party_number(&client_id)
            .ok_or(StateError::ClientNotFound(client_id))?;
        Ok(party_number)
    }

    /// Helper function that validates if group and session are valid.
    pub async fn validate_group_and_session(
        &self,
        group_id: GroupId,
        session_id: SessionId,
    ) -> Result<()> {
        let groups = self.groups.read().await;
        let group = groups
            .get(&group_id)
            .ok_or(StateError::GroupNotFound(group_id))?;
        group
            .get_session(&session_id)
            .ok_or(StateError::SessionNotFound(session_id, group_id))?;
        Ok(())
    }
}
