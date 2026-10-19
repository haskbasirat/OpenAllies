//! FX command execution in the VWorld frame (package 4).
//!
//! * 43EC60's FX segment ([`run_fx_commands_43ec60`]): `n = E8` read once;
//!   for each of the n commands pop the head, run 4289A0 over its graph roots
//!   (`cmd+4`); false re-appends it (+68 = 0), true runs 429050 (FXL view
//!   +19C decrement through the attached view), 45C7A0 on cmd+5C, then frees.
//! * 4289A0 ([`run_nodes_4289a0`]): returns 0 while VWorld+AD is clear;
//!   `prev = cmd+3C - cmd+38`, cmd+3C = VWorld+8C, `elapsed = now - cmd+38`
//!   (binary32 stores); a node whose delay (+28) exceeds `elapsed` keeps the
//!   command alive; a node with `prev < delay <= elapsed` runs vf1C then vf18.
//! * vf1C 42A770 ([`filter_42a770`]) for every FXC kind, with its vf20 and
//!   vf30 per kind (CreateVFX 428360/42AF30, Equipment 6A71D0, PlaySFX
//!   42AE10, every other kind 6A71F0).
//! * vf18 per kind: PlaySFX 42A920 (fully here, to the B36548 audio manager
//!   through [`Hooks::play_sfx_7474f0`]), StopSFX 438AC0 (returns 1, nothing
//!   else), StopVFX 428C80, CreateVFX 42A040 (native order here, view-side
//!   callees through [`VfxHooks`]). Equipment 428D10, ChangeAspect 42A330,
//!   ChangeSkin 42A3A0, CreateRFX 428E60, PlayObjectAnimation 42A410 and
//!   PlayHitEffect 42AAE0 are explicit unported errors.
//!
//! The command words are the 0x6C-byte 428480 event: +C kingdom, +10/+20
//! source/target thing, +14/+24 positions, +34 counter, +38 start, +3C last,
//! +40/+44 health, +54 actor state, byte +58 (existing/42A590 form).
use crate::fx_graph::{Command, FxcKind, Graph, Node, NodeId, Reference};
use aa_actors::ActorId;
use aa_numeric::Precision;
use std::sync::Arc;

pub type Result<T> = std::result::Result<T, String>;
#[path = "view_fx_execute_core.rs"]
mod core;
pub use core::{filter_values_42a770, node_step, sfx_args_42a920, times_4289a0, FilterInput, FilterReads, SfxArgs, Step, ThingState};
use core::{f, fx_loop_43ec60, st, x};

/// VWorld +AD (initialized) and +8C (view time, binary32 bits).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ViewTime {
    pub ad: bool,
    pub now_8c: u32,
}

/// The PlaySFX request 42A920 hands to 7474F0 on [B36548].
#[derive(Clone, Debug)]
pub struct SfxRequest {
    /// node+44 Audio reference.
    pub audio: Reference,
    /// node+40 when attached, else 0.
    pub loop_38: bool,
    /// The attached thing (AT 0 source / 1 target) when vtB4 holds.
    pub attached: Option<ActorId>,
    /// cmd+C thing, the owner object 7474F0 stores at +24.
    pub owner: Option<ActorId>,
    /// The position when its x or y is positive, else null (global).
    pub position: Option<[u32; 3]>,
    /// Graph and node for the +54/+58 completion link (428AF0 over node+10).
    pub graph: Arc<Graph>,
    pub node: NodeId,
    /// cmd words at the call (the 745CE0 0x6C-byte copy).
    pub context: Vec<(u32, u32)>,
}

