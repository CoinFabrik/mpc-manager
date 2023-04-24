//! Session state
//!
//! This module contains all the logic related to session management.

use super::ClientId;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use strum::EnumString;
use thiserror::Error;
use uuid::Uuid;

/// Value associated to a session.
pub type SessionValue = Option<Value>;
/// Unique ID of a session.
pub type SessionId = Uuid;
/// Party number of a session.
pub type SessionPartyNumber = u16;

/// Error type for session operations.
#[derive(Debug, Error)]
pub enum SessionError {
    #[error("party number `{0}` is already occupied by another party")]
    PartyNumberAlreadyOccupied(SessionPartyNumber),
}

/// Session kinds available in this implementation.
#[derive(Debug, Clone, Copy, Deserialize, Serialize, EnumString)]
pub enum SessionKind {
    /// Key generation session.
    #[serde(rename = "keygen")]
    #[strum(serialize = "keygen")]
    Keygen,
    /// Signing session.
    #[serde(rename = "sign")]
    #[strum(serialize = "sign")]
    Sign,
}

/// Session is subgroup of clients intended to be used for a specific purpose.
#[derive(Debug, Deserialize, Serialize)]
pub struct Session {
    /// Unique ID of the session.
    pub id: SessionId,
    /// Session kind
    pub kind: SessionKind,
    /// Public value associated to this session.
    ///
    /// This value can be set at the moment of creation.
    /// It can be a message or transaction intended
    /// to be signed by the session.
    pub value: SessionValue,
    /// Map party number to client id, starting at 1.
    #[serde(skip)]
    pub party_signups: HashMap<SessionPartyNumber, ClientId>,
    /// Occupied party numbers, starting at 1.
    #[serde(skip)]
    pub occupied_party_numbers: Vec<SessionPartyNumber>,
    ///
    /// Party numbers of finished clients
    #[serde(skip)]
    pub finished: HashSet<u16>,
}

impl Session {
    /// Creates a new session with the given parameters.
    pub fn new(id: Uuid, kind: SessionKind, value: SessionValue) -> Self {
        Self {
            id,
            kind,
            value,
            party_signups: HashMap::new(),
            occupied_party_numbers: Vec::new(),
            finished: HashSet::new(),
        }
    }

    /// Registers a client in the session and returns its party number.
    #[cfg(feature = "server")]
    pub fn signup(&mut self, client_id: ClientId) -> SessionPartyNumber {
        if self.is_client_in_session(&client_id) {
            return self.get_party_number(&client_id).unwrap();
        }
        let party_number = self.get_next_party_number();
        self.add_party(client_id, party_number);
        party_number
    }

    /// Signs in a client in the session with a given party number.
    #[cfg(feature = "server")]
    pub fn login(
        &mut self,
        client_id: ClientId,
        party_number: SessionPartyNumber,
    ) -> anyhow::Result<()> {
        if self.is_client_in_session(&client_id) {
            return Ok(()); //TODO: think of a better way to handle this (should we return an error?)
        }
        if self.occupied_party_numbers.contains(&party_number) {
            return Err(SessionError::PartyNumberAlreadyOccupied(party_number).into());
        }
        self.add_party(client_id, party_number);
        Ok(())
    }

    /// Adds new party assuming `party_number` doesn't exist already.
    #[cfg(feature = "server")]
    fn add_party(&mut self, client_id: ClientId, party_number: SessionPartyNumber) {
        self.occupied_party_numbers.push(party_number);
        self.occupied_party_numbers.sort();
        self.party_signups.insert(party_number, client_id);
    }

    /// Gets the party number of a client.
    #[cfg(feature = "server")]
    pub fn get_party_number(&self, client_id: &ClientId) -> Option<SessionPartyNumber> {
        self.party_signups
            .iter()
            .find(|(_, id)| id == &client_id)
            .map(|(party, _)| *party)
    }

    /// Returns boolean indicating if the client is already in this session.
    #[cfg(feature = "server")]
    pub fn is_client_in_session(&self, client_id: &ClientId) -> bool {
        self.party_signups.values().any(|id| id == client_id)
    }

    /// Returns the client id of a given party number.
    #[cfg(feature = "server")]
    pub fn get_client_id(&self, party_number: SessionPartyNumber) -> Option<ClientId> {
        self.party_signups
            .iter()
            .find(|(&pn, _)| pn == party_number)
            .map(|(_, id)| *id)
    }

    /// Returns all the client ids associated with the session.
    #[cfg(feature = "server")]
    pub fn get_all_client_ids(&self) -> Vec<ClientId> {
        self.party_signups.values().copied().collect()
    }

    /// Returns the number of clients associated with this session.
    #[cfg(feature = "server")]
    pub fn get_number_of_clients(&self) -> usize {
        self.party_signups.len()
    }

    /// Gets the next missing party number, assuming `occupied_party_numbers`
    /// is a sorted array.
    ///
    /// # Examples
    ///
    /// - if `[1,2,3,4]` it will return 5
    /// - if `[1,4,5,6]` it will return 2
    #[cfg(feature = "server")]
    fn get_next_party_number(&self) -> SessionPartyNumber {
        for (i, party) in self.occupied_party_numbers.iter().enumerate() {
            if (i + 1) != *party as usize {
                return (i + 1) as SessionPartyNumber;
            }
        }

        match self.occupied_party_numbers.last() {
            Some(party) => party + 1,
            None => 1,
        }
    }
}

impl Clone for Session {
    /// Clones session parameters, disregarding sensitive information.
    ///
    /// Should be used only for logging purposes.
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            kind: self.kind,
            value: self.value.clone(),
            party_signups: HashMap::new(),
            occupied_party_numbers: Vec::new(),
            finished: HashSet::new(),
        }
    }
}
