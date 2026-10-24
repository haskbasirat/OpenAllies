//! The generals' special operations in the battle (combat owner):
//! * each kingdom's general at battle start: the staging Faction choice or a
//!   computer's SAI profile faction (`PendingGenerals`), else the original
//!   random pick for the nation (`special_ops::assign_missing_generals`),
//!   with the session's StartingExperience rule;
//! * the HUD's FactionAbilitiesFloater items (`0x607FC0`, per frame
//!   `0x6073C0`): the ability icon, the recharge bar (1 - remaining /
//!   recharge), the command "UseFactionAbility %i", the button disabled
//!   while recharging, unaffordable or its summon is alive / unique;
//! * the item tooltip (`0x6081B0`): the name, the Cost row (`0x63E050`,
//!   the experience glyph), the Area row (`0x63E290`, Game/descriptions.tgi
//!   AREA / AREA_VOLLEY), the description, "Recharging (m:ss left)";
//! * KKC_GameInterface::CommandUseFactionAbility (`0x612750`): the
//!   GENERIC_ERROR messages ui_error_faction_ability_recharge_time / _cost /
//!   _already_summoned, a +SELF ability used at once, any other one arms the
//!   targeting state;
//! * the targeting state's cursors faction_ability_target and
//!   faction_ability_target_valid (cursor ids 0x13 / 0x14, `0x4977C0`):
//!   valid where the click would be admitted (`4C8F20`).
use super::hud::{GameAction, TargetOrder};
use crate::session::Battle;
use aa_play::special_ops::{self, AbilityView, OpTarget};
use aa_play::{ActorId, KingdomId, World};
use aa_ui::hud::HudAbility;
use aa_ui::ui::Ui;
use bevy::prelude::*;
use std::sync::OnceLock;

/// The launched session's generals: (player index = kingdom id, faction
/// ids; None = random for the nation) and its StartingExperience rule.
#[derive(Resource, Clone, Debug, Default)]
pub struct PendingGenerals {
    pub factions: Vec<(usize, Option<String>)>,
    pub starting_experience: Option<f32>,
}

/// Battle start: every kingdom's general and the starting experience.
/// Test runs without the staging screen: AA_BEVY_GENERAL=<faction ids>
/// picks the local general, AA_BEVY_EXPERIENCE=<value> the starting
/// experience.
pub fn start(battle: Option<ResMut<Battle>>, pending: Option<Res<PendingGenerals>>, mut commands: Commands) {
    let Some(mut b) = battle else { return };
    if !b.is_added() {
        return;
    }
    let local = b.local;
    let w = &mut b.world;
    if special_ops::data(w).is_none() {
        warn!("generals: no special operations data, no general abilities");
        return;
    }
    let mut chosen: Vec<(KingdomId, String)> = Vec::new();
    if let Some(p) = pending.as_deref() {
        for (player, f) in &p.factions {
            if let Some(f) = f {
                chosen.push((*player as KingdomId, f.clone()));
            }
        }
    }
    // a computer without a staging choice: its SAI profile's faction
    if let Some(ai) = w.ai.data.clone() {
        for pl in &w.ai.players {
            if !chosen.iter().any(|c| c.0 == pl.kingdom)
                && let Some(f) = ai.profiles.get(pl.profile).and_then(|p| p.factions.first())
            {
                chosen.push((pl.kingdom, f.clone()));
            }
        }
    }
    if let Ok(g) = std::env::var("AA_BEVY_GENERAL") {
        chosen.retain(|c| c.0 != local);
        chosen.push((local, g));
    }
    for (k, f) in &chosen {
        if special_ops::set_general(w, *k, Some(f)).is_none() {
            warn!("generals: kingdom {k} has no general {f}");
        }
    }
    special_ops::assign_missing_generals(w);
    let xp = std::env::var("AA_BEVY_EXPERIENCE").ok().and_then(|v| v.trim().parse::<f32>().ok()).or(pending.as_deref().and_then(|p| p.starting_experience));
    if let Some(v) = xp {
        special_ops::set_starting_experience(w, v);
    }
    commands.remove_resource::<PendingGenerals>();
    let list: Vec<String> = w.kingdoms.iter().map(|k| format!("{}: {:?}", k.name, special_ops::general(w, k.id).map(|g| g.id.clone()))).collect();
    info!("generals: {}", list.join(", "));
}

/// The floater items (`0x6073C0`): icon, command, recharge bar.
pub fn hud_items(w: &World, k: KingdomId) -> Vec<HudAbility> {
    special_ops::abilities(w, k)
        .iter()
        .map(|v| HudAbility {
            texture: v.icon.clone(),
            command: format!("UseFactionAbility {}", v.index),
            recharge: if v.recharge > 0.0 { 1.0 - v.remaining / v.recharge } else { 1.0 },
        })
        .collect()
}

const ITEMS: &str = "Game/FactionAbilitiesFloater/Abilities/ItemPanel";
const ITEM: &str = "FactionAbilitiesFloater/Abilities/ItemPanel/Ability";

fn item(i: usize) -> String {
    if i == 0 { format!("{ITEMS}/Ability") } else { format!("{ITEMS}/Ability#{i}") }
}

/// `0x6073C0`: the button is disabled while recharging (remaining > 0),
/// unaffordable (`4E24B0`) or `4C8A40` gives 9 / 10.
pub fn apply_enabled(ui: &mut Ui, views: &[AbilityView]) {
    for v in views {
        ui.with_widget(&format!("{}/AbilityButton", item(v.index)), |w| w.set_enabled(v.usable));
    }
}

