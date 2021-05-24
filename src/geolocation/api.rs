pub const MAXMIND_CITY_URI: &str = "https://geoip.maxmind.com/geoip/v2.1/city";

pub struct MaxMindAPIKey {
    username: String,
    password: String,
}

impl MaxMindAPIKey {
    pub fn new(username: &str, password: &str) -> Self {
        Self {
            username: username.to_owned(),
            password: password.to_owned(),
        }
    }

    pub fn username(&self) -> &str {
        &self.username
    }

    pub fn password(&self) -> &str {
        &self.password
    }
}
