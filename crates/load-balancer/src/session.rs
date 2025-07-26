//! Session management

use crate::backend::Backend;
use crate::config::SessionAffinityConfig;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Session affinity manager
#[derive(Debug, Clone)]
pub struct SessionAffinity {
    config: SessionAffinityConfig,
    sessions: Arc<RwLock<HashMap<String, String>>>, // session_id -> backend_id
}

impl SessionAffinity {
    /// Create new session affinity manager
    pub fn new(config: SessionAffinityConfig) -> Self {
        Self {
            config,
            sessions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Get backend for session
    pub async fn get_backend_for_session(&self, session_id: &str) -> Option<String> {
        if !self.config.enabled {
            return None;
        }

        let sessions = self.sessions.read().await;
        sessions.get(session_id).cloned()
    }

    /// Bind session to backend
    pub async fn bind_session(&self, session_id: String, backend_id: String) {
        if !self.config.enabled {
            return;
        }

        let mut sessions = self.sessions.write().await;
        sessions.insert(session_id, backend_id);
    }

    /// Remove session binding
    pub async fn remove_session(&self, session_id: &str) {
        let mut sessions = self.sessions.write().await;
        sessions.remove(session_id);
    }

    /// Clear all sessions
    pub async fn clear_sessions(&self) {
        let mut sessions = self.sessions.write().await;
        sessions.clear();
    }
}

/// Session manager
#[derive(Debug)]
pub struct SessionManager {
    affinity: Option<SessionAffinity>,
}

impl SessionManager {
    /// Create new session manager
    pub fn new(config: Option<SessionAffinityConfig>) -> Self {
        let affinity = config.map(SessionAffinity::new);
        Self { affinity }
    }

    /// Extract session ID from request
    pub fn extract_session_id(&self, _headers: &hyper::HeaderMap) -> Option<String> {
        // TODO: Implement session ID extraction from cookies or headers
        None
    }

    /// Get backend for session
    pub async fn get_backend_for_session(&self, session_id: &str) -> Option<String> {
        match &self.affinity {
            Some(affinity) => affinity.get_backend_for_session(session_id).await,
            None => None,
        }
    }

    /// Bind session to backend
    pub async fn bind_session(&self, session_id: String, backend: &Backend) {
        if let Some(affinity) = &self.affinity {
            affinity.bind_session(session_id, backend.id.clone()).await;
        }
    }
}
