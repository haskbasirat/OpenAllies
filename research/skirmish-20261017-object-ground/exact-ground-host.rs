pub use aa_spatial::*;
mod position{
// Original Object/Actor position-change orchestration and small state leaves.
// Component handles refer to the caller's authoritative component registry.
// Required world mutations are fallible services, never omitted callbacks.
use crate::{
    Error, Result,
    collision::Shape,
    geometry::{Placement, marks_blocker},
    index::{ActorKey, SpatialIndex},
};
use aa_numeric::{Binary, Precision};
use serde::{Deserialize, Serialize};

pub type ComponentKey = u32;
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Components {
    pub organization: Option<ComponentKey>,
    pub element: Option<ComponentKey>,
    pub mover: Option<ComponentKey>,
    pub settlement: Option<ComponentKey>,
    pub history: Option<ComponentKey>,
    pub follower_reference: u32,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PositionHistory {
    pub samples: Vec<[u32; 3]>,
    pub read_index: i32,
    pub write_index: i32,
    pub reset_latch: bool,
    pub last_frame: i32,
}
impl PositionHistory {
    /// Original50A710. The cursor resets only on the false→true transition.
    pub fn set_reset(&mut self, requested: bool) {
        if self.reset_latch != requested {
            self.reset_latch = requested;
            if requested {
                self.write_index = self.read_index;
            }
        }
    }
    /// Original50A970, including signed/wrapping frame admission and modulo
    /// ring cursors. Caller supplies the actual SVar interval and world clock.
    pub fn sample(
        &mut self,
        frame: i32,
        interval: i32,
        time_bits: u32,
        xy: [u32; 2],
    ) -> Result<bool> {
        if self.last_frame.wrapping_add(interval) > frame {
            return Ok(false);
        }
        self.last_frame = frame;
        let count = i32::try_from(self.samples.len()).map_err(|_| {
            Error::World("history capacity exceeds native signed representation".into())
        })?;
        if count <= 0
            || self.read_index < 0
            || self.read_index >= count
            || self.write_index < 0
            || self.write_index >= count
        {
            return Err(Error::World("invalid native position-history ring".into()));
        }
        for bits in xy {
            Binary::from_f32_bits(bits)?;
        }
        self.samples[self.write_index as usize] = [xy[0], xy[1], time_bits];
        self.write_index = self.write_index.wrapping_add(1) % count;
        if self.write_index == self.read_index {
            self.read_index = self.read_index.wrapping_add(1) % count;
        }
        Ok(true)
    }
}
///46B000 and inlined4D0F61 compare ordered values: signed zeros are equal,
/// either NaN is different. No approximate epsilon/bitwise equality substitute.
pub fn different_xy(a: [u32; 2], b: [u32; 2]) -> bool {
    f32::from_bits(a[0]) != f32::from_bits(b[0]) || f32::from_bits(a[1]) != f32::from_bits(b[1])
}

/// Native5528F0 footprint expansion. Circle/point preserve unused bounds;
/// rectangles retain the original setter's kind/radius arithmetic and stores.
pub fn expand_footprint(shape: &Shape, padding_bits: u32, p: Precision) -> Result<Shape> {
    let mut out = shape.clone();
    let b = |bits| Binary::from_f32_bits(bits).map_err(Error::from);
    let padding = b(padding_bits)?;
    let zero = Binary::from_i32(0);
    let half = b(0x3f000000)?;
    if shape.kind <= 1 {
        let radius = padding.mul(&half, p)?.add(&b(shape.radius_bits)?, p)?;
        let stored = radius.store_f32()?;
        if radius.compare(&zero).is_gt() {
            out.kind = 1;
            out.radius_bits = stored;
        } else {
            out.kind = 0;
            out.radius_bits = 0;
        }
        return Ok(out);
    }
    if shape.kind > 3 {
        return Err(Error::World("unsupported footprint shape".into()));
    }
    let dimension = |bits| -> Result<Binary> {
        let x = b(bits)?;
        Ok(x.add(&x, p)?.add(&padding, p)?)
    };
    let clamp = |x: Binary| -> Result<Binary> {
        if x.is_negative_nonzero() {
            return Ok(Binary::from_i32(0));
        }
        let x = b(x.store_f32()?)?;
        let max = b(0x7f7fffff)?;
        Ok(if x.compare(&max).is_gt() { max } else { x })
    };
    let (width, height) = if shape.kind == 2 {
        let width = clamp(dimension(shape.bounds[2])?)?;
        (width.clone(), width)
    } else {
        // RECT stores both raw expanded dimensions before its comparisons.
        (
            clamp(b(dimension(shape.bounds[2])?.store_f32()?)?)?,
            clamp(b(dimension(shape.bounds[3])?.store_f32()?)?)?,
        )
    };
    out.kind = if width.compare(&height).is_eq() { 2 } else { 3 };
    let min_x = width.mul(&half.clone().negated(), p)?;
    let max_y = height.mul(&half, p)?;
    out.bounds = [
        min_x.store_f32()?,
        height.mul(&half.clone().negated(), p)?.store_f32()?,
        width.mul(&half, p)?.store_f32()?,
        max_y.store_f32()?,
    ];
    out.radius_bits = max_y
        .mul(&max_y, p)?
        .add(&min_x.mul(&min_x, p)?, p)?
        .sqrt(p)?
        .store_f32()?;
    Ok(out)
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HeightEdit {
    pub placement: Placement,
    /// Native settings+0=mode,+4=height,+8=margin,+C=min,+10=max.
    pub settings_words: [u32; 5],
    pub flags: [bool; 2],
    pub strength_bits: u32,
    pub trailing_flag: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TerrainPaint {
    pub placement: Placement,
    pub variation: u32,
    pub deferred: bool,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum Diagnostic {
    Identity(u32),
    X(u32),
    Y(u32),
}

/// Reads without &mut are snapshots. Mutating services must preserve the active
/// actor/component lifetimes and return errors for unsupported effects. Earlier
/// native effects survive a later error. No service has a permissive default.
pub trait World {
    fn components(&self, actor: ActorKey) -> Result<Components>;
    fn session_mode(&self) -> Result<i32>;
    fn world_word_48(&self) -> Result<u32>;
    fn height_margin_bits(&self) -> Result<u32>;
    fn ground_height(
        &mut self,
        index: &mut SpatialIndex,
        position: [u32; 2],
        flags: u32,
    ) -> Result<Binary>;
    fn edit_height(
        &mut self,
        index: &mut SpatialIndex,
        actor: ActorKey,
        edit: HeightEdit,
    ) -> Result<()>;
    fn clear_doodads(
        &mut self,
        index: &mut SpatialIndex,
        actor: ActorKey,
        placement: Placement,
    ) -> Result<()>;
    /// Actual4EA260 lookup, including effective parent/biome definitions.
    fn ground_variation(
        &mut self,
        index: &mut SpatialIndex,
        actor: ActorKey,
    ) -> Result<Option<u32>>;
    fn footprint_padding_bits(&self, actor: ActorKey) -> Result<u32>;
    fn paint_terrain(
        &mut self,
        index: &mut SpatialIndex,
        actor: ActorKey,
        paint: TerrainPaint,
    ) -> Result<()>;
    /// Actual virtual+124 (ordinary4D6350): fog/terrain/region transition work.
    fn cell_changed(
        &mut self,
        index: &mut SpatialIndex,
        actor: ActorKey,
        old: [i32; 2],
    ) -> Result<()>;
    fn diagnostic(
        &mut self,
        index: &mut SpatialIndex,
        actor: ActorKey,
        value: Diagnostic,
    ) -> Result<()>;
    fn component_owner(&self, component: ComponentKey) -> Result<ActorKey>;
    /// Write real Organization/Regiment+BC/+C0 words; no independent copy.
    fn set_organization_saved_xy(&mut self, component: ComponentKey, xy: [u32; 2]) -> Result<()>;
    /// Actual5705E0→57C800 organization movement service.
    fn element_position_changed(
        &mut self,
        index: &mut SpatialIndex,
        component: ComponentKey,
    ) -> Result<()>;
    /// Actual Actor virtual+158 and +15C, e.g.aa_actors::attachment helpers.
    fn is_attached(&mut self, index: &mut SpatialIndex, actor: ActorKey) -> Result<bool>;
    fn attachment_parent(&mut self, index: &mut SpatialIndex, actor: ActorKey) -> Result<ActorKey>;
    fn settlement_attached_actor(&self, component: ComponentKey) -> Result<Option<ActorKey>>;
    /// Calls actual4EEB40, including this position callback recursively.
    fn move_ground(
        &mut self,
        index: &mut SpatialIndex,
        actor: ActorKey,
        position: [u32; 2],
    ) -> Result<()>;
    /// Definition+270/+274 attachment offset for the mover's owner.
    fn follower_offset(&self, owner: ActorKey) -> Result<[u32; 2]>;
    /// Actual577240 on the resolved follower's mover, not an issued game order.
    fn follower_target(
        &mut self,
        index: &mut SpatialIndex,
        follower: ActorKey,
        position: [u32; 3],
    ) -> Result<()>;
    fn history_mut(&mut self, component: ComponentKey) -> Result<&mut PositionHistory>;
    fn history_interval(&self) -> Result<i32>;
    fn frame(&self) -> Result<i32>;
    fn world_time_bits(&self) -> Result<u32>;
}

///The reached4EB500 services over the current object. Each field and child is
///read only at its native point. The runtime may release short index loans
///before destruction callbacks mutate that same world.
pub trait ObjectEffects {
    fn precision(&self) -> Precision;
    fn flags(&self) -> Result<u32>;
    fn blocking_radius(&self) -> Result<u32>;
    fn cell_size(&self) -> Result<u32>;
    fn position(&self) -> Result<[u32; 2]>;
    fn blocking_shape(&self) -> Result<Shape>;
    fn facing(&self) -> Result<[u32; 2]>;
    fn actor_category(&self) -> Result<bool>;
    fn wall_present(&self) -> Result<bool>;
    fn session_mode(&self) -> Result<i32>;
    fn world_word_48(&self) -> Result<u32>;
    fn height_margin_bits(&self) -> Result<u32>;
    fn ground_height(&mut self, position: [u32; 2], flags: u32) -> Result<Binary>;
    fn edit_height(&mut self, edit: HeightEdit) -> Result<()>;
    fn clear_doodads(&mut self, placement: Placement) -> Result<()>;
    fn ground_variation(&mut self) -> Result<Option<u32>>;
    fn footprint_padding_bits(&self) -> Result<u32>;
    fn paint_terrain(&mut self, paint: TerrainPaint) -> Result<()>;
}
///4EB52D..579 for a genuine ActorParams (A25B28 vf0=true). The+4C GTID
///is not read unless mode6 is reached. Invalid/retired references fall back to
///the ground sampler; a valid referenced Thing supplies its current+28 word.
pub fn actor_post_create_height(
    mode: u32,
    reference: impl FnOnce() -> Result<u32>,
    mut referenced_height: impl FnMut(u32) -> Result<Option<Binary>>,
    ground: impl FnOnce() -> Result<Binary>,
) -> Result<Binary> {
    if mode == 6 {
        let reference = reference()?;
        if reference != 0
            && let Some(height) = referenced_height(reference)?
        {
            return Ok(height);
        }
    }
    ground()
}
struct PositionObjectEffects<'a, W> {
    index: &'a mut SpatialIndex,
    actor: ActorKey,
    world: &'a mut W,
}
impl<W: World> ObjectEffects for PositionObjectEffects<'_, W> {
    fn precision(&self) -> Precision {
        self.index.precision()
    }
    fn flags(&self) -> Result<u32> {
        Ok(self.index.member(self.actor)?.actor.flags)
    }
    fn blocking_radius(&self) -> Result<u32> {
        Ok(self.index.member(self.actor)?.blocking.radius_bits)
    }
    fn cell_size(&self) -> Result<u32> {
        Ok(self.index.grid().cell_size_bits)
    }
    fn position(&self) -> Result<[u32; 2]> {
        Ok(self.index.member(self.actor)?.actor.position)
    }
    fn blocking_shape(&self) -> Result<Shape> {
        Ok(self.index.member(self.actor)?.blocking.clone())
    }
    fn facing(&self) -> Result<[u32; 2]> {
        Ok(self.index.member(self.actor)?.actor.facing)
    }
    fn actor_category(&self) -> Result<bool> {
        Ok(self.index.member(self.actor)?.actor_category)
    }
    fn wall_present(&self) -> Result<bool> {
        Ok(self.index.member(self.actor)?.wall_present)
    }
    fn session_mode(&self) -> Result<i32> {
        World::session_mode(self.world)
    }
    fn world_word_48(&self) -> Result<u32> {
        World::world_word_48(self.world)
    }
    fn height_margin_bits(&self) -> Result<u32> {
        World::height_margin_bits(self.world)
    }
    fn ground_height(&mut self, p: [u32; 2], f: u32) -> Result<Binary> {
        World::ground_height(self.world, self.index, p, f)
    }
    fn edit_height(&mut self, e: HeightEdit) -> Result<()> {
        World::edit_height(self.world, self.index, self.actor, e)
    }
    fn clear_doodads(&mut self, p: Placement) -> Result<()> {
        World::clear_doodads(self.world, self.index, self.actor, p)
    }
    fn ground_variation(&mut self) -> Result<Option<u32>> {
        World::ground_variation(self.world, self.index, self.actor)
    }
    fn footprint_padding_bits(&self) -> Result<u32> {
        World::footprint_padding_bits(self.world, self.actor)
    }
    fn paint_terrain(&mut self, p: TerrainPaint) -> Result<()> {
        World::paint_terrain(self.world, self.index, self.actor, p)
    }
}
///4EB500 null-context branch used by every4EB820 position notification.
pub fn object_effects<W: World>(
    index: &mut SpatialIndex,
    actor: ActorKey,
    world: &mut W,
) -> Result<()> {
    object_effects_on_owners(&mut PositionObjectEffects {
        index,
        actor,
        world,
    })
}
fn current_placement(world: &impl ObjectEffects) -> Result<Placement> {
    //Object+0C yields its current immutable blocking shape. Following field
    //reads have no mutating native callback between them.
    Ok(Placement {
        shape: world.blocking_shape()?,
        facing: world.facing()?,
        position: world.position()?,
    })
}
///One native orchestration for both null and genuine nonnull Params callers.
///The height service decides the reached Params branch; all later flag,
///shape, wall, variation and Session reads retain the original order.
pub fn object_effects_on_owners(world: &mut impl ObjectEffects) -> Result<()> {
    let flags = world.flags()?;
    if flags & 0x20000400 == 0x20000400
        && marks_blocker(
            flags,
            world.blocking_radius()?,
            world.cell_size()?,
            world.precision(),
        )?
        && world.flags()? & (1 << 13) != 0
    {
        let position = world.position()?;
        let height = world.ground_height(position, 0x122)?.store_f32()?;
        let margin = world.height_margin_bits()?;
        let placement = current_placement(world)?;
        world.edit_height(HeightEdit {
            placement,
            settings_words: [0, height, margin, 0xff7fffff, 0x7f7fffff],
            flags: [true, true],
            strength_bits: 0x3f800000,
            trailing_flag: false,
        })?;
    }
    if world.flags()? & 0x20000400 == 0x20000400 {
        let placement = current_placement(world)?;
        world.clear_doodads(placement)?;
    }
    if world.world_word_48()? == 0
        && let Some(variation) = world.ground_variation()?
    {
        if !world.actor_category()? || !world.wall_present()? {
            let position = world.position()?;
            let shape = world.blocking_shape()?;
            let padding = world.footprint_padding_bits()?;
            let shape = expand_footprint(&shape, padding, world.precision())?;
            let deferred = world.session_mode()? != 3;
            let facing = world.facing()?;
            world.paint_terrain(TerrainPaint {
                placement: Placement {
                    shape,
                    facing,
                    position,
                },
                variation,
                deferred,
            })?;
        }
    }
    Ok(())
}

mod replay{use super::*;
use serde_json::{Value, json};
struct Ground {
    input: Value,
    pc: Precision,
    events: Vec<Value>,
    stop: Option<&'static str>,
}
impl Ground {
    fn word(&self, name: &str) -> u32 {
        self.input[name].as_u64().unwrap() as u32
    }
    fn event(&mut self, event: Value) -> Result<()> {
        let stop = self.stop == event["op"].as_str();
        self.events.push(event);
        if stop {
            Err(Error::World("required child".into()))
        } else {
            Ok(())
        }
    }
}
impl ObjectEffects for Ground {
    fn precision(&self) -> Precision {
        self.pc
    }
    fn flags(&self) -> Result<u32> {
        Ok(self.word("flags"))
    }
    fn blocking_radius(&self) -> Result<u32> {
        Ok(self.input["shape"][1].as_u64().unwrap() as u32)
    }
    fn cell_size(&self) -> Result<u32> {
        Ok(self.word("cell"))
    }
    fn position(&self) -> Result<[u32; 2]> {
        Ok(std::array::from_fn(|i| {
            self.input["position"][i].as_u64().unwrap() as u32
        }))
    }
    fn blocking_shape(&self) -> Result<Shape> {
        let v = &self.input["shape"];
        Ok(Shape {
            kind: v[0].as_u64().unwrap() as u32,
            radius_bits: v[1].as_u64().unwrap() as u32,
            bounds: std::array::from_fn(|i| v[i + 2].as_u64().unwrap() as u32),
        })
    }
    fn facing(&self) -> Result<[u32; 2]> {
        Ok(std::array::from_fn(|i| {
            self.input["facing"][i].as_u64().unwrap() as u32
        }))
    }
    fn actor_category(&self) -> Result<bool> {
        Ok(self.input["actor"].as_bool().unwrap())
    }
    fn wall_present(&self) -> Result<bool> {
        Ok(self.input["wall"].as_bool().unwrap())
    }
    fn session_mode(&self) -> Result<i32> {
        Ok(self.word("session") as i32)
    }
    fn world_word_48(&self) -> Result<u32> {
        Ok(self.word("world48"))
    }
    fn height_margin_bits(&self) -> Result<u32> {
        Ok(self.word("slope"))
    }
    fn ground_height(&mut self, position: [u32; 2], flags: u32) -> Result<Binary> {
        let mode = self.word("mode");
        let reference = self.word("reference");
        let valid = self.input["reference_valid"].as_bool().unwrap();
        let reference_height = self.word("reference_height");
        let height = self.input["height64"].as_u64().unwrap();
        actor_post_create_height(
            mode,
            || Ok(reference),
            |r| {
                assert_eq!(r, 7);
                Ok(if valid {
                    Some(Binary::from_f32_bits(reference_height)?)
                } else {
                    None
                })
            },
            || {
                self.event(json!({"op":"ground_height","position":position,"flags":flags}))?;
                Ok(Binary::from_f64_bits(height)?)
            },
        )
    }
    fn edit_height(&mut self, edit: HeightEdit) -> Result<()> {
        self.event(json!({"op":"edit_height","edit":edit}))
    }
    fn clear_doodads(&mut self, placement: Placement) -> Result<()> {
        self.event(json!({"op":"clear_doodads","placement":placement}))
    }
    fn ground_variation(&mut self) -> Result<Option<u32>> {
        self.event(json!({"op":"ground_variation"}))?;
        Ok((self.word("variation") != 0).then_some(self.word("variation")))
    }
    fn footprint_padding_bits(&self) -> Result<u32> {
        Ok(self.word("padding"))
    }
    fn paint_terrain(&mut self, paint: TerrainPaint) -> Result<()> {
        self.event(json!({"op":"paint_terrain","paint":paint}))
    }
}
#[test]
fn native_nonnull_gacd_ground_orchestration_all_precisions_and_required_prefixes() {
    let f: Value = serde_json::from_str(include_str!(
        "E:/Games/Axis and Alies/rust-rebuild/research/skirmish-20261017-object-ground/native-ground-nonnull.json"
    ))
    .unwrap();
    assert_eq!(f["rows"].as_array().unwrap().len(), 192);
    for row in f["rows"].as_array().unwrap() {
        let pc = match row["pc"].as_str().unwrap() {
            "Single24" => Precision::Single24,
            "Double53" => Precision::Double53,
            _ => Precision::Extended64,
        };
        let mut world = Ground {
            input: row["input"].clone(),
            pc,
            events: vec![],
            stop: None,
        };
        object_effects_on_owners(&mut world).unwrap();
        assert_eq!(json!(world.events), row["native"]["events"], "{row}");
        for stop in ["edit_height", "clear_doodads", "paint_terrain"] {
            let Some(position) = world.events.iter().position(|v| v["op"] == stop) else {
                continue;
            };
            let mut prefix = Ground {
                input: row["input"].clone(),
                pc,
                events: vec![],
                stop: Some(stop),
            };
            assert!(object_effects_on_owners(&mut prefix).is_err());
            assert_eq!(prefix.events, world.events[..=position]);
        }
    }
}
#[test]
fn non_mode6_never_reads_reference_and_zero_never_resolves() {
    for mode in [0, 1, 2, 3, 4, 5, u32::MAX] {
        assert_eq!(
            actor_post_create_height(
                mode,
                || panic!("unreached+4C"),
                |_| panic!("unreachedGTID"),
                || Ok(Binary::from_i32(3))
            )
            .unwrap().store_f32().unwrap(),
            3f32.to_bits()
        );
    }
    actor_post_create_height(
        6,
        || Ok(0),
        |_| panic!("null reference"),
        || Ok(Binary::from_i32(3)),
    )
    .unwrap();
}
}}
