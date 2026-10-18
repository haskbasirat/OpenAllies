//! Replays native4E6180 cases through the borrowed-storage entry, not a
//! copied production Registry. Required child stops match the native fixture.
use aa_economy::{EconomyError, kingdom_registry::*};
use serde_json::{Value, json};
use std::collections::BTreeMap;
type R<T> = Result<T, EconomyError>;
fn stop<T>(s: &str) -> R<T> {
    Err(EconomyError::World(s.into()))
}
struct W<'a> {
    c: &'a Value,
    flags: u32,
    lists: BTreeMap<u32, Vec<u32>>,
    capacity: BTreeMap<u32, u32>,
    supply: Vec<u32>,
    class: Vec<u32>,
    counted: u32,
    capital: Option<u32>,
}
impl<'a> W<'a> {
    fn new(c: &'a Value) -> Self {
        let mut lists = BTreeMap::new();
        let mut capacity = BTreeMap::new();
        for (o, cap) in [(0x2c4, 10), (0x2d0, 3), (0x2dc, 5), (0x2e8, 20)] {
            let mut values = vec![];
            if o == 0x2e8 && b(c, "prior_org") {
                values.push(4098);
            }
            lists.insert(o, values);
            capacity.insert(o, u(c, "capacity", cap));
        }
        Self {
            c,
            flags: u(c, "flags", 0),
            lists,
            capacity,
            supply: vec![],
            class: vec![],
            counted: 0,
            capital: b(c, "capital").then_some(4098),
        }
    }
    fn offset(l: ActorList) -> u32 {
        match l {
            ActorList::Structures => 0x2c4,
            ActorList::Settlements => 0x2d0,
            ActorList::Hqs => 0x2dc,
            ActorList::Organizations => 0x2e8,
            ActorList::Flares => panic!("not in native216 domain"),
        }
    }
    fn output(&self, result: &str, service: Option<&str>) -> Value {
        let vectors: serde_json::Map<String, Value> = self
            .lists
            .iter()
            .map(|(o, v)| {
                (
                    format!("{o:#x}"),
                    json!({"values":v,"count":v.len(),"capacity":self.capacity[o]}),
                )
            })
            .collect();
        json!({"result":result,"service":service,"class3":self.class,"flags":self.flags,"counted":self.counted,"supply":self.supply,"capital":self.capital,"vectors":vectors})
    }
}
fn b(c: &Value, key: &str) -> bool {
    c[key].as_bool().unwrap_or(false)
}
fn u(c: &Value, key: &str, default: u32) -> u32 {
    c[key].as_u64().map(|v| v as u32).unwrap_or(default)
}
impl RegistryWorld for W<'_> {
    fn actor(&self, _: u32) -> R<RegistryActor> {
        Ok(RegistryActor {
            class_index: 3,
            class_kind: u(self.c, "kind", 2),
            flags8: self.flags,
            states: self.c["states"].as_u64().unwrap_or(1 << 21),
            economy: b(self.c, "economy"),
            structure: b(self.c, "structure"),
            settlement: b(self.c, "settlement"),
            hq: b(self.c, "hq"),
            organization: b(self.c, "organization"),
            supply: b(self.c, "supply"),
            properties: b(self.c, "properties"),
            component_ac: b(self.c, "mobile"),
            flare: false,
        })
    }
    fn set_flags8(&mut self, _: u32, f: u32) -> R<()> {
        self.flags = f;
        Ok(())
    }
    fn structure_excluded(&mut self, _: u32) -> R<bool> {
        Ok(u(self.c, "structure_handle", 0) != 0)
    }
    fn economy_add(&mut self, _: u32) -> R<()> {
        stop("economy")
    }
    fn economy_remove(&mut self, _: u32) -> R<()> {
        panic!("not add")
    }
    fn refresh_organization(&mut self, _: u32) -> R<()> {
        stop("organization")
    }
    fn counted(&mut self, _: u32) -> R<bool> {
        if self.c.get("dying").is_some() && f32::from_bits(u(self.c, "dying", 0)) >= 0.0 {
            return Ok(false);
        }
        Ok(u(self.c, "source_flags", 0) & ((1 << 7) | (1 << 16)) != 0)
    }
    fn statistics_present(&mut self) -> R<bool> {
        Ok(b(self.c, "statistics"))
    }
    fn statistics_track(&mut self, _: u32, _: bool) -> R<()> {
        stop("statistics")
    }
    fn statistics_classified(&mut self, _: u32) -> R<bool> {
        panic!("stopped before")
    }
    fn add_score(&mut self, _: usize, _: u32) -> R<()> {
        panic!("stopped before")
    }
    fn ownership_track(&mut self, _: u32, _: bool) -> R<()> {
        Ok(())
    } // native null158/noCity domain
    fn property_records(&mut self) -> R<Vec<PropertyApplication>> {
        Ok(if b(self.c, "props") {
            vec![PropertyApplication {
                definition: 17,
                source: 0,
                origin: None,
            }]
        } else {
            vec![]
        })
    }
    fn apply_property(&mut self, _: u32, _: PropertyApplication) -> R<()> {
        stop("property")
    }
    fn remove_property_effect(&mut self, _: u32, _: PropertyApplication) -> R<()> {
        panic!("not add")
    }
    fn structure_properties(&mut self, _: u32) -> R<Vec<u32>> {
        stop("research")
    }
    fn add_kingdom_property(&mut self, _: u32, _: u32) -> R<()> {
        panic!("stopped before")
    }
    fn remove_kingdom_property(&mut self, _: u32, _: u32) -> R<()> {
        panic!("not add")
    }
    fn max_flares(&mut self) -> R<i32> {
        panic!("not in native216 domain")
    }
    fn flare_time(&mut self, _: u32) -> R<u32> {
        panic!("not in native216 domain")
    }
    fn destroy_actor(&mut self, _: u32) -> R<()> {
        stop("destroy")
    }
}
impl StoredRegistryWorld for W<'_> {
    fn class_insert_head(&mut self, index: u32, a: u32) -> R<()> {
        assert_eq!(index, 3);
        self.class.insert(0, a);
        Ok(())
    }
    fn list_push(&mut self, l: ActorList, a: u32) -> R<()> {
        let o = Self::offset(l);
        let values = self.lists.get_mut(&o).unwrap();
        let count = values.len() as u32 + 1;
        let old = self.capacity[&o];
        let twice = old.wrapping_mul(2);
        let cap = if (count as i32) > (twice as i32) {
            count
        } else if (count as i32) > (old as i32) {
            twice
        } else {
            old
        };
        self.capacity.insert(o, cap);
        values.push(a);
        Ok(())
    }
    fn list_insert_head(&mut self, _: ActorList, _: u32) -> R<()> {
        panic!("not in native216 domain")
    }
    fn list_len(&self, l: ActorList) -> R<usize> {
        Ok(self.lists[&Self::offset(l)].len())
    }
    fn list_at(&self, l: ActorList, index: usize) -> R<u32> {
        Ok(self.lists[&Self::offset(l)][index])
    }
    fn list_contains(&self, l: ActorList, a: u32) -> R<bool> {
        Ok(self.lists[&Self::offset(l)].contains(&a))
    }
    fn supply_contains(&self, a: u32) -> R<bool> {
        Ok(self.supply.contains(&a))
    }
    fn supply_insert_head(&mut self, a: u32) -> R<()> {
        self.supply.insert(0, a);
        Ok(())
    }
    fn increment_counted(&mut self) -> R<()> {
        self.counted = self.counted.wrapping_add(1);
        Ok(())
    }
    fn capital(&self) -> R<Option<u32>> {
        Ok(self.capital)
    }
    fn set_capital(&mut self, a: u32) -> R<()> {
        self.capital = Some(a);
        Ok(())
    }
    fn word_190(&self) -> R<u32> {
        Ok(u(self.c, "word190", 1))
    }
}
#[test]
fn borrowed_storage_replays_every_native_owner_case() {
    let f: Value = serde_json::from_str(include_str!(
        "../../../research/skirmish-20261016-load-owner-add/native-owner-add.json"
    ))
    .unwrap();
    let rows = f["cases"].as_array().unwrap();
    assert_eq!(rows.len(), 216);
    for row in rows {
        let c = &row["input"];
        let mut w = W::new(c);
        let mut service = None;
        for _ in 0..u(c, "repeat", 1) {
            match add_actor_stored(4097, &mut w) {
                Ok(()) => (),
                Err(EconomyError::World(s)) => {
                    service = Some(s);
                    break;
                }
                Err(e) => panic!("unexpected {e:?}"),
            }
        }
        let actual = w.output(
            if service.is_some() {
                "required_service"
            } else {
                "complete"
            },
            service.as_deref(),
        );
        assert_eq!(
            actual, row["output"],
            "{} {} {c}",
            row["image"], row["precision"]
        );
    }
}
