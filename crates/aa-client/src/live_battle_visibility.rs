//! Concrete read-only native visibility leaves and immutable KingdomColor data.
//! These getters do not substitute for remembered-model selection4EAB20,
//!4EACC0 or a live local player's765E10 VWorldState owner.
use super::same_proof;
use aa_actors::{ActorId, components::ComponentSlot, loading::LoadKind, owned::Component};
use aa_mapworld::special_state::SpecialShell;
use aa_numeric::Precision;
use aa_rules::{actors::ActorCatalog, native::TypedValue, resources::SourceIdentity};
use aa_runtime::{
    actor_world::{ActorStore, RuntimeObject, Source, SpecialSource},
    restoration::Staged,
    spatial_world::SpatialWorld,
};
use anyhow::{Context, Result, bail, ensure};
use std::collections::{BTreeMap, BTreeSet};

/// Current application owners, read at the native boundary. Implementations
/// must borrow real player/session/variable/thread owners; none is defaulted.
pub trait CurrentView {
    fn kingdom(&self) -> Result<Option<ActorId>>;
    fn fog_enabled(&self) -> Result<bool>;
    fn current_thread_is_simulation(&self) -> Result<bool>;
    fn application_state(&self) -> Result<u32>;
    ///4F0930's first read: actual [[B39AB0]+1BC] editor option.
    fn editor_option_enabled(&self) -> Result<bool>;
    ///4F0930's second read, reached only when the option is false.
    fn world_editor_override(&self) -> Result<bool>;
}

fn object(actors: &ActorStore, id: ActorId) -> Result<&RuntimeObject> {
    ensure!(!id.is_null(), "native visual actor reference is null");
    let key = actors
        .key(id)
        .context("native visual actor generation is stale")?;
    let value = actors.object(key)?;
    ensure!(
        same_proof(&value.thing.definition, &value.definition.proof),
        "native visual Thing/definition source differs"
    );
    let layout = match value.definition.native_class {
        2 | 6 | 7 | 9 => {
            value.kind == LoadKind::Object && value.object.is_some() && value.actor.is_none()
        }
        3 | 4 => value.kind == LoadKind::Actor && value.object.is_some() && value.actor.is_some(),
        _ => false,
    };
    ensure!(
        layout && value.definition.native_class == value.definition.proof.category.runtime_class,
        "native visibility virtual table/storage class is unsupported"
    );
    Ok(value)
}

fn kingdom(actors: &ActorStore, id: ActorId) -> Result<&SpecialShell> {
    let value = object(actors, id)?;
    ensure!(
        value.definition.native_class == 4,
        "native view reference is not a Kingdom"
    );
    let entity = actors.entity(
        actors
            .key(id)
            .context("native Kingdom generation is stale")?,
    )?;
    let Source::Special(saved) = &entity.source else {
        bail!("native Kingdom derived owner is absent")
    };
    ensure!(
        matches!(saved.source, SpecialSource::Kingdom(_)),
        "native Kingdom derived class differs"
    );
    saved
        .derived
        .as_ref()
        .context("native Kingdom derived allocation is absent")
}

fn word(kingdom: &SpecialShell, offset: u32) -> Result<u32> {
    kingdom
        .words
        .get(&offset)
        .copied()
        .with_context(|| format!("native Kingdom+{offset:X} is unavailable"))
}

/// Actual4EEF70 and class-specific+70/+74 getters over the same ActorStore.
/// Deep member/Element traversal reads current typed components in native order.
pub struct NativeVisibility<'a, V: CurrentView + ?Sized> {
    view: &'a V,
}
impl<'a, V: CurrentView + ?Sized> NativeVisibility<'a, V> {
    pub fn new(view: &'a V) -> Self {
        Self { view }
    }

    ///4EEF70. The current view Kingdom is the actual caller argument; dead,
    ///thread and Session/editor gates retain their native lazy ordering.
    pub fn visible(
        &self,
        actors: &ActorStore,
        spatial: &SpatialWorld,
        actor: ActorId,
        deep: bool,
    ) -> Result<bool> {
        let target = self.view.kingdom()?;
        object(actors, actor)?;
        let key = actors
            .key(actor)
            .context("native visual actor generation is stale")?;
        let flags = actors.placement(key, spatial)?.flags;
        if flags & (1 << 7) != 0 {
            return Ok(false);
        }
        if self.view.current_thread_is_simulation()? {
            if self.view.application_state()? == 3
                || self.view.editor_option_enabled()?
                || self.view.world_editor_override()?
            {
                return Ok(true);
            }
        }
        self.seen(actors, actor, target, deep, false, &mut BTreeSet::new())
    }

