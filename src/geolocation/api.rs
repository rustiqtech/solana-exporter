use geoip2_city::CityApiResponse;
use std::net::IpAddr;

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

pub fn query_ip_address(
    ip: &IpAddr,
    api_key: &MaxMindAPIKey,
) -> Result<CityApiResponse, reqwest::Error> {
    todo!()
}