/// World and view reads plus the side effects the FX nodes reach.
pub trait Hooks {
    /// B364C0 thing exists and vtB4 holds.
    fn thing_alive_b4(&mut self, thing: ActorId) -> Result<bool>;
    /// The thing's world position (+20..+28) when it exists.
    fn thing_position(&mut self, thing: ActorId) -> Result<Option<[u32; 3]>>;
    /// 4CF410 actor state flags.
    fn thing_flags_4cf410(&mut self, thing: ActorId) -> Result<u32>;
    /// 4282A0 on [B36534]+30 then 42A530/54ECE0+34: the terrain group id at
    /// xy, None when 4282A0 rejects the point.
    fn terrain_group_at(&mut self, xy: [u32; 2]) -> Result<Option<String>>;
    /// 42A770 actor state 0x14: source thing vt100()+98 owner is the thing.
    fn state_14_source_owns(&mut self, source: ActorId) -> Result<bool>;
    /// [B36534]+3C/+40 (map size) for AT 6.
    fn map_size(&mut self) -> Result<[u32; 2]>;
    /// 7474F0 on [B36548].
    fn play_sfx_7474f0(&mut self, request: SfxRequest) -> Result<()>;
    /// 428C80: the def+40 thing's view (+C) vt9C, then 454A70(def).
    fn stop_vfx_428c80(&mut self, node: &Node, command: &Command) -> Result<bool>;
    /// 42A040 view-side callees.
    fn vfx(&mut self) -> &mut dyn VfxHooks;
    /// 429050: cmd+60 FXL view vtBC, then 45A950 (view+19C - 1).
    fn completion_429050(&mut self, command: &Command) -> Result<()>;
    /// 45C7A0 on cmd+5C (FXL member destructor, owner vf28(1)).
    fn fxl_member_45c7a0(&mut self, command: &Command) -> Result<()>;
}

/// The VFX side of 42A040. Every method is one native callee, called in
/// native order by [`create_vfx_42a040`].
pub trait VfxHooks {
    /// 4445C0(def): definition/detail gate; false makes 42A040 return 1.
    fn gate_4445c0(&mut self, definition: &Reference) -> Result<bool>;
    /// 428360 vf20 reads def+24.
    fn definition_24(&mut self, definition: &Reference) -> Result<u32>;
    /// 429950 ConsolidateEventParameters: Ok(None) is its false return
    /// (404F90 line 0x554 follows); Some(thing view, position).
    fn consolidate_429950(&mut self, node: &Node, command: &Command) -> Result<Option<(Option<ActorId>, [u32; 3])>>;
    /// The thing view vt9C (attachable model) and vtA8 calls of 42A111.
    fn view_9c(&mut self, view: ActorId) -> Result<bool>;
    fn view_a8(&mut self, view: ActorId) -> Result<()>;
    /// 43CD00 attached create; returns the VFX view or None.
    fn create_attached_43cd00(&mut self, definition: &Reference, view: ActorId, location: u32, translation: [u32; 3], duration: u32) -> Result<Option<u64>>;
    /// 43CAE0 detached/ground create.
    fn create_43cae0(&mut self, definition: &Reference, position: [u32; 3], duration: u32) -> Result<Option<u64>>;
    /// 429880 SetVFXOrientation(vfx, thing view).
    fn orient_429880(&mut self, vfx: u64, node: &Node, view: ActorId) -> Result<bool>;
    /// vfx+78 = cmd+8, 444520(vfx, node, cmd), vfx+50 = cmd+38 + node+28.
    fn bind_444520(&mut self, vfx: u64, node: &Node, command: &Command, created_50: u32) -> Result<()>;
    /// def vt74, then the target orientation (432E40, 44CFD0 or 44D470).
    fn orient_target(&mut self, vfx: u64, definition: &Reference, command: &Command) -> Result<()>;
    /// 4449D0, then vfx vt0 when vfx+18 == 2.
    fn finish_4449d0(&mut self, vfx: u64) -> Result<()>;
}

fn word(command: &Command, offset: u32) -> Result<u32> {
    command.word(offset).ok_or_else(|| format!("FX command +{offset:X} is unwritten"))
}
fn node_word(node: &Node, offset: u32) -> Result<u32> {
    node.word(offset).ok_or_else(|| format!("FXC node +{offset:X} is unwritten"))
}

