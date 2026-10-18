//! Native multiplayer protocol layers of the original AA.exe (TimeGate TGC
//! networking): UDP transport sequencing, admin channels and message history,
//! file transfer and the admin node. See research/phase8-net/README.txt.
pub mod transport;
pub mod sync;
pub mod admin;
pub mod seams;
