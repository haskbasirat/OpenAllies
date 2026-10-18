//! Typed original definition inputs. Unknown expressions/types remain errors.
use crate::resources::Provenance;
use aa_numeric::{Binary, Precision};
use serde::Serialize;
#[path = "native_actor_command.rs"]
pub mod actor_command;
#[path = "native_actor_morale.rs"]
pub mod actor_morale;
#[path = "native_categories.rs"]
mod categories;
#[path = "native_character_view.rs"]
pub mod character_view;
#[path = "native_denizen.rs"]
pub mod denizen;
#[path = "native_display_name.rs"]
pub mod display_name;
#[path = "native_index.rs"]
pub mod index;
#[path = "native_layout_data.rs"]
pub mod layout_data;
#[path = "native_object_definition.rs"]
pub mod object_definition;
#[path = "native_recruit_backrefs.rs"]
pub mod recruit_backrefs;
#[path = "native_recruit_data.rs"]
pub mod recruit_data;
#[path = "native_statistics.rs"]
pub mod statistics;
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct CategoryDescriptor {
    pub name: &'static str,
    pub registry_index: u32,
    pub native_kind: u32,
    pub runtime_class: u32,
    pub view_type: u32,
}
pub use categories::CATEGORIES;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("unsupported native definition: {0}")]
    Unsupported(String),
    #[error(transparent)]
    Numeric(#[from] aa_numeric::Error),
}
pub type Result<T> = std::result::Result<T, Error>;

pub const THING_FLAGS: &[(&str, u32)] = &[
    ("ethereal", 0x40000),
    ("revealed", 0x100000),
    ("tributable", 0x200),
    ("blocking", 0x400),
    ("corpse_blocking", 0x80000),
    ("invulnerable", 0x800),
    ("flying", 0x1000),
    ("water_based", 0x200000),
    ("flatten_terrain", 0x2000),
    ("targetable", 0x4000),
    ("random_orientation", 0x8000),
    ("tile_locked", 0x10000),
    ("axis_locked", 0x20000),
    ("invisible", 0x400000),
    ("radar", 0x100),
    ("ownable", 0x800000),
    ("conform_to_terrain", 0x1000000),
    ("allow_alpha_blend", 0x2000000),
];
pub const COMPONENTS: &[(&str, u8)] = &[
    ("BodyComponent", 0),
    ("ProjectileComponent", 1),
    ("CharacterComponent", 2),
    ("MoverComponent", 3),
    ("PerformerComponent", 5),
    ("PropertiesComponent", 6),
    ("OrganizationComponent", 7),
    ("ElementComponent", 8),
    ("EconomyComponent", 9),
    ("DenizenComponent", 10),
    ("SupplyComponent", 11),
    ("WallComponent", 12),
    ("StructureComponent", 13),
    ("SettlementComponent", 14),
    ("ConstructionComponent", 15),
    ("RecruitComponent", 16),
    ("ExperienceComponent", 17),
    ("BuilderComponent", 18),
    ("MobileDeploymentComponent", 19),
    ("DefenderComponent", 20),
    ("ResearchComponent", 21),
    ("DetectionComponent", 22),
    ("AircraftComponent", 23),
    ("TransportComponent", 24),
    ("LairComponent", 25),
    ("FlareComponent", 26),
    ("MetaGameComponent", 27),
    ("PortalComponent", 28),
    ("CityComponent", 29),
    ("HQComponent", 30),
];

