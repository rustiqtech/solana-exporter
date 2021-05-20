use crate::geolocation::GEO_DB_CACHE_LOCATION;
use geoip2_city::CityApiResponse;
use serde::{Deserialize, Serialize};
use std::net::IpAddr;
use time::{Date, OffsetDateTime};

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
    pub fn add_ip_address(
        &self,
        ip_address: IpAddr,
        info: GeoInfo,
    ) -> sled::Result<Option<GeoInfo>> {
        self.db
            .insert(
                bincode::serialize(&ip_address).unwrap(),
                bincode::serialize(&info).unwrap(),
            )
            .map(|x| x.map(|y| bincode::deserialize(&y).unwrap()))
    }

    /// Fetch the cached information about an IP address
    pub fn fetch_ip_address(&self, ip_address: IpAddr) -> sled::Result<Option<GeoInfo>> {
        self.db
            .get(bincode::serialize(&ip_address).unwrap())
            .map(|x| x.map(|y| bincode::deserialize(&y).unwrap()))
    }
}

impl Default for GeoCache {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct GeoInfo {
    response: CityApiResponse,
    fetched_at: Date,
}

/// Converts a response from IP-API into something the database can store. We also store the date
/// the response was fetched so we can invalidate it at a later time.
impl From<CityApiResponse> for GeoInfo {
    fn from(value: CityApiResponse) -> Self {
        Self {
            response: value,
            fetched_at: OffsetDateTime::now_utc().date(),
        }
    }
}
