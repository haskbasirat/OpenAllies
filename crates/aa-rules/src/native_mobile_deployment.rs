//! Source-bound MobileDeployment references and original4605B0 pairing.
//! 473AD0 prepends deferred references;475230 resolves them in reverse
//! creation order. A later executed write replaces target+548, including
//! when two different authored aliases resolve to the same definition.
use crate::{
    actors::{ActorCatalog, ResolvedNode},
    native::{CATEGORIES, Error, Result, TypedValue},
    resources::Provenance,
};
use aa_numeric::{Binary, Precision};
use serde::Serialize;
use std::{cmp::Ordering, collections::BTreeMap};

fn invalid(message: impl Into<String>) -> Error {
    Error::Unsupported(message.into())
}

#[derive(Clone, Debug, Serialize)]
pub struct Definition {
    pub id: String,
    /// Only the three +174 bits read by4605B0; not a completed Performer mask.
    pub body: bool,
    pub mobile: bool,
    pub element: bool,
    /// +538/+53C, authored only on the forward-unpack side.
    pub packed_position_offset_bits: [u32; 2],
    /// +54C: constructor zero, overwritten only for an empty forward IDS.
    pub pack_health_min_bits: u32,
    /// +534/+548 canonical registered Thing identities; None is proven null.
    pub unpack_to: Option<String>,
    pub pack_to: Option<String>,
    /// Target ObjectData+54, populated only for a reached shape validation.
    pub soft_radius_bits: Option<u32>,
    pub sources: Vec<Provenance>,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct NativeMobileDeploymentData {
    /// Canonical IDS keys are ASCII-lowercase; values preserve source spelling.
    pub definitions: BTreeMap<String, Definition>,
    /// Actual deferred mobile callback execution order (source, target).
    pub resolution_order: Vec<(String, Option<String>)>,
}
impl NativeMobileDeploymentData {
    pub fn definition(&self, canonical_id: &str) -> Result<&Definition> {
        self.definitions
            .get(&canonical_id.to_ascii_lowercase())
            .ok_or_else(|| {
                invalid(format!(
                    "MobileDeployment ActorData unavailable: {canonical_id}"
                ))
            })
    }

