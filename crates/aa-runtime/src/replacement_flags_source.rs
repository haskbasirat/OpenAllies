//! The unchanged fixed point of4BC7F0's Placeholder4BC5E0 flag writer.
//! This is an immutable definition projection, never a completed World/source-
//! loader certificate. A reached flag mutation still requires its actual owner.
use crate::{
    catalog::Catalog,
    definitions::Bank,
    skirmish_start::{substitution, Definitions, Substitution},
};
use aa_numeric::Precision;
use aa_rules::{
    native::CategoryDescriptor,
    native_source_requirements::NativeSourceRequirements,
    resources::{Provenance, SourceIdentity},
};
use std::collections::BTreeMap;
type Result<T> = std::result::Result<T, String>;

///Native4BD800's post-CV4BC7F0 suffix on the SAME source Bank. This
///materializes only the already-proved unchanged fixed-point projection;
///it does not certify unrelated source-loader passes or World readiness.
///Call before moving source storage into the resource worker, including a
///cold source whose saved records never happened to read a Placeholder.
pub fn complete_source_suffix(catalog:&Catalog<'_>)->Result<()>{
    catalog.require_source_owner()?;
    let mut bank=catalog.definition_bank.try_borrow_mut().map_err(|e|e.to_string())?;
    let source=bank.combat_source()?;
    source.completed_means()?;
    let pc=source.source_precision();
    let fields=catalog.actors.native_source_requirements(pc).map_err(|e|e.to_string())?;
    if bank.replacement_flags.is_none(){
        bank.replacement_flags=Some(State::bind(catalog,&mut bank,&fields)?);
    }
    //Reaching the suffix with a retained projection still validates SAME
    //source ownership. It never replaces a cache or repeats native mutations.
    let current=bank.replacement_flags.as_ref().ok_or("Placeholder suffix lost its storage")?;
    if current.sources!=catalog.actors.sources || current.precision!=pc{
        return Err("Placeholder suffix retained a different source/precision owner".into())
    }
    Ok(())
}

