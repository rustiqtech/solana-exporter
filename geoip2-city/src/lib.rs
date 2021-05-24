use std::collections::HashMap;
use std::net::IpAddr;

#[derive(Clone, Debug)]
#[cfg_attr(
    feature = "serde_support",
    derive(serde::Serialize, serde::Deserialize)
)]
/// A JSON object returned by MaxMind's GeoIP2 City API.
pub struct CityApiResponse {
    pub city: Option<City>,
    pub continent: Option<Continent>,
    pub country: Option<Country>,
    pub location: Option<Location>,
    pub postal: Option<Postal>,
    pub registered_country: Country,
    pub represented_country: Option<RepresentedCountry>,
    pub subdivisions: Option<Vec<Subdivisions>>,
    pub traits: Traits,
}

#[derive(Clone, Debug)]
#[cfg_attr(
    feature = "serde_support",
    derive(serde::Serialize, serde::Deserialize)
)]
/// A JSON object containing details about the city associated with the IP address.
pub struct City {
    /// A unique identifier for the city as specified by GeoNames.
    pub geoname_id: u32,
    /// A map from locale codes, such as `en`, to the localized names for the feature.
    pub names: HashMap<String, String>,
}

#[derive(Clone, Debug)]
#[cfg_attr(
    feature = "serde_support",
    derive(serde::Serialize, serde::Deserialize)
)]
/// A JSON object containing information about the continent associated with the IP address.
pub struct Continent {
    /// A two-character code for the continent associated with the IP address. The possible codes
    /// are: AF, AN, AS, EU, NA, OC, SA
    // TODO: Consider making continent code an enum
    pub code: String,
    /// A unique identifier for the continent as specified by GeoNames.
    pub geoname_id: u32,
    /// A map from locale codes, such as `en`, to the localized names for the feature.
    pub names: HashMap<String, String>,
}

// I swear we need this. See below.
#[allow(dead_code)]
fn bool_false() -> bool {
    false
}

#[derive(Clone, Debug)]
#[cfg_attr(
    feature = "serde_support",
    derive(serde::Serialize, serde::Deserialize)
)]
/// A JSON object containing details about a country.
pub struct Country {
    /// A unique identifier for the continent as specified by GeoNames.
    pub geoname_id: u32,
    /// This is `true` if the country is a member state of the European Union. Otherwise, the key is not included in the country object.
    #[cfg_attr(feature = "serde_support", serde(default = "bool_false"))]
    pub is_in_european_union: bool,
    /// A two-character ISO 3166-1 country code for the country associated with the IP address.
    pub iso_code: String,
    /// A map from locale codes, such as `en`, to the localized names for the feature.
    pub names: HashMap<String, String>,
}

#[derive(Clone, Debug)]
#[cfg_attr(
feature = "serde_support",
derive(serde::Serialize, serde::Deserialize)
)]
/// A JSON object containing details about a country.
pub struct RepresentedCountry {
    /// A unique identifier for the continent as specified by GeoNames.
    pub geoname_id: u32,
    /// This is `true` if the country is a member state of the European Union. Otherwise, the key is not included in the country object.
    #[cfg_attr(feature = "serde_support", serde(default = "bool_false"))]
    pub is_in_european_union: bool,
    /// A two-character ISO 3166-1 country code for the country associated with the IP address.
    pub iso_code: String,
    /// A map from locale codes, such as `en`, to the localized names for the feature.
    pub names: HashMap<String, String>,
    #[cfg_attr(feature = "serde_support", serde(rename = "type"))]
    pub repr_type: String,
}

#[derive(Clone, Debug)]
#[cfg_attr(
    feature = "serde_support",
    derive(serde::Serialize, serde::Deserialize)
)]
/// A JSON object containing specific details about the location associated with the IP address.
pub struct Location {
    /// The approximate accuracy radius, in kilometers, around the latitude and longitude for the
    /// geographical entity (country, subdivision, city or postal code) associated with the IP address.
    pub accuracy_radius: u32,
    /// The approximate WGS84 latitude of the postal code, city, subdivision or country associated with the IP address.
    pub latitude: f32,
    /// The approximate WGS84 longitude of the postal code, city, subdivision or country associated with the IP address.
    pub longitude: f32,
    /// The metro code associated with the IP address. These are only available for IP addresses in the US.
    pub metro_code: Option<u32>,
    /// The time zone associated with location, as specified by the IANA Time Zone Database, e.g., "America/New_York".
    pub time_zone: String,
}

#[derive(Clone, Debug)]
#[cfg_attr(
    feature = "serde_support",
    derive(serde::Serialize, serde::Deserialize)
)]
/// A JSON object containing details about the postal code associated with the IP address.
pub struct Postal {
    /// A postal code close to the user’s location.
    pub code: String,
}

#[derive(Clone, Debug)]
#[cfg_attr(
    feature = "serde_support",
    derive(serde::Serialize, serde::Deserialize)
)]
/// A JSON object containing information about location (e.g. county, country within union) associated with the IP address.
pub struct Subdivisions {
    /// A unique identifier for the region as specified by GeoNames.
    pub geoname_id: u32,
    /// A string of up to three characters containing the region-portion of the ISO 3166-2 code for the region associated with the IP address.
    pub iso_code: String,
    /// A map from locale codes, such as en, to the localized names for the feature.
    pub names: HashMap<String, String>,
}

#[derive(Clone, Debug)]
#[cfg_attr(
    feature = "serde_support",
    derive(serde::Serialize, serde::Deserialize)
)]
/// A JSON object containing general traits associated with the IP address.
pub struct Traits {
    /// The autonomous system number associated with the IP address.
    pub autonomous_system_number: u32,
    /// The organization associated with the registered autonomous system number for the IP address.
    pub autonomous_system_organization: String,
    /// The second level domain associated with the IP address.This will be something like "example.com" or "example.co.uk", not "foo.example.com".
    pub domain: Option<String>,
    /// The requested IP address.
    pub ip_address: IpAddr,
    /// The name of the ISP associated with the IP address.
    pub isp: String,
    /// The network in CIDR notation associated with the record.
    /// In particular, this is the largest network where all of the fields besides `ip_address` have the same value.
    pub network: String,
    /// The name of the organization associated with the IP address.
    pub organization: String,
}
