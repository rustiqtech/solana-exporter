pub const MAXMIND_CITY_URI: &str = "https://geoip.maxmind.com/geoip/v2.1/city";

/// An API key that can be used to access MaxMind services.
pub struct MaxMindAPIKey {
    username: String,
    password: String,
}

impl MaxMindAPIKey {
    /// Make a new key from a username and password.
    pub fn new(username: &str, password: &str) -> Self {
        Self {
            username: username.to_owned(),
            password: password.to_owned(),
        }
    }

    /// Get the username of the API key.
    pub fn username(&self) -> &str {
        &self.username
    }

    /// Get the password of the API key.
    pub fn password(&self) -> &str {
        &self.password
    }
}
