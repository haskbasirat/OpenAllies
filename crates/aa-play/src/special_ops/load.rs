//! The generals (Data/Generals: `Faction` roots of template AAGeneral) and
//! their special operations, read once from the original catalog through the
//! aa-rules typed readers (FactionData 48BC25 -> 4BF990 per Ability, the
//! effect loaders 4BEF80/4CA880 and their LoadData), plus the computer
//! players' GeneralPowers of every Ego (SAIData 4AFD50 / 4AFB60). Cached as
//! JSON next to the GameData cache.
use super::*;
use crate::data::loader::{bl, fx, st};
use aa_numeric::Precision;
use aa_rules::{
    abilities::{EffectData, EnchantmentModifier, LoadedEffect},
    actors::{ActorCatalog, ResolvedNode},
    native::TypedValue,
};

const PC: Precision = Precision::Double53;
const CACHE_VERSION: &str = "aa-play-generals-v3";

fn f(bits: u32) -> f32 {
    f32::from_bits(bits)
}
fn text(v: &TypedValue) -> Option<String> {
    match v {
        TypedValue::String { value, .. } => Some(value.clone()),
        _ => None,
    }
}
fn string_field(node: &ResolvedNode<'_>, name: &str) -> Option<String> {
    st(node, name).filter(|s| !s.is_empty() && !s.eq_ignore_ascii_case("null"))
}

/// A `localized` field's key (e.g. "#rommel_name"), resolved by the UI.
fn localized(node: &ResolvedNode<'_>, name: &str) -> Option<String> {
    node.localized(name).ok().flatten().map(|l| l.text).or_else(|| string_field(node, name)).filter(|s| !s.is_empty())
}

fn cache_path(game_root: &std::path::Path) -> std::path::PathBuf {
    if let Ok(p) = std::env::var("AA_PLAY_GENERALS_CACHE") {
        return p.into();
    }
    game_root.join("rust-rebuild").join("assets-cache").join("aa-play").join("generals.json")
}

impl GeneralData {
    /// From the user's original data, cached (`AA_PLAY_NO_CACHE=1` rereads).
    pub fn load(game_root: &std::path::Path, data: &GameData) -> anyhow::Result<Self> {
        let key = format!("{CACHE_VERSION}:{}", aa_rules::sources::BASELINE_LEDGER_SHA256);
        let path = cache_path(game_root);
        let use_cache = std::env::var("AA_PLAY_NO_CACHE").map_or(true, |v| v.is_empty() || v == "0");
        if use_cache
            && let Ok(bytes) = std::fs::read(&path)
            && let Ok(g) = serde_json::from_slice::<GeneralData>(&bytes)
            && g.key == key
        {
            return Ok(g);
        }
        let mut sources = crate::data::Sources::open(game_root)?;
        let catalog = ActorCatalog::load_v14(&mut sources.files).map_err(|e| anyhow::anyhow!("loading the original rules catalog: {e}"))?;
        let mut g = Self::from_catalog(&catalog, data)?;
        g.key = key;
        if use_cache {
            if let Some(dir) = path.parent() {
                let _ = std::fs::create_dir_all(dir);
            }
            if let Ok(bytes) = serde_json::to_vec(&g) {
                let tmp = path.with_extension(format!("json.{}.tmp", std::process::id()));
                if std::fs::write(&tmp, bytes).is_ok() && std::fs::rename(&tmp, &path).is_err() {
                    let _ = std::fs::remove_file(&tmp);
                }
            }
        }
        Ok(g)
    }

    pub fn from_catalog(catalog: &ActorCatalog, data: &GameData) -> anyhow::Result<Self> {
        let mut out = GeneralData::default();
        let domain = catalog.native_target_domain(PC).map_err(|e| anyhow::anyhow!("target types: {e}"))?;
        let factions = catalog.resolved_definitions("Faction").map_err(|e| anyhow::anyhow!("Faction roots: {e}"))?;
        for node in &factions {
            let Some(id) = string_field(node, "IDS") else { continue };
            match general(catalog, data, &domain, node, &id) {
                Ok(g) => out.generals.push(g),
                Err(e) => out.notes.push(format!("general {id}: {e}")),
            }
        }
        match catalog.resolved_definitions("Ego") {
            Ok(egos) => {
                for ego in &egos {
                    let Some(id) = string_field(ego, "IDS") else { continue };
                    let powers = ego_powers(ego, &mut out.notes);
                    if !powers.is_empty() {
                        out.ego_powers.push((id.to_ascii_lowercase(), powers));
                    }
                }
            }
            Err(e) => out.notes.push(format!("Ego roots: {e}")),
        }
        Ok(out)
    }
}