/// Original4BB310 flag loop after typed flag reads. `is_actor` is native
/// definition class classification, not inferred from an ID/name.
pub fn base_flag_word(initial: u32, values: &[bool; 18], is_actor: bool) -> u32 {
    let mut result = initial;
    for ((_, mask), value) in THING_FLAGS.iter().zip(values) {
        if *value {
            result |= mask;
        } else {
            result &= !mask;
        }
    }
    result |= 1 << 29;
    if !is_actor && result & (1 << 11) == 0 {
        result |= 1 << 11;
    }
    result
}
/// Original final performer filter. Called only after actual ability loading.
pub fn finalize_component_mask(mut mask: u32, performer_ability_count: u32) -> u32 {
    if mask & (1 << 5) != 0
        && performer_ability_count == 0
        && mask & (1 << 20) == 0
        && mask & (1 << 23) == 0
    {
        mask &= !(1 << 5);
    }
    mask
}
/// Original465055..465089 for binary32 typed Fixed getter values. Both x87
/// comparisons must be ordered less-than OR EQUAL to zero; unordered retains
/// Detection. Fresh4624B0 supplies zero at+218 BEFORE Defender is loaded.
/// This projects the mask decision, not x87 exception/status side effects.
pub fn finalize_detection_component_mask(mask: u32, control_bits: u32, remote_bits: u32) -> u32 {
    if f32::from_bits(control_bits) <= 0.0 && f32::from_bits(remote_bits) <= 0.0 {
        mask & !(1 << 22)
    } else {
        mask
    }
}
/// Original ActorData468995..468A43 final flag writes for the standard ActorData
/// vtable (virtual3C=false), finite speed and verified category registry index.
pub fn finalize_actor_flags(
    mut flags: u32,
    component_mask: u32,
    speed_bits: u32,
    category_index: u32,
) -> Result<u32> {
    let speed = Binary::from_f32_bits(speed_bits)?;
    if (component_mask & (1 << 3) != 0
        && speed.compare(&Binary::from_u32(0)) == std::cmp::Ordering::Greater)
        || component_mask & ((1 << 7) | (1 << 1)) != 0
    {
        flags &= !(1 << 29);
    }
    if component_mask & ((1 << 0) | (1 << 7) | (1 << 14)) == 0
        && category_index != 31
        && flags & (1 << 11) == 0
    {
        flags |= 1 << 11;
    }
    Ok(flags)
}

#[derive(Clone, Debug, Serialize)]
pub struct ComponentMask {
    pub bits: u32,
    pub component_sources: std::collections::BTreeMap<String, Vec<Provenance>>,
    /// The count came from the caller's completed native-equivalent ability
    /// load; this module does not count raw list declarations as abilities.
    pub performer_ability_count: Option<u32>,
}

