use serde::{Deserialize, Serialize};
use std::net::IpAddr;

pub const IP_API: &str = "https://ip-api.com";

/// Response from https://ip-api.com
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct IpApiResponse {
    query: String,
    status: String,
    country: String,
    country_code: String,
    region: String,
    region_name: String,
    city: String,
    zip: String,
    lat: f64,
    lon: f64,
    timezone: String,
    isp: String,
    org: String,
    #[serde(rename = "as")]
    autonomous_system: String,
}

pub fn locate_ip(ip: &IpAddr) -> Result<IpApiResponse, reqwest::Error> {
    reqwest::blocking::get(format!("{}/json/{}", IP_API, ip.to_string()))?.json::<IpApiResponse>()
}

pub fn locate_batch_ips(ips: &[IpAddr]) -> Result<Vec<IpApiResponse>, reqwest::Error> {
    let client = reqwest::blocking::Client::new();
    client
        .post(format!("{}/batch", IP_API))
        .json(ips)
        .send()?
        .json::<Vec<IpApiResponse>>()
}