    /// Direct+70 (4EA1D0/4D3B80) or+74 (4EA0D0/4D3C50). This deliberately
    /// accepts the caller's resolved Kingdom, rather than looking it up again
    /// while following members and changing the native argument mid-call.
    pub fn visible_to(
        &self,
        actors: &ActorStore,
        actor: ActorId,
        target: Option<ActorId>,
        deep: bool,
    ) -> Result<bool> {
        self.seen(actors, actor, target, deep, false, &mut BTreeSet::new())
    }
    pub fn explored_by(
        &self,
        actors: &ActorStore,
        actor: ActorId,
        target: Option<ActorId>,
        deep: bool,
    ) -> Result<bool> {
        self.seen(actors, actor, target, deep, true, &mut BTreeSet::new())
    }

    fn shared(
        &self,
        actors: &ActorStore,
        actor: &RuntimeObject,
        target: Option<ActorId>,
        explored: bool,
    ) -> Result<bool> {
        if !explored && !self.view.fog_enabled()? {
            return Ok(true);
        }
        let Some(target) = target else {
            return Ok(true);
        };
        let target = kingdom(actors, target)?;
        if word(target, 0x190)? != 0 {
            return Ok(true);
        }
        let shared = word(target, 0x228)?;
        let detection = actor
            .object
            .as_ref()
            .context("native Object+54 is absent")?
            .words_50_to_54[1];
        Ok(shared & detection & if explored { 0xffff } else { 0xffff0000 } != 0)
    }

    fn seen(
        &self,
        actors: &ActorStore,
        id: ActorId,
        target: Option<ActorId>,
        deep: bool,
        explored: bool,
        active: &mut BTreeSet<ActorId>,
    ) -> Result<bool> {
        let actor = object(actors, id)?;
        let is_actor = matches!(actor.definition.native_class, 3 | 4);
        if is_actor {
            // Actor+C8 is component slot26 (Flare), not Transport at+C0.
            //4D3C50 returns false without calling the positive component;
            //4D3B80 reaches mandatory5733A0 before reading the fog variable.
            if actor.components()?.present(ComponentSlot::Flare) {
                if explored {
                    return Ok(false);
                }
                bail!("native visibility requires actual Flare5733A0 owner/source services");
            }
        }
        if self.shared(actors, actor, target, explored)? {
            return Ok(true);
        }
        if !is_actor || !deep {
            return Ok(false);
        }
        ensure!(
            active.insert(id),
            "native deep visibility has a cyclic Element chain"
        );
        let result = (|| {
            if let Some(component) = actor.components()?.get(ComponentSlot::Organization) {
                let Component::Organization(org) = component else {
                    bail!("native Organization payload differs")
                };
                ensure!(org.header.owner == id, "native Organization owner differs");
                for member in &org.members {
                    if !member.is_null()
                        && self.seen(actors, *member, target, false, explored, active)?
                    {
                        return Ok(true);
                    }
                }
                return Ok(false);
            }
            if let Some(component) = actor.components()?.get(ComponentSlot::Element) {
                let Component::Element(element) = component else {
                    bail!("native Element payload differs")
                };
                ensure!(element.header.owner == id, "native Element owner differs");
                if let Some(parent) = element.organization {
                    //4D3C3B passes true;4D3D0B in explored path passes false.
                    return self.seen(actors, parent, target, !explored, explored, active);
                }
            }
            Ok(false)
        })();
        active.remove(&id);
        result
    }
}

