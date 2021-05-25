use geoip2_city::CityApiResponse;
use std::fmt;
use std::fmt::{Display, Formatter};

#[derive(Clone, Debug, Hash, Ord, PartialOrd, Eq, PartialEq)]
pub struct DatacenterIdentifier {
    autonomous_system_number: u32,
    country_code: String,
    city_name: Option<String>,
}

impl From<CityApiResponse> for DatacenterIdentifier {
    fn from(val: CityApiResponse) -> Self {
        Self {
            autonomous_system_number: val.traits.autonomous_system_number,
            country_code: val
                .country
                .map(|c| c.iso_code)
                .unwrap_or_else(|| "XX".to_string()),
            city_name: val.city.and_then(|c| c.names.get("en").cloned()),
        }
    }
}

impl Display for DatacenterIdentifier {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match &self.city_name {
            Some(cn) => {
                write!(
                    f,
                    "{}-{}-{}",
                    self.autonomous_system_number, self.country_code, cn
                )
            }
            None => {
                write!(f, "{}-{}", self.autonomous_system_number, self.country_code)
            }
        }
    }
}