fn finite(bits: u32) -> Result<f32> {
    let value = f32::from_bits(bits);
    if value.is_finite() {
        Ok(value)
    } else {
        Err("replacement containment requires the reached finite source field".into())
    }
}
///Exact46D8F0 finite-field branches. `container` is replacement+38 and
///`item` is original+38. The native lower-kind recursive result is negated;
///the rect/circle branch calls444420 on ITEM, not on the container.
fn contains(container: &[u32; 6], item: &[u32; 6]) -> Result<bool> {
    if item[0] > container[0] {
        return Ok(!contains(item, container)?);
    }
    match container[0] {
        0 => Ok(true),
        1 => {
            if item[0] == 0 {
                Ok(true)
            } else {
                Ok(finite(container[1])? >= finite(item[1])?)
            }
        }
        2 | 3 => {
            if item[0] <= 1 {
                //444420(item) returns item.radius for these two kinds.
                let extent = finite(item[1])?;
                Ok(extent >= finite(item[1])?)
            } else {
                let x = finite(container[4])?;
                if finite(item[4])? > x {
                    return Ok(false);
                }
                let y = finite(container[5])?;
                Ok(finite(item[5])? <= y)
            }
        }
        _ => Ok(false),
    }
}
///ActorData ctor4624B0 installs A05540; its+3C is real4C9FA0(FALSE).
///It does NOT select the other mask just because the definition is an Actor.
const STANDARD_ACTOR_MASK: u32 = 0xa74c00;
fn fixed_pair(
    original: u32,
    replacement: u32,
    original_shape: &[u32; 6],
    replacement_shape: &[u32; 6],
    virtual_true: bool,
) -> Result<u32> {
    if !contains(replacement_shape, original_shape)? {
        return Err("4BC5E0 reached required containment diagnostic before flag comparison".into());
    }
    let mask = if virtual_true {
        0xa34c00
    } else {
        STANDARD_ACTOR_MASK
    };
    if original & mask != replacement & mask {
        return Err("4BC5E0 reached warning and mutable replacement-flag storage; unchanged source projection refused".into());
    }
    Ok(original)
}
#[derive(Debug)]
struct Bound {
    source: Provenance,
    category: CategoryDescriptor,
    flags: u32,
}
///Private construction records every actual Nation pair and excludes the
///other shared writers through the source-owned placement/Upgrade receipt.
///No Default/Clone/Deserialize or caller-supplied ordered pair array.
#[derive(Debug)]
pub struct State {
    sources: Vec<SourceIdentity>,
    precision: Precision,
    placeholders: BTreeMap<String, Bound>,
    pairs: usize,
}
impl State {
    pub fn bind(
        catalog: &Catalog<'_>,
        bank: &mut Bank,
        fields: &NativeSourceRequirements,
    ) -> Result<Self> {
        catalog.require_source_owner()?;
        let source = bank.combat_source()?;
        let pc = source.source_precision();
        if source.source_identities() != catalog.actors.sources.as_slice()
            || fields.receipt().sources != catalog.actors.sources
            || fields.source_precision() != pc
        {
            return Err("replacement flags source/Bank/field-writer universe or PC differs".into());
        }
        if !fields
            .receipt()
            .upgrade_census
            .explicit_instances
            .is_empty()
            || !fields
                .receipt()
                .writer_source_census
                .opaque_sources_without_field_names
                .is_empty()
        {
            return Err(
                "replacement flags require actual complete shared-writer/Upgrade census".into(),
            );
        }
        //This factory alone obtains the canonical historical pair projection;
        //public mutable Definitions supplied by a caller are not certificates.
        let defs = Definitions::bind(catalog.actors, bank, pc)?;
        if defs.things.len() != fields.receipt().registered_things {
            return Err("replacement flags complete Thing census differs".into());
        }
        for (&token, definition) in &defs.things {
            let entry = bank.entry(token)?;
            if entry.key.domain != "actordefinition"
                || !entry.key.name.eq_ignore_ascii_case(&definition.ids)
            {
                return Err("replacement flags canonical Bank identity differs".into());
            }
            let field = fields
                .definition(&definition.ids)
                .map_err(|e| e.to_string())?;
            //An already registered object is a pointer, not a fresh Find.
            //A later Translate of an unrelated original IDS must not redirect
            //this complete registration census to the replacement object.
            let actual = catalog
                .actors
                .actors
                .iter()
                .find(|actual| {
                    actual.id.eq_ignore_ascii_case(&definition.ids)
                        && &actual.source == field.token().source()
                })
                .ok_or("replacement original registration provenance absent")?;
            if field.token().source() != &actual.source
                || field.registry_index() != definition.category
                || field.native_kind() != definition.native_kind
            {
                return Err(
                    "replacement canonical registration/constructor provenance differs".into(),
                );
            }
        }
        //4BC7F0's two earlier lists mutate their RandomObject/RandomActor
        //receiver. Their actual native kinds18/19 cannot be category31 Actor.
        for key in defs.random.keys() {
            let def = &defs.things[key];
            if !matches!(def.native_kind, 18 | 19) || def.category == 31 {
                return Err("replacement random-writer receiver family is unbound".into());
            }
        }
        let nations = catalog
            .actors
            .native_data_registry("Nation")
            .ok_or("replacement Nation registry absent")?;
        let mut placeholders = BTreeMap::new();
        let mut pairs = 0;
        for (&token, definition) in &defs.things {
            if definition.category != 31 {
                continue;
            }
            if definition.native_kind != 2 {
                return Err("Placeholder actual ActorData constructor/vtable is unbound".into());
            }
            catalog.require_untranslated_actor_identity(&definition.ids)?;
            let category = catalog
                .actors
                .native_category(&definition.ids)
                .map_err(|e| e.to_string())?;
            let original = catalog
                .actors
                .native_final_flags(&definition.ids, pc)
                .map_err(|e| e.to_string())?;
            let original_shape = catalog
                .actors
                .native_object_definition(&definition.ids, pc)
                .map_err(|e| e.to_string())?
                .blocking
                .words;
            for nation in &nations.entries {
                catalog
                    .actors
                    .native_data_node(nation)
                    .map_err(|e| e.to_string())?;
                let rows = defs
                    .nations
                    .get(&nation.ids.to_ascii_lowercase())
                    .ok_or("actual Nation replacement list absent")?;
                let target = match substitution(rows, token) {
                    Substitution::Present(Some(target)) => target,
                    //Native473370 returns NULL and reaches DataCheckingStrict.
                    //Do not fabricate that current UVar/diagnostic service.
                    _ => return Err(
                        "4BC7F0 missing/null replacement requires actual DataCheckingStrict owner"
                            .into(),
                    ),
                };
                let replacement = &defs.things[&target];
                let target_flags = catalog
                    .actors
                    .native_final_flags(&replacement.ids, pc)
                    .map_err(|e| e.to_string())?;
                let target_shape = catalog
                    .actors
                    .native_object_definition(&replacement.ids, pc)
                    .map_err(|e| e.to_string())?
                    .blocking
                    .words;
                fixed_pair(
                    original,
                    target_flags,
                    &original_shape,
                    &target_shape,
                    false,
                )?;
                pairs += 1;
            }
            placeholders.insert(
                definition.ids.to_ascii_lowercase(),
                Bound {
                    source: catalog
                        .actors
                        .find(&definition.ids)
                        .ok_or("Placeholder source disappeared")?
                        .source
                        .clone(),
                    category,
                    flags: original,
                },
            );
        }
        Ok(Self {
            sources: catalog.actors.sources.clone(),
            precision: pc,
            placeholders,
            pairs,
        })
    }
    pub fn precision(&self) -> Precision {
        self.precision
    }
    pub fn verified_pairs(&self) -> usize {
        self.pairs
    }
    pub fn placeholder_count(&self) -> usize {
        self.placeholders.len()
    }
    pub fn flags(&self, catalog: &Catalog<'_>, bank: &Bank, id: &str) -> Result<u32> {
        catalog.require_source_owner()?;
        let source = bank.combat_source()?;
        if source.source_precision() != self.precision
            || source.source_identities() != self.sources.as_slice()
            || catalog.actors.sources != self.sources
        {
            return Err("replacement fixed point belongs to another source universe/PC".into());
        }
        catalog.require_untranslated_actor_identity(id)?;
        let bound = self
            .placeholders
            .get(&id.to_ascii_lowercase())
            .ok_or("definition is outside verified Placeholder fixed point")?;
        let actual = catalog
            .actors
            .find(id)
            .ok_or("verified Placeholder source absent")?;
        if actual.source != bound.source
            || catalog
                .actors
                .native_category(id)
                .map_err(|e| e.to_string())?
                != bound.category
            || catalog
                .actors
                .native_final_flags(id, self.precision)
                .map_err(|e| e.to_string())?
                != bound.flags
        {
            return Err("verified Placeholder source/category/base flags changed".into());
        }
        Ok(bound.flags)
    }
}

#[cfg(test)]
#[path = "replacement_flags_source_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "replacement_flags_transfer_tests.rs"]
mod transfer_tests;
