//! Kingdom resource state preparation. Native Init resets only the three flow
//! arrays, preserving saved stockpiles. This stage is separate from the still
//! required recruitment, SAI, owner lists and whole-world publication stages.
use aa_economy::{
    Catalog, Economy, F32Bits, Limits, ResourceDefinition, ResourceKind, State, X87Precision,
};
use aa_mapworld::special_state::{NativeVectorState, SpecialShell};
use aa_numeric::Precision;
use aa_rules::{
    actors::ActorCatalog,
    resources::{ResourceCatalog, ResourceCategory},
};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Kingdom resource source: {0}")]
    Source(String),
    #[error("Kingdom resource allocation failed")]
    Allocation,
    #[error(transparent)]
    Economy(#[from] aa_economy::EconomyError),
}
pub type Result<T> = std::result::Result<T, Error>;

fn zero_index(vector: &mut NativeVectorState, index: usize) -> Result<()> {
    if index >= vector.values.len() {
        let new_count = u32::try_from(index + 1).map_err(|_| Error::Allocation)?;
        let old_capacity = vector.capacity_hint;
        let capacity = if new_count > old_capacity {
            new_count.max(old_capacity.checked_mul(2).ok_or(Error::Allocation)?)
        } else {
            old_capacity
        };
        vector
            .values
            .try_reserve_exact((capacity as usize).saturating_sub(vector.values.len()))
            .map_err(|_| Error::Allocation)?;
        vector.capacity_hint = capacity;
        vector.values.resize(index + 1, 0);
    }
    vector.values[index] = 0;
    Ok(())
}
/// Original4E5EAB..4E5FB5. Iteration is by resource, then effective/raw/consumed
/// vector, so failures preserve the original already-written prefix. It never
/// shrinks an oversized vector and never changes the stockpiles at1C4.
pub fn reset_flow_vectors(state: &mut SpecialShell, resource_count: i32) -> Result<()> {
    for index in 0..resource_count.max(0) as usize {
        for offset in [0x1a0, 0x1ac, 0x1b8] {
            let vector = state
                .vectors
                .get_mut(&offset)
                .ok_or_else(|| Error::Source(format!("missing Kingdom vector {offset:x}")))?;
            zero_index(vector, index)?;
        }
    }
    Ok(())
}

/// Source definitions arranged by the independently recovered native Resource
/// registry, never by an actor map's key ordering or ResourceList UI order.
#[derive(Clone)]
pub struct Definitions {
    catalog: Catalog,
    limits: Limits,
    precision: X87Precision,
}
impl Definitions {
    pub fn bind(source: &ResourceCatalog, actors: &ActorCatalog, pc: Precision) -> Result<Self> {
        for file in &source.sources {
            let mut matches = actors
                .sources
                .iter()
                .filter(|other| other.path.eq_ignore_ascii_case(&file.path));
            let Some(actual) = matches.next() else {
                return Err(Error::Source(format!(
                    "resource source {} is absent from the Actor catalog",
                    file.path
                )));
            };
            if matches.next().is_some() || actual.sha256 != file.sha256 {
                return Err(Error::Source(format!(
                    "resource/Actor source identity mismatch for {}",
                    file.path
                )));
            }
        }
        let registry = actors
            .native_resource_registry(pc)
            .map_err(|e| Error::Source(e.to_string()))?;
        if registry.ids.len() != source.resources.len() {
            return Err(Error::Source(
                "Resource source/registry counts disagree".into(),
            ));
        }
        let mut resources = Vec::with_capacity(registry.ids.len());
        for id in &registry.ids {
            let def = source
                .find(id)
                .ok_or_else(|| Error::Source(format!("Resource {id} is missing")))?;
            let mut cost = vec![F32Bits::ZERO; registry.ids.len()];
            for (index, other) in registry.ids.iter().enumerate() {
                let other_def = source
                    .find(other)
                    .ok_or_else(|| Error::Source(format!("Resource {other} is missing")))?;
                // Original4E1F60 never consumes other destination columns.
                if other_def.category != ResourceCategory::Stockpiled {
                    continue;
                }
                let coefficient = def
                    .purchase_cost
                    .iter()
                    .find(|v| v.resource_id == *other)
                    .or_else(|| {
                        source
                            .cost_defaults
                            .iter()
                            .find(|v| v.resource_id == *other)
                    })
                    .ok_or_else(|| {
                        Error::Source(format!(
                            "Resource {id} has no authored cost/default for {other}"
                        ))
                    })?;
                cost[index] = F32Bits(coefficient.value.f32_bits);
            }
            resources.push(ResourceDefinition {
                id: def.id.clone(),
                limited_provider: def.limited_provider,
                purchase_cost: cost,
                kind: match def.category {
                    ResourceCategory::Stockpiled => ResourceKind::Stockpiled,
                    ResourceCategory::Upkeep => ResourceKind::Upkeep,
                    ResourceCategory::LimitedConsumed => ResourceKind::LimitedConsumed,
                },
            });
        }
        Ok(Self {
            catalog: Catalog { resources },
            limits: Limits {
                provided_maximum: source.limited_resource_max,
                stockpiled_maximum: F32Bits(source.stockpiled_resource_max.f32_bits),
            },
            precision: match pc {
                Precision::Single24 => X87Precision::Single24,
                Precision::Double53 => X87Precision::Double53,
                Precision::Extended64 => X87Precision::Extended64,
            },
        })
    }
    pub fn resource_count(&self) -> usize {
        self.catalog.resources.len()
    }
    pub fn catalog(&self) -> &Catalog {
        &self.catalog
    }
    /// Ownership conversion only, after the real Init reset. A constructor's
    /// zero-length flow arrays do not qualify; no resource defaults are supplied.
    /// Validation failure retains the entire original state. After success the
    /// four vectors are absent from SpecialShell and have one mutable owner here.
    pub fn take_completed_arrays(&self, source: &mut SpecialShell) -> Result<OwnedEconomy> {
        let offsets = [0x1ac, 0x1a0, 0x1b8, 0x1c4];
        let mut capacities = [0; 4];
        let mut values = Vec::with_capacity(4);
        for (index, offset) in offsets.iter().enumerate() {
            let vector = source
                .vectors
                .get(offset)
                .ok_or_else(|| Error::Source(format!("missing Kingdom vector {offset:x}")))?;
            if vector.values.len() != self.resource_count() {
                return Err(Error::Source(format!(
                    "Kingdom vector {offset:x} has not completed resource initialization"
                )));
            }
            capacities[index] = vector.capacity_hint;
            values.push(
                vector
                    .values
                    .iter()
                    .copied()
                    .map(F32Bits)
                    .collect::<Vec<_>>(),
            );
        }
        let mut values = values.into_iter();
        let state = State {
            raw_provided: values.next().unwrap(),
            effective_provided: values.next().unwrap(),
            consumed: values.next().unwrap(),
            stockpiled: values.next().unwrap(),
        };
        let economy = Economy::new(self.catalog.clone(), state, self.limits, self.precision)?;
        for offset in offsets {
            source.vectors.remove(&offset);
        }
        Ok(OwnedEconomy {
            economy,
            capacities,
        })
    }
}
pub struct OwnedEconomy {
    pub economy: Economy,
    /// Storage metadata only, ordered raw/effective/consumed/stockpiled.
    pub capacities: [u32; 4],
}

