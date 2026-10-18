//! Shared authoritative world orchestration. Required actor services are
//! explicit and a failed partially committed step cannot be silently retried.
#![forbid(unsafe_code)]

pub mod catalog;
pub mod clocked;
pub mod relations;
pub mod settings;
pub mod localization;
pub mod updatables;
pub mod saved_queues;
pub mod restoration;
pub mod economy_refresh;
pub mod economy_values_world;
pub mod definitions;
pub mod names;
pub mod trigger_world;
pub mod actor_world;
pub mod spatial_world;
pub mod spatial_policy;
pub mod movement_world;
pub mod scheduler_owners;
pub mod post_load_world;
pub mod kingdom_economy;
pub mod mind_world;
pub mod mind_actions;

pub mod saved_performer;
pub mod saved_commands;
pub mod simulation;
pub mod load_tail_world;
pub mod saved_organization_actions;
pub(crate) mod saved_recruit_actions;
pub mod saved_mind_actions;
pub mod world_start_world;
pub mod saved_action_settings;
