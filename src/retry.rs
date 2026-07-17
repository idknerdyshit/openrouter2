use std::time::Duration;
use std::time::SystemTime;

use crate::routes::HttpMethod;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RetryPolicy {
    pub max_retries: u32,
    pub initial_backoff: Duration,
    pub max_backoff: Duration,
    pub retry_non_idempotent: bool,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_retries: 0,
            initial_backoff: Duration::from_millis(100),
            max_backoff: Duration::from_secs(10),
            retry_non_idempotent: false,
        }
    }
}

impl RetryPolicy {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn max_retries(mut self, value: u32) -> Self {
        self.max_retries = value;
        self
    }

    pub fn initial_backoff(mut self, value: Duration) -> Self {
        self.initial_backoff = value;
        self
    }

    pub fn max_backoff(mut self, value: Duration) -> Self {
        self.max_backoff = value;
        self
    }

    pub fn retry_non_idempotent(mut self, value: bool) -> Self {
        self.retry_non_idempotent = value;
        self
    }

    pub(crate) fn allows_method(&self, method: HttpMethod) -> bool {
        matches!(method, HttpMethod::Get | HttpMethod::Delete) || self.retry_non_idempotent
    }

    pub(crate) fn should_retry_status(
        &self,
        method: HttpMethod,
        attempt: u32,
        status: u16,
    ) -> bool {
        attempt < self.max_retries
            && self.allows_method(method)
            && matches!(status, 408 | 425 | 429 | 500 | 502 | 503 | 504)
    }

    pub(crate) fn should_retry_transport(&self, method: HttpMethod, attempt: u32) -> bool {
        attempt < self.max_retries && self.allows_method(method)
    }

    pub(crate) fn backoff(&self, attempt: u32, retry_after: Option<Duration>) -> Duration {
        if let Some(value) = retry_after {
            return value.min(self.max_backoff);
        }
        let multiplier = 2u32.saturating_pow(attempt);
        self.initial_backoff
            .saturating_mul(multiplier)
            .min(self.max_backoff)
    }
}

pub(crate) fn retry_after(headers: &reqwest::header::HeaderMap) -> Option<Duration> {
    let value = headers
        .get(reqwest::header::RETRY_AFTER)
        .and_then(|value| value.to_str().ok())?;

    if let Ok(seconds) = value.trim().parse::<u64>() {
        return Some(Duration::from_secs(seconds));
    }

    let at = httpdate::parse_http_date(value.trim()).ok()?;
    Some(at.duration_since(SystemTime::now()).unwrap_or_default())
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use reqwest::header::{HeaderMap, HeaderValue};

    use super::{RetryPolicy, retry_after};
    use crate::routes::HttpMethod;

    #[test]
    fn retries_are_opt_in_and_idempotency_aware() {
        let policy = RetryPolicy::new().max_retries(2);
        assert!(policy.should_retry_status(HttpMethod::Get, 0, 429));
        assert!(!policy.should_retry_status(HttpMethod::Post, 0, 429));
        assert!(!policy.should_retry_status(HttpMethod::Get, 2, 429));
        assert!(
            policy
                .retry_non_idempotent(true)
                .should_retry_status(HttpMethod::Post, 0, 429)
        );
    }

    #[test]
    fn parses_and_bounds_retry_after_values() {
        let mut headers = HeaderMap::new();
        headers.insert("retry-after", HeaderValue::from_static("3"));
        assert_eq!(retry_after(&headers), Some(Duration::from_secs(3)));
        assert_eq!(
            RetryPolicy::new()
                .max_backoff(Duration::from_secs(1))
                .backoff(0, retry_after(&headers)),
            Duration::from_secs(1)
        );
    }

    #[test]
    fn parses_retry_after_http_dates() {
        let mut headers = HeaderMap::new();
        headers.insert(
            "retry-after",
            HeaderValue::from_static("Thu, 01 Jan 2099 00:00:00 GMT"),
        );
        assert!(retry_after(&headers).unwrap() > Duration::ZERO);

        headers.insert(
            "retry-after",
            HeaderValue::from_static("Thu, 01 Jan 1970 00:00:00 GMT"),
        );
        assert_eq!(retry_after(&headers), Some(Duration::ZERO));

        headers.insert(
            "retry-after",
            HeaderValue::from_static("not-a-retry-after-value"),
        );
        assert_eq!(retry_after(&headers), None);
    }
}
