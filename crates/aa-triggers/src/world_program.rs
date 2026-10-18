//! Actual world action dispatch for546AF0. Control and technology actions call
//! the recovered implementations, and presentation actions update owned fields.
//! Selectors must use the runtime's canonical ID registry/definition bank.
use crate::{
    decode::{Element, Parameter},
    manager::{Elements, Error, Manager, Result},
    program::{CoreProgram, text},
    selectors::{self, Registry},
};
use aa_body::{credit::Object as BodyObject, property_admission};
use aa_navigation::{control, startup::Object};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Transition {
    pub enabled: u8,
    pub started_bits: u32,
    pub duration_bits: u32,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Presentation {
    /// View subobject+70/+74/+78 (World+BC/+C0/+C4). Renderer consumption is separate.
    pub letterbox: Transition,
    /// View subobject+7C/+80/+84 (World+C8/+CC/+D0).
    pub screenfade: Transition,
}
impl Presentation {
    /// Original4ED890/4ED8B0 copy fields; there is no duration clamp or earlyout.
    pub fn toggle(&mut self, screenfade: bool, enabled: bool, now: u32, duration: u32) {
        let target = if screenfade {
            &mut self.screenfade
        } else {
            &mut self.letterbox
        };
        target.enabled = u8::from(enabled);
        target.started_bits = now;
        target.duration_bits = duration;
    }
}

/// Control uses canonical ObjectKey values. Body's existing32-bit actor identity
/// is obtained through an explicit live-store mapping, never pointer truncation.
/// Component effects remain required; no default-success leaf is supplied.
/// Selector lookups (raw ID, 4F8E90/4F8F00 names) come from `selectors::Registry`.
pub trait World: Registry + control::World + property_admission::World {
    fn current_time_bits(&self) -> u32;
    fn presentation_mut(&mut self) -> Result<&mut Presentation>;
    fn body_actor_identity(&self, actor: Object) -> Result<BodyObject>;
    ///54E080/463FA0 SharedProperty lookup, including native invalid-ID diagnostic
    /// policy. Empty name is handled before this service. Definition IDs must
    /// belong to the same bank used by property_admission and mixed insertion.
    fn named_property(&mut self, name: &[u16]) -> Result<Option<BodyObject>>;
    /// Actual canonical Kingdom+238 byte write. A packed-word backing must
    ///preserve neighboring bytes; no second cached field may be created.
    fn set_default_victory(&mut self, kingdom: Object, value: u8) -> Result<()>;
}

pub struct WorldProgram<'a, W: World> {
    pub core: CoreProgram,
    pub world: &'a mut W,
}
fn control_result<T>(result: control::Result<T>) -> Result<T> {
    result.map_err(|e| Error::Callback(e.to_string()))
}
fn body_result<T>(result: aa_body::Result<T>) -> Result<T> {
    result.map_err(|e| Error::Callback(e.to_string()))
}
fn boolean(p: &Parameter) -> Result<bool> {
    selectors::boolean(p)
}
/// Opcodes implemented by `world_action` (phase7 world wrappers).
pub const SUPPORTED: &[u32] = &[43, 44, 54, 61, 62, 63, 64, 114];

/// Phase7 world actions 43/44/54/61/62/63/64/114 (after the core opcodes).
/// Selectors are the shared 54DAC0/54DC20 ports; actions record no context.
pub fn world_action<W: World>(
    core: &mut CoreProgram,
    world: &mut W,
    manager: &mut Manager,
    element: &Element,
) -> Result<bool> {
    let p = &element.parameters;
    match element.opcode {
        43 | 44 => {
            // Native evaluates/stores duration before the bool conversion.
            let duration = core.value(manager, &p[1])?;
            let enabled = boolean(&p[0])?;
            let now = world.current_time_bits();
            world
                .presentation_mut()?
                .toggle(element.opcode == 44, enabled, now, duration);
            Ok(true)
        }
        54 => {
            // Native resolves the property before resolving the actor.
            let name = text(&p[1])?;
            let property = if name.is_empty() {
                None
            } else {
                world.named_property(name)?
            };
            let kind = selectors::kind(element)?;
            let actor = selectors::actor(manager, kind, &p[0], world)?
                .map(|object| world.body_actor_identity(object))
                .transpose()?;
            body_result(property_admission::grant_technology(actor, property, world))
        }
        61 | 62 => {
            let kind = selectors::kind(element)?;
            let kingdom = selectors::kingdom(manager, kind, &p[0], world)?;
            control_result(if element.opcode == 61 {
                control::enable(kingdom, world)
            } else {
                control::disable_kingdom(kingdom, world)
            })
        }
        63 | 64 => {
            let kind = selectors::kind(element)?;
            let actor = selectors::actor(manager, kind, &p[0], world)?;
            control_result(if element.opcode == 63 {
                control::enable(actor, world)
            } else {
                control::disable_actor(actor, world)
            })
        }
        114 => {
            let enabled = boolean(&p[1])?;
            let kind = selectors::kind(element)?;
            let Some(kingdom) = selectors::kingdom(manager, kind, &p[0], world)? else {
                return Ok(false);
            };
            world.set_default_victory(kingdom, u8::from(enabled))?;
            Ok(true)
        }
        _ => Err(Error::State(format!(
            "opcode{} ({}) requires its actual world service",
            element.opcode, element.name
        ))),
    }
}
impl<W: World> WorldProgram<'_, W> {
    pub fn evaluate_action(
        &mut self,
        manager: &mut Manager,
        ordinal: usize,
        element: &Element,
    ) -> Result<bool> {
        self.core.time_bits = self.world.current_time_bits();
        if let Some(value) = self.core.evaluate_core(manager, ordinal, element)? {
            return Ok(value);
        }
        world_action(&mut self.core, self.world, manager, element)
    }
}
/// Complete 546AF0 dispatch for the runtime: manager-local core opcodes, the
/// phase8 `elements`/`actions` ports, then the phase7 world wrappers. Every
/// other opcode stops the manager with an explicit unsupported error.
pub struct Program<'a, W: World + crate::elements::Services> {
    pub core: CoreProgram,
    pub world: &'a mut W,
}
impl<W: World + crate::elements::Services> Elements for Program<'_, W> {
    fn evaluate(&mut self, manager: &mut Manager, ordinal: usize, element: &Element) -> Result<bool> {
        self.core.time_bits = World::current_time_bits(self.world);
        if let Some(value) = self.core.evaluate_core(manager, ordinal, element)? {
            return Ok(value);
        }
        if let Some(value) = crate::elements::evaluate(&mut self.core, manager, element, self.world)? {
            return Ok(value);
        }
        world_action(&mut self.core, self.world, manager, element)
    }
    fn world_time_bits(&self) -> u32 {
        World::current_time_bits(self.world)
    }
    fn action_failed(&mut self, manager: &mut Manager, ordinal: usize, element: &Element) -> Result<()> {
        self.core.action_failed(manager, ordinal, element)
    }
}
impl<W: World> Elements for WorldProgram<'_, W> {
    fn evaluate(
        &mut self,
        manager: &mut Manager,
        ordinal: usize,
        element: &Element,
    ) -> Result<bool> {
        self.evaluate_action(manager, ordinal, element)
    }
    fn world_time_bits(&self) -> u32 {
        self.world.current_time_bits()
    }
    fn action_failed(
        &mut self,
        manager: &mut Manager,
        ordinal: usize,
        element: &Element,
    ) -> Result<()> {
        self.core.action_failed(manager, ordinal, element)
    }
}
