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

pub type GroupId = Uuid;

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
    pub fn new(id: GroupId, params: Parameters) -> Self {
        Self {
            id,
            params,
            sessions: HashMap::new(),
            clients: HashSet::new(),
        }
    }

    #[cfg(feature = "server")]
    pub fn add_client(&mut self, client_id: ClientId) -> anyhow::Result<()> {
        let clients = self.clients.len();
        if clients >= self.params.n().into() {
            return Err(GroupError::GroupFull.into());
        }
        self.clients.insert(client_id);
        Ok(())
    }

    #[cfg(feature = "server")]
    pub fn drop_client(&mut self, client_id: ClientId) {
        self.clients.remove(&client_id);
        // FIXME: delete from sessions too
    }

    #[cfg(feature = "server")]
    pub fn add_session(&mut self, kind: SessionKind, value: SessionValue) -> Session {
        let session_id = Uuid::new_v4();
        let session = Session::new(session_id, kind, value);
        let session_c = session.clone();
        self.sessions.insert(session_id, session);
        session_c
    }

    #[cfg(feature = "server")]
    pub fn get_session(&self, session_id: &SessionId) -> Option<&Session> {
        self.sessions.get(session_id)
    }

    #[cfg(feature = "server")]
    pub fn get_session_mut(&mut self, session_id: &SessionId) -> Option<&mut Session> {
        self.sessions.get_mut(session_id)
    }

    #[cfg(feature = "server")]
    pub fn is_empty(&self) -> bool {
        self.clients.len() == 0
    }

    #[cfg(feature = "server")]
    pub fn is_full(&self) -> bool {
        self.clients.len() == self.params.n() as usize
    }

    #[cfg(feature = "server")]
    pub fn id(&self) -> GroupId {
        self.id
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