/// 43EC60's FX segment, run with VWorld+8C after the +208 drain.
pub fn run_fx_commands_43ec60(
    state: &mut crate::view_fx_commands::State,
    view: ViewTime,
    pc: Precision,
    hooks: &mut impl Hooks,
) -> Result<()> {
    fx_loop_43ec60(
        state.list_mut(),
        |command, hooks: &mut _| {
            let graph = Arc::clone(command.graph());
            let roots = command.roots().to_vec();
            run_nodes_4289a0(&graph, &roots, command, view, pc, hooks)
        },
        |command, hooks: &mut _| {
            hooks.completion_429050(command)?;
            hooks.fxl_member_45c7a0(command)
        },
        hooks,
    )
}
/// 4289A0 over one node list.
pub fn run_nodes_4289a0(
    graph: &Arc<Graph>,
    list: &[NodeId],
    command: &mut Command,
    view: ViewTime,
    pc: Precision,
    hooks: &mut impl Hooks,
) -> Result<bool> {
    if !view.ad {
        return Ok(false);
    }
    let (prev, elapsed) = times_4289a0(word(command, 0x38)?, word(command, 0x3c)?, view.now_8c, pc)?;
    command.set_word(0x3c, view.now_8c);
    let mut result = true;
    for &id in list {
        let node = graph.nodes().get(id.index()).ok_or("FX node outside its arena")?;
        match node_step(node_word(node, 0x28)?, prev, elapsed)? {
            Step::NotReached => result = false,
            Step::Passed => {}
            Step::Fire => {
                let kind = graph.node_kind(id).ok_or("FX node kind missing")?;
                if filter_42a770(graph, node, kind, command, hooks)? {
                    execute_vf18(graph, id, kind, command, view, pc, hooks)?;
                }
            }
        }
    }
    Ok(result)
}
/// vf20 per kind: CreateVFX 428360 (def+24 == 1 and node+64 == 0),
/// Equipment 6A71D0 (true), every other kind 6A71F0 (false).
fn vf20(node: &Node, kind: FxcKind, hooks: &mut impl Hooks) -> Result<bool> {
    match kind {
        FxcKind::CreateVfx => {
            let definition = node.reference(0x3c).and_then(Option::as_ref).ok_or("428360 CreateVFX node+3C definition is null")?;
            Ok(hooks.vfx().definition_24(definition)? == 1 && node.byte(0x64) == Some(0))
        }
        FxcKind::Equipment => Ok(true),
        _ => Ok(false),
    }
}
/// vf30: CreateVFX 42AF30 (node+40 AT in {1,3,5}), PlaySFX 42AE10 (node+3C
/// AT in {1,3,5}), others false.
fn vf30(node: &Node, kind: FxcKind) -> Result<bool> {
    let at = match kind {
        FxcKind::CreateVfx => node_word(node, 0x40)?,
        FxcKind::PlaySfx => node_word(node, 0x3c)?,
        _ => return Ok(false),
    };
    Ok(matches!(at, 1 | 3 | 5))
}

