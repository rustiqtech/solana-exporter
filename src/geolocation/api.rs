use futures::future::AndThen;
use futures::TryFutureExt;
use geoip2_city::CityApiResponse;
use reqwest::{Client, Error, Response};
use std::future::Future;
use std::net::IpAddr;

pub const MAXMIND_CITY_URI: &str = "https://geoip.maxmind.com/geoip/v2.1/city";

pub struct MaxMindAPIKey(String, String);

impl MaxMindAPIKey {
    pub fn new(username: &str, password: &str) -> Self {
        Self {
            0: username.to_owned(),
            1: password.to_owned(),
        }
    }

    pub fn username(&self) -> &str {
        &self.0
    }

    pub fn password(&self) -> &str {
        &self.1
    }
}