/// GKingdom words used by the economy Tick `0x4E9C40` (aa_economy::tick):
/// +0x190 overdraw suppression, +0x1F0/+0x1F4 shortage/announced ratios and
/// +0x334 statistics snapshot timer (constructor `0x4E5370` stores 60.0 there,
/// so the first statistics tick snapshots immediately). A missing word is an
/// explicit error; no default is invented.
pub fn tick_state(shell: &SpecialShell) -> Result<aa_economy::tick::TickState> {
    let word = |offset: u32| {
        shell
            .words
            .get(&offset)
            .copied()
            .ok_or_else(|| Error::Source(format!("missing Kingdom tick word {offset:x}")))
    };
    Ok(aa_economy::tick::TickState {
        suppress_overdrawn: word(0x190)? != 0,
        shortage_ratio: F32Bits(word(0x1f0)?),
        announced_ratio: F32Bits(word(0x1f4)?),
        snapshot_timer: F32Bits(word(0x334)?),
    })
}

/// Writes tick-mutated words back to their native offsets. +0x190 is read-only
/// for the tick and is left untouched.
pub fn store_tick_state(shell: &mut SpecialShell, state: &aa_economy::tick::TickState) {
    shell.words.insert(0x1f0, state.shortage_ratio.0);
    shell.words.insert(0x1f4, state.announced_ratio.0);
    shell.words.insert(0x334, state.snapshot_timer.0);
}

/// Recruit-item-single list +0x214 with charging index +0x220. Only the empty
/// list restored today is supported: populated singles need the native item
/// records (`0x4E6C20` loader) with ActorData+0x2CC rates and GTIDs.
pub fn recruit_singles(shell: &SpecialShell) -> Result<aa_economy::tick::RecruitSingles> {
    let list = shell
        .vectors
        .get(&0x214)
        .ok_or_else(|| Error::Source("missing Kingdom recruit-single vector 214".into()))?;
    if !list.values.is_empty() {
        return Err(Error::Source(
            "populated recruit singles require native item binding".into(),
        ));
    }
    let charging = shell
        .words
        .get(&0x220)
        .copied()
        .ok_or_else(|| Error::Source("missing Kingdom word 220".into()))?;
    Ok(aa_economy::tick::RecruitSingles {
        charging: charging as i32,
        items: Vec::new(),
    })
}

#[cfg(test)]
mod tick_binding_tests {
    use super::*;

    #[test]
    fn constructed_kingdom_shell_exposes_native_tick_defaults() {
        let shell = SpecialShell::kingdom(aa_actors::ActorId::from_raw(0x0001_0007), 3).unwrap();
        let state = tick_state(&shell).unwrap();
        // 0x4E5370: +0x190 = 1, ratios zero, snapshot timer 60.0f.
        assert!(state.suppress_overdrawn);
        assert_eq!(state.shortage_ratio, F32Bits::ZERO);
        assert_eq!(state.announced_ratio, F32Bits::ZERO);
        assert_eq!(state.snapshot_timer, F32Bits(0x4270_0000));
        let singles = recruit_singles(&shell).unwrap();
        assert_eq!(singles.charging, -1);
        assert!(singles.items.is_empty());
        let mut shell = shell;
        let mut updated = state.clone();
        updated.shortage_ratio = F32Bits(0x3e80_0000);
        store_tick_state(&mut shell, &updated);
        assert_eq!(tick_state(&shell).unwrap(), updated);
        shell.words.remove(&0x1f4);
        assert!(matches!(tick_state(&shell), Err(Error::Source(_))));
    }
}
