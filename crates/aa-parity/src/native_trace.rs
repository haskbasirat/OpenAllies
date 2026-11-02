//! Reader of the native state sampler's trace (QA tool `NativeSampler`, a
//! debugger attached to the original AA.exe while it plays a film).
//!
//! After every GWorld update (breakpoint 0x76F850, right after the 0x4F9970
//! call) the sampler records, with every float as its binary32 bits:
//! * `T <tick> rng <GWorld+D4>`;
//! * `K <tick> <Kingdom+188 index> ptr .. gtid .. prov= eff= cons= stock= short=`:
//!   the Kingdom resource vectors +1A0, +1AC, +1B8, +1C4 and the shortage
//!   ratio +1F0;
//! * `N <gtid> <definition IDS>` once per thing;
//! * `A <tick> <gtid> key=value...` for every class-3 GActor of the GTID
//!   table (`[B364C0]`): flags `+8`, position `+20/+24/+28`, direction
//!   `+2C/+30`, the active component slot (`+E4`), the state word `+100`, Body
//!   health `+10/+14`, Mover destination `+10/+14`, speed `+20`, `+24`, maximum
//!   `+28`, falling `+56`, Organization leader `+40`, formation `+70` and
//!   members (`+28/+2C`), Structure queue count `+18`, deadline `+20`, paused
//!   `+24`, `+28`, duration `+2C` and head item words, Supply `+18/+1C`,
//!   Detection `+10/+14`, and every zone of `+148/+14C` (`KKC_Zone` radius
//!   `+8`, tile `+C/+10`, flags `+14`, state `+16`).
//!
//! It also keeps RngTrace's getter lines (`G`), from which the world-RNG
//! state at every getter is taken. The trace stays in the QA copy.
use serde::Serialize;
use std::collections::BTreeMap;

