# API Key Management and Rate Limiting

This module provides a complete API key management system for the Form-State application, including:

1. API Key generation and storage
2. Authentication middleware
3. Rate limiting with tiered limits based on subscription level

## Rate Limiting

The rate limiting system implements a sliding window approach with three time windows:

- Per-minute limits (short bursts)
- Per-hour limits (sustained usage)
- Per-day limits (overall quota)

### Tiered Rate Limits

Rate limits are based on the account's subscription tier:

| Subscription Tier | Requests/Minute | Requests/Hour | Requests/Day |
|------------------|----------------|--------------|-------------|
| Free             | 30             | 500          | 5,000       |
| Pro              | 60             | 1,000        | 10,000      |
| ProPlus          | 120            | 2,500        | 25,000      |
| Power            | 300            | 10,000       | 100,000     |
| PowerPlus        | 600            | 25,000       | 250,000     |

### Rate Limit Headers

Response headers include rate limit information:

- `X-RateLimit-Limit-Minute`: Maximum requests per minute
- `X-RateLimit-Remaining-Minute`: Remaining requests in current minute window
- `X-RateLimit-Limit-Hour`: Maximum requests per hour
- `X-RateLimit-Remaining-Hour`: Remaining requests in current hour window
- `X-RateLimit-Limit-Day`: Maximum requests per day
- `X-RateLimit-Remaining-Day`: Remaining requests in current day window

When rate limits are exceeded:
- The response will have status code `429 Too Many Requests`
- `X-RateLimit-Reset`: Seconds until the rate limit resets
- `Retry-After`: Suggested seconds to wait before retrying

## Implementation Details

The rate limiter uses an in-memory storage system with automatic cleanup to prevent memory growth. A background task runs periodically to remove expired entries.

The rate limiter is configured as a global singleton using `once_cell::Lazy` to ensure it's initialized only once and shared across all requests.

## Usage Recommendations

To avoid rate limiting issues:

1. Implement exponential backoff with jitter when receiving 429 responses
2. Batch operations when possible to reduce the number of API calls
3. Consider upgrading to a higher subscription tier for production workloads
4. Use the rate limit headers to adapt request rates dynamically 