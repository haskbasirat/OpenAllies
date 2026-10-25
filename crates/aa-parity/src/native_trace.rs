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
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct NativeTick {
    pub tick: u32,
    pub rng: u32,
    pub kingdoms: Vec<NativeKingdom>,
    pub actors: BTreeMap<u32, NativeActor>,
}

#[derive(Clone, Debug, Default)]
pub struct NativeTrace {
    pub ticks: BTreeMap<u32, NativeTick>,
    /// GTID → definition IDS.
    pub names: BTreeMap<u32, String>,
    /// (tick, state, call site) of every world-RNG getter.
    pub getters: Vec<(u32, u32, u32)>,
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
        for line in text.lines() {
            let mut it = line.split(' ');
            match it.next() {
                Some("T") => {
                    let tick: u32 = it.next().and_then(|v| v.parse().ok()).unwrap_or(0);
                    let rng = it.nth(1).map_or(0, hex);
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
                        out.getters.push((
                            tick,
                            get("state").map_or(0, |v| hex(v)),
                            get("site").map_or(0, |v| hex(v)),
                        ));
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