/// Immutable completed source definitions. A Kingdom's mutable+1E8 reference
/// stays in its actual derived owner and is reread when its color is requested.
pub struct NativeColors {
    sources: Vec<SourceIdentity>,
    precision: Precision,
    colors: BTreeMap<String, (u32, [u32; 3])>,
}
impl NativeColors {
    pub fn bind(staged: &Staged, catalog: &ActorCatalog) -> Result<Self> {
        Self::bind_source(&staged.definitions,catalog)
    }
    pub fn bind_live_world(world:&aa_runtime::live_world_frame::World<'_>,catalog:&ActorCatalog)->Result<Self> {
        Self::bind_source(world.presentation_parts().map_err(anyhow::Error::msg)?.definitions,catalog)
    }
    fn bind_source(bank:&aa_runtime::definitions::Bank,catalog:&ActorCatalog)->Result<Self> {
        let source = bank
            .combat_source()
            .map_err(anyhow::Error::msg)?;
        ensure!(
            source.source_identities() == catalog.sources,
            "native color source owner differs from the completed Bank"
        );
        let precision = source.source_precision();
        // Uses private resolved source-set validation, never public diagnostics
        // or a caller-created list as evidence that this is a source owner.
        catalog.native_source_requirements(precision)?;
        Self::from_source(catalog, precision)
    }

    fn from_source(catalog: &ActorCatalog, precision: Precision) -> Result<Self> {
        let registry = catalog
            .native_data_registry("KingdomColor")
            .context("native KingdomColor registry is absent")?;
        let mut colors = BTreeMap::new();
        for (index, entry) in registry.entries.iter().enumerate() {
            let node = catalog.native_data_node(entry)?;
            let id = node
                .field("IDS", precision)?
                .context("native Color IDS is absent")?;
            let TypedValue::String {
                value: id, source, ..
            } = id
            else {
                bail!("native Color IDS type differs")
            };
            ensure!(
                id.eq_ignore_ascii_case(&entry.ids)
                    && source.path == entry.source.path
                    && source.sha256 == entry.source.sha256
                    && source.span.start >= entry.source.span.start
                    && source.span.end <= entry.source.span.end,
                "native Color registry and private source node differ"
            );
            let value = node
                .field("color", precision)?
                .context("native Color point3 is absent")?;
            let TypedValue::Point { bits, .. } = value else {
                bail!("native Color field is not point3")
            };
            let bits: [u32; 3] = bits
                .try_into()
                .map_err(|_| anyhow::anyhow!("native Color requires exactly three words"))?;
            ensure!(
                colors
                    .insert(id.to_ascii_lowercase(), (u32::try_from(index)?, bits))
                    .is_none(),
                "native canonical Color identity is ambiguous"
            );
        }
        Ok(Self {
            sources: catalog.sources.clone(),
            precision,
            colors,
        })
    }
    /// Color+18/+1C/+20 after actual47E0F0 GetPoint3. Null is an explicit
    /// native reference result, not an invented white/global replacement color.
    pub fn kingdom_color(&self, staged: &Staged, id: ActorId) -> Result<Option<[f32; 3]>> {
        self.source_color(&staged.definitions,&staged.actors,id)
    }
    pub fn kingdom_color_live_world(&self,world:&aa_runtime::live_world_frame::World<'_>,id:ActorId)->Result<Option<[f32;3]>> {
        let p=world.presentation_parts().map_err(anyhow::Error::msg)?;
        self.source_color(p.definitions,p.actors,id)
    }
    fn source_color(&self,bank:&aa_runtime::definitions::Bank,actors:&ActorStore,id:ActorId)->Result<Option<[f32;3]>> {
        let source = bank
            .combat_source()
            .map_err(anyhow::Error::msg)?;
        ensure!(
            source.source_identities() == self.sources
                && source.source_precision() == self.precision,
            "native Color source receipt changed"
        );
        self.reference_color(actors, id)
    }

    fn reference_color(&self, actors: &ActorStore, id: ActorId) -> Result<Option<[f32; 3]>> {
        let derived = kingdom(actors, id)?;
        let reference = derived
            .references
            .get(&0x1e8)
            .context("native Kingdom Color reference is absent")?;
        let Some(reference) = reference else {
            return Ok(None);
        };
        let (index, color) = self
            .colors
            .get(&reference.id.to_ascii_lowercase())
            .context("native canonical Color reference is unbound")?;
        ensure!(
            reference.index.is_none_or(|i| i == *index),
            "native Color ordinal differs from its source allocation"
        );
        Ok(Some(color.map(f32::from_bits)))
    }
}

#[cfg(test)]
#[path = "live_battle_visibility_tests.rs"]
mod tests;
