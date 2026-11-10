//! Team command flares of the map scripts as the view shows them: the trigger
//! actions team_command_flare_on_position / team_command_flare_on_actor
//! (544A50 / 544BA0 -> 572E50) with a flare thing of Data/Flares.
//!
//! The original makes the flare an actor of the kingdom (4FA030) with a
//! FlareComponent:
//! * the component's data (464BD0, 0x4681B8..0x468283; the defaults are
//!   Templates/Components/template_flare.tgi): `default_duration` (ActorData
//!   +560), `actor_offset` (+564, the template's own words: "if attached to
//!   another actor, this is the offset at which the flare should be
//!   displayed"), `all_ally_color` (+570), `command` (+57C), `target_actor` /
//!   `target_location` (+580 / +581);
//! * its time (572E50): a value below 0 (`[0x9F32DC]`, 0.0) takes
//!   `default_duration`; only a time above 0 runs out (component +3C), so 0
//!   never ends. Each tick (573290) the time goes down by the tick, and 5723A0
//!   removes the flare (4F6760) when the time is out, when its kingdom is
//!   gone, or when its target actor is gone; an actor flare stands where its
//!   target stands (4EE5C0 with the target's +20..+28);
//! * the game event (572E50, 0x5730B5..0x573222): my_flare_created (0x19) or
//!   my_flare_created_on_actor (0x1B) when the local kingdom is the flare's;
//!   ally_flare_created(_on_actor) (0x1A / 0x1C) only when an ally of the local
//!   kingdom made it and its recipient kingdom is none or the local one
//!   (0x57316A..0x573174). The trigger action passes the flare's kingdom as
//!   the recipient (544A50 / 544BA0 hand 572E50 the same kingdom twice), so an
//!   ally hears nothing of a scripted flare.
//!
//! Compare against the original (labelled rendition): aa-play has no flare
//! actor (`aa_play::triggers::Notice::Flare`), so the effects system shows the
//! flare's model where it starts (`ui::order_marks::TransportRun` without a
//! way, `effects` trigger_transport) for its time: an actor flare at its
//! target's position of that moment raised by `actor_offset`, a location flare
//! on the ground under its point (the location flares are not flying things,
//! Flares/*.tgi). It does not follow a moving target or end with its target.
use super::datafile::{self, Node};
use std::collections::HashMap;

/// The FlareComponent fields the view needs.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct FlareData {
    /// `default_duration` (ActorData +560).
    pub default_duration: f32,
    /// `actor_offset` (ActorData +564), native x, y, z.
    pub actor_offset: [f32; 3],
    /// `target_actor` (ActorData +580).
    pub target_actor: bool,
}

/// The flare things' FlareComponents by IDS (lower case).
#[derive(Clone, Debug, Default)]
pub struct FlareTable {
    base: FlareData,
    defs: HashMap<String, FlareData>,
}

fn point3(v: &str) -> Option<[f32; 3]> {
    let mut it = v.split(',').map(|x| x.trim().trim_end_matches(['f', 'F']).parse::<f32>());
    let p = [it.next()?.ok()?, it.next()?.ok()?, it.next()?.ok()?];
    it.next().is_none().then_some(p)
}

/// The fields a FlareComponent node sets over `d`.
fn apply(d: &mut FlareData, n: &Node) {
    if let Some(v) = n.float("default_duration") {
        d.default_duration = v;
    }
    if let Some(p) = n.get("actor_offset").and_then(point3) {
        d.actor_offset = p;
    }
    if let Some(v) = n.boolean("target_actor") {
        d.target_actor = v;
    }
}

impl FlareTable {
    /// The template's defaults, then a flare template's FlareComponent, then
    /// the thing's own.
    pub fn from_nodes(template: &[Node], flares: &[Vec<Node>]) -> Self {
        let mut base = FlareData::default();
        for n in template.iter().filter(|n| n.args.iter().any(|a| a.eq_ignore_ascii_case("FlareComponent"))) {
            apply(&mut base, n);
        }
        let mut templates: HashMap<String, FlareData> = HashMap::new();
        let mut defs = HashMap::new();
        for nodes in flares {
            for n in nodes.iter().filter(|n| n.name.eq_ignore_ascii_case("Template")) {
                let Some(name) = n.args.first() else { continue };
                let mut d = base;
                if let Some(c) = n.child("FlareComponent") {
                    apply(&mut d, c);
                }
                templates.insert(name.to_ascii_lowercase(), d);
            }
            for n in nodes.iter().filter(|n| n.name.eq_ignore_ascii_case("Thing")) {
                let Some(ids) = n.get("IDS") else { continue };
                let mut d = n.template().and_then(|t| templates.get(&t.to_ascii_lowercase())).copied().unwrap_or(base);
                if let Some(c) = n.child("FlareComponent") {
                    apply(&mut d, c);
                }
                defs.insert(ids.to_ascii_lowercase(), d);
            }
        }
        Self { base, defs }
    }

