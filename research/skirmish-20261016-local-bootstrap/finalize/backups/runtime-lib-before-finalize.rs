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
pub mod saved_actor_actions;
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
pub mod saved_organization_position;
pub mod saved_actor_properties;
pub mod combat_value_source_context;
pub mod combat_source_completion;
pub mod saved_owner_economy;
pub mod saved_owner_actions;
pub mod camera_projection;
pub mod element_move_command;
pub mod saved_member_properties;
pub mod world_object_list;
pub mod live_registration;
pub mod live_object_init;

pub mod live_placement;
pub mod battle_terrain;
pub mod combat_source_receipt;
pub mod live_required_placement;
pub mod load_spatial;
pub mod load_component_init_world;

pub mod saved_hq;

pub mod load_post_create_world;

pub mod load_world_globals;
pub mod load_zone_services;
pub mod load_actor_init_world;
pub mod load_object_init_world;
pub mod restriction_groups;

pub mod definition_fx;
pub mod load_kingdom_init_world;
pub mod load_body_init_world;
pub mod load_zone_entry;

pub mod load_actor_registration;
pub mod load_world;
pub mod load_constant_component_init;
pub mod sun_state;
pub mod saved_misc;
pub mod load_mover_init_world;
pub mod load_sun_reference;
pub mod live_scheduler_dispatch;
pub mod live_clocked;
pub mod load_actor_state_reentry;
pub mod load_economy_refresh_world;

pub mod load_placement_children;
pub mod map_resource_mode;

pub mod load_actor_overlaps;
pub mod load_owner_add_actor;
pub mod load_mind_init_world;
pub mod load_special_object_init;
pub mod load_tail_source;
pub mod load_tail_binding;
pub mod live_first_tick;

pub mod load_world_settings;
pub mod load_detection_memory;
pub(crate) mod load_actor_reveal;
pub mod load_actor_action_refresh;

pub mod load_actor_terrain_refresh;
pub mod player_selection;
pub mod load_detection_init_world;
pub mod load_detection_activation;
pub mod load_zone_globals;
pub(crate) mod load_object_blocking;
pub mod load_organization_terrain_refresh;
pub mod player_order_world;
pub mod load_economy_init_world;
pub mod fx_manager;
pub mod server_clock;
pub mod load_sai_world;
pub mod load_host_settings;
pub mod fx_graph;
pub mod map_resource_load;
pub mod execution_context;
pub mod mission_fields;
pub mod world_revision;
pub mod load_mission_start_world;
pub(crate) mod mission_camera_world;
pub(crate) mod mission_world_start;
pub(crate) mod load_mission_variables;
pub mod load_view_variables;
pub mod load_world_runtime_fields;
pub mod load_denizen_value_refresh;
pub mod load_organization_value_refresh;
pub mod kingdom_mission_start;
pub mod application_mode;
pub mod application_strings;
pub mod resource_map_cache;
pub mod map_resource_streams;
pub mod live_sim;
pub mod load_combat_variables;
pub mod live_map_strings;
pub mod load_sai_actor;
pub mod load_sai_players;
pub mod load_sai_startup;
mod load_sai_lists;
pub mod film_session;
pub mod load_world_rules;
pub mod resource_creator;
pub mod actor_command_ui;
pub mod film_header;
pub mod load_performer_value_refresh;
pub mod player_order_decree;
pub mod film_resource;
pub mod load_creation_fx;
pub mod load_property_instances;
pub mod film_text;
pub mod load_sim_world;
pub mod film_variables;
pub mod film_paths;
pub mod film_header_capture;
pub mod load_store_fog;
pub mod film_creator;
pub mod load_performer_property_refresh;
pub mod film_header_entries;
pub mod creator_variables;
pub mod creator_labels;
pub mod game_settings_value;
pub mod film_parameters;
pub mod film_header_fields;
pub mod film_kingdom_parameters;
pub mod film_parameter_serialization;
pub mod film_current_header;
pub mod film_parameter_extras;
pub mod film_process_metadata;
pub mod player_glyph;
pub mod creator_human_assignment;
pub mod film_header_owner;
pub mod campaign_manager;

pub mod load_store_memory_refresh;

pub mod load_property_numeric;

pub mod load_actor_state_fx;
pub mod standalone_network;
pub mod merge_kingdoms_world;
pub mod load_kingdom_glyph;
pub mod game_spy_manager_gate;
pub mod world_objectives;

pub mod localization_escape;
pub mod definition_technology_properties;
pub mod fx_static_position;
pub mod world_creator_extents;
pub mod film_world_capture;
pub mod map_prepare_world;
pub mod film_resource_factory;
pub mod film_create;
pub(crate) mod configure_world;
pub mod world_frame_inputs;
pub mod map_replacement_drain;

pub mod load_kingdom_properties;
pub mod object_aspect_source;
pub mod target_header;

pub(crate) mod configure_world_relations;

pub mod live_world_frame;
pub mod resource_depot;
pub(crate) mod configure_world_nations;
