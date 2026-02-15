use std::env;

static GLOBAL_CAPACITY_DEFAULT: u128 = 1000;
static GLOBAL_REFILL_RATE_DEFAULT: u128 = 20;
static IP_CAPACITY_DEFAULT: u128 = 100;
static IP_REFILL_RATE_DEFAULT: u128 = 10;
static ROUTE_CAPACITY_DEFAULT: u128 = 50;
static ROUTE_REFILL_RATE_DEFAULT: u128 = 15;

pub struct GatewayConfig {
    pub global_capacity: u128,
    pub global_refill_rate: u128,
    pub ip_capacity: u128,
    pub ip_refill_rate: u128,
    pub route_capacity: u128,
    pub route_refill_rate: u128,
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
            global_capacity: Self::read_u128("GLOBAL_CAPACITY", GLOBAL_CAPACITY_DEFAULT)?,
            global_refill_rate: Self::read_u128("GLOBAL_REFILL_RATE", GLOBAL_REFILL_RATE_DEFAULT)?,
            ip_capacity: Self::read_u128("IP_CAPACITY", IP_CAPACITY_DEFAULT)?,
            ip_refill_rate: Self::read_u128("IP_REFILL_RATE", IP_REFILL_RATE_DEFAULT)?,
            route_capacity: Self::read_u128("ROUTE_CAPACITY", ROUTE_CAPACITY_DEFAULT)?,
            route_refill_rate: Self::read_u128("ROUTE_REFILL_RATE", ROUTE_REFILL_RATE_DEFAULT)?,
        })
    }
    fn read_u128(key: &'static str, default: u128) -> Result<u128, ConfigError> {
        match env::var(key) {
            Ok(val) => {
                let parsed: u128 = val.parse().map_err(|_| ConfigError::InvalidNumber(key))?;
                if parsed == 0 {
                    return Err(ConfigError::InvalidValue(key));
                }
                Ok(parsed)
            }
            Err(_) => Ok(default),
        }
    }
}
