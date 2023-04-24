//! Group state
//!
//! This module contains all the logic related to group management.

use super::{
    parameters::Parameters,
    session::{Session, SessionId},
    ClientId,
};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use thiserror::Error;
use uuid::Uuid;

#[cfg(feature = "server")]
use super::session::{SessionKind, SessionValue};

/// Unique ID of a group.
pub type GroupId = Uuid;

/// Error type for group operations.
#[derive(Debug, Error)]
pub enum GroupError {
    /// Error generated when the group is full.
    #[error("group is full")]
    GroupFull,
}

/// Group is a collection of clients. It is the main unit of communication.
#[derive(Debug, Deserialize, Serialize)]
pub struct Group {
    /// Unique ID of the group.
    pub id: GroupId,
    /// Parameters of the group.
    pub params: Parameters,
    /// Sessions belonging to this group.
    #[serde(skip)]
    pub(crate) sessions: HashMap<SessionId, Session>,
    /// Clients that joined this group.
    #[serde(skip)]
    pub(crate) clients: HashSet<ClientId>,
}

impl Group {
    /// Creates a new group with the given parameters.
    pub fn new(id: GroupId, params: Parameters) -> Self {
        Self {
            id,
            params,
            sessions: HashMap::new(),
            clients: HashSet::new(),
        }
    }

    /// Adds a client to the group.
    #[cfg(feature = "server")]
    pub fn add_client(&mut self, client_id: ClientId) -> anyhow::Result<()> {
        let clients = self.clients.len();
        if clients >= self.params.n().into() {
            return Err(GroupError::GroupFull.into());
        }
        self.clients.insert(client_id);
        Ok(())
    }

    /// Removes a client from the group.
    #[cfg(feature = "server")]
    pub fn drop_client(&mut self, client_id: ClientId) {
        self.clients.remove(&client_id);
        // FIXME: delete from sessions too
    }

    /// Adds a new session and adds it to the group.
    #[cfg(feature = "server")]
    pub fn add_session(&mut self, kind: SessionKind, value: SessionValue) -> Session {
        let session_id = Uuid::new_v4();
        let session = Session::new(session_id, kind, value);
        let session_c = session.clone();
        self.sessions.insert(session_id, session);
        session_c
    }

    /// Returns a session by its ID, if it exists.
    #[cfg(feature = "server")]
    pub fn get_session(&self, session_id: &SessionId) -> Option<&Session> {
        self.sessions.get(session_id)
    }

    /// Returns a mutable session by its ID, if it exists.
    #[cfg(feature = "server")]
    pub fn get_session_mut(&mut self, session_id: &SessionId) -> Option<&mut Session> {
        self.sessions.get_mut(session_id)
    }

    /// Returns a boolean indicating if the group is empty.
    #[cfg(feature = "server")]
    pub fn is_empty(&self) -> bool {
        self.clients.len() == 0
    }

    /// Returns a boolean indicating if the group is full.
    #[cfg(feature = "server")]
    pub fn is_full(&self) -> bool {
        self.clients.len() == self.params.n() as usize
    }

    /// Returns the ID of the group.
    #[cfg(feature = "server")]
    pub fn id(&self) -> GroupId {
        self.id
    }

    /// Returns the client ids associated with the group.
    #[cfg(feature = "server")]
    pub fn clients(&self) -> &HashSet<ClientId> {
        &self.clients
    }
}

impl Clone for Group {
    /// Clones group parameters, disregarding sensitive information.
    ///
    /// Should be used only for logging purposes.
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            params: self.params.clone(),
            sessions: HashMap::new(),
            clients: HashSet::new(),
        }
    }
}
