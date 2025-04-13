use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use chrono::{DateTime, Utc};
use crate::billing::SubscriptionTier;

/// Represents rate limit configuration for different subscription tiers
#[derive(Debug, Clone)]
pub struct RateLimitConfig {
    /// Requests per minute allowed
    pub requests_per_minute: u32,
    /// Requests per hour allowed
    pub requests_per_hour: u32,
    /// Requests per day allowed
    pub requests_per_day: u32,
}

/// Default rate limits for different subscription tiers
impl RateLimitConfig {
    pub fn for_subscription_tier(tier: &SubscriptionTier) -> Self {
        match tier {
            SubscriptionTier::Free => Self {
                requests_per_minute: 30,    // 30 requests per minute
                requests_per_hour: 500,     // 500 requests per hour
                requests_per_day: 5_000,    // 5K requests per day
            },
            SubscriptionTier::Pro => Self {
                requests_per_minute: 60,     // 60 requests per minute
                requests_per_hour: 1_000,    // 1K requests per hour
                requests_per_day: 10_000,    // 10K requests per day
            },
            SubscriptionTier::ProPlus => Self {
                requests_per_minute: 120,    // 120 requests per minute
                requests_per_hour: 2_500,    // 2.5K requests per hour
                requests_per_day: 25_000,    // 25K requests per day
            },
            SubscriptionTier::Power => Self {
                requests_per_minute: 300,    // 300 requests per minute
                requests_per_hour: 10_000,   // 10K requests per hour
                requests_per_day: 100_000,   // 100K requests per day
            },
            SubscriptionTier::PowerPlus => Self {
                requests_per_minute: 600,    // 600 requests per minute
                requests_per_hour: 25_000,   // 25K requests per hour
                requests_per_day: 250_000,   // 250K requests per day
            },
        }
    }
}

/// A sliding window rate limiter entry for tracking usage in different time windows
#[derive(Debug, Clone)]
struct RateLimiterEntry {
    /// The API key ID this entry is for
    key_id: String,
    /// Request count in the minute window
    minute_requests: u32,
    /// Last update timestamp for minute window
    minute_window_start: Instant,
    /// Request count in the hour window
    hour_requests: u32,
    /// Last update timestamp for hour window
    hour_window_start: Instant,
    /// Request count in the day window
    day_requests: u32,
    /// Last update timestamp for day window
    day_window_start: Instant,
}

impl RateLimiterEntry {
    fn new(key_id: String) -> Self {
        let now = Instant::now();
        Self {
            key_id,
            minute_requests: 0,
            minute_window_start: now,
            hour_requests: 0,
            hour_window_start: now,
            day_requests: 0,
            day_window_start: now,
        }
    }

    /// Checks if a request should be allowed and updates counters
    fn check_and_update(&mut self, config: &RateLimitConfig) -> RateLimitCheckResult {
        let now = Instant::now();
        
        // Reset windows if they've expired
        if now.duration_since(self.minute_window_start) > Duration::from_secs(60) {
            self.minute_requests = 0;
            self.minute_window_start = now;
        }
        
        if now.duration_since(self.hour_window_start) > Duration::from_secs(3600) {
            self.hour_requests = 0;
            self.hour_window_start = now;
        }
        
        if now.duration_since(self.day_window_start) > Duration::from_secs(86400) {
            self.day_requests = 0;
            self.day_window_start = now;
        }
        
        // Check if any limits are exceeded
        if self.minute_requests >= config.requests_per_minute {
            return RateLimitCheckResult::ExceededPerMinute {
                current: self.minute_requests,
                limit: config.requests_per_minute,
                reset_after: 60 - now.duration_since(self.minute_window_start).as_secs(),
            };
        }
        
        if self.hour_requests >= config.requests_per_hour {
            return RateLimitCheckResult::ExceededPerHour {
                current: self.hour_requests,
                limit: config.requests_per_hour,
                reset_after: 3600 - now.duration_since(self.hour_window_start).as_secs(),
            };
        }
        
        if self.day_requests >= config.requests_per_day {
            return RateLimitCheckResult::ExceededPerDay {
                current: self.day_requests,
                limit: config.requests_per_day,
                reset_after: 86400 - now.duration_since(self.day_window_start).as_secs(),
            };
        }
        
        // Increment counters
        self.minute_requests += 1;
        self.hour_requests += 1;
        self.day_requests += 1;
        
        // Calculate remaining limits
        RateLimitCheckResult::Allowed {
            remaining_minute: config.requests_per_minute - self.minute_requests,
            remaining_hour: config.requests_per_hour - self.hour_requests,
            remaining_day: config.requests_per_day - self.day_requests,
        }
    }
}

