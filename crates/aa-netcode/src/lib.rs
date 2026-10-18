//! Port of the original AA.exe multiplayer session layer: the
//! `aa_runtime::simulation::NetworkSeams` adapter, admin channels, message
//! history, file transfer, connect/join/launch and heartbeat, built on the
//! proven aa-orders transport, synchronizer and decree flow.
//!
//! Evidence and addresses: research/phase8-net/README.txt. Every module cites
//! the original functions it ports; oracle fixtures live in research/phase8-net.
#![forbid(unsafe_code)]

pub mod channel;
pub mod file;
pub mod heartbeat;
pub mod history;
pub mod lobby;
pub mod packets;
pub mod seams;
pub mod send;
pub mod session;
pub mod speed;
pub mod staging;
pub mod udp;
pub mod launch_assignment;
pub mod lobby_staging;
pub mod local_bootstrap;
pub mod local_host;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Orders(#[from] aa_orders::Error),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Runtime(#[from] aa_runtime::simulation::Error),
    #[error("native path is not ported: {0}")]
    Unsupported(&'static str),
    #[error("protocol violation: {0}")]
    Protocol(String),
}
pub type Result<T> = std::result::Result<T, Error>;

/// Which accepted executable's admin-layer rules apply. docs/BASELINE.txt:
/// the rewrite targets the user's v14 AA.exe; the 2004 original stays
/// selectable. In the admin layer v14 differs only at 0x74BEC9 (send-gate
/// backlog test redirected to the caves 0x9EFF40/0x9EFEC0), 0x74E370 (type-5
/// file budget target 0xC03 -> 0x2403) and the file terminal-length sites
/// 0x76146A/0x761270 (crate::file).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum NativeImage {
    Original,
    #[default]
    V14,
}

pub mod local_order_host;
pub mod command_sync;
pub mod local_skirmish_startup;
