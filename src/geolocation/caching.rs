use crate::EXPORTER_DATA_DIR;
use geoip2_city::CityApiResponse;
use serde::{Deserialize, Serialize};
use std::fs::create_dir_all;
use std::net::IpAddr;
use time::{Date, OffsetDateTime};

pub const GEO_DB_CACHE_NAME: &str = "geolocation_cache.db";

pub struct GeoCache {
    db: sled::Db,
}

impl GeoCache {
    pub fn new() -> Self {
        let exporter_dir = dirs::home_dir().unwrap().join(EXPORTER_DATA_DIR);
        create_dir_all(&exporter_dir).unwrap();
        Self {
            db: sled::open(exporter_dir.join(GEO_DB_CACHE_NAME)).unwrap(),
        }
    }

    /// Add an IP address and its corresponding information to the database
    pub fn add_ip_address(
        &self,
        ip_address: &IpAddr,
        info: &GeoInfo,
    ) -> anyhow::Result<Option<GeoInfo>> {
        Ok(self
            .db
            .insert(bincode::serialize(ip_address)?, bincode::serialize(info)?)?
            .map(|x| bincode::deserialize(&x))
            .transpose()?)
    }

    /// Fetch the cached information about an IP address
    pub fn fetch_ip_address(&self, ip_address: &IpAddr) -> anyhow::Result<Option<GeoInfo>> {
        Ok(self
            .db
            .get(bincode::serialize(ip_address)?)?
            .map(|x| bincode::deserialize(&x))
            .transpose()?)
    }

    /// Fetch the cached information about an IP address, after checking if will be invalidated.
    /// `f` is a function that will return `true` if, given a date, the cached data should be considered stale.
    pub fn fetch_ip_address_with_invalidation(
        &self,
        ip_address: &IpAddr,
        f: fn(Date) -> bool,
    ) -> anyhow::Result<Option<GeoInfo>> {
        match self.fetch_ip_address(ip_address)? {
            // Database has it cached...
            Some(g) => {
                if f(g.fetched_at) {
                    // ... but it is considered stale. Remove it.
                    self.remove_ip_address(ip_address)?;
                    Ok(None)
                } else {
                    // ... and it's fine to use!
                    Ok(Some(g))
                }
            }
            // Database doesn't have it.
            None => Ok(None),
        }
    }

    /// Remove cached information about an IP address.
    pub fn remove_ip_address(&self, ip_address: &IpAddr) -> anyhow::Result<Option<GeoInfo>> {
        Ok(self
            .db
            .remove(bincode::serialize(ip_address)?)?
            .map(|x| bincode::deserialize(&x))
            .transpose()?)
    }
}

impl Default for GeoCache {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct GeoInfo {
    pub response: CityApiResponse,
    pub fetched_at: Date,
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