#[derive(Clone, Debug, Default, Serialize)]
pub struct NativeKingdom {
    pub index: u32,
    pub gtid: u32,
    pub provided: Vec<u32>,
    pub effective: Vec<u32>,
    pub consumed: Vec<u32>,
    pub stockpiled: Vec<u32>,
    pub shortage: u32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
pub struct NativeMover {
    pub dest: [u32; 2],
    pub speed: u32,
    pub word_24: u32,
    pub max_speed: u32,
    pub falling: u8,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct NativeOrganization {
    pub leader: u32,
    pub formation: u32,
    pub members: Vec<u32>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
pub struct NativeQueue {
    pub count: u32,
    pub deadline: u32,
    pub paused: u32,
    pub word_28: u32,
    pub duration: u32,
    pub head: Option<[u32; 4]>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
pub struct NativeZone {
    pub slot: u32,
    pub radius: u32,
    pub x: i32,
    pub y: i32,
    pub flags: u16,
    pub state: u8,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct NativeActor {
    pub gtid: u32,
    pub flags: u32,
    pub pos: [u32; 3],
    pub dir: [u32; 2],
    pub active: i32,
    pub state: u32,
    pub health: Option<[u32; 2]>,
    pub mover: Option<NativeMover>,
    pub organization: Option<NativeOrganization>,
    pub queue: Option<NativeQueue>,
    pub supply: Option<[u32; 2]>,
    pub detection: Option<[u32; 2]>,
    pub zones: Vec<NativeZone>,
    /// `mor=` of the fight traces (`FightSampler`): Organization morale +5C,
    /// maximum +60, breakpoint +B8, living +64, combat value +68 / +6C, attack
    /// efficiency +84 / +88, watched target +D4.
    pub morale: Option<[u32; 9]>,
    /// `ms=` of the fight traces: the Mind's vtable and its current state's vtable.
    pub mind: Option<[u32; 2]>,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct NativeTick {
    pub tick: u32,
    pub rng: u32,
    pub kingdoms: Vec<NativeKingdom>,
    pub actors: BTreeMap<u32, NativeActor>,
}

/// One world-RNG getter 0x4F02B0 of the original (`G` line): the GWorld tick
/// counter when it ran (decrees of record `r` run at tick `r`, update `u` at
/// tick `u`), the state before its draw, the call site and the return
/// addresses the sampler's stack scan found (nearest first; a scan, so an
/// entry can be stale).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct NativeGetter {
    pub tick: u32,
    pub state: u32,
    pub site: u32,
    pub chain: Vec<u32>,
    /// The last sampled tick before it in the trace (0: none yet). The
    /// snapshot at tick `t` closes after update `t`, so the getters with
    /// `after == t` are the decrees of record `t` and the updates up to the
    /// next sampled tick: the replay's steps `t + 1 ..= next`.
    pub after: u32,
}

/// One thing creation of the original (`M ... CreateGThing_4F9C10` line).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct NativeCreation {
    pub tick: u32,
    pub definition: String,
    pub ret: u32,
}

#[derive(Clone, Debug, Default)]
pub struct NativeTrace {
    pub ticks: BTreeMap<u32, NativeTick>,
    /// GTID → definition IDS.
    pub names: BTreeMap<u32, String>,
    /// Every world-RNG getter of the battle and of the world creation.
    pub getters: Vec<NativeGetter>,
    /// Every thing created after the battle started.
    pub creations: Vec<NativeCreation>,
}

/// What a world-RNG getter belongs to, from its call site and the nearest
/// return addresses. The Berlin film has exactly nine getter sites:
/// * 0x50B2E4 periodic registration 0x50B2C0, by queue and registrant;
/// * 0x530384 ActionGo 0x5300A0 (re-think delay), by the Mind state that
///   runs it;
/// * 0x4E9FF8 random scale (InitCreate 0x4E9FC0), 0x530E5F element Mind
///   jitter, 0x50AB60 the mover's countdown phase, 0x4E2663 the kingdom's HQ
///   number;
/// * 0x563C59 / 0x563D12 Character 0x563BA0 / 0x563CA0 (animation pick),
///   0x590323 the Disturbable delay 0x5902C0.
pub fn getter_group(site: u32, chain: &[u32]) -> String {
    let c = |i: usize| chain.get(i).copied().unwrap_or(0);
    match site {
        0x50B2E4 => match c(0) {
            0x50AC27 => match c(1) {
                0x577489 | 0x577469 => "mover order 5773C0 -> queue 0".into(),
                0x5894AD => "production start 589420 -> queue 0".into(),
                0x57C751 => "organization goal 57C680 -> queue 0".into(),
                0x580BE1 => "projectile launch 580BC0 -> queue 0".into(),
                o => format!("queue 0 from {o:x}"),
            },
            0x50AC87 => match c(1) {
                0x563FDA => match c(2) {
                    0x4D3068 => "unpack end 4D21A0 -> component 563FD0 -> queue 1".into(),
                    _ => "component InitCreate 563FD0 -> queue 1".into(),
                },
                0x56F26C => "economy InitCreate 56F250 -> queue 1".into(),
                0x56438E => "564300 -> queue 1".into(),
                0x57465D => "hq InitCreate 5745F0 -> queue 1".into(),
                o => format!("queue 1 from {o:x}"),
            },
            o => format!("periodic registration from {o:x}"),
        },
        0x530384 => match c(0) {
            0x537251 | 0x5372A3 | 0x53728B | 0x53725F => "ActionGo from 537150".into(),
            0x537A71 | 0x537A9B | 0x5379F7 | 0x537A8F | 0x537AE4 => {
                "ActionGo from continuation 5379C0".into()
            }
            0x5330DA => "ActionGo from get-back-to-path 532B50".into(),
            0x533699 => "ActionGo from 533140".into(),
            0x533C10 => "ActionGo from low path 5336B0".into(),
            0x537005 | 0x537106 | 0x53713A | 0x537142 | 0x5370A4 => "ActionGo from 536F40".into(),
            0x531766 => "ActionGo from 531520".into(),
            0x536DBA | 0x536BEE => "ActionGo from queue wait 536AD0".into(),
            o => format!("ActionGo from {o:x}"),
        },
        0x4E9FF8 => "random scale InitCreate 4E9FC0".into(),
        0x530E5F => "element mind jitter 530E50".into(),
        0x50AB60 => "mover countdown phase 50AB10".into(),
        0x4E2663 => "HQ number 4E2640".into(),
        0x563C59 => match c(0) {
            0x4D6FE0 => "character 563BA0 at creation".into(),
            0x530DA9 => "character 563BA0 from node reached 530AE0".into(),
            0x563EB9 => "character 563BA0 queue 1 tick 563E10".into(),
            o => format!("character 563BA0 from {o:x}"),
        },
        0x563D12 => "character 563CA0 queue 1 tick 563E10".into(),
        0x590323 => "disturbable delay 5902C0".into(),
        0x5798BB | 0x5798CD => "organization scatter 5797E0".into(),
        s => format!("site {s:x}"),
    }
}

fn hex(s: &str) -> u32 {
    u32::from_str_radix(s, 16).unwrap_or(0)
}
fn list(s: &str) -> Vec<u32> {
    if s.is_empty() {
        Vec::new()
    } else {
        s.split(',').map(hex).collect()
    }
}

impl NativeTrace {
    pub fn read(path: &std::path::Path) -> std::io::Result<Self> {
        Ok(Self::parse(&std::fs::read_to_string(path)?))
    }

    pub fn parse(text: &str) -> Self {
        let mut out = NativeTrace::default();
        let mut sampled = 0u32;
        for line in text.lines() {
            let mut it = line.split(' ');
            match it.next() {
                Some("T") => {
                    let tick: u32 = it.next().and_then(|v| v.parse().ok()).unwrap_or(0);
                    let rng = it.nth(1).map_or(0, hex);
                    sampled = tick;
                    out.ticks.insert(
                        tick,
                        NativeTick {
                            tick,
                            rng,
                            ..Default::default()
                        },
                    );
                }
                Some("K") => {
                    let tick: u32 = it.next().and_then(|v| v.parse().ok()).unwrap_or(0);
                    let mut k = NativeKingdom {
                        index: it.next().map_or(0, hex),
                        ..Default::default()
                    };
                    let rest: Vec<&str> = it.collect();
                    for (i, t) in rest.iter().enumerate() {
                        if *t == "gtid" {
                            k.gtid = rest.get(i + 1).map_or(0, |v| hex(v));
                        }
                        if let Some((key, value)) = t.split_once('=') {
                            match key {
                                "prov" => k.provided = list(value),
                                "eff" => k.effective = list(value),
                                "cons" => k.consumed = list(value),
                                "stock" => k.stockpiled = list(value),
                                "short" => k.shortage = hex(value),
                                _ => {}
                            }
                        }
                    }
                    if let Some(t) = out.ticks.get_mut(&tick) {
                        t.kingdoms.push(k);
                    }
                }
                Some("N") => {
                    let gtid = it.next().map_or(0, hex);
                    let name = it.next().unwrap_or("").to_owned();
                    out.names.insert(gtid, name);
                }
                Some("G") => {
                    // G seq tTID tick N gw X state S site C chain ...
                    let toks: Vec<&str> = it.collect();
                    let get = |key: &str| {
                        toks.iter()
                            .position(|t| *t == key)
                            .and_then(|i| toks.get(i + 1))
                    };
                    let tick = get("tick").and_then(|v| v.parse().ok()).unwrap_or(0);
                    if tick < 0x0100_0000 {
                        out.getters.push(NativeGetter {
                            tick,
                            state: get("state").map_or(0, |v| hex(v)),
                            site: get("site").map_or(0, |v| hex(v)),
                            chain: get("chain").map_or(Vec::new(), |v| list(v)),
                            after: sampled,
                        });
                    }
                }
                Some("M") => {
                    // M seq tTID tick N CreateGThing_4F9C10 def D ids NAME ret R chain ...
                    let toks: Vec<&str> = it.collect();
                    let get = |key: &str| {
                        toks.iter()
                            .position(|t| *t == key)
                            .and_then(|i| toks.get(i + 1))
                    };
                    let tick: u32 = get("tick").and_then(|v| v.parse().ok()).unwrap_or(u32::MAX);
                    if (1..0x0100_0000).contains(&tick) && toks.contains(&"CreateGThing_4F9C10") {
                        out.creations.push(NativeCreation {
                            tick,
                            definition: get("ids").map_or(String::new(), |v| (*v).to_owned()),
                            ret: get("ret").map_or(0, |v| hex(v)),
                        });
                    }
                }
                Some("A") => {
                    let tick: u32 = it.next().and_then(|v| v.parse().ok()).unwrap_or(0);
                    let gtid = it.next().map_or(0, hex);
                    let mut a = NativeActor {
                        gtid,
                        active: -1,
                        ..Default::default()
                    };
                    for t in it {
                        let Some((key, value)) = t.split_once('=') else {
                            continue;
                        };
                        match key {
                            "f" => a.flags = hex(value),
                            "p" => {
                                let v = list(value);
                                a.pos = [
                                    v.first().copied().unwrap_or(0),
                                    v.get(1).copied().unwrap_or(0),
                                    v.get(2).copied().unwrap_or(0),
                                ];
                            }
                            "d" => {
                                let v = list(value);
                                a.dir = [
                                    v.first().copied().unwrap_or(0),
                                    v.get(1).copied().unwrap_or(0),
                                ];
                            }
                            "mor" => {
                                let v = list(value);
                                let mut m = [0u32; 9];
                                for (slot, x) in m.iter_mut().zip(&v) {
                                    *slot = *x;
                                }
                                a.morale = Some(m);
                            }
                            "ms" => {
                                let v = list(value);
                                a.mind = Some([v.first().copied().unwrap_or(0), v.get(1).copied().unwrap_or(0)]);
                            }
                            "act" => a.active = value.parse().unwrap_or(-1),
                            "st" => a.state = hex(value),
                            "hp" => {
                                let v = list(value);
                                a.health = Some([
                                    v.first().copied().unwrap_or(0),
                                    v.get(1).copied().unwrap_or(0),
                                ]);
                            }
                            "mv" => {
                                let v = list(value);
                                let g = |i: usize| v.get(i).copied().unwrap_or(0);
                                a.mover = Some(NativeMover {
                                    dest: [g(0), g(1)],
                                    speed: g(2),
                                    word_24: g(3),
                                    max_speed: g(4),
                                    falling: value
                                        .rsplit(',')
                                        .next()
                                        .and_then(|f| f.parse().ok())
                                        .unwrap_or(0),
                                });
                            }
                            "org" => {
                                let (head, members) = value.split_once(':').unwrap_or((value, ""));
                                let v = list(head);
                                a.organization = Some(NativeOrganization {
                                    leader: v.first().copied().unwrap_or(0),
                                    formation: v.get(1).copied().unwrap_or(0),
                                    members: list(members),
                                });
                            }
                            "q" => {
                                let (head, item) = value.split_once(':').unwrap_or((value, ""));
                                let mut parts = head.split(',');
                                let count = parts.next().and_then(|c| c.parse().ok()).unwrap_or(0);
                                let v: Vec<u32> = parts.map(hex).collect();
                                let g = |i: usize| v.get(i).copied().unwrap_or(0);
                                let item = list(item);
                                a.queue = Some(NativeQueue {
                                    count,
                                    deadline: g(0),
                                    paused: g(1),
                                    word_28: g(2),
                                    duration: g(3),
                                    head: (item.len() == 4)
                                        .then(|| [item[0], item[1], item[2], item[3]]),
                                });
                            }
                            "sp" => {
                                let v = list(value);
                                a.supply = Some([
                                    v.first().copied().unwrap_or(0),
                                    v.get(1).copied().unwrap_or(0),
                                ]);
                            }
                            "det" => {
                                let v = list(value);
                                a.detection = Some([
                                    v.first().copied().unwrap_or(0),
                                    v.get(1).copied().unwrap_or(0),
                                ]);
                            }
                            "z" => {
                                for z in value.split(';') {
                                    let p: Vec<&str> = z.split(':').collect();
                                    if p.len() == 6 {
                                        a.zones.push(NativeZone {
                                            slot: p[0].parse().unwrap_or(0),
                                            radius: hex(p[1]),
                                            x: p[2].parse().unwrap_or(0),
                                            y: p[3].parse().unwrap_or(0),
                                            flags: u16::from_str_radix(p[4], 16).unwrap_or(0),
                                            state: u8::from_str_radix(p[5], 16).unwrap_or(0),
                                        });
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                    if let Some(t) = out.ticks.get_mut(&tick) {
                        t.actors.insert(gtid, a);
                    }
                }
                _ => {}
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEXT: &str = "\
G 1 t1 tick 0 gw 2 state 8e4b12a5 site 76ab30 chain 5126fd,5a2e19
T 1 rng 5
G 2 t1 tick 2 gw 2 state 4f1e05e7 site 50b2e4 chain 50ac27,5894ad,589eb2
M 3 t1 tick 402 CreateGThing_4F9C10 def 0915d97c ids regiment_rus_hq_infantry ret 5830ac chain 58a414
M 4 t1 tick 402 SpawnMember_4FBF40 ecx 2 gw 2 ret 58334b chain 58a414
T 3 rng 6
G 5 t1 tick 4 gw 2 state 21e5d5bb site 530384 chain 537251,53725f
";

    /// A getter remembers the sampled tick before it, which is what puts the
    /// decrees of a record with the update that follows them.
    #[test]
    fn getters_keep_their_window_and_chain() {
        let t = NativeTrace::parse(TEXT);
        let g: Vec<(u32, u32, u32)> = t
            .getters
            .iter()
            .map(|g| (g.tick, g.state, g.after))
            .collect();
        assert_eq!(
            g,
            vec![(0, 0x8e4b12a5, 0), (2, 0x4f1e05e7, 1), (4, 0x21e5d5bb, 3)]
        );
        assert_eq!(t.getters[1].chain, vec![0x50ac27, 0x5894ad, 0x589eb2]);
        assert_eq!(t.creations.len(), 1, "only CreateGThing lines");
        assert_eq!(t.creations[0].definition, "regiment_rus_hq_infantry");
    }

    #[test]
    fn getters_group_by_site_and_registrant() {
        let t = NativeTrace::parse(TEXT);
        let group = |i: usize| getter_group(t.getters[i].site, &t.getters[i].chain);
        assert_eq!(group(1), "production start 589420 -> queue 0");
        assert_eq!(group(2), "ActionGo from 537150");
        assert_eq!(getter_group(0x590323, &[]), "disturbable delay 5902C0");
    }
}
