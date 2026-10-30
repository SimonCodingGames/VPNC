//! Domain services for vpnc-control.

pub mod auth;
pub mod crypto;
pub mod ipam;
pub mod users;
pub mod wireguard;

pub use auth::AuthService;
pub use crypto::CryptoService;
pub use users::{bootstrap_admin, UserRecord, UserRole};
