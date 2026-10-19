//! Live 546AF0 element evaluator for the World+13C trigger manager.
//!
//! Manager-local opcodes run through aa_triggers::program::CoreProgram exactly
//! as before. World opcodes are bound individually to their native case on the
//! SAME retained owners; every other world opcode keeps the explicit
//! "requires its actual world service" error.
//!
//! Bound world opcodes:
//! * 46 `global_set_sun_colors` (case 547B3E): 54D9E0 duration (binary32),
//!   54E990 GetDataByIDSWarn on the Sun DataManager (GameData+73C), then 541DA0
//!   -> 4EDB10 on World+4C with SunData+18/+24/+30 (diffuse, ambient,
//!   specular point3 fields at the Bank source precision). A null Sun returns
//!   false; a non-empty unresolved IDS (the warning path) is an explicit error.
use crate::scheduler_owners::{Owners, Target};
use aa_numeric::Precision;
use aa_triggers::{
    decode::{Element, Value},
    manager::{Elements, Error, Manager, Result},
    program::CoreProgram,
};
use aa_world::queues::Queues;

fn state(e: impl std::fmt::Display) -> Error {
    Error::State(e.to_string())
}

/// World+4C loan and its Sun source definitions. Only the live World frame
/// constructs it; other callers pass `None` (opcode 46 stays a required error).
pub struct SunServices<'a> {
    pub(crate) actors: &'a aa_rules::actors::ActorCatalog,
    pub(crate) sun: &'a mut crate::sun_state::State,
    pub(crate) source_precision: Precision,
}

pub(crate) struct Program<'a, 'b> {
    pub core: &'a mut CoreProgram,
    pub sun: Option<&'a mut SunServices<'b>>,
    pub queues: &'a mut Queues,
    pub owners: &'a Owners,
    /// World+E0.
    pub now: u32,
}

fn sun_colors(actors: &aa_rules::actors::ActorCatalog, id: &str, pc: Precision) -> std::result::Result<Option<[[u32; 3]; 3]>, String> {
    let registry = actors
        .native_data_registry("Sun")
        .ok_or("actual Sun DataManager registry absent")?;
    let Some(entry) = registry.find(id) else {
        if id.is_empty() {
            return Ok(None);
        }
        return Err(format!(
            "54E990 GetDataByIDSWarn warning path for unresolved Sun IDS '{id}' is not bound"
        ));
    };
    let node = actors.native_data_node(entry).map_err(|e| e.to_string())?;
    let mut colors = [[0; 3]; 3];
    for (i, field) in ["diffuse", "ambient", "specular"].iter().enumerate() {
        let Some(aa_rules::native::TypedValue::Point { bits, .. }) =
            node.field(field, pc).map_err(|e| e.to_string())?
        else {
            return Err(format!("SunData {field} point3 field absent"));
        };
        colors[i] = bits.try_into().map_err(|_| "SunData point3 width differs")?;
    }
    Ok(Some(colors))
}

impl Program<'_, '_> {
    fn set_sun_colors(&mut self, manager: &mut Manager, element: &Element) -> Result<bool> {
        let p = &element.parameters;
        if p.len() != 2 {
            return Err(state("opcode46 requires its two native parameters"));
        }
        let duration = self.core.value(manager, &p[1])?;
        let Value::Text(name) = &p[0].value else {
            return Err(state("opcode46 Sun IDS parameter requires text storage"));
        };
        let end = name.iter().position(|&c| c == 0).unwrap_or(name.len());
        let id = String::from_utf16(&name[..end]).map_err(state)?;
        let sun = self
            .sun
            .as_deref_mut()
            .ok_or_else(|| state("opcode46 (global_set_sun_colors) requires the World+4C Sun owner"))?;
        let Some(colors) = sun_colors(sun.actors, &id, sun.source_precision).map_err(state)? else {
            return Ok(false);
        };
        let owner = self.owners.get(Target::Sun).map_err(state)?;
        crate::live_updates::sun::set_colors(sun.sun, colors, duration, self.queues, owner, self.now)
            .map_err(state)?;
        Ok(true)
    }
}

impl Elements for Program<'_, '_> {
    fn evaluate(&mut self, manager: &mut Manager, ordinal: usize, element: &Element) -> Result<bool> {
        //evaluate_core performs 541900 validity and 5422F0 for every opcode.
        if let Some(value) = self.core.evaluate_core(manager, ordinal, element)? {
            return Ok(value);
        }
        if !aa_triggers::program::valid(element) {
            return Ok(false);
        }
        match element.opcode {
            46 => self.set_sun_colors(manager, element),
            _ => Err(Error::State(format!(
                "opcode{} ({}) requires its actual world service",
                element.opcode, element.name
            ))),
        }
    }
    fn world_time_bits(&self) -> u32 {
        self.core.time_bits
    }
    fn action_failed(&mut self, manager: &mut Manager, ordinal: usize, element: &Element) -> Result<()> {
        self.core.action_failed(manager, ordinal, element)
    }
}
