use anyhow::Context;
use geoip2_city::CityApiResponse;
use serde::{Deserialize, Serialize};
use std::net::IpAddr;
use time::{Date, OffsetDateTime};

/// Name of the caching database.
pub const GEO_DB_CACHE_TREE_NAME: &str = "geolocation_cache";

/// A caching database for geolocation information fetched from MaxMind.
pub struct GeoCache {
    db: sled::Tree,
}

impl GeoCache {
    /// Creates a new cache with the name stored in `GEO_DB_CACHE_NAME`.
    pub fn new(tree: sled::Tree) -> Self {
        Self { db: tree }
    }

    /// Adds an IP address and its corresponding information to the database
    pub fn add_ip_address(
        &self,
        ip_address: &IpAddr,
        info: &GeoInfo,
    ) -> anyhow::Result<Option<GeoInfo>> {
        self.db
            .insert(bincode::serialize(ip_address)?, bincode::serialize(info)?)
            .context("could not insert into database")?
            .map(|x| bincode::deserialize(&x))
            .transpose()
            .context("could not deserialize the inserted GeoInfo")
    }

    /// Fetches the cached information about an IP address
    pub fn fetch_ip_address(&self, ip_address: &IpAddr) -> anyhow::Result<Option<GeoInfo>> {
        self.db
            .get(bincode::serialize(ip_address)?)
            .context("could not fetch from database")?
            .map(|x| bincode::deserialize(&x))
            .transpose()
            .context("could not deserialize the fetched GeoInfo")
    }

    /// Fetches the cached information about an IP address, after checking if will be invalidated.
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
                    self.remove_ip_address(ip_address)
                        .context("could not remove stale IP address")?;
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

    /// Removes cached information about an IP address.
    pub fn remove_ip_address(&self, ip_address: &IpAddr) -> anyhow::Result<Option<GeoInfo>> {
        self.db
            .remove(bincode::serialize(ip_address)?)
            .context("could not remove IP address")?
            .map(|x| bincode::deserialize(&x))
            .transpose()
            .context("could not deserialize removed GeoInfo")
    }
}

/// The value (in key-value) for the caching database, consisting of the structured response
/// from the API alongside metadata such as when the data was fetched.
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