fn general(catalog: &ActorCatalog, data: &GameData, domain: &aa_rules::targets::TargetDomain, node: &ResolvedNode<'_>, id: &str) -> anyhow::Result<General> {
    let ctor = catalog.native_commander_constructor_inputs(id, PC).map_err(|e| anyhow::anyhow!("{e}"))?;
    let effects = catalog.native_faction_ability_effects(id, PC).map_err(|e| anyhow::anyhow!("{e}"))?;
    let mut g = General {
        id: id.to_string(),
        name: localized(node, "name").unwrap_or_default(),
        description: localized(node, "description").unwrap_or_default(),
        staging_name: localized(node, "staging_name").unwrap_or_default(),
        icon: string_field(node, "icon_texture_ids"),
        portrait: string_field(node, "portrait_ids"),
        public: bl(node, "public").unwrap_or(true),
        experience_max: f(ctor.experience_max_bits),
        abilities: Vec::new(),
    };
    let nodes = node.children("Ability").map_err(|e| anyhow::anyhow!("{e}"))?;
    for (ability, (flags, power_bits, loaded)) in nodes.iter().zip(effects) {
        let input = catalog.native_ability_inputs(ability, PC).map_err(|e| anyhow::anyhow!("{e}"))?;
        let def_node = match &input.shared_definition_id {
            Some(TypedValue::String { value, .. }) => catalog.resolved_definition("AbilityDefinition", value, PC).map_err(|e| anyhow::anyhow!("{e}"))?,
            _ => ability.child("AbilityDefinition").map_err(|e| anyhow::anyhow!("{e}"))?.ok_or_else(|| anyhow::anyhow!("no AbilityDefinition"))?,
        };
        let df = &input.definition.fields;
        let fixed = |k: &str| df.get(k).and_then(|v| v.fixed_bits().ok()).map(f).unwrap_or(0.0);
        let int = |m: &std::collections::BTreeMap<String, TypedValue>, k: &str| match m.get(k) {
            Some(TypedValue::Integer { value, .. }) => *value as f32,
            Some(v) => v.fixed_bits().map(f).unwrap_or(0.0),
            None => 0.0,
        };
        // An unparsed list ("NULL") keeps the AbilityDefinition 4BF411 defaults.
        let targets = df.get("targets").and_then(|v| domain.parse(v, true).ok()).unwrap_or(aa_rules::targets::TargetFilter::EMPTY.for_ability());
        let projectile = df.get("projectile_IDS").and_then(text).filter(|s| !s.is_empty() && !s.eq_ignore_ascii_case("null"));
        let projectile_life = projectile.as_ref().and_then(|p| data.find(p)).map_or(0.0, |d| data.def(d).projectile_life);
        let ability_type = df.get("ability_type").and_then(text).filter(|s| !s.is_empty() && !s.eq_ignore_ascii_case("null"));
        g.abilities.push(SpecialOp {
            id: df.get("IDS").and_then(text).unwrap_or_default(),
            name: localized(&def_node, "name").unwrap_or_default(),
            description: localized(&def_node, "description").unwrap_or_default(),
            icon: string_field(&def_node, "icon"),
            target_location_vfx: string_field(&def_node, "target_location_vfx"),
            cost: int(&input.fields, "cost_faction_experience"),
            recharge: fixed("recharge_time"),
            // FactionData 48BD05 overwrites the definition range with FLT_MAX.
            range: f32::MAX,
            area: fixed("area"),
            volley_area: fixed("volley_area"),
            max_targets: int(df, "max_targets").max(0.0) as u32,
            required: targets.required,
            forbidden: targets.forbidden,
            flags,
            power: f(power_bits),
            attenuation_min: fixed("attenuation_range_min"),
            attenuation_factor: fixed("attenuation_factor_max"),
            projectile,
            projectile_life,
            ability_type,
            effects: loaded.iter().map(|e| effect(catalog, data, e)).collect(),
        });
        // The enchantments' own localized name and icon (ModifierProperty
        // fields the property loader does not keep), matched by IDS.
        let op = g.abilities.last_mut().expect("pushed");
        for node in def_node.children("Effect").unwrap_or_default() {
            for p in node.children("EnchantmentProperty").unwrap_or_default() {
                let Some(id) = string_field(&p, "IDS") else { continue };
                for e in op.effects.iter_mut().filter_map(|e| e.enchantment.as_mut()).filter(|e| e.id.eq_ignore_ascii_case(&id)) {
                    if e.name.is_empty() {
                        e.name = localized(&p, "name").unwrap_or_default();
                    }
                    if e.icon.is_none() {
                        e.icon = string_field(&p, "icon");
                    }
                }
            }
        }
    }
    Ok(g)
}

