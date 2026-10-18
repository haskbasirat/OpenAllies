//! Ports of the original strategic AI (`Source/Sai/*.cpp`, AA.exe 79D300..7D0A70).
//!
//! Architecture facts from the original code (see research/phase8-ai):
//! - `KKC_SAI::Think` (`7A0080`) runs the goal engine, RAI and MGAI in Win32
//!   fibers with wall-clock budgets from `QueryPerformanceCounter` (`59ABA0`).
//! - The SAI RNG (`[B36528]+2C`) is seeded from that counter in the SAI
//!   constructor `7A0AA0`, not from the synchronized world seed.
//! - Decisions leave the SAI only as orders (`KKC_SAICommandData::Send`,
//!   `7AF220`), so SAI evaluation is host-local input to the simulation.
//!
//! These modules therefore model each native routine exactly, with explicit
//! world services; scheduling/threads are a caller concern.
pub mod actor_notify;
pub mod assign;
pub mod bridge;
pub mod crt_qsort;
pub mod data;
pub mod ego;
pub mod ego_lists;
pub mod engine;
pub mod goal_base;
pub mod goal_priority;
pub mod map_analysis;
pub mod native_hash;
pub mod profile;
pub mod rules_data;
pub mod tables;
pub mod travel;
pub mod base_priority;
pub mod base_priority_misc;
pub mod reinforce;
pub mod command;
pub mod manager;
pub mod map_build;
pub mod data_refs;
pub mod base_priority_more;