struct LiveFilter<'a, H: Hooks> {
    node: &'a Node,
    kind: FxcKind,
    command: &'a Command,
    hooks: &'a mut H,
}
impl<H: Hooks> FilterReads for LiveFilter<'_, H> {
    fn vf20(&mut self) -> Result<bool> {
        vf20(self.node, self.kind, self.hooks)
    }
    fn target_alive(&mut self) -> Result<bool> {
        match self.command.target() {
            Some(t) => self.hooks.thing_alive_b4(t),
            None => Ok(false),
        }
    }
    fn target_flags_4cf410(&mut self) -> Result<u32> {
        self.hooks.thing_flags_4cf410(self.command.target().ok_or("4CF410 target thing")?)
    }
    fn terrain_group_at(&mut self, xy: [u32; 2]) -> Result<Option<String>> {
        self.hooks.terrain_group_at(xy)
    }
    fn source_alive(&mut self) -> Result<bool> {
        match self.command.source() {
            Some(s) => self.hooks.thing_alive_b4(s),
            None => Ok(false),
        }
    }
    fn source_owns_14(&mut self) -> Result<bool> {
        self.hooks.state_14_source_owns(self.command.source().ok_or("42A8F6 source thing")?)
    }
}
/// 42A770 on a compiled node and live command.
pub fn filter_42a770(_graph: &Arc<Graph>, node: &Node, kind: FxcKind, command: &Command, hooks: &mut impl Hooks) -> Result<bool> {
    let input = FilterInput {
        byte58: command.byte(0x58).unwrap_or(0) != 0,
        must_18: node_word(node, 0x18)?,
        must_not_1c: node_word(node, 0x1c)?,
        group_20: node.reference(0x20).and_then(Option::as_ref).map(|r| r.id.clone()),
        vf30: vf30(node, kind)?,
        source_xy: [word(command, 0x14)?, word(command, 0x18)?],
        target_xy: [word(command, 0x24)?, word(command, 0x28)?],
        threshold_2c: node_word(node, 0x2c)?,
        mode_30: node_word(node, 0x30)?,
        before_40: word(command, 0x40)?,
        now_44: word(command, 0x44)?,
        state_34: node_word(node, 0x34)?,
        ctx_state_54: word(command, 0x54)?,
    };
    filter_values_42a770(&input, &mut LiveFilter { node, kind, command, hooks })
}

fn execute_vf18(graph: &Arc<Graph>, id: NodeId, kind: FxcKind, command: &mut Command, view: ViewTime, pc: Precision, hooks: &mut impl Hooks) -> Result<bool> {
    let node = graph.nodes().get(id.index()).ok_or("FX node outside its arena")?;
    match kind {
        FxcKind::PlaySfx => play_sfx_42a920(graph, id, node, command, view, pc, hooks),
        FxcKind::StopSfx => Ok(true),
        FxcKind::StopVfx => {
            let result = hooks.stop_vfx_428c80(node, command)?;
            run_nodes_4289a0(graph, node.immediate(), command, view, pc, hooks)?;
            Ok(result)
        }
        FxcKind::CreateVfx => create_vfx_42a040(graph, node, command, view, pc, hooks),
        FxcKind::Equipment => Err("FXC Equipment vf18 428D10 is not ported".into()),
        FxcKind::ChangeAspect => Err("FXC ChangeAspect vf18 42A330 (45A280) is not ported".into()),
        FxcKind::ChangeSkin => Err("FXC ChangeSkin vf18 42A3A0 (454B60) is not ported".into()),
        FxcKind::CreateRfx => Err("FXC CreateRFX vf18 428E60 is not ported".into()),
        FxcKind::PlayObjectAnimation => Err("FXC PlayObjectAnimation vf18 42A410 is not ported".into()),
        FxcKind::PlayHitEffect => Err("FXC PlayHitEffect vf18 42AAE0 is not ported".into()),
        FxcKind::RequestOnly { native_type } => Err(format!("request-only FXC type {native_type} reached vf18")),
    }
}

/// 42A920 PlaySFX vf18.
fn play_sfx_42a920(graph: &Arc<Graph>, id: NodeId, node: &Node, command: &mut Command, view: ViewTime, pc: Precision, hooks: &mut impl Hooks) -> Result<bool> {
    run_nodes_4289a0(graph, node.immediate(), command, view, pc, hooks)?;
    let words = |base: u32| -> Result<[u32; 3]> { Ok([word(command, base)?, word(command, base + 4)?, word(command, base + 8)?]) };
    let (source_14, target_24) = (words(0x14)?, words(0x24)?);
    let mut state = |thing: Option<ActorId>| -> Result<Option<ThingState>> {
        let Some(t) = thing else { return Ok(None) };
        let Some(position) = hooks.thing_position(t)? else { return Ok(None) };
        Ok(Some(ThingState { alive_b4: hooks.thing_alive_b4(t)?, position }))
    };
    let at = node_word(node, 0x3c)?;
    let (source, target) = match at {
        0 => (state(command.source())?, None),
        1 => (None, state(command.target())?),
        _ => (None, None),
    };
    let map = if at == 6 { hooks.map_size()? } else { [0, 0] };
    let args = sfx_args_42a920(at, node.byte(0x40).unwrap_or(0), source_14, target_24, source, target, map, pc)?;
    let attached = args.attached.map(|slot| if slot == 0 { command.source() } else { command.target() }).flatten();
    let audio = node.reference(0x44).and_then(Option::as_ref).cloned().ok_or("42A920 PlaySFX node+44 Audio is null")?;
    let owner = command.kingdom();
    let context = [0x8u32, 0xc, 0x14, 0x18, 0x1c, 0x24, 0x28, 0x2c, 0x30, 0x34, 0x38, 0x3c, 0x40, 0x44, 0x48, 0x4c, 0x50, 0x54]
        .into_iter().filter_map(|o| command.word(o).map(|v| (o, v))).collect();
    hooks.play_sfx_7474f0(SfxRequest { audio, loop_38: args.loop_38, attached, owner, position: args.position,
        graph: Arc::clone(graph), node: id, context })?;
    Ok(true)
}

