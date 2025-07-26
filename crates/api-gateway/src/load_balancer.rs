//! Load balancing module

/// Load balancer
pub struct LoadBalancer;

/// Load balancing algorithm
pub enum LoadBalancingAlgorithm {
    RoundRobin,
    LeastConnections,
    WeightedRoundRobin,
}

/// Backend server
pub struct Backend;
