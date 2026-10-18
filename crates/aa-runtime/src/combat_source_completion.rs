//! One Bank-owned combat-value dependency slice of native `0x4BD800`.
//!
//! This is not a full source-loader or world-start certificate. The preceding
//! passes have separately proved CV-disjoint writes on typed nonalias objects.
//! Effective numeric/CSV inputs, source FP state and fresh native globals stay
//! explicit. Diagnostic contexts can compute evidence but cannot authorize a
//! published native-startup value. No constructor marker is a completed value.
//! The same manager's lazy named-pointer guards and constructed stream state
//! now remain owned here across source completion and runtime use. Current
//! numeric SVar cells live separately in the actual World.variables owner.
use crate::{
    catalog::Catalog,
    combat_value_source::{self, DefinitionInputs},
    combat_value_source_context::{self, CsvBinding, NumericValue, SourceFpContext},
    definitions::Bank,
};
use aa_body::F32;
use aa_combat::{
    Error,
    combat_mean_coordinator::{self as mean, Field},
    combat_value::{self as cv, DefinitionField},
};
use aa_numeric::{Binary, Precision};
use aa_rules::{
    native::TypedValue,
    resources::{Provenance, SourceIdentity},
    sources::V14Sources,
};
use std::collections::BTreeMap;

pub type Result<T> = std::result::Result<T, String>;
fn error(value: impl std::fmt::Display) -> String {
    value.to_string()
}
fn world_error(value: impl Into<String>) -> Error {
    Error::World(value.into())
}
fn number(value: F32) -> aa_combat::Result<Binary> {
    Ok(Binary::from_f32_bits(value.0)?)
}

