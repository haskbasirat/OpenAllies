//! Bounded map-start source/selection services. These do not create replacement
//! actors, drain4FC4A0 or authorize4FDCE0 mission/publication success.
use crate::{
    actor_world::{ActorStore, Source, SpecialSource},
    definitions::Bank,
};
use aa_actors::ActorId;
use aa_numeric::{Binary, Precision};
use aa_rules::{
    actors::{ActorCatalog, ResolvedNode},
    native::TypedValue,
};
use aa_simulation::NativeRng;
use serde::{Deserialize, Serialize};
use std::{cmp::Ordering, collections::BTreeMap};

type Result<T> = std::result::Result<T, String>;
fn err(e: impl std::fmt::Display) -> String {
    e.to_string()
}
fn text(node: &ResolvedNode<'_>, field: &str, pc: Precision) -> Result<String> {
    match node.field(field, pc).map_err(err)? {
        Some(TypedValue::String { value, .. }) => Ok(value),
        _ => Err(format!("native {field} requires assigned string")),
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WeightedEntry {
    pub definition: Option<u32>,
    pub weight_bits: u32,
}
/// RandomData table in native head order. Its total is an independently rounded
/// field, not recomputed from a sorted/reversed reduction at draw time.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct WeightedTable {
    pub entries: Vec<WeightedEntry>,
    pub total_bits: u32,
}
impl WeightedTable {
    ///4B6BA0: finite weights<=0 do nothing; positive weights prepend and add
    /// to the existing total with a binary32 store. NaN/Inf are explicit limits.
    pub fn add(&mut self, definition: Option<u32>, weight_bits: u32, pc: Precision) -> Result<()> {
        let weight = Binary::from_f32_bits(weight_bits).map_err(err)?;
        if weight.compare(&Binary::from_u32(0)) != Ordering::Greater {
            return Ok(());
        }
        let total = weight
            .add(&Binary::from_f32_bits(self.total_bits).map_err(err)?, pc)
            .map_err(err)?
            .store_f32()
            .map_err(err)?;
        self.entries.insert(
            0,
            WeightedEntry {
                definition: definition.filter(|&id| id != 0),
                weight_bits,
            },
        );
        self.total_bits = total;
        Ok(())
    }
    ///4B6FF0: empty consumes no RNG; otherwise one85D840 draw and unrounded
    /// x87 multiply/subtracts. Exact cumulative boundaries advance to next row.
    /// Walking off the native list becomes an error after the same RNG draw.
    pub fn choose(&self, rng: &mut NativeRng, pc: Precision) -> Result<Option<u32>> {
        if self.entries.is_empty() {
            return Ok(None);
        }
        let raw = rng.next_u32();
        let mut number = Binary::from_i32(raw as i32);
        if raw & 0x8000_0000 != 0 {
            number = number
                .add(
                    &Binary::from_f64_bits(0x41f0_0000_0000_0000).map_err(err)?,
                    pc,
                )
                .map_err(err)?;
        }
        let mut remaining = number
            .mul(
                &Binary::from_f64_bits(0x3df0_0000_0010_0000).map_err(err)?,
                pc,
            )
            .map_err(err)?
            .mul(&Binary::from_f32_bits(self.total_bits).map_err(err)?, pc)
            .map_err(err)?;
        for entry in &self.entries {
            remaining = remaining
                .sub(&Binary::from_f32_bits(entry.weight_bits).map_err(err)?, pc)
                .map_err(err)?;
            if remaining.compare(&Binary::from_u32(0)) == Ordering::Less {
                return Ok(entry.definition.filter(|&id| id != 0));
            }
        }
        Err("4B6FF0 selection ran beyond native weighted list; RNG draw retained".into())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubstitutionEntry {
    pub placeholder: u32,
    pub replacement: Option<u32>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Substitution {
    Missing,
    Present(Option<u32>),
}
///473370: pointer equality in head-first list, including duplicate/null rows.
pub fn substitution(entries: &[SubstitutionEntry], placeholder: u32) -> Substitution {
    entries
        .iter()
        .find(|e| e.placeholder == placeholder)
        .map_or(Substitution::Missing, |e| {
            Substitution::Present(e.replacement)
        })
}

#[derive(Clone, Debug, Serialize)]
pub struct Definition {
    pub ids: String,
    pub category: u32,
    pub native_kind: u32,
    pub runtime_class: u32,
}
#[derive(Clone, Debug, Default, Serialize)]
pub struct Definitions {
    pub things: BTreeMap<u32, Definition>,
    /// Keys are canonical lowercase Nation IDS; values retain48C050 prepend order.
    pub nations: BTreeMap<String, Vec<SubstitutionEntry>>,
    pub random: BTreeMap<u32, WeightedTable>,
}
fn thing(source: &ActorCatalog, bank: &mut Bank, name: &str) -> Result<u32> {
    let found = source
        .find(name)
        .ok_or_else(|| format!("unknown ThingData {name}"))?;
    bank.intern("ActorDefinition", &found.id)
}
fn object_definition(definition: &Definition) -> Result<()> {
    // The bounded producer covers the recovered ObjectData/ActorData families,
    // not arbitrary ThingData (4BDBE0 also requires virtual+30).
    if matches!(definition.native_kind, 1 | 2 | 18 | 19 | 20 | 21) {
        Ok(())
    } else {
        Err(format!(
            "473370 source requires recovered ObjectData family: {}",
            definition.ids
        ))
    }
}
fn source_order(
    source: &ActorCatalog,
    p: &aa_rules::resources::Provenance,
) -> Result<(usize, usize)> {
    let file = source
        .sources
        .iter()
        .position(|s| s.path.eq_ignore_ascii_case(&p.path) && s.sha256 == p.sha256)
        .ok_or("native source file order unavailable")?;
    Ok((file, p.span.start))
}
fn immediate_thing(
    source: &ActorCatalog,
    bank: &mut Bank,
    name: &str,
    caller: &aa_rules::resources::Provenance,
    translated: &[String],
) -> Result<u32> {
    // A final alias map cannot prove what427BD0 saw midway through LoadData.
    // Refuse such references until a historical registry is supplied.
    if translated.iter().any(|old| old.eq_ignore_ascii_case(name)) {
        return Err(format!(
            "immediate Thing reference {name} needs translation-at-load-time binding"
        ));
    }
    let found = source
        .find(name)
        .ok_or_else(|| format!("unknown immediate ThingData {name}"))?;
    if source_order(source, &found.source)? > source_order(source, caller)? {
        return Err(format!(
            "ThingData {name} was not registered at this source LoadData call"
        ));
    }
    bank.intern("ActorDefinition", &found.id)
}
impl Definitions {
    /// Fresh canonical source projection. Existing native-order catalog proves
    /// registration order; this binds only the48C050 pairs and49F075/4688D6
    /// random contribution leaves, not all Nation/Actor LoadData side effects.
    pub fn bind(source: &ActorCatalog, bank: &mut Bank, pc: Precision) -> Result<Self> {
        let mut out = Self::default();
        let mut translated = Vec::new();
        for node in source.resolved_definitions("Translate").map_err(err)? {
            let manager = text(&node, "node", pc)?;
            if aa_rules::actors::native_load::THING_TAGS
                .iter()
                .any(|tag| manager.eq_ignore_ascii_case(tag))
            {
                translated.push(text(&node, "ids_old", pc)?);
            }
        }
        // These are registered native pointers, not lookup strings. Match the
        // exact authored provenance so a canonical IDS translation cannot
        // redirect its existing definition to a different object.
        let mut nodes = BTreeMap::new();
        for tag in aa_rules::actors::native_load::THING_TAGS {
            for node in source.resolved_definitions(tag).map_err(err)? {
                let origin = node
                    .sources()
                    .into_iter()
                    .next()
                    .ok_or("Thing source provenance absent")?;
                let key = (
                    origin.path,
                    origin.sha256,
                    origin.span.start,
                    origin.span.end,
                );
                if nodes.insert(key, node).is_some() {
                    return Err("Thing provenance is ambiguous".into());
                }
            }
        }
        for actor in &source.actors {
            let p = &actor.source;
            let node = nodes
                .get(&(p.path.clone(), p.sha256.clone(), p.span.start, p.span.end))
                .ok_or_else(|| format!("registered Thing source unavailable: {}", actor.id))?;
            let category = text(node, "category", pc)?;
            let cat = aa_rules::native::CATEGORIES
                .iter()
                .find(|c| c.name.eq_ignore_ascii_case(&category))
                .ok_or("unrecognized registered Thing category")?;
            let token = bank.intern("ActorDefinition", &actor.id)?;
            out.things.insert(
                token,
                Definition {
                    ids: actor.id.clone(),
                    category: cat.registry_index,
                    native_kind: cat.native_kind,
                    runtime_class: cat.runtime_class,
                },
            );
            if matches!(cat.native_kind, 18 | 19) {
                out.random.insert(token, WeightedTable::default());
            }
        }
        let nation_registry = source
            .native_data_registry("Nation")
            .ok_or("native Nation registry unavailable")?;
        for entry in &nation_registry.entries {
            let node = source.native_data_node(entry).map_err(err)?;
            let caller = node
                .sources()
                .into_iter()
                .next()
                .ok_or("Nation source provenance absent")?;
            let mut rows = Vec::new();
            for row in node.children("Placeholder").map_err(err)? {
                let placeholder = text(&row, "placeholder_IDS", pc)?;
                let replacement = text(&row, "thing_IDS", pc)?;
                let placeholder =
                    immediate_thing(source, bank, &placeholder, &caller, &translated)?;
                object_definition(&out.things[&placeholder])?;
                let replacement = if replacement.is_empty() {
                    None
                } else {
                    let target = thing(source, bank, &replacement)?;
                    object_definition(&out.things[&target])?;
                    Some(target)
                };
                rows.push(SubstitutionEntry {
                    placeholder,
                    replacement,
                });
            }
            rows.reverse();
            out.nations.insert(entry.ids.to_ascii_lowercase(), rows);
        }
        // LoadData visits the Thing registry in file/root order. Object base
        // contributions precede Actor contributions for each definition.
        for actor in &source.actors {
            let token = bank.intern("ActorDefinition", &actor.id)?;
            let p = &actor.source;
            let node = nodes
                .get(&(p.path.clone(), p.sha256.clone(), p.span.start, p.span.end))
                .ok_or("registered Thing source unavailable")?;
            for (label, field, kind) in [
                ("RandomObject", "random_object_ids", 18),
                ("RandomActor", "random_actor_ids", 19),
            ] {
                for row in node.children(label).map_err(err)? {
                    let contributor = out.things[&token].native_kind;
                    if (kind == 19 && !matches!(contributor, 2 | 19))
                        || (kind == 18 && !matches!(contributor, 1 | 2 | 18 | 19 | 20 | 21))
                    {
                        return Err(format!(
                            "{label} contribution on unbound source class {contributor}"
                        ));
                    }
                    let target = immediate_thing(
                        source,
                        bank,
                        &text(&row, field, pc)?,
                        &actor.source,
                        &translated,
                    )?;
                    if out.things[&target].native_kind != kind {
                        return Err(format!("{label} target has wrong native data class"));
                    }
                    let weight = row
                        .field("weight", pc)
                        .map_err(err)?
                        .ok_or("random weight missing")?
                        .fixed_bits()
                        .map_err(err)?;
                    out.random
                        .get_mut(&target)
                        .ok_or("random table owner missing")?
                        .add(Some(token), weight, pc)?;
                }
            }
        }
        Ok(out)
    }
    /// Read actual ordinary owner+E8 and Kingdom+234; no nation is invented.
    /// Missing Nation is an explicit error because473370 would dereference it.
    pub fn placeholder_for_actor(
        &self,
        actors: &ActorStore,
        actor: ActorId,
        definition: u32,
    ) -> Result<Substitution> {
        let object_key = actors.key(actor).ok_or("placeholder GTID stale")?;
        let actual = &actors.object(object_key).map_err(err)?.definition.proof.id;
        let expected = &self
            .things
            .get(&definition)
            .ok_or("placeholder definition token unknown")?
            .ids;
        if !actual.eq_ignore_ascii_case(expected) {
            return Err("placeholder token does not identify actual ActorData owner".into());
        }
        let owner = actors
            .ordinary_owner(actor)
            .map_err(err)?
            .ok_or("placeholder has no owner")?;
        let key = actors.key(owner).ok_or("placeholder owner GTID stale")?;
        let entity = actors.entity(key).map_err(err)?;
        let Source::Special(s) = &entity.source else {
            return Err("placeholder owner is not Kingdom".into());
        };
        if !matches!(s.source, SpecialSource::Kingdom(_)) {
            return Err("placeholder owner is not Kingdom".into());
        }
        let state = s.derived.as_ref().ok_or("Kingdom state absent")?;
        let nation = state
            .references
            .get(&0x234)
            .and_then(Option::as_ref)
            .ok_or("473370 requires actual Kingdom+234 Nation")?;
        let rows = self
            .nations
            .get(&nation.id.to_ascii_lowercase())
            .ok_or("canonical Nation source unavailable")?;
        Ok(substitution(rows, definition))
    }
}

/// This boundary cannot certify map preparation until replacement capture,
/// old-object destruction, creation, property transfer and publication bind.
pub fn require_replacement_drain() -> Result<()> {
    Err("4FC4A0/4FBA50 replacement drain requires owned destruction, fresh creation, property transfer and publication".into())
}
