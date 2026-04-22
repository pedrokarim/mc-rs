//! Auth : password hash (Argon2id) + JWT HS256 + middleware Axum.

pub mod jwt;
pub mod middleware;
pub mod password;
pub mod ratelimit;

pub use jwt::{Claims, JwtCodec};
pub use middleware::CurrentUser;
pub use password::{hash_password, verify_password};
pub use ratelimit::RateLimiter;
