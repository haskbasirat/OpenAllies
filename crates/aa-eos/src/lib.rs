//! Online play through Epic Online Services: sign-in by device ID, lobbies with
//! the game's name, map and players as attributes, and a datagram link over P2P.
//!
//! The SDK is not thread safe and works through callbacks, so one thread owns it
//! ([`worker`]): it makes every SDK call and ticks the platform, and the game
//! talks to it through [`Service`], which never blocks and shares no SDK handle.
//! The SDK's library is found at run time ([`api`]), so a machine without it gets
//! an error it can show, and a build needs nothing of Epic's beyond the bindings.
//!
//! One platform per process and one sign-in per platform, as the SDK allows:
//! after `EOS_Shutdown` it cannot start again in that process.
pub mod addr;
mod api;
pub mod config;
pub mod frame;
mod service;
mod worker;

pub use api::{LoadError, LIBRARY_FILE};
pub use config::{Config, ConfigError};
pub use service::{Event, Failure, Link, LobbyInfo, LobbySpec, Service, StartError, UserId, CLOSED_BY_PEER};