    ///4605B0 after native4BDBA0 identity lookup. Unknown target and no Body
    /// fail before+534 is written; later errors preserve that forward write.
    /// The inverse+548 is written only after all validation succeeds.
    pub fn resolve_reference(
        &mut self,
        source: &str,
        target: Option<&str>,
        pc: Precision,
    ) -> Result<()> {
        let source_key = source.to_ascii_lowercase();
        let source_id = self.definition(source)?.id.clone();
        let target_key = target.map(str::to_ascii_lowercase);
        self.resolution_order.push((
            source_id.clone(),
            target_key
                .as_deref()
                .and_then(|key| self.definitions.get(key))
                .map(|d| d.id.clone()),
        ));
        let Some(target_key) = target_key.filter(|key| self.definitions.contains_key(key)) else {
            return Err(invalid("Unknown deployment actor '%s' for actor '%s'"));
        };
        let target = self.definitions[&target_key].clone();
        if !target.body {
            return Err(invalid(
                "Actor '%s' does not have a body (loading deployment actor '%s')",
            ));
        }
        self.definitions.get_mut(&source_key).unwrap().unpack_to = Some(target.id.clone());
        if !target.mobile {
            return Err(invalid(
                "Actor '%s' is missing a mobile deployment component",
            ));
        }
        let source = &self.definitions[&source_key];
        if source.element != target.element {
            return Err(invalid(
                "Both or neither of the pack and unpack actors '%s' and '%s' must be elements",
            ));
        }
        let radius = target
            .soft_radius_bits
            .ok_or_else(|| invalid("target ObjectData+54 soft radius is unbound"))?;
        if !inside_soft_shape(source.packed_position_offset_bits, radius, pc)? {
            return Err(invalid(
                "Packed position offset must be inside actor '%s' soft shape",
            ));
        }
        self.definitions.get_mut(&target_key).unwrap().pack_to = Some(source_id);
        Ok(())
    }
}

///460669..693: sqrt(x*x+y*y) <= radius OR unordered. No intermediate f32
/// store; x87 multiplication, addition and sqrt use the explicit precision.
pub fn inside_soft_shape(offset: [u32; 2], radius: u32, pc: Precision) -> Result<bool> {
    let nan = |bits: u32| bits & 0x7fff_ffff > 0x7f80_0000;
    if nan(offset[0]) || nan(offset[1]) || nan(radius) {
        return Ok(true);
    }
    let infinite = |bits: u32| bits & 0x7fff_ffff == 0x7f80_0000;
    if radius == 0x7f80_0000 {
        return Ok(true);
    }
    if infinite(offset[0]) || infinite(offset[1]) || radius == 0xff80_0000 {
        return Ok(false);
    }
    let x = Binary::from_f32_bits(offset[0])?;
    let y = Binary::from_f32_bits(offset[1])?;
    let length = x.mul(&x, pc)?.add(&y.mul(&y, pc)?, pc)?.sqrt(pc)?;
    Ok(length.compare(&Binary::from_f32_bits(radius)?) != Ordering::Greater)
}
fn required(node: &ResolvedNode<'_>, key: &str, pc: Precision) -> Result<TypedValue> {
    node.field(key, pc)?
        .ok_or_else(|| invalid(format!("MobileDeployment field {key} missing")))
}
fn point(node: &ResolvedNode<'_>, key: &str, pc: Precision) -> Result<[u32; 2]> {
    match required(node, key, pc)? {
        TypedValue::Point { bits, .. } if bits.len() == 2 => Ok([bits[0], bits[1]]),
        _ => Err(invalid(format!("MobileDeployment {key} must be point2"))),
    }
}

impl ActorCatalog {
    /// Read-only projection of constructor-zero fields, MobileDeployment
    /// inputs of464BD0 and completed4605B0 links. Requires the native-order
    /// registry loader; generic from_files scan order is not authoritative.
    /// Existing native registry aliases select target identity. Owners are
    /// selected by registered source provenance, not re-looked up through an
    /// alias that might redirect their own IDS.
    pub fn native_mobile_deployment_data(
        &self,
        pc: Precision,
    ) -> Result<NativeMobileDeploymentData> {
        if self.native_data_registry("NameList").is_none() {
            return Err(invalid(
                "MobileDeployment projection requires native registration order",
            ));
        }
        let mut roots = Vec::new();
        for tag in crate::actors::native_load::THING_TAGS {
            roots.extend(self.resolved_definitions(tag)?);
        }
        let mut out = NativeMobileDeploymentData::default();
        let mut references = Vec::new();
        for actor in &self.actors {
            let category = actor
                .field("category")
                .and_then(|f| f.raw_text())
                .ok_or_else(|| invalid(format!("registered category missing for {}", actor.id)))?;
            if !CATEGORIES
                .iter()
                .any(|c| c.name.eq_ignore_ascii_case(category.trim()) && c.native_kind == 2)
            {
                continue;
            }
            let node = roots
                .iter()
                .find(|node| node.sources().first() == Some(&actor.source))
                .ok_or_else(|| {
                    invalid(format!("registered source unavailable for {}", actor.id))
                })?;
            let mobile = node.child("MobileDeploymentComponent")?;
            let mut entry = Definition {
                id: actor.id.clone(),
                body: node.child("BodyComponent")?.is_some(),
                mobile: mobile.is_some(),
                element: node.child("ElementComponent")?.is_some(),
                packed_position_offset_bits: [0, 0],
                pack_health_min_bits: 0,
                unpack_to: None,
                pack_to: None,
                soft_radius_bits: None,
                sources: node.sources(),
            };
            if let Some(mobile) = mobile {
                let reference = match required(&mobile, "unpack_to_actor_ids", pc)? {
                    TypedValue::String { value, .. } => value,
                    _ => return Err(invalid("unpack_to_actor_ids must be string")),
                };
                if reference.is_empty() {
                    entry.pack_health_min_bits =
                        required(&mobile, "pack_health_min", pc)?.fixed_bits()?;
                } else {
                    entry.packed_position_offset_bits =
                        point(&mobile, "packed_position_offset", pc)?;
                    references.push((actor.id.clone(), reference));
                }
            }
            if out
                .definitions
                .insert(actor.id.to_ascii_lowercase(), entry)
                .is_some()
            {
                return Err(invalid("duplicate registered mobile definition identity"));
            }
        }
        //473AD0 prepends each callback;475230 walks head first. No sorting
        //or filesystem inversion is used, and alias identity is canonicalized
        //only by the native Thing registry Find.
        for (source, reference) in references.into_iter().rev() {
            let target = self.find(&reference).map(|d| d.id.clone());
            if let Some(target) = target
                .as_ref()
                .and_then(|id| out.definitions.get(&id.to_ascii_lowercase()))
                && target.body
                && target.mobile
                && target.element == out.definition(&source)?.element
            {
                let id = target.id.to_ascii_lowercase();
                let radius = self.native_object_definition(&reference, pc)?.soft.words[1];
                out.definitions.get_mut(&id).unwrap().soft_radius_bits = Some(radius);
            }
            out.resolve_reference(&source, target.as_deref(), pc)?;
        }
        Ok(out)
    }
}
