use std::env;

static GLOBAL_CAPACITY_DEFAULT: u128 = 1000;
static GLOBAL_REFILL_RATE_DEFAULT: u128 = 1000;
static IP_CAPACITY_DEFAULT: u128 = 5;
static IP_REFILL_RATE_DEFAULT: u128 = 2;
static ROUTE_CAPACITY_DEFAULT: u128 = 1000;
static ROUTE_REFILL_RATE_DEFAULT: u128 = 1000;

#[derive(Clone)]
pub struct GatewayConfig {
    pub global_capacity: u128,
    pub global_refill_rate: u128,
    pub ip_capacity: u128,
    pub ip_refill_rate: u128,
    pub route_capacity: u128,
    pub route_refill_rate: u128,
    pub upstream_base_url: String,
}

#[derive(Debug)]
pub enum ConfigError {
    InvalidNumber(&'static str),
    InvalidValue(&'static str),
}

//
impl GatewayConfig {
    pub fn from_env() -> Result<Self, ConfigError> {
        Ok(Self {
            global_capacity: Self::read_u128("GLOBAL_CAPACITY", 1000),
            global_refill_rate: Self::read_u128("GLOBAL_REFILL_RATE", 20),

            ip_capacity: Self::read_u128("IP_CAPACITY", 100),
            ip_refill_rate: Self::read_u128("IP_REFILL_RATE", 10),

            route_capacity: Self::read_u128("ROUTE_CAPACITY", 50),
            route_refill_rate: Self::read_u128("ROUTE_REFILL_RATE", 15),

            upstream_base_url: Self::read_string("UPSTREAM_BASE_URL", "https://httpbin.org"),
        })
    }
    fn read_u128(key: &str, default: u128) -> u128 {
        env::var(key)
            .ok()
            .and_then(|v| v.parse::<u128>().ok())
            .unwrap_or(default)
    }

    fn read_string(key: &str, default: &str) -> String {
        env::var(key).unwrap_or_else(|_| default.to_string())
    }
}
