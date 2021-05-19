use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A JSON object returned by MaxMind's GeoIP2 City API.
pub struct GeoIp2CityResponse {
    city: GeoIp2City,
    continent: GeoIp2Continent,
    country: GeoIp2Country,
    location: GeoIp2Location,
    postal: GeoIp2Postal,
    registered_country: GeoIp2RegisteredCountry,
    subdivisions: GeoIp2Subdivisions,
    traits: GeoIp2Traits,
}

/// A JSON object containing details about the city associated with the IP address.
pub struct GeoIp2City {
    /// A unique identifier for the city as specified by GeoNames.
    geoname_id: u32,
    /// A map from locale codes, such as `en`, to the localized names for the feature.
    names: HashMap<String, String>
}

/// A JSON object containing information about the continent associated with the IP address.
pub struct GeoIp2Continent {
    /// A two-character code for the continent associated with the IP address. The possible codes
    /// are: AF, AN, AS, EU, NA, OC, SA
    // TODO: Consider making continent code an enum
    code: String,
    /// A unique identifier for the continent as specified by GeoNames.
    geoname_id: u32,
    /// A map from locale codes, such as `en`, to the localized names for the feature.
    names: HashMap<String, String>,
}

// I swear we need this. See below.
fn bool_false() -> bool {
    false
}

/// A JSON object containing details about the country where MaxMind believes the end user is located.
pub struct GeoIp2Country {
    /// A unique identifier for the continent as specified by GeoNames.
    geoname_id: u32,
    /// This is `true` if the country is a member state of the European Union. Otherwise, the key is not included in the country object.
    // #[serde(default = "bool_false")]
    is_in_european_union: bool,
    /// A two-character ISO 3166-1 country code for the country associated with the IP address.
    iso_code: String,
    /// A map from locale codes, such as `en`, to the localized names for the feature.
    names: HashMap<String, String>,
}