/// Result of a rate limit check
#[derive(Debug, Clone)]
pub enum RateLimitCheckResult {
    /// Request is allowed, with remaining limits
    Allowed {
        remaining_minute: u32,
        remaining_hour: u32,
        remaining_day: u32,
    },
    /// Per-minute limit exceeded
    ExceededPerMinute {
        current: u32,
        limit: u32,
        reset_after: u64, // seconds
    },
    /// Per-hour limit exceeded
    ExceededPerHour {
        current: u32,
        limit: u32,
        reset_after: u64, // seconds
    },
    /// Per-day limit exceeded
    ExceededPerDay {
        current: u32,
        limit: u32,
        reset_after: u64, // seconds
    },
}

/// Gets formatted rate limit headers for a response based on the check result
pub fn get_rate_limit_headers(result: &RateLimitCheckResult) -> HashMap<String, String> {
    let mut headers = HashMap::new();
    
    match result {
        RateLimitCheckResult::Allowed { remaining_minute, remaining_hour, remaining_day } => {
            headers.insert("X-RateLimit-Limit-Minute".to_string(), remaining_minute.to_string());
            headers.insert("X-RateLimit-Remaining-Minute".to_string(), remaining_minute.to_string());
            headers.insert("X-RateLimit-Limit-Hour".to_string(), remaining_hour.to_string());
            headers.insert("X-RateLimit-Remaining-Hour".to_string(), remaining_hour.to_string());
            headers.insert("X-RateLimit-Limit-Day".to_string(), remaining_day.to_string());
            headers.insert("X-RateLimit-Remaining-Day".to_string(), remaining_day.to_string());
        },
        RateLimitCheckResult::ExceededPerMinute { current, limit, reset_after } => {
            headers.insert("X-RateLimit-Limit-Minute".to_string(), limit.to_string());
            headers.insert("X-RateLimit-Remaining-Minute".to_string(), "0".to_string());
            headers.insert("X-RateLimit-Reset".to_string(), reset_after.to_string());
            headers.insert("Retry-After".to_string(), reset_after.to_string());
        },
        RateLimitCheckResult::ExceededPerHour { current, limit, reset_after } => {
            headers.insert("X-RateLimit-Limit-Hour".to_string(), limit.to_string());
            headers.insert("X-RateLimit-Remaining-Hour".to_string(), "0".to_string());
            headers.insert("X-RateLimit-Reset".to_string(), reset_after.to_string());
            headers.insert("Retry-After".to_string(), reset_after.to_string());
        },
        RateLimitCheckResult::ExceededPerDay { current, limit, reset_after } => {
            headers.insert("X-RateLimit-Limit-Day".to_string(), limit.to_string());
            headers.insert("X-RateLimit-Remaining-Day".to_string(), "0".to_string());
            headers.insert("X-RateLimit-Reset".to_string(), reset_after.to_string());
            headers.insert("Retry-After".to_string(), reset_after.to_string());
        },
    }
    
    headers
}

/// Rate limiter implementation for API keys
#[derive(Debug, Clone)]
pub struct ApiKeyRateLimiter {
    /// In-memory store of rate limit entries by API key ID
    entries: Arc<Mutex<HashMap<String, RateLimiterEntry>>>,
}

impl ApiKeyRateLimiter {
    /// Create a new rate limiter
    pub fn new() -> Self {
        Self {
            entries: Arc::new(Mutex::new(HashMap::new())),
        }
    }
    
    /// Check if a request should be allowed for the given API key
    pub fn check_rate_limit(&self, key_id: &str, subscription_tier: &SubscriptionTier) -> RateLimitCheckResult {
        let config = RateLimitConfig::for_subscription_tier(subscription_tier);
        let mut entries = self.entries.lock().unwrap();
        
        // Get or create entry for this API key
        let entry = entries.entry(key_id.to_string())
            .or_insert_with(|| RateLimiterEntry::new(key_id.to_string()));
            
        // Check and update the entry
        entry.check_and_update(&config)
    }
    
    /// Clean up expired entries (call periodically to avoid memory growth)
    pub fn cleanup_expired(&self) {
        let now = Instant::now();
        let day = Duration::from_secs(86400);
        
        let mut entries = self.entries.lock().unwrap();
        entries.retain(|_, entry| {
            // Keep entry if used in the last day
            now.duration_since(entry.day_window_start) < day
        });
    }
}

impl Default for ApiKeyRateLimiter {
    fn default() -> Self {
        Self::new()
    }
} 