fn effect(catalog: &ActorCatalog, data: &GameData, e: &LoadedEffect) -> OpEffect {
    let damage_index = |id: &str| data.damage_types.iter().position(|d| d.eq_ignore_ascii_case(id)).map(|i| i as u32);
    let mut out = OpEffect { kind: e.kind, required: e.targets.required, forbidden: e.targets.forbidden, hide: e.hide_from_tooltip, ..Default::default() };
    match &e.data {
        EffectData::Morale { amount_bits } => out.amount = f(*amount_bits),
        EffectData::Heal { amount_bits, heal_type } => {
            out.amount = f(*amount_bits);
            out.heal_type = *heal_type;
        }
        EffectData::Damage { damage_type, amount_bits } => {
            out.amount = f(*amount_bits);
            out.damage_type = damage_index(&damage_type.id);
        }
        EffectData::Enchantment(p) => {
            let meta = |k: &str| p.metadata.get(k).and_then(text).filter(|s| !s.is_empty() && !s.eq_ignore_ascii_case("null"));
            out.enchantment = Some(Enchantment {
                id: p.id.clone(),
                name: meta("name").unwrap_or_default(),
                icon: meta("icon"),
                duration: f(p.duration_bits),
                kind: p.enchantment_type.id.to_ascii_lowercase(),
                required_properties: p.required_properties.iter().map(|r| r.id.to_ascii_lowercase()).collect(),
                modifiers: p.modifiers.iter().map(|m| modifier(catalog, data, m)).collect(),
            });
        }
        EffectData::Summon(s) => {
            let actor = catalog.resolved_actor(s.object.id.as_str()).ok();
            out.summon = Some(Summon {
                object: s.object.id.to_ascii_lowercase(),
                controllable: s.controllable,
                property: s.property.as_ref().map(|p| p.id.to_ascii_lowercase()),
                unique_required: s.unique_targets.required,
                unique_forbidden: s.unique_targets.forbidden,
                transport: actor.as_ref().and_then(|a| string_field(a, "transport_actor")).map(|t| t.to_ascii_lowercase()),
                ethereal: actor.as_ref().and_then(|a| a.child("Flags").ok().flatten()).and_then(|fl| bl(&fl, "ethereal")).unwrap_or(false),
            });
        }
        EffectData::StealResources(cost) => {
            out.steal = data
                .resources
                .iter()
                .map(|r| cost.resource_ids.iter().position(|id| id.eq_ignore_ascii_case(&r.id)).and_then(|i| cost.bits.get(i)).map_or(0.0, |b| f(*b)))
                .collect();
        }
        EffectData::Plain => {}
    }
    out
}

fn modifier(catalog: &ActorCatalog, data: &GameData, m: &EnchantmentModifier) -> OpModifier {
    match m {
        EnchantmentModifier::Add { attribute, amount_bits, .. } => OpModifier::Add { attribute: *attribute, amount: f(*amount_bits) },
        EnchantmentModifier::Multiply { attribute, amount_bits, .. } => OpModifier::Multiply { attribute: *attribute, amount: f(*amount_bits) },
        EnchantmentModifier::ResistDamage { amount_bits, damage_type_index, .. } => OpModifier::Resist { damage_type: *damage_type_index, amount: f(*amount_bits) },
        EnchantmentModifier::PeriodicDamage { damage_type_index, amount_bits, .. } => OpModifier::PeriodicDamage { damage_type: *damage_type_index, amount: f(*amount_bits) },
        EnchantmentModifier::PeriodicHeal { amount_bits, .. } => OpModifier::PeriodicHeal { amount: f(*amount_bits) },
        EnchantmentModifier::EnchantmentImmunity { enchantment_type, .. } => OpModifier::Immunity { kind: enchantment_type.as_ref().map(|t| t.id.to_ascii_lowercase()) },
        EnchantmentModifier::AbilityEffect { ability_type, effect: e, .. } => OpModifier::AbilityEffect { ability_type: ability_type.id.to_ascii_lowercase(), effect: Box::new(effect(catalog, data, e)) },
        EnchantmentModifier::Flag(inputs) => OpModifier::Flag { flag: text(&inputs.flag).unwrap_or_default().to_ascii_uppercase() },
        _ => OpModifier::Other,
    }
}

/// `4AFB60` GeneralPowerActorRequirements.
fn requirements(node: Option<&ResolvedNode<'_>>) -> Requirements {
    let Some(n) = node else { return Requirements::default() };
    Requirements {
        max_health: fx(n, "max_percent_health").unwrap_or(1.0),
        must_be_engaged: bl(n, "must_be_engaged").unwrap_or(false),
        must_not_be_engaged: bl(n, "must_not_be_engaged").unwrap_or(false),
        friendly: bl(n, "must_be_friendly").unwrap_or(false),
        enemy: bl(n, "must_be_enemy").unwrap_or(false),
        owned: bl(n, "must_be_owned").unwrap_or(false),
        properties: n.children("RequiredProperty").unwrap_or_default().iter().filter_map(|p| string_field(p, "ids")).map(|s| s.to_ascii_lowercase()).collect(),
        multiplier: fx(n, "multiplier").unwrap_or(1.0),
    }
}