/// Original fixed and float TGI values both store binary32 but are distinct
/// runtime field types. Native GetFixed(virtual+18) rejects a float field.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum ScalarType {
    Fixed,
    Float,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct TypedScalar {
    pub bits: u32,
    pub field_type: ScalarType,
    pub source: Provenance,
    pub declaration_source: Provenance,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub enum TypedValue {
    Scalar(TypedScalar),
    Integer {
        value: i32,
        source: Provenance,
        declaration_source: Provenance,
    },
    Point {
        bits: Vec<u32>,
        source: Provenance,
        declaration_source: Provenance,
    },
    Boolean {
        value: bool,
        source: Provenance,
        declaration_source: Provenance,
    },
    String {
        value: String,
        source: Provenance,
        declaration_source: Provenance,
    },
}
impl TypedValue {
    pub fn integer(&self) -> Result<i32> {
        match self {
            Self::Integer { value, .. } => Ok(*value),
            _ => Err(Error::Unsupported(
                "native GetInteger rejects non-integer field".into(),
            )),
        }
    }
    pub fn fixed_bits(&self) -> Result<u32> {
        match self {
            Self::Scalar(s) if s.field_type == ScalarType::Fixed => Ok(s.bits),
            _ => Err(Error::Unsupported(
                "native GetFixed rejects non-fixed field".into(),
            )),
        }
    }
    pub fn boolean(&self) -> Result<bool> {
        match self {
            Self::Boolean { value, .. } => Ok(*value),
            _ => Err(Error::Unsupported(
                "native GetBoolean rejects non-boolean field".into(),
            )),
        }
    }
}

/// Explicit resolved fields consumed by native Body construction/lifecycle.
/// This is definition data, never current health or a restored component state.
#[derive(Clone, Debug, Serialize)]
pub struct BodyDefinition {
    pub health: TypedScalar,
    pub defense: TypedScalar,
    pub rot_time: TypedScalar,
    pub sink_time: TypedScalar,
    pub captureable: TypedValue,
    pub destructible: TypedValue,
}
#[derive(Clone, Debug, Serialize)]
pub struct MoverDefinition {
    pub speed_max: TypedScalar,
    pub acceleration: TypedScalar,
    pub turn_radius: TypedScalar,
    pub turn_type: TypedValue,
}
impl MoverDefinition {
    /// Actual original463136 enum registration. These integers are consumed
    /// by the native Mover constructor; unknown names do not select a fallback.
    pub fn native_turn_type(&self) -> Result<u32> {
        let TypedValue::String { value, .. } = &self.turn_type else {
            return Err(Error::Unsupported("Mover turn_type must be string".into()));
        };
        ["INSTANT", "IN_PLACE", "WHILE_MOVING"]
            .iter()
            .position(|name| value.eq_ignore_ascii_case(name))
            .map(|value| value as u32)
            .ok_or_else(|| Error::Unsupported(format!("unknown native mover turn_type {value}")))
    }
}
#[derive(Clone, Debug, Serialize)]
pub struct ActorAttributes {
    pub id: String,
    pub source: Provenance,
    pub body: Option<BodyDefinition>,
    pub mover: Option<MoverDefinition>,
}

impl crate::actors::ActorCatalog {
    /// Native built-in category descriptor fields from the executed original
    /// registry initializer. This validates authored category text independently
    /// of an untrusted serialized runtime class. Registry ordinal comes from
    /// the actual native assignment/append, not from a sorted source list.
    pub fn native_category(&self, id: &str) -> Result<CategoryDescriptor> {
        match self.native_field(id, "category", Precision::Double53)? {
            Some(TypedValue::String { value, .. }) => CATEGORIES
                .iter()
                .find(|c| c.name.eq_ignore_ascii_case(&value))
                .copied()
                .ok_or_else(|| Error::Unsupported(format!("unknown native category {value}"))),
            _ => Err(Error::Unsupported(format!(
                "missing typed category for {id}"
            ))),
        }
    }
    pub fn native_class(&self, id: &str) -> Result<u32> {
        Ok(self.native_category(id)?.runtime_class)
    }
    /// Complete known Thing/Object/standardActor definition flag writes, based
    /// on independently resolved category. Other derived data classes remain
    /// unsupported rather than assuming they use the same flag finalizer.
    pub fn native_final_flags(&self, id: &str, precision: Precision) -> Result<u32> {
        let category = self.native_category(id)?;
        match category.native_kind {
            // DynamicDoodadData4922E0 calls ObjectData49DC40 and leaves its
            // flag word unchanged; native-doodads.json pins all derived writes.
            0 | 1 | 20 | 21 => Ok(self.native_base_flags(id, false, precision)?.0),
            2 => {
                let base = self.native_base_flags(id, true, precision)?.0;
                let mut mask = 0;
                for (name, bit) in [
                    ("BodyComponent", 0),
                    ("ProjectileComponent", 1),
                    ("MoverComponent", 3),
                    ("OrganizationComponent", 7),
                    ("SettlementComponent", 14),
                ] {
                    if !self.native_node_sources(id, &[name])?.is_empty() {
                        mask |= 1 << bit;
                    }
                }
                let speed = if mask & (1 << 3) != 0 {
                    self.native_field(id, "MoverComponent.speed_max", precision)?
                        .ok_or_else(|| Error::Unsupported("Mover speed_max absent".into()))?
                        .fixed_bits()?
                } else {
                    0
                };
                finalize_actor_flags(base, mask, speed, category.registry_index)
            }
            other => Err(Error::Unsupported(format!(
                "definition kind {other} flag finalizer not ported"
            ))),
        }
    }
    /// Fresh definition mask at the observed Windows/v14 source target PC53.
    /// Call native_component_mask_at when the caller already owns precision.
    /// Mind has no definition bit; completed Performer loading remains required.
    pub fn native_component_mask(
        &self,
        id: &str,
        performer_ability_count: Option<u32>,
    ) -> Result<ComponentMask> {
        self.native_component_mask_at(id, performer_ability_count, Precision::Double53)
    }
    /// Fresh464BD0 projection with explicit typed-field precision. Detection
    /// loads before Defender and sees constructor+218=0: its ordered <=0/<=0
    /// clear INCLUDES equality. Later authored Defender control_range cannot
    /// restore that bit. Reloading an existing ActorData is outside this API.
    pub fn native_component_mask_at(
        &self,
        id: &str,
        performer_ability_count: Option<u32>,
        precision: Precision,
    ) -> Result<ComponentMask> {
        if self.native_category(id)?.native_kind != 2 {
            return Err(Error::Unsupported(
                "component mask requires standard ActorData definition kind2".into(),
            ));
        }
        let mut bits = 0;
        let mut sources = std::collections::BTreeMap::new();
        for (name, bit) in COMPONENTS {
            let found = self.native_node_sources(id, &[name])?;
            if !found.is_empty() {
                bits |= 1u32 << bit;
                sources.insert((*name).to_owned(), found);
            }
        }
        if bits & (1 << 22) != 0 {
            let remote = self
                .native_field(id, "DetectionComponent.remote_detection_range", precision)?
                .ok_or_else(|| {
                    Error::Unsupported("Detection remote_detection_range absent".into())
                })?
                .fixed_bits()?;
            bits = finalize_detection_component_mask(bits, 0, remote);
        }
        if bits & (1 << 21) != 0 && bits & (1 << 13) == 0 {
            return Err(Error::Unsupported(
                "ResearchComponent requires StructureComponent in native actor data".into(),
            ));
        }
        if bits & (1 << 5) != 0 && bits & ((1 << 20) | (1 << 23)) == 0 {
            let count = performer_ability_count.ok_or_else(|| {
                Error::Unsupported(
                    "component mask requires completed PerformerComponent Ability count".into(),
                )
            })?;
            bits = finalize_component_mask(bits, count);
        }
        Ok(ComponentMask {
            bits,
            component_sources: sources,
            performer_ability_count,
        })
    }
    /// Typed Flags child -> native base ThingData+1C. ActorData's later flag
    /// adjustments are a separate step; this return must not be mislabeled as
    /// the final actor definition flags.
    pub fn native_base_flags(
        &self,
        id: &str,
        is_actor: bool,
        precision: Precision,
    ) -> Result<(u32, Vec<TypedValue>)> {
        let mut values = [false; 18];
        let mut provenance = Vec::new();
        for (i, (name, _)) in THING_FLAGS.iter().enumerate() {
            let path = format!("Flags.{name}");
            let field = self
                .native_field(id, &path, precision)?
                .ok_or_else(|| Error::Unsupported(format!("required flag {path} absent")))?;
            values[i] = field.boolean()?;
            provenance.push(field);
        }
        Ok((base_flag_word(0, &values, is_actor), provenance))
    }
    /// Resolve actual inherited fields required by the bounded native Body and
    /// Mover constructors. Component absence remains None; missing assigned
    /// values/types are errors. No component mask or actor factory is implied.
    pub fn native_attributes(&self, id: &str, precision: Precision) -> Result<ActorAttributes> {
        let actor = self
            .find(id)
            .ok_or_else(|| Error::Unsupported(format!("missing actor {id}")))?;
        let field = |path: &str| {
            self.native_field(id, path, precision)?
                .ok_or_else(|| Error::Unsupported(format!("required field {path} missing")))
        };
        let fixed = |path: &str| -> Result<TypedScalar> {
            match field(path)? {
                TypedValue::Scalar(s) if s.field_type == ScalarType::Fixed => Ok(s),
                _ => Err(Error::Unsupported(format!(
                    "{path} must be native fixed type"
                ))),
            }
        };
        let boolean = |path: &str| -> Result<TypedValue> {
            let v = field(path)?;
            v.boolean()?;
            Ok(v)
        };
        let body = if self.native_node_sources(id, &["BodyComponent"])?.is_empty() {
            None
        } else {
            Some(BodyDefinition {
                health: fixed("BodyComponent.health")?,
                defense: fixed("BodyComponent.defense")?,
                rot_time: fixed("BodyComponent.rot_time")?,
                sink_time: fixed("BodyComponent.sink_time")?,
                captureable: boolean("BodyComponent.captureable")?,
                destructible: boolean("BodyComponent.destructible")?,
            })
        };
        let mover = if self
            .native_node_sources(id, &["MoverComponent"])?
            .is_empty()
        {
            None
        } else {
            let turn_type = field("MoverComponent.turn_type")?;
            if !matches!(turn_type, TypedValue::String { .. }) {
                return Err(Error::Unsupported("Mover turn_type must be string".into()));
            }
            Some(MoverDefinition {
                speed_max: fixed("MoverComponent.speed_max")?,
                acceleration: fixed("MoverComponent.acceleration")?,
                turn_radius: fixed("MoverComponent.turn_radius")?,
                turn_type,
            })
        };
        Ok(ActorAttributes {
            id: actor.id.clone(),
            source: actor.source.clone(),
            body,
            mover,
        })
    }
}

/// Canonical finite ASCII decimal subset of 5BD6E0 ->5B9E20/5BB9F0.
/// Exponents/references/arithmetic are not silently evaluated. CRT __wtof
/// produces a double, then native stores binary32 BEFORE optional percent scale.
pub fn scalar_literal(text: &str, precision: Precision) -> Result<u32> {
    let text = text.trim();
    let (without_percent, percent) = match text.strip_suffix('%') {
        Some(v) => (v, true),
        None => (text, false),
    };
    let (number, suffix) = match without_percent.as_bytes().last() {
        Some(b'f' | b'd' | b'k') => (
            &without_percent[..without_percent.len() - 1],
            without_percent.as_bytes().last().copied(),
        ),
        _ => (without_percent, None),
    };
    let unsigned = number.strip_prefix(['+', '-']).unwrap_or(number);
    let mut dot = false;
    let mut digits = 0;
    for byte in unsigned.bytes() {
        if byte == b'.' && !dot {
            dot = true;
        } else if byte.is_ascii_digit() {
            digits += 1;
        } else {
            return Err(Error::Unsupported(format!("nonliteral scalar {text:?}")));
        }
    }
    if digits == 0 || suffix == Some(b'd') && dot || percent && suffix.is_some() {
        return Err(Error::Unsupported(format!(
            "invalid scalar literal {text:?}"
        )));
    }
    let double: f64 = number
        .parse()
        .map_err(|_| Error::Unsupported(format!("invalid scalar literal {text:?}")))?;
    let initial = Binary::from_f64_bits(double.to_bits())?.store_f32()?;
    if percent {
        Ok(Binary::from_f32_bits(initial)?
            .mul(&Binary::from_f32_bits(0x3c23_d70a)?, precision)?
            .store_f32()?)
    } else {
        Ok(initial)
    }
}

/// Bounded point2 component assignment through 5BD6E0 -> 5BB7F0.
/// The native classifier accepts additional decimal dots; CRT __wtof then
/// consumes the first decimal prefix before the binary32 store. Keep this
/// proven digits/dots subset local to point2 rather than relaxing scalar fields.
pub(crate) fn point2_component_literal(text: &str, precision: Precision) -> Result<u32> {
    let strict_error = match scalar_literal(text, precision) {
        Ok(bits) => return Ok(bits),
        Err(error) => error,
    };
    let text = text.trim();
    let unsigned = text.strip_prefix(['+', '-']).unwrap_or(text);
    if !unsigned.bytes().all(|b| b.is_ascii_digit() || b == b'.') {
        return Err(strict_error);
    }
    let Some(second_dot) = text.match_indices('.').nth(1).map(|(at, _)| at) else {
        return Err(strict_error);
    };
    let prefix = &text[..second_dot];
    if !prefix.bytes().any(|b| b.is_ascii_digit()) {
        return Err(strict_error);
    }
    scalar_literal(prefix, precision)
}
