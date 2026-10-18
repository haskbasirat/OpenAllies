//! Port of the original AA.exe user-interface system: TGI interface/template
//! loading, view elements, widgets, layout units, focus/input and command
//! dispatch, reconstructed from native code. Rendering goes through a
//! renderer-neutral draw list.
#![forbid(unsafe_code)]

pub mod control_load;
pub mod controls;
pub mod credits;
pub mod data;
pub mod draw;
pub mod fonts;
pub mod fs;
pub mod geometry;
pub mod hotkeys;
pub mod hud;
pub mod layout;
pub mod message_box;
pub mod message_log;
pub mod minimap;
pub mod node;
pub mod options;
pub mod resources;
pub mod rdraw;
pub mod rtext;
pub mod screens;
pub mod sprite;
pub mod staging;
pub mod state;
pub mod text;
pub mod tgi;
pub mod ui;
pub mod application_random;
pub mod ui_controls;
pub mod uvars;
pub mod ve;
pub mod widget;
pub mod view_variables;
mod message_variables;
pub mod native_messages;
mod camera_source;
