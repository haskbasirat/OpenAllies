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
pub mod skirmish_start;
pub mod saved_action_settings;
pub(crate) mod saved_research_actions;
pub(crate) mod saved_owner_recruitment;
pub mod saved_regiment_mind;
pub(crate) mod saved_structure_actions;
pub(crate) mod saved_denizen_actions;
pub(crate) mod saved_economy_refresh;
pub(crate) mod saved_construction_actions;
pub mod map_replacement_queue;
pub(crate) mod saved_economy_values;
pub mod restore_lifecycle;
pub mod saved_kingdom_flows;
pub mod saved_organization_combat;
pub mod combat_value_source;
pub mod saved_actor_state;
pub mod saved_organization_leader;
