use serde_json::Value;

use crate::state::{group::GroupId, session::SessionId, ClientId};

/// Notification sent by the server to multiple connected clients.
#[derive(Debug)]
pub enum Notification {
    /// Send to all the clients in a group.
    Group {
        /// The group identifier.
        group_id: GroupId,
        /// Ignore these clients.
        filter: Vec<ClientId>,
        /// The method name.
        method: String,
        /// Message to send to the clients.
        message: Value,
    },

    /// Sends to all clients in a session.
    Session {
        /// The group identifier.
        group_id: GroupId,
        /// The session identifier.
        session_id: SessionId,
        /// Ignore these clients.
        filter: Vec<ClientId>,
        /// The method name.
        method: String,
        /// Message to send to the clients.
        message: Value,
    },

    /// Relay messages to specific clients.
    ///
    /// Used for relaying peer to peer messages.
    Relay {
        /// The method name.
        method: String,
        /// Mapping of client connection identifiers to messages.
        messages: Vec<(ClientId, Value)>,
    },
}
