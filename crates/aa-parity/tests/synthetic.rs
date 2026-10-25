//! Synthetic films: a TGCK file built here (raw chunks), with Orders records
//! encoded through the same aa-orders schemas the reader decodes with.
use aa_orders::Position;
use aa_orders::commands::WireWriter;
use aa_orders::payload::{self, Field, ThingCheck};
use aa_parity::decree::{Decree, id};
use aa_parity::registries::{ACTOR_COMMANDS, OBJECT_DEFINITIONS, Provenance, Registries};
use aa_parity::slog::{self, Outcome, SEntryKind, SLog};
use aa_parity::{Checkpoint, Film};

fn word(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_le_bytes());
}
fn string(out: &mut Vec<u8>, text: &str) {
    let units: Vec<_> = text.encode_utf16().collect();
    out.extend_from_slice(&(units.len() as u16).to_le_bytes());
    for u in units {
        out.extend_from_slice(&u.to_le_bytes());
    }
}
fn rules(out: &mut Vec<u8>, mode: u32) {
    word(out, 4);
    word(out, mode);
    string(out, "NORMAL");
    word(out, 0);
}
fn parameters(out: &mut Vec<u8>) {
    word(out, 13);
    word(out, 0);
    word(out, 0);
}
/// A film MAP_HEADER: header rules in film mode 3, random creator.
fn map_header(seed: u32) -> Vec<u8> {
    let mut b = Vec::new();
    word(&mut b, 7);
    rules(&mut b, 3);
    parameters(&mut b);
    word(&mut b, 7);
    word(&mut b, 0);
    string(&mut b, "map_name");
    string(&mut b, "map_description");
    word(&mut b, 384.0_f32.to_bits());
    word(&mut b, 384.0_f32.to_bits());
    word(&mut b, 2004.0_f32.to_bits());
    word(&mut b, 123);
    string(&mut b, "1.0");
    string(&mut b, "A&A");
    string(&mut b, "test");
    b.push(0);
    string(&mut b, "");
    string(&mut b, "");
    word(&mut b, 0);
    word(&mut b, 1);
    word(&mut b, 8);
    b.push(0);
    string(&mut b, "KKC_WorldCreatorRandom");
    word(&mut b, 3);
    rules(&mut b, 0);
    parameters(&mut b);
    word(&mut b, 384.0_f32.to_bits());
    word(&mut b, 384.0_f32.to_bits());
    string(&mut b, "chaparral");
    string(&mut b, "");
    word(&mut b, seed);
    word(&mut b, 1);
    string(&mut b, "rmc_test");
    word(&mut b, 0);
    word(&mut b, 120.0_f32.to_bits());
    word(&mut b, 0);
    word(&mut b, 0);
    b
}
fn session_players() -> Vec<u8> {
    let mut b = Vec::new();
    word(&mut b, 2);
    for (id, name, kingdom) in [(1, "side01", 0), (2, "side02", 1)] {
        word(&mut b, id);
        string(&mut b, name);
        word(&mut b, kingdom);
    }
    b
}
/// One Orders packet from (descriptor, fields) pairs.
fn packet(decrees: &[(usize, Vec<Field>)], registries: &Registries) -> Vec<u8> {
    let mut w = WireWriter::new(Position::PACKET_START).unwrap();
    for (descriptor, fields) in decrees {
        w.write_uint(*descriptor as u32, 9).unwrap();
        payload::encode(*descriptor, fields, &mut w, registries).unwrap();
    }
    aa_orders::net::transport::finalize(&w).unwrap()
}
fn u32f(value: u32) -> Field {
    Field::Uint { width: 32, value }
}
fn tgck(chunks: &[(&str, Vec<u8>)]) -> Vec<u8> {
    let mut out = b"TGCK".to_vec();
    word(&mut out, 2);
    word(&mut out, chunks.len() as u32);
    let mut directory = Vec::new();
    for (name, bytes) in chunks {
        let offset = out.len() as u64;
        out.extend_from_slice(bytes);
        let mut row = [0u8; 96];
        for (i, u) in name.encode_utf16().enumerate() {
            row[2 * i..2 * i + 2].copy_from_slice(&u.to_le_bytes());
        }
        row[64..72].copy_from_slice(&offset.to_le_bytes());
        row[72..80].copy_from_slice(&(bytes.len() as u64).to_le_bytes());
        row[88..96].copy_from_slice(&(bytes.len() as u64).to_le_bytes());
        directory.extend_from_slice(&row);
    }
    out.extend(directory);
    out
}
fn registries() -> Registries {
    Registries::builtin().with_count(OBJECT_DEFINITIONS, 1959, Provenance::Proven)
}
/// Move command (0x61EBA0): flags bit6 (ActorCommand) + bit2 (position).
fn move_fields(x: f32, y: f32) -> Vec<Field> {
    vec![
        Field::Bounded {
            n: 0x800,
            value: 0x44,
        },
        Field::Def {
            registry: ACTOR_COMMANDS,
            index: 5,
        },
        u32f(x.to_bits()),
        u32f(y.to_bits()),
        Field::Uint { width: 1, value: 0 },
    ]
}
/// Recruit command: bit6 + bit1 (object definition).
fn recruit_fields(object: u32) -> Vec<Field> {
    vec![
        Field::Bounded {
            n: 0x800,
            value: 0x42,
        },
        Field::Def {
            registry: ACTOR_COMMANDS,
            index: 15,
        },
        Field::Def {
            registry: OBJECT_DEFINITIONS,
            index: object,
        },
        Field::Uint { width: 1, value: 0 },
    ]
}
fn film_bytes() -> Vec<u8> {
    let r = registries();
    let ticks = vec![
        packet(
            &[
                (id::SET_PLAYER, vec![u32f(1)]),
                (
                    id::SELECTION_SET,
                    vec![Field::Thing {
                        check: ThingCheck::B0,
                        slot: 7,
                    }],
                ),
            ],
            &r,
        ),
        packet(&[], &r),
        packet(
            &[
                (id::SET_PLAYER, vec![u32f(1)]),
                (id::TELL_SELECTED_COMMAND, move_fields(10.5, -3.25)),
            ],
            &r,
        ),
        packet(
            &[
                (id::SET_PLAYER, vec![u32f(2)]),
                (id::SIM_SYNC_VALUE, vec![u32f(4), u32f((-1234i32) as u32)]),
            ],
            &r,
        ),
        packet(
            &[
                (id::SET_PLAYER, vec![u32f(2)]),
                (id::TELL_SELECTED_COMMAND, recruit_fields(42)),
            ],
            &r,
        ),
    ];
    let mut orders = Vec::new();
    for t in ticks {
        word(&mut orders, t.len() as u32);
        orders.extend(t);
    }
    tgck(&[
        ("MAP", vec![0, 0, 0, 0]),
        ("MAP_HEADER", map_header(972_257_557)),
        ("MAPSTRINGS", vec![1, 2, 3]),
        ("SessionPlayers", session_players()),
        ("Orders", orders),
    ])
}

