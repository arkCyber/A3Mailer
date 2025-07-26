//! Leader module

/// Leader placeholder
pub struct Leader;

/// Leader election
pub struct LeaderElection;

/// Leadership state
#[derive(Debug, Clone)]
pub struct LeadershipState {
    pub is_leader: bool,
    pub leader_id: Option<String>,
    pub term: u64,
}
