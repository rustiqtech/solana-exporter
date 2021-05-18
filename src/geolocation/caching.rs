use crate::geolocation::GEO_DB_CACHE_LOCATION;
use serde::{Deserialize, Serialize};
use sled::IVec;
use std::net::IpAddr;

pub struct GeoCache {
    db: sled::Db,
}

impl GeoCache {
    pub fn new() -> Self {
        Self {
            db: sled::open(GEO_DB_CACHE_LOCATION).unwrap(),
        }
    }

    /// Add an IP address and its corresponding information to the database
    pub fn add_ip_address(&self, ip_address: IpAddr, info: GeoInfo) -> sled::Result<Option<IVec>> {
        todo!()
    }

    /// Fetch the cached information about an IP address
    pub fn fetch_ip_address(&self, ip_address: IpAddr) {
        todo!()
    }
}

impl Default for GeoCache {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Serialize, Deserialize)]
pub struct GeoInfo {
    // TODO: What information do we need to cache? This will probably depend on which provider we go with.
}