#[test]
fn synthetic_film_decodes_into_a_per_tick_timeline() {
    let film = Film::from_bytes(film_bytes(), &registries()).unwrap();
    assert_eq!(film.chunks.len(), 5);
    assert_eq!(film.header.creator_class, "KKC_WorldCreatorRandom");
    assert_eq!(film.header.seed, 972_257_557);
    assert_eq!(film.header.rmc.as_deref(), Some("rmc_test"));
    assert_eq!(film.players.len(), 2);
    assert_eq!(film.players[1].kingdom_index, Some(1));
    assert_eq!(film.world_ticks(), 5);
    assert_eq!(film.non_empty_ticks(), 4);
    assert_eq!(film.decree_count(), 8);
    assert_eq!(film.failures().count(), 0);
    assert_eq!(
        film.checkpoints,
        vec![Checkpoint {
            tick: 3,
            count: 4,
            hash: -1234
        }]
    );
    // Selection and the ordering player carry the native meaning.
    let t0 = film.decrees_at(0);
    assert_eq!(t0[1].decree, Decree::SelectionSet { slot: 7 });
    assert_eq!(t0[1].player, Some(1));
    let Decree::TellSelectedCommand(wire) = &film.decrees_at(2)[1].decree else {
        panic!("command expected")
    };
    assert_eq!(wire.command, Some(5));
    assert_eq!(wire.position, [10.5_f32.to_bits(), (-3.25_f32).to_bits()]);
    let Decree::TellSelectedCommand(wire) = &film.decrees_at(4)[1].decree else {
        panic!("command expected")
    };
    assert_eq!((wire.command, wire.object), (Some(15), Some(42)));
    assert_eq!(film.decrees_at(4)[1].player, Some(2));
    assert!(film.decrees_at(1).is_empty());
    let h = film.histogram();
    assert_eq!(h["298 TGC_SetPlayerOrderDecree"], 4);
    assert_eq!(h[" 86 KKC_TellSelectedCommandOrderDecree"], 2);
}

#[test]
fn unknown_registry_counts_stop_a_record_explicitly_and_keep_earlier_decrees() {
    // Without the Thing registry the recruit object reference cannot be read.
    let film = Film::from_bytes(film_bytes(), &Registries::builtin()).unwrap();
    let failed: Vec<_> = film.failures().map(|t| t.tick).collect();
    assert_eq!(failed, vec![4]);
    let t4 = &film.ticks[4];
    assert_eq!(
        t4.decrees.len(),
        1,
        "the SetPlayer before the failure is kept"
    );
    assert!(t4.error.as_ref().unwrap().message.contains("0xb39f58"));
}

