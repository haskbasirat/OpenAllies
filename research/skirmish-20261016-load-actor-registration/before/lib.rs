//! Evidence-backed actor identity/lifecycle foundation. Native behavior is added
//! only with recorded original-code fixtures; no authored-data defaults inferred.
#![forbid(unsafe_code)]

pub mod ability_flags;
pub mod attachment;
pub mod components;
pub mod construction_actions;
pub mod construction_state;
pub mod defender_state;
pub mod denizen_state;
pub mod mobile_deployment_state;
pub mod recruit_state;
pub mod recruit_actions;
pub mod research_state;
pub mod structure_state;
pub mod supply_state;
pub mod creation;
pub mod ground_states;
pub mod element_position;
pub mod economy_values;
pub mod init_create;
pub mod lair_city_state;
pub mod leaf_init;
pub mod state;
pub mod state_effects;
pub mod loading;
pub mod registry;
pub mod owned;
pub mod organization_interpolation;
pub use registry::{ActorId, IdentityRegistry};

pub mod world_publication;

pub mod action_publication;
pub mod organization_state;
pub mod transport_state;
pub mod organization_callees;
pub mod actor_names;
pub mod world_creation;
pub mod builder_state;
pub mod post_load;
pub mod hq_state;
pub mod actor_creation;
pub mod world_load_tail;
pub mod component_virtuals;
pub mod mobile_deployment_actions;
pub mod research_actions;
pub mod organization_recruitment;
pub mod structure_actions;
pub mod denizen_actions;
pub mod actor_init_null;
pub mod object_init_null;
