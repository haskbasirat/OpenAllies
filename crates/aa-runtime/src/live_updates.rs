//! Live scheduled-Update bindings reached by GWorld 4F9970 on a started
//! skirmish World. Each submodule binds one native Updatable virtual to the
//! SAME retained owners; unported leaves stay explicit errors at their call.
/// One 404F60-family debug-log call (channel B38B70; 43D309 stores its
/// threshold 0x14, so level-10 calls pass) made by a scheduled Update, with its
/// exact arguments in call order. The channel's virtual +4 sink (formatting and
/// output file) is not ported; the calls are retained on the live World.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct NativeLog {
    pub level: u32,
    pub file: &'static str,
    pub line: u32,
    pub function: &'static str,
    pub format: &'static str,
    pub arguments: Vec<u32>,
}
#[path = "live_kingdom_tick.rs"]
pub(crate) mod kingdom;
#[path = "live_properties_tick.rs"]
pub(crate) mod properties;
#[path = "live_supply_tick.rs"]
pub(crate) mod supply;
#[path = "live_siege_refresh.rs"]
pub(crate) mod siege;
#[path = "live_construction_tick.rs"]
pub(crate) mod construction;
#[path = "live_city_tick.rs"]
pub(crate) mod city;
#[path = "live_detection_tick.rs"]
pub(crate) mod detection;
#[path = "live_element_tick.rs"]
pub(crate) mod element;
#[path = "live_element_mind.rs"]
pub(crate) mod element_mind;
#[path = "live_sun.rs"]
pub(crate) mod sun;
#[path = "live_trigger_program.rs"]
pub mod trigger_program;
#[path = "live_economy_tick.rs"]
pub(crate) mod economy;
#[path = "live_doodad_tick.rs"]
pub(crate) mod doodad;
