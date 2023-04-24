//! Parameters state
//!
//! This module contains all the logic related to parameters management.

use super::session::SessionKind;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Parameters error.
#[derive(Error, Debug)]
pub enum ParametersError {
    #[error("invalid threshold {0}")]
    InvalidThreshold(u16),
    #[error("invalid number of parties {0}")]
    InvalidParties(u16),
}

/// Parameters for the secret sharing scheme.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Parameters {
    /// Number of parties `n`.
    n: u16,

    /// Threshold for the secret sharing scheme `t`.
    ///
    /// Threshold must be in range `0 < t < n`.
    /// `t + 1` shares are required for the secret sharing scheme.
    t: u16,
}

impl Parameters {
    /// Creates new parameters for the secret sharing scheme.
    ///
    /// # Errors
    ///
    /// * Returns an error if `t` is not in range `0 < t < n`.
    /// * Returns an error if `n` is less than 2.
    pub fn new(n: u16, t: u16) -> Result<Self> {
        let params = Self { n, t };
        params.validate()?;
        Ok(params)
    }

    /// Returns the number of parties `n`.
    pub fn n(&self) -> u16 {
        self.n
    }

    /// Returns the threshold `t`.
    pub fn t(&self) -> u16 {
        self.t
    }

    /// Checks if parameters are valid.
    pub fn validate(&self) -> Result<()> {
        if self.n < 2 {
            return Err(ParametersError::InvalidParties(self.n).into());
        }
        if self.t == 0 || self.t >= self.n {
            return Err(ParametersError::InvalidThreshold(self.t).into());
        }
        Ok(())
    }

    /// Returns boolean indicating if threshold has been reached.
    pub fn threshold_reached(&self, kind: SessionKind, parties: usize) -> bool {
        match kind {
            SessionKind::Keygen => parties == self.n as usize,
            SessionKind::Sign => parties > self.t as usize,
        }
    }
}
