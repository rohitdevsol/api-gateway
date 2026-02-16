use std::sync::atomic::AtomicU64;

// Atomic type -> No locking, thread safe and high performance
pub struct GatewayMetrices {
    pub total_requests: AtomicU64,
    pub total_rate_limited: AtomicU64,
    pub global_rate_limited: AtomicU64,
    pub route_rate_limited: AtomicU64,
    pub ip_rate_limited: AtomicU64,
    pub total_allowed: AtomicU64,
}

impl GatewayMetrices {
    pub fn new() -> Self {
        Self {
            total_requests: AtomicU64::new(0),
            total_rate_limited: AtomicU64::new(0),
            global_rate_limited: AtomicU64::new(0),
            route_rate_limited: AtomicU64::new(0),
            ip_rate_limited: AtomicU64::new(0),
            total_allowed: AtomicU64::new(0),
        }
    }
}
