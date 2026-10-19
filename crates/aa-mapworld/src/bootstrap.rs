//! The executable beginning of native indexed-object restoration.
//!
//! A caller must resolve a native definition/class/flag word first. This module
//! creates the actual proved load shell, registers its saved ID without reseeding,
//! then applies the decoded GThing body. Object/actor remainder stays pending;
//! these entries are deliberately not exposed as complete live game actors.
use crate::inventory::Placement;
use aa_actors::{
    ActorId, IdentityRegistry,
    loading::{LoadKind, LoadShell, construct_shell},
    registry::RegistryError,
};
use serde::Serialize;

#[derive(Clone, Debug)]
pub struct ResolvedDefinition<D> {
    pub definition: D,
    /// Evaluated native descriptor class, independently of the serialized class.
    pub native_class: u32,
    pub flag_word: u32,
}
#[derive(Clone, Debug, Serialize)]
pub struct DefinitionProof {
    pub id: String,
    pub source: aa_rules::resources::Provenance,
    pub template_chain: Vec<aa_rules::resources::Provenance>,
    pub category: aa_rules::native::CategoryDescriptor,
    pub base_flag_inputs: Vec<aa_rules::native::TypedValue>,
}

/// Definition class and final flags come from the typed native evaluator, never
/// from the serialized class, an ID naming convention or inspector text.
pub fn resolve_definition(
    catalog: &aa_rules::actors::ActorCatalog,
    id_utf16: &[u16],
    precision: aa_numeric::Precision,
) -> aa_rules::native::Result<ResolvedDefinition<DefinitionProof>> {
    use aa_rules::native::Error;
    let id = String::from_utf16(id_utf16)
        .map_err(|_| Error::Unsupported("definition ID has unpaired UTF16 units".into()))?;
    resolve_thing(catalog, aa_rules::actors::ThingRef::find(&id), precision)
}
/// The same descriptor for an explicitly selected allocation. With
/// [`aa_rules::actors::ThingRef::registered`] this is the allocation 4BDC50
/// loads (vf0 LoadData) even when a later Thing translation shadows its IDS.
pub fn resolve_thing(
    catalog: &aa_rules::actors::ActorCatalog,
    id: aa_rules::actors::ThingRef<'_>,
    precision: aa_numeric::Precision,
) -> aa_rules::native::Result<ResolvedDefinition<DefinitionProof>> {
    use aa_rules::native::Error;
    let source = catalog
        .select(id)
        .ok_or_else(|| Error::Unsupported(format!("missing definition {id}")))?;
    let category = catalog.native_category(id)?;
    let flag_word = catalog.native_final_flags(id, precision)?;
    // KINGDOM also uses ActorData (native_kind2), although its runtime class is4.
    let (_, base_flag_inputs) =
        catalog.native_base_flags(id, category.native_kind == 2, precision)?;
    Ok(ResolvedDefinition {
        native_class: category.runtime_class,
        flag_word,
        definition: DefinitionProof {
            id: source.id.clone(),
            source: source.source.clone(),
            template_chain: source.template_chain.clone(),
            category,
            base_flag_inputs,
        },
    })
}
#[derive(Clone, Debug)]
pub struct LoadedBase<D> {
    pub shell: LoadShell<D>,
    pub source_record: Placement,
    pub remaining: Vec<PendingStep>,
    pub long_name_cleared: bool,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum PendingStep {
    ObjectDefinitionScaleAndVariant,
    ObjectNestedState,
    ActorOwnerAndTransientState,
    ActorComponentsAndSubtypeTail,
    PostLoadWorldPublication,
}
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("definition class {definition} differs from serialized class {serialized}")]
    ClassMismatch { definition: u32, serialized: u32 },
    #[error("load constructor for native class {0} is not implemented")]
    UnsupportedConstructor(u32),
    #[error(transparent)]
    Identity(#[from] RegistryError),
}

/// Register and restore only the completed GThing portion, preserving native
/// source order when called in index order. Unlike a fresh create, this never
/// draws RNG, normalizes direction, selects defaults or reseeds a saved ID.
/// The higher-level loader must load teams/kingdoms/required objects first and
/// must finish pending steps before exposing these entries to gameplay.
pub fn restore_base<D>(
    registry: &mut IdentityRegistry<LoadedBase<D>>,
    record: Placement,
    definition: ResolvedDefinition<D>,
) -> Result<ActorId, Error> {
    if definition.native_class != record.class_code {
        return Err(Error::ClassMismatch {
            definition: definition.native_class,
            serialized: record.class_code,
        });
    }
    let kind = match record.class_code {
        1 => LoadKind::Thing,
        2 => LoadKind::Object,
        3 => LoadKind::Actor,
        other => return Err(Error::UnsupportedConstructor(other)),
    };
    let id = ActorId::from_raw(record.serialized_id);
    let mut remaining = Vec::new();
    if kind != LoadKind::Thing {
        remaining.extend([
            PendingStep::ObjectDefinitionScaleAndVariant,
            PendingStep::ObjectNestedState,
        ]);
    }
    if kind == LoadKind::Actor {
        remaining.extend([
            PendingStep::ActorOwnerAndTransientState,
            PendingStep::ActorComponentsAndSubtypeTail,
        ]);
    }
    remaining.push(PendingStep::PostLoadWorldPublication);
    registry.register_saved(
        id,
        kind == LoadKind::Actor,
        LoadedBase {
            shell: construct_shell(kind, definition.definition),
            source_record: record,
            remaining,
            long_name_cleared: false,
        },
    )?;
    // Native base construction has registered the ID before virtual Load. Do
    // not remove/reseed it if later derived restoration fails.
    let loaded = registry.value_mut(id).ok_or(RegistryError::Stale)?;
    let input = &loaded.source_record.thing;
    let state = &mut loaded.shell.thing;
    state.flags = input.runtime_flags(state.flags, definition.flag_word);
    state.word_1c = Some(input.word_1c);
    state.position_bits = input.position_bits;
    state.facing_bits = input.facing_bits;
    loaded.long_name_cleared = input.name_utf16.len() > 1000;
    state.name_utf16 = if loaded.long_name_cleared {
        Vec::new()
    } else {
        input.name_utf16.clone()
    };
    Ok(id)
}
