use std::fmt;

use zeroize::Zeroizing;

#[derive(Default)]
pub struct ApiKey {
    secret: Zeroizing<String>,
}

impl ApiKey {
    pub fn new(secret: impl Into<String>) -> Self {
        Self {
            secret: Zeroizing::new(secret.into()),
        }
    }

    pub(crate) fn expose_secret(&self) -> &str {
        &self.secret
    }
}

impl Clone for ApiKey {
    fn clone(&self) -> Self {
        Self::new(self.expose_secret().to_owned())
    }
}

impl PartialEq for ApiKey {
    fn eq(&self, other: &Self) -> bool {
        self.expose_secret() == other.expose_secret()
    }
}

impl Eq for ApiKey {}

impl fmt::Debug for ApiKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("ApiKey([REDACTED])")
    }
}

impl From<String> for ApiKey {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

impl From<&str> for ApiKey {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AuthRequirement {
    Required,
    Optional,
    Default,
}

#[cfg(test)]
mod tests {
    use super::ApiKey;

    #[test]
    fn api_key_debug_is_redacted() {
        let key = ApiKey::new("sk-test-secret");
        let debug = format!("{key:?}");
        assert!(debug.contains("[REDACTED]"));
        assert!(!debug.contains("sk-test-secret"));
    }
}