    pub fn load(data: &aa_ui::fs::DataRoot) -> Self {
        let template = datafile::load(data, "Templates/Components/template_flare.tgi");
        let flares: Vec<Vec<Node>> =
            data.list_dir("Flares").iter().filter(|f| f.to_ascii_lowercase().ends_with(".tgi")).map(|f| datafile::load(data, &format!("Flares/{f}"))).collect();
        Self::from_nodes(&template, &flares)
    }

    pub fn get(&self, ids: &str) -> FlareData {
        self.defs.get(&ids.to_ascii_lowercase()).copied().unwrap_or(self.base)
    }
}

/// 572E50: the flare's time from the trigger's value; None never ends.
pub fn duration(value: f32, d: &FlareData) -> Option<f32> {
    let t = if value < 0.0 { d.default_duration } else { value };
    (t > 0.0).then_some(t)
}

/// Where the view shows the flare's model (native x, y, z): an actor flare
/// raised by `actor_offset` over its target's point, a location flare on
/// the `ground` under its point.
pub fn shown_at(at: [f32; 3], on_actor: bool, d: &FlareData, ground: f32) -> [f32; 3] {
    if on_actor {
        [at[0] + d.actor_offset[0], at[1] + d.actor_offset[1], at[2] + d.actor_offset[2]]
    } else {
        [at[0], at[1], ground]
    }
}

/// 572E50: the game event the local player gets for a scripted flare of
/// `kingdom`: only its own (the trigger's recipient is the flare's kingdom).
pub fn event_for(local: u32, kingdom: u32) -> bool {
    local == kingdom
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEMPLATE: &str = "[template FlareComponent]\n{\n\tstring\tcommand\n\tfixed\tdefault_duration\t= 180\n\tpoint3\tactor_offset\t\t= 0,0,7\t\t;; if attached to another actor\n\tpoint3\tall_ally_color\t\t= 1,1,1\n\tflag\ttarget_actor\t\t= false\n\tflag\ttarget_location\t\t= false\n}\n";
    const ATTACK: &str = "[Template AAFlareAttack Inherit=Actor]\n{\n\tcategory = SPECIAL\n\t[Flags]\n\t{\n\t\tflying = false\n\t}\n\t[View]\n\t{\n\t\tresource_prefix = FlareAttack/FlareAttack\n\t}\n}\n\n[Thing Template=AAFlareAttack]\n{\n\tIDS = attack_actor_flare\n\t[Flags]\n\tflying = true\n\n\t[FlareComponent]\n\t{\n\t\ttarget_actor = true\n\t\tcommand = kill\n\t}\n}\n\n[Thing Template=AAFlareAttack]\n{\n\tIDS = attack_location_flare\n\t[FlareComponent]\n\t{\n\t\ttarget_location = true\n\t\tcommand = kill\n\t}\n}\n";

    fn table() -> FlareTable {
        FlareTable::from_nodes(&datafile::parse(TEMPLATE), &[datafile::parse(ATTACK)])
    }

    #[test]
    fn component_fields_resolve_over_the_template() {
        let t = table();
        assert_eq!(t.get("attack_actor_flare"), FlareData { default_duration: 180.0, actor_offset: [0.0, 0.0, 7.0], target_actor: true });
        assert_eq!(t.get("ATTACK_LOCATION_FLARE"), FlareData { default_duration: 180.0, actor_offset: [0.0, 0.0, 7.0], target_actor: false });
    }

    #[test]
    fn time_follows_572e50() {
        let d = table().get("attack_actor_flare");
        // allies02 300 s, allies04 999999 s (mi_int), axis10 1e9 s (mi_fixed)
        assert_eq!(duration(300.0, &d), Some(300.0));
        assert_eq!(duration(999_999.0, &d), Some(999_999.0));
        assert_eq!(duration(-1.0, &d), Some(180.0));
        assert_eq!(duration(0.0, &d), None);
    }

    #[test]
    fn shown_point() {
        let t = table();
        assert_eq!(shown_at([10.0, 20.0, 3.0], true, &t.get("attack_actor_flare"), 5.0), [10.0, 20.0, 10.0]);
        assert_eq!(shown_at([186.22, 182.25, 0.0], false, &t.get("attack_location_flare"), 4.5), [186.22, 182.25, 4.5]);
    }

    #[test]
    fn scripted_flare_event_is_the_kingdoms_own() {
        assert!(event_for(1, 1));
        assert!(!event_for(0, 1));
    }
}