#[test]
fn slog_reproduces_native_hashing_and_checks_checkpoints_by_index() {
    // Reset 0x77D900(0, 0): first entry index 1, hash 0.
    let mut log = SLog::new();
    assert_eq!(log.checkpoint(), (0, 0));
    for update in 1..=3u32 {
        log.push(
            update,
            SEntryKind::Clock,
            slog::clock_bits_before_update(update),
        );
    }
    // Clock values k/16 have zero low 16 bits: the hash stays 0.
    assert_eq!(log.checkpoint(), (3, 0));
    log.push(3, SEntryKind::RngGetter, 0x8e4b_12a5);
    log.push(3, SEntryKind::Marker("m"), 77);
    log.push(4, SEntryKind::Bool("b"), 5);
    let mut h = 0i32;
    for v in [0x8e4b_12a5u32, 0, 1] {
        h = aa_orders::net::sync::step(h, v);
    }
    assert_eq!(log.checkpoint(), (6, h));
    let at4 = log.entry(4).unwrap();
    let checks = [
        Checkpoint {
            tick: 3,
            count: 4,
            hash: at4.3,
        },
        Checkpoint {
            tick: 4,
            count: 6,
            hash: h ^ 1,
        },
        Checkpoint {
            tick: 9,
            count: 9,
            hash: 0,
        },
    ];
    let c = log.compare(&checks);
    let outcomes: Vec<_> = c.results.iter().map(|r| r.outcome).collect();
    assert_eq!(
        outcomes,
        vec![Outcome::Match, Outcome::Mismatch, Outcome::NotReached]
    );
    assert_eq!(c.results[0].our_tick, Some(3));
    assert_eq!(c.results[0].our_count_at_film_tick, 5);
    assert_eq!(c.matched(), 1);
    assert_eq!(log.checkpoint_after_tick(0), (0, 0));
}

#[test]
fn clock_entries_follow_the_native_begin_time() {
    // GWorld+0xE0 before update k: 0, 0, then (k - 2) / 16.
    let values: Vec<f32> = (1..=5)
        .map(|k| f32::from_bits(slog::clock_bits_before_update(k)))
        .collect();
    assert_eq!(values, vec![0.0, 0.0, 0.0625, 0.125, 0.1875]);
    // A clock-only interval multiplies the hash by 37 per tick.
    let checks = [
        Checkpoint {
            tick: 17,
            count: 30,
            hash: 100,
        },
        Checkpoint {
            tick: 19,
            count: 32,
            hash: aa_orders::net::sync::step(aa_orders::net::sync::step(100, 0), 0),
        },
    ];
    assert_eq!(slog::clock_only_intervals(&checks), vec![(17, 2, true)]);
}

#[cfg(feature = "play")]
mod mapping {
    use super::*;
    use aa_parity::mapping::{Mapped, Mapper, NativeIds};
    use aa_play::{ActorId, Command, DefId, KingdomId, OrgId, Unit};

    struct Ids;
    impl NativeIds for Ids {
        fn kingdom(&self, player: u32) -> Option<KingdomId> {
            Some(player - 1)
        }
        fn thing(&self, slot: u16) -> Option<Unit> {
            match slot {
                7 => Some(Unit::Org(3)),
                _ => None,
            }
        }
        fn gtid(&self, _: u32) -> Option<Unit> {
            None
        }
        fn target_actor(&self, unit: Unit) -> Option<ActorId> {
            match unit {
                Unit::Actor(a) => Some(a),
                Unit::Org(_) => None,
            }
        }
        fn definition(&self, object: u32) -> Option<DefId> {
            Some(object + 1000)
        }
        fn formation(&self, _: OrgId, _: u32) -> Option<usize> {
            None
        }
        fn property(&self, _: u32) -> Option<String> {
            None
        }
        fn general_ability(&self, _: KingdomId, _: u32) -> Option<u32> {
            None
        }
        fn named(&self, _: &[u16]) -> Option<Unit> {
            None
        }
    }

    #[test]
    fn decrees_map_onto_aa_play_commands_through_the_id_table() {
        let film = Film::from_bytes(film_bytes(), &registries()).unwrap();
        let mut mapper = Mapper::new();
        let mut out = Vec::new();
        for t in &film.ticks {
            for d in &t.decrees {
                out.push(mapper.map(d, &Ids));
            }
        }
        assert_eq!(out[0], Mapped::State);
        assert_eq!(out[1], Mapped::State);
        assert_eq!(
            out[3],
            Mapped::Commands(vec![Command::Move {
                kingdom: 0,
                units: vec![Unit::Org(3)],
                to: glam::Vec2::new(10.5, -3.25),
                attack_move: false
            }])
        );
        assert_eq!(out[5], Mapped::NoEffect("sync checkpoint"));
        // Player 2 has an empty selection: the recruit has no building.
        assert!(matches!(out[7], Mapped::Unresolved(_)));
    }
}