/// Explicit source-global state; no Default and no automatic native receipt.
#[derive(Clone, Copy, Debug)]
pub struct Globals {
    pub assertion_guard: u8,
    pub ranged_lookup_guard: u32,
    pub csv_lookup_guard: u32,
    pub debug_stream_is_closed: bool,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Bound,
    ComputingMeans,
    MeansComplete,
    ComputingValues,
    Complete,
    Failed,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Values {
    pub value_1e4: F32,
    pub children_30c: F32,
    pub weighted_36c: F32,
}
#[derive(Clone, Debug)]
pub enum Event {
    Definition(u32),
    Store {
        definition: u32,
        field: DefinitionField,
        value: F32,
    },
    Lookup {
        name: &'static str,
        kind: u32,
    },
    MeanLog {
        line: u32,
        value_binary64: u64,
    },
}
#[derive(Debug)]
struct Actor {
    id: String,
    source: Provenance,
    flags: u32,
    body: Option<(F32, F32)>,
    speed: Option<F32>,
    bias: Option<F32>,
    properties: Vec<u32>,
    abilities: Vec<u32>,
    children: Vec<Option<u32>>,
    weighted: Vec<(u32, i32)>,
    values: Values,
}
#[derive(Debug)]
struct LocalAbility {
    definition: u32,
    reload: F32,
    recharge: F32,
    animation: Option<F32>,
}

/// The same manager stream constructed before source completion and retained
/// with this Bank. Existing install preflight proves Closed for the controlled
/// empty-CSV scope. No independent runtime stream mirror exists.
#[derive(Debug)]
enum CurrentStream {
    Closed,
    /// A future actual open producer must retain its real writer/output prefix;
    /// it must not silently inherit the source phase's closed observation.
    #[allow(dead_code)]
    RequiresWriter,
}

/// The single authoritative mutable cache and immutable dependency storage for
/// this Bank. Intentionally not Clone; installation, failed progress and values
/// move together when the Bank moves from Catalog to a restored/live owner.
#[derive(Debug)]
pub struct State {
    context: SourceFpContext,
    csv: CsvBinding,
    numeric: BTreeMap<String, NumericValue>,
    sources: Vec<SourceIdentity>,
    phase: Phase,
    failure: Option<String>,
    manager_order: Vec<u32>,
    links: BTreeMap<u32, (u32, Option<u32>)>,
    first: Option<u32>,
    actors: BTreeMap<u32, Actor>,
    abilities: BTreeMap<u32, cv::AbilityDefinition>,
    locals: BTreeMap<u32, LocalAbility>,
    properties: BTreeMap<u32, F32>,
    means: [Option<F32>; 8],
    assertion_guard: u8,
    ranged_cached: bool,
    weight_cached: bool,
    csv_cached: bool,
    events: Vec<Event>,
    reference_result_verified: bool,
    current_stream: CurrentStream,
}
impl State {
    pub fn phase(&self) -> Phase {
        self.phase
    }
    pub fn failure(&self) -> Option<&str> {
        self.failure.as_deref()
    }
    pub fn source_precision(&self) -> Precision {
        self.context.precision()
    }
    pub fn context(&self) -> &SourceFpContext {
        &self.context
    }
    pub fn source_identities(&self) -> &[SourceIdentity] {
        &self.sources
    }
    pub fn events(&self) -> &[Event] {
        &self.events
    }
    /// Same manager-global lazy named pointer flag as the source path. Runtime
    /// callers read the matching CURRENT World.variables cell after binding.
    pub(crate) fn bind_current_ranged_minimum(&mut self) {
        if !self.ranged_cached {
            self.ranged_cached = true;
            self.events.push(Event::Lookup { name: "CombatValueRangedMinimum", kind: 2 });
        }
    }
    pub(crate) fn current_log_enabled(&self) -> aa_combat::Result<bool> {
        match &self.current_stream {
            CurrentStream::Closed => Ok(false),
            CurrentStream::RequiresWriter => Err(world_error("runtime CV manager open stream requires actual writer/output-prefix owner")),
        }
    }
    pub(crate) fn current_defense_weight_is_bound(&self) -> bool { self.weight_cached }
    /// Observe the same manager's two actual lazy SVar pointer guards. This
    /// does not resolve either pointer or change current numeric values.
    pub fn current_cached_svars(&self) -> [bool; 2] { [self.ranged_cached, self.weight_cached] }
    pub fn manager_order(&self) -> &[u32] {
        &self.manager_order
    }
    pub fn definition_id(&self, token: u32) -> Result<&str> {
        Ok(&self.actor(token).map_err(error)?.id)
    }
    pub fn definition_source(&self, token: u32) -> Result<&Provenance> {
        Ok(&self.actor(token).map_err(error)?.source)
    }
    pub fn diagnostic_values(&self, token: u32) -> Result<Values> {
        Ok(self.actor(token).map_err(error)?.values)
    }
    pub fn diagnostic_means(&self) -> Result<cv::MeanValues> {
        self.mean_values().map_err(error)
    }
    pub fn is_observed_native_startup(&self) -> bool {
        self.context.reference_startup().is_some()
            && self.context.reference_startup() == self.csv.reference_startup()
    }
    /// This guard does not convert diagnostic explicit-PC/FreshNoProfile inputs
    /// into an observed native startup. The later lifecycle owner must supply
    /// the actual capability; neither caller labels nor computed numbers do so.
    pub fn completed_values(&self, token: u32) -> Result<Values> {
        self.require_completed_reference()?;
        self.diagnostic_values(token)
    }
    pub fn completed_means(&self) -> Result<cv::MeanValues> {
        self.require_completed_reference()?;
        self.diagnostic_means()
    }
    pub fn completed_maximum_74(&self) -> Result<F32> {
        Ok(self.completed_means()?.maximum_74)
    }
    fn require_completed_reference(&self) -> Result<()> {
        if self.phase != Phase::Complete {
            return Err("combat source cache is not complete".into());
        }
        if !self.is_observed_native_startup() {
            return Err("combat source context is diagnostic; native startup FP/profile lifecycle is not certified".into());
        }
        if !self.reference_result_verified {
            return Err(
                "computed combat source values have not matched the sealed reference result".into(),
            );
        }
        Ok(())
    }
    pub fn result_fingerprint(&self) -> Result<String> {
        let m = self.diagnostic_means()?;
        let means = [
            m.maximum_74.0,
            m.attack.0,
            m.attack_rate.0,
            m.defense.0,
            m.movement.0,
            m.health.0,
            m.lifetime.0,
            m.offense.0,
        ];
        let rows = self.manager_order.iter().map(|id| {
            let a = &self.actors[id];
            (
                a.id.clone(),
                a.flags,
                [
                    a.values.value_1e4.0,
                    a.values.children_30c.0,
                    a.values.weighted_36c.0,
                ],
            )
        });
        Ok(crate::combat_source_receipt::result_fingerprint(
            means, rows,
        ))
    }
    fn actor(&self, id: u32) -> aa_combat::Result<&Actor> {
        self.actors
            .get(&id)
            .ok_or_else(|| world_error("unbound canonical ActorDefinition"))
    }
    fn local(&self, id: u32) -> aa_combat::Result<&LocalAbility> {
        self.locals
            .get(&id)
            .ok_or_else(|| world_error("unbound canonical ActorLocalAbility"))
    }
    fn numeric(&self, name: &str) -> aa_combat::Result<F32> {
        self.numeric
            .get(&name.to_ascii_lowercase())
            .map(|v| F32(v.bits))
            .ok_or_else(|| world_error(format!("unbound typed numeric {name}")))
    }
    fn mean_values(&self) -> aa_combat::Result<cv::MeanValues> {
        let read =
            |i: usize| self.means[i].ok_or_else(|| world_error("mean field has not been produced"));
        Ok(cv::MeanValues {
            maximum_74: read(0)?,
            attack: read(1)?,
            attack_rate: read(2)?,
            defense: read(3)?,
            movement: read(4)?,
            health: read(5)?,
            lifetime: read(6)?,
            offense: read(7)?,
        })
    }
    fn ability_snapshot(&self, id: u32) -> aa_combat::Result<cv::AbilityLocal> {
        let local = self.local(id)?;
        let definition = *self
            .abilities
            .get(&local.definition)
            .ok_or_else(|| world_error("unbound shared AbilityDefinition cell"))?;
        Ok(cv::AbilityLocal {
            definition,
            value_04: local.reload,
            recharge_18: local.recharge,
            first_entry_10: local.animation,
        })
    }
}

/// Install the entire current native-kind-2 dependency closure exactly once in
/// the existing definition Bank. This supported shipped slice requires no
/// Settlement/upgrade fitting, no shared Performer reference and no Faction
/// write to its embedded definitions. Other corpora fail explicitly.
pub fn install(
    catalog: &Catalog<'_>,
    sources: &mut V14Sources,
    context: &SourceFpContext,
    csv: &CsvBinding,
    globals: Globals,
) -> Result<()> {
    catalog.require_source_owner()?;
    if context.reference_startup() != csv.reference_startup() {
        return Err("source FP and CSV reference receipt ownership differs".into());
    }
    if let Some(reference) = context.reference_startup()
        && crate::combat_source_receipt::source_fingerprint(&catalog.actors.sources)
            != reference.source_identities_sha256
    {
        return Err(
            "destination logical source identities differ from the verified reference".into(),
        );
    }
    if context.reference_startup().is_some() && globals.assertion_guard != 0 {
        return Err("source assertion guard differs from the verified reference entry".into());
    }
    if context.crt_environment().x87_control & 0x3f != 0x3f {
        return Err(
            "CV completion requires masked x87 exceptions in its supported finite domain".into(),
        );
    }
    if !globals.debug_stream_is_closed {
        return Err(
            "CV source stream is already open; its actual diagnostic owner is required".into(),
        );
    }
    if globals.ranged_lookup_guard != 0 || globals.csv_lookup_guard != 0 {
        return Err(
            "preexisting source lazy caches require their actual shared variable owners".into(),
        );
    }
    csv.require_empty_debug_stream()?;
    let mut root_names = sources
        .catalog()
        .into_iter()
        .map(|entry| entry.logical_path)
        .filter(|name| {
            let lower = name.to_ascii_lowercase();
            !lower.contains(['/', '\\']) && lower.starts_with("uvars") && lower.ends_with(".tgi")
        })
        .collect::<Vec<_>>();
    root_names.sort();
    if root_names.len() != csv.inputs().len()
        || root_names
            .iter()
            .zip(csv.inputs())
            .any(|(name, input)| name != &input.path)
    {
        return Err("CV completion requires the entire actual root UVars wildcard set; external override ownership is not supplied".into());
    }
    for source in csv.inputs() {
        let actual = sources.read(&source.path).map_err(error)?;
        if actual.logical_path != source.path
            || actual.sha256 != source.sha256
            || actual.origin != source.origin
        {
            return Err(
                "CSV input does not belong to this actual application source provider".into(),
            );
        }
    }
    let numeric = combat_value_source_context::bind_numeric(catalog, context)?;
    if context.reference_startup().is_some() {
        crate::combat_source_receipt::require_reference_numeric(&numeric)?;
    }
    let order = combat_value_source::actor_manager_order(catalog)?;
    if order
        .source_diagnostics
        .iter()
        .any(|v| !v.message.starts_with("native syntax notice "))
    {
        return Err("CV source load has an unclassified failure".into());
    }
    if !catalog
        .actors
        .construction_upgrade_census()
        .map_err(error)?
        .explicit_instances
        .is_empty()
    {
        return Err("nonempty Upgrade graph requires its actual completion owner".into());
    }
    let mut inputs = Vec::new();
    let mut embedded_ids = Vec::new();
    for id in &order.ids {
        if catalog
            .actors
            .native_category(id)
            .map_err(error)?
            .registry_index
            == 21
        {
            return Err("Settlement category requires its actual source completion owner".into());
        }
        let input = combat_value_source::definition_inputs(catalog, id, context.precision())?;
        if input.flags_174 & (1 << 14) != 0 {
            return Err("Settlement extent requires completed fitted geometry".into());
        }
        let binding = catalog.actor_bindings(id, context.precision())?;
        if let Some(performer) = &binding.performer {
            for ability in performer.abilities() {
                if ability.authored.shared_definition_id.is_some() {
                    return Err("shared Performer AbilityDefinition requires its source-order mutable definition owner".into());
                }
                if let Some(TypedValue::String { value, .. }) =
                    ability.authored.definition.fields.get("IDS")
                    && !value.is_empty()
                {
                    embedded_ids.push(value.to_ascii_lowercase())
                }
            }
        }
        inputs.push(input);
    }
    if let Some(factions) = catalog.actors.native_data_registry("Faction") {
        for faction in &factions.entries {
            for ability in catalog
                .actors
                .native_commander_constructor_inputs(&faction.ids, context.precision())
                .map_err(error)?
                .abilities
            {
                if let Some(TypedValue::String { value, .. }) = ability.shared_definition_id
                    && embedded_ids
                        .iter()
                        .any(|id| id.eq_ignore_ascii_case(&value))
                {
                    return Err("Faction overwrites an embedded Performer definition; actual source-order alias binding is required".into());
                }
            }
        }
    }
    let mut bank = catalog
        .definition_bank
        .try_borrow_mut()
        .map_err(|_| "nested CV source Bank ownership")?;
    if bank.combat_source.is_some() {
        return Err(
            "combat source State is already installed; its partial progress cannot be replaced"
                .into(),
        );
    }
    let mut state = State {
        context: context.clone(),
        csv: csv.clone(),
        numeric: numeric.values().clone(),
        sources: catalog.actors.sources.clone(),
        phase: Phase::Bound,
        failure: None,
        manager_order: Vec::new(),
        links: BTreeMap::new(),
        first: None,
        actors: BTreeMap::new(),
        abilities: BTreeMap::new(),
        locals: BTreeMap::new(),
        properties: BTreeMap::new(),
        means: [None; 8],
        assertion_guard: globals.assertion_guard,
        ranged_cached: false,
        weight_cached: false,
        csv_cached: false,
        events: Vec::new(),
        reference_result_verified: false,
        //Both Globals.debug_stream_is_closed and effective empty CSV were
        //required above before this same retained manager owner is created.
        current_stream: CurrentStream::Closed,
    };
    let tokens = inputs
        .iter()
        .map(|input| {
            Ok((
                input.canonical_id.to_ascii_lowercase(),
                bank.intern("ActorDefinition", &input.canonical_id)?,
            ))
        })
        .collect::<Result<BTreeMap<_, _>>>()?;
    let mut previous = None;
    for input in inputs {
        let actor = tokens[&input.canonical_id.to_ascii_lowercase()];
        let link = bank.intern("CombatSourceManagerLink", &input.canonical_id)?;
        state.links.insert(link, (actor, None));
        if let Some(before) = previous {
            state.links.get_mut(&before).unwrap().1 = Some(link)
        } else {
            state.first = Some(link)
        };
        previous = Some(link);
        state.manager_order.push(actor);
        bind_actor(&mut state, &mut bank, &tokens, actor, input)?;
    }
    bank.combat_source = Some(Box::new(state));
    Ok(())
}
fn bind_actor(
    state: &mut State,
    bank: &mut Bank,
    tokens: &BTreeMap<String, u32>,
    actor: u32,
    input: DefinitionInputs,
) -> Result<()> {
    let reference = |id: &str| {
        tokens.get(&id.to_ascii_lowercase()).copied().ok_or_else(||format!("CV reference {id} lies outside the bound kind-2 closure; typed kind19/other ownership required"))
    };
    let mut properties = Vec::new();
    for (id, value) in input.properties_1d4 {
        let token = bank.intern("Property", &id)?;
        if let Some(previous) = state.properties.insert(token, value)
            && previous != value
        {
            return Err("one shared Property has conflicting completed CV bias".into());
        };
        properties.push(token)
    }
    let mut abilities = Vec::new();
    for (index, ability) in input.abilities.into_iter().enumerate() {
        let name = format!("{}/ability/{index}", input.canonical_id);
        let local = bank.intern("ActorLocalAbility", &name)?;
        let definition = bank.intern("ActorLocalAbilityDefinition", &name)?;
        if state
            .abilities
            .insert(definition, ability.definition)
            .is_some()
            || state
                .locals
                .insert(
                    local,
                    LocalAbility {
                        definition,
                        reload: ability.value_04,
                        recharge: ability.recharge_18,
                        animation: ability.first_entry_10,
                    },
                )
                .is_some()
        {
            return Err("duplicate canonical CV ability allocation".into());
        };
        abilities.push(local);
    }
    let children = input
        .children_2f0
        .iter()
        .map(|id| id.as_deref().map(&reference).transpose())
        .collect::<Result<Vec<_>>>()?;
    let weighted = input
        .weighted_35c
        .iter()
        .map(|(id, count)| Ok((reference(id)?, *count)))
        .collect::<Result<Vec<_>>>()?;
    state.actors.insert(
        actor,
        Actor {
            id: input.canonical_id,
            source: input.source,
            flags: input.flags_174,
            body: input.body,
            speed: input.speed_228,
            bias: input.performer_bias_210,
            properties,
            abilities,
            children,
            weighted,
            values: Values {
                value_1e4: cv::UNCOMPUTED,
                children_30c: F32::ZERO,
                weighted_36c: F32::ZERO,
            },
        },
    );
    Ok(())
}

/// Run 4BBF90 followed by 4BB590/4618B0 in the one installed Bank State.
/// Failure retains prior writes and becomes terminal for this source scope.
pub fn compute(catalog: &Catalog<'_>) -> Result<()> {
    catalog.require_source_owner()?;
    let mut bank = catalog
        .definition_bank
        .try_borrow_mut()
        .map_err(|_| "nested CV source Bank ownership")?;
    let state = bank
        .combat_source
        .as_deref_mut()
        .ok_or("combat source State is not installed")?;
    if state.sources != catalog.actors.sources {
        return Err("CV source identity set changed after installation".into());
    }
    if state.phase != Phase::Bound {
        return Err(
            "CV source computation has already started; partial state cannot be retried implicitly"
                .into(),
        );
    }
    let current = combat_value_source_context::bind_numeric(catalog, &state.context)?;
    if current.values().iter().any(|(key, value)| {
        state
            .numeric
            .get(key)
            .is_none_or(|before| before.bits != value.bits)
    }) {
        return Err("CV numeric owner changed after source installation".into());
    }
    let precision = state.context.precision();
    state.phase = Phase::ComputingMeans;
    let result = (|| {
        mean::compute(state, precision)?;
        state.phase = Phase::MeansComplete;
        let means = state.mean_values()?;
        let settings = current.formula_settings().map_err(world_error)?;
        let environment = state.context.crt_environment();
        let order = state.manager_order.clone();
        state.phase = Phase::ComputingValues;
        cv::actor_data_values(&order, &means, &settings, environment, state, precision)?;
        if let Some(reference) = state.context.reference_startup() {
            if state.result_fingerprint().map_err(world_error)? != reference.computed_result_sha256
            {
                return Err(world_error(
                    "computed whole-source CV fingerprint differs from the captured reference target",
                ));
            }
            state.reference_result_verified = true;
        }
        Ok::<_, Error>(())
    })();
    match result {
        Ok(()) => {
            state.phase = Phase::Complete;
            Ok(())
        }
        Err(failure) => {
            let text = failure.to_string();
            state.phase = Phase::Failed;
            state.failure = Some(text.clone());
            Err(text)
        }
    }
}

impl mean::World for State {
    fn first_link(&mut self) -> aa_combat::Result<Option<u32>> {
        Ok(self.first)
    }
    fn next_link(&mut self, link: u32) -> aa_combat::Result<Option<u32>> {
        Ok(self
            .links
            .get(&link)
            .ok_or_else(|| world_error("unknown source list link"))?
            .1)
    }
    fn link_definition(&mut self, link: u32) -> aa_combat::Result<u32> {
        Ok(self
            .links
            .get(&link)
            .ok_or_else(|| world_error("unknown source list link"))?
            .0)
    }
    fn flags_174(&mut self, id: u32) -> aa_combat::Result<u32> {
        Ok(self.actor(id)?.flags)
    }
    fn extent(&mut self, _: u32) -> aa_combat::Result<Binary> {
        Err(world_error(
            "Settlement extent is not available in this CV source slice",
        ))
    }
    fn ability_count(&mut self, id: u32) -> aa_combat::Result<i32> {
        i32::try_from(self.actor(id)?.abilities.len())
            .map_err(|_| world_error("source Ability count exceeds native int32"))
    }
    fn ability_at(&mut self, id: u32, index: i32) -> aa_combat::Result<u32> {
        self.actor(id)?
            .abilities
            .get(index as usize)
            .copied()
            .ok_or_else(|| world_error("source Ability index outside current array"))
    }
    fn ability_definition(&mut self, local: u32) -> aa_combat::Result<u32> {
        Ok(self.local(local)?.definition)
    }
    fn base_power_64(&mut self, id: u32) -> aa_combat::Result<F32> {
        self.abilities
            .get(&id)
            .map(|v| v.power_64)
            .ok_or_else(|| world_error("unbound AbilityDefinition power"))
    }
    fn power(&mut self, id: u32) -> aa_combat::Result<Binary> {
        number(self.base_power_64(id)?)
    }
    fn reload_04(&mut self, id: u32) -> aa_combat::Result<F32> {
        Ok(self.local(id)?.reload)
    }
    fn first_animation_length(&mut self, id: u32) -> aa_combat::Result<Binary> {
        number(self.local(id)?.animation.unwrap_or(F32::ZERO))
    }
    fn recharge(&mut self, id: u32) -> aa_combat::Result<Binary> {
        number(self.local(id)?.recharge)
    }
    fn defense(&mut self, id: u32) -> aa_combat::Result<Binary> {
        number(
            self.actor(id)?
                .body
                .ok_or_else(|| world_error("mean Body defense absent"))?
                .0,
        )
    }
    fn movement(&mut self, id: u32) -> aa_combat::Result<Binary> {
        number(
            self.actor(id)?
                .speed
                .ok_or_else(|| world_error("mean Mover speed absent"))?,
        )
    }
    fn health(&mut self, id: u32) -> aa_combat::Result<Binary> {
        number(
            self.actor(id)?
                .body
                .ok_or_else(|| world_error("mean Body health absent"))?
                .1,
        )
    }
    fn discard_fraction(&mut self) -> aa_combat::Result<F32> {
        self.numeric("CombatValueMeanValuesDiscardPercent")
    }
    fn read_mean(&mut self, field: Field) -> aa_combat::Result<F32> {
        self.means[(field as usize - 0x74) / 4]
            .ok_or_else(|| world_error("mean field has not been written"))
    }
    fn write_mean(&mut self, field: Field, value: F32) -> aa_combat::Result<()> {
        self.means[(field as usize - 0x74) / 4] = Some(value);
        Ok(())
    }
    fn assertion_guard(&mut self) -> aa_combat::Result<u8> {
        Ok(self.assertion_guard)
    }
    fn assertion(&mut self) -> aa_combat::Result<u8> {
        Err(world_error(
            "native mean assertion requires its actual application callback",
        ))
    }
    fn write_assertion_guard(&mut self, value: u8) -> aa_combat::Result<()> {
        self.assertion_guard = value;
        Ok(())
    }
    fn log_mean(&mut self, _: Field, line: u32, value_binary64: u64) -> aa_combat::Result<()> {
        self.events.push(Event::MeanLog {
            line,
            value_binary64,
        });
        Ok(())
    }
}
impl cv::World for State {
    fn ranged_minimum(&mut self) -> aa_combat::Result<F32> {
        self.bind_current_ranged_minimum();
        self.numeric("CombatValueRangedMinimum")
    }
    fn bind_defense_weight_ranged(&mut self) -> aa_combat::Result<()> {
        if !self.weight_cached {
            self.weight_cached = true;
            self.events.push(Event::Lookup {
                name: "CombatValueDefenseWeightRanged",
                kind: 2,
            })
        };
        Ok(())
    }
    fn defense_weight_ranged(&mut self) -> aa_combat::Result<F32> {
        if !self.weight_cached {
            return Err(world_error(
                "ranged defense scalar read before its native pointer binding",
            ));
        }
        self.numeric("CombatValueDefenseWeightRanged")
    }
    fn modify_projectile_range(&mut self, _: u32, _: F32) -> aa_combat::Result<Binary> {
        Err(world_error("source CV has no runtime modifier context"))
    }
    fn modify_bombard_range(&mut self, _: u32, _: F32) -> aa_combat::Result<Binary> {
        Err(world_error("source CV has no runtime modifier context"))
    }
    fn modify_power(&mut self, _: u32, _: F32, _: bool) -> aa_combat::Result<Binary> {
        Err(world_error("source CV has no runtime modifier context"))
    }
    fn modify_recharge(&mut self, _: u32, _: F32) -> aa_combat::Result<Binary> {
        Err(world_error("source CV has no runtime modifier context"))
    }
    fn log_enabled(&mut self) -> aa_combat::Result<bool> {
        self.current_log_enabled()
    }
}
impl cv::DefinitionWorld for State {
    fn definition(&mut self, id: u32) -> aa_combat::Result<cv::DefinitionThing> {
        self.events.push(Event::Definition(id));
        let actor = self.actor(id)?;
        Ok(cv::DefinitionThing {
            flags_174: actor.flags,
            list_1d4: actor
                .properties
                .iter()
                .map(|id| {
                    self.properties
                        .get(id)
                        .copied()
                        .ok_or_else(|| world_error("shared Property bias missing"))
                })
                .collect::<aa_combat::Result<Vec<_>>>()?,
            defense_288: actor.body.map_or(F32::ZERO, |v| v.0),
            health_284: actor.body.map_or(F32::ZERO, |v| v.1),
            speed_228: actor.speed.unwrap_or(F32::ZERO),
            bonus_210: actor.bias.unwrap_or(F32::ZERO),
            abilities: actor
                .abilities
                .iter()
                .map(|&id| self.ability_snapshot(id))
                .collect::<aa_combat::Result<Vec<_>>>()?,
            children_2f0: actor.children.clone(),
            weighted_35c: actor.weighted.clone(),
        })
    }
    fn flags_174(&mut self, id: u32) -> aa_combat::Result<u32> {
        Ok(self.actor(id)?.flags)
    }
    fn field(&mut self, id: u32, field: DefinitionField) -> aa_combat::Result<F32> {
        let values = self.actor(id)?.values;
        Ok(match field {
            DefinitionField::Value1E4 => values.value_1e4,
            DefinitionField::Children30C => values.children_30c,
            DefinitionField::Weighted36C => values.weighted_36c,
        })
    }
    fn set_field(&mut self, id: u32, field: DefinitionField, value: F32) -> aa_combat::Result<()> {
        let actor = self
            .actors
            .get_mut(&id)
            .ok_or_else(|| world_error("unknown shared ActorDefinition cache"))?;
        match field {
            DefinitionField::Value1E4 => actor.values.value_1e4 = value,
            DefinitionField::Children30C => actor.values.children_30c = value,
            DefinitionField::Weighted36C => actor.values.weighted_36c = value,
        };
        self.events.push(Event::Store {
            definition: id,
            field,
            value,
        });
        Ok(())
    }
    fn log_prefix(&mut self, _: u32) -> aa_combat::Result<()> {
        Err(world_error("nonempty source CV CSV stream is unsupported"))
    }
    fn log_row(&mut self, _: u32, _: [u64; 17]) -> aa_combat::Result<()> {
        Err(world_error("nonempty source CV CSV stream is unsupported"))
    }
    fn csv_begin(&mut self, _: &cv::MeanValues) -> aa_combat::Result<()> {
        self.csv.require_empty_debug_stream().map_err(world_error)?;
        if !self.csv_cached {
            self.csv_cached = true;
            self.events.push(Event::Lookup {
                name: "CombatValueCSV",
                kind: 4,
            })
        };
        Ok(())
    }
    fn csv_close(&mut self) -> aa_combat::Result<()> {
        Err(world_error(
            "closed source CV stream must not be closed again",
        ))
    }
}