/// 42A040 CreateVFX vf18.
fn create_vfx_42a040(graph: &Arc<Graph>, node: &Node, command: &mut Command, view: ViewTime, pc: Precision, hooks: &mut impl Hooks) -> Result<bool> {
    let definition = node.reference(0x3c).and_then(Option::as_ref).cloned().ok_or("42A040 CreateVFX node+3C definition is null")?;
    if !hooks.vfx().gate_4445c0(&definition)? {
        return Ok(true);
    }
    let counter = word(command, 0x34)?;
    let frequency = node_word(node, 0x24)?;
    if counter != u32::MAX {
        if frequency == 0 {
            return Err("42A06E divides cmd+34 by a zero FXC frequency".into());
        }
        if counter % frequency != 0 {
            return Ok(false);
        }
    }
    //vf20 is called for its effects only; its result is ignored here.
    vf20(node, FxcKind::CreateVfx, hooks)?;
    let Some((thing, position)) = hooks.vfx().consolidate_429950(node, command)? else {
        //404F90 FXC.cpp 0x554.
        return Ok(false);
    };
    let at = node_word(node, 0x40)?;
    let location = node_word(node, 0x44)?;
    let duration = node_word(node, 0x60)?;
    let mut vfx = None;
    if at <= 1 {
        if let Some(view_thing) = thing {
            let attachable = hooks.vfx().view_9c(view_thing)?;
            hooks.vfx().view_a8(view_thing)?;
            //42A13D tests the location only for a view without vt9C; an
            //attachable view whose node is not 'attached' jumps to 42A148.
            if attachable && node.byte(0x65).unwrap_or(0) != 0 {
                let translation = [node_word(node, 0x48)?, node_word(node, 0x4c)?, node_word(node, 0x50)?];
                vfx = hooks.vfx().create_attached_43cd00(&definition, view_thing, location, translation, duration)?;
            } else if attachable || location == 0 {
                vfx = hooks.vfx().create_43cae0(&definition, position, duration)?;
                if let Some(v) = vfx {
                    hooks.vfx().orient_429880(v, node, view_thing)?;
                }
            }
        }
    } else if at <= 6 {
        let unset = position == [0xc2c8_0000; 3];
        if !unset {
            let duration = if node.byte(0x64).unwrap_or(0) != 0 { duration } else { 0 };
            vfx = hooks.vfx().create_43cae0(&definition, position, duration)?;
            if let (Some(v), Some(view_thing)) = (vfx, thing) {
                hooks.vfx().orient_429880(v, node, view_thing)?;
            }
        }
    }
    if let Some(v) = vfx {
        let created = st(&x(f(word(command, 0x38)?)?.add(&f(node_word(node, 0x28)?)?, pc))?)?;
        hooks.vfx().bind_444520(v, node, command, created)?;
        hooks.vfx().orient_target(v, &definition, command)?;
        hooks.vfx().finish_4449d0(v)?;
    }
    run_nodes_4289a0(graph, node.immediate(), command, view, pc, hooks)?;
    Ok(vfx.is_some())
}