/// The ability item index of a widget path below the floater.
pub fn item_index(path: &str) -> Option<usize> {
    let at = path.find(ITEM)? + ITEM.len();
    match path[at..].strip_prefix('#') {
        Some(n) => n.split('/').next()?.parse().ok(),
        None => Some(0),
    }
}

/// Game/descriptions.tgi: (category, min, max, description key).
fn descriptions(ui: &Ui) -> &'static [(String, f32, f32, String)] {
    static D: OnceLock<Vec<(String, f32, f32, String)>> = OnceLock::new();
    D.get_or_init(|| {
        super::datafile::load(&ui.store.root, "Game/descriptions.tgi")
            .iter()
            .filter(|n| n.name.eq_ignore_ascii_case("Description"))
            .filter_map(|n| {
                let range: Vec<f32> = n.get("range")?.split(',').filter_map(|x| x.trim().parse().ok()).collect();
                Some((n.get("category")?.trim().to_string(), *range.first()?, *range.get(1)?, n.get("description")?.trim_matches('"').to_string()))
            })
            .collect()
    })
}

/// `0x491160`: the description of the category whose [min, max) holds the value.
fn description(ui: &Ui, category: &str, value: f32) -> Option<String> {
    descriptions(ui).iter().find(|(c, lo, hi, _)| c.eq_ignore_ascii_case(category) && *lo <= value && value < *hi).map(|d| ui.strings.resolve(&d.3))
}

/// `0x4CE3A0` for an experience amount: the AVar
/// FontFactionExperienceGlyphIndex glyph and the value.
fn experience_text(ui: &Ui, value: f32) -> String {
    let glyph = ui.avars.float("FontFactionExperienceGlyphIndex").map(|v| v as u32).unwrap_or(0);
    match char::from_u32(glyph).filter(|_| glyph != 0) {
        Some(c) => format!("{c} {value:.0}"),
        None => format!("{value:.0}"),
    }
}

/// `0x6081B0`: (name, text) of an ability item's tooltip.
pub fn tip(ui: &Ui, v: &AbilityView) -> (String, String) {
    let resolve = |k: &str| if k.is_empty() { String::new() } else { super::hud::name(ui, k) };
    let mut rows = Vec::new();
    // 0x63E050: Cost (the ability's experience cost)
    if v.cost > 0.0 {
        rows.push(format!("{}<setx=100>{}", resolve("#ui_tooltip_item_title_cost"), experience_text(ui, v.cost)));
    }
    // 0x63E290: Area, the volley area when it is not smaller
    let (category, value) = if v.area <= v.volley_area { ("AREA_VOLLEY", v.volley_area) } else { ("AREA", v.area) };
    if let Some(d) = description(ui, category, value) {
        rows.push(format!("{}<setx=100>{}", resolve("#ui_tooltip_item_title_area"), d));
    }
    let mut text = rows.join("\n");
    // 0x638D90: the description
    let desc = resolve(&v.description);
    if !desc.is_empty() {
        if !text.is_empty() {
            text.push_str("\n\n");
        }
        text.push_str(&desc);
    }
    // the recharge time left (ftol: whole seconds)
    let left = v.remaining as i32;
    if left > 0 {
        let t = ui.strings.resolve("#ui_floater_ability_recharge_time|dd").replacen("%.2d", &format!("{:02}", left % 60), 1).replacen("%d", &(left / 60).to_string(), 1);
        if !text.is_empty() {
            text.push_str("\n\n");
        }
        text.push_str(&t);
    }
    (resolve(&v.name), text)
}

fn error(ui: &mut Ui, text: String) {
    let uv = ui.uvars.clone();
    ui.messages.send(&uv, "GENERIC_ERROR", &text, None);
}

/// KKC_GameInterface::CommandUseFactionAbility (`0x612750`).
pub fn command(ui: &mut Ui, views: &[AbilityView], index: usize) -> Option<GameAction> {
    let Some(v) = views.get(index) else {
        info!("invalid ability index {index} for the local faction");
        return None;
    };
    let name = super::hud::name(ui, &v.name);
    if v.remaining > 0.0 {
        let t = ui.strings.resolve("#ui_error_faction_ability_recharge_time|s").replacen("%s", &name, 1);
        error(ui, t);
        return None;
    }
    if !v.affordable {
        let t = ui.strings.resolve("#ui_error_faction_ability_cost|s").replacen("%s", &name, 1);
        error(ui, t);
        return None;
    }
    if v.available == 9 || v.available == 10 {
        let summon = v.summon_name.as_deref().map(|s| super::hud::name(ui, s)).unwrap_or_default();
        let t = ui.strings.resolve("#ui_error_faction_ability_already_summoned|ss").replacen("%s", &name, 1).replacen("%s", &summon, 1);
        error(ui, t);
        return None;
    }
    // +SELF (definition targets +0x28 bit 5): the order goes out at once
    if v.self_target { Some(GameAction::UseAbility(index)) } else { Some(GameAction::Target(TargetOrder::Ability(index))) }
}

/// The targeting state's valid cursor: the clicked actor (or, where the
/// ability takes a location, the ground) is admitted (`4C8F20`).
pub fn valid_target(w: &World, k: KingdomId, index: usize, hover: Option<ActorId>) -> bool {
    let mut t = hover.map(OpTarget::Actor);
    special_ops::admit(w, k, index, &mut t).is_ok()
}