fn factor(node: Option<ResolvedNode<'_>>) -> Factor {
    let Some(n) = node else { return Factor::default() };
    Factor {
        minimum: fx(&n, "minimum").unwrap_or(0.0),
        multiplier: fx(&n, "multiplier").unwrap_or(0.0),
        maximum: fx(&n, "maximum").unwrap_or(-1.0),
        consider_friendly: bl(&n, "consider_friendly").unwrap_or(false),
        consider_enemy: bl(&n, "consider_enemy").unwrap_or(false),
        consider_owned: bl(&n, "consider_owned").unwrap_or(false),
        categories: n.children("Category").unwrap_or_default().iter().map(|c| requirements(Some(c))).collect(),
        exponent: fx(&n, "exponent").unwrap_or(1.0),
        exponent_over_one: fx(&n, "exponent_over_one").unwrap_or(1.0),
        exponent_under_one: fx(&n, "exponent_under_one").unwrap_or(1.0),
        count_owned_only: bl(&n, "count_owned_only").unwrap_or(false),
        resources: n.children("Resource").unwrap_or_default().iter().filter_map(|r| Some((string_field(r, "ids")?.to_ascii_lowercase(), fx(r, "amount").unwrap_or(0.0)))).collect(),
    }
}

/// `4B0560` GoalEngine GeneralPowers of one Ego (`4AFD50` per Power).
fn ego_powers(ego: &ResolvedNode<'_>, notes: &mut Vec<String>) -> Vec<PowerConfig> {
    let Some(list) = ego.child("GoalEngine").ok().flatten().and_then(|ge| ge.child("GeneralPowers").ok().flatten()) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for p in list.children("Power").unwrap_or_default() {
        let Some(ability) = string_field(&p, "ability_ids") else { continue };
        let location = p.child("Location").ok().flatten();
        let kind = location.as_ref().and_then(|l| string_field(l, "type")).unwrap_or_default();
        let placement = location.as_ref().and_then(|l| l.child("Placement").ok().flatten());
        let general_req = location.as_ref().and_then(|l| l.child("GeneralRequirements").ok().flatten());
        let priority = p.child("Priority").ok().flatten();
        let pchild = |name: &str| priority.as_ref().and_then(|pr| pr.child(name).ok().flatten());
        let followup = p.child("Followup").ok().flatten();
        let location_kind = match kind.to_ascii_uppercase().as_str() {
            "NEAR_ACTOR" => 1,
            "ON_ACTOR" => 2,
            "FOG" => 3,
            "NONE" | "" => 0,
            other => {
                notes.push(format!("general power {ability}: location type {other}"));
                0
            }
        };
        out.push(PowerConfig {
            ability: ability.to_ascii_lowercase(),
            location: location_kind,
            actor_requirements: requirements(location.as_ref().and_then(|l| l.child("ActorRequirements").ok().flatten()).as_ref()),
            radius: placement.as_ref().and_then(|pl| fx(pl, "radius")).unwrap_or(-1.0),
            candidates: placement.as_ref().and_then(|pl| fx(pl, "num_candidates")).unwrap_or(3.0).max(0.0) as u32,
            no_friendly: general_req.as_ref().and_then(|g| bl(g, "no_friendly_actors")).unwrap_or(false),
            no_enemy: general_req.as_ref().and_then(|g| bl(g, "no_enemy_actors")).unwrap_or(false),
            no_owned: general_req.as_ref().and_then(|g| bl(g, "no_owned_actors")).unwrap_or(false),
            requires_supply: general_req.as_ref().and_then(|g| bl(g, "requires_supply")).unwrap_or(false),
            base: priority.as_ref().and_then(|pr| fx(pr, "base")).unwrap_or(0.0),
            minimum: priority.as_ref().and_then(|pr| fx(pr, "minimum")).unwrap_or(0.0),
            fuzzy: priority.as_ref().and_then(|pr| fx(pr, "fuzzy_factor")).unwrap_or(100.0),
            cv: factor(pchild("CV")),
            actor_value: factor(pchild("ActorValue")),
            actor_economy: factor(pchild("ActorEconomy")),
            enemy_influence: factor(pchild("EnemyInfluence")),
            influence_ratio: factor(pchild("InfluenceRatio")),
            time_since_explored: factor(pchild("TimeSinceExplored")),
            economy_generated: factor(pchild("EconomyGenerated")),
            flare_priority: followup.as_ref().and_then(|fl| fx(fl, "attack_flare_priority")).unwrap_or(0.0),
            flare_duration: followup.as_ref().and_then(|fl| fx(fl, "attack_flare_duration")).unwrap_or(0.0),
        });
    }
    out
}
