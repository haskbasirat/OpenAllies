use super::*;

//The fixture replay is filled alongside the immutable native oracle before
//acceptance. This owner guard does not fabricate a map or a Supply payload.
#[test]
fn tick_relay_search_error_preserves_immediate_clear() {
    struct Probe {
        relay: u32,
    }
    impl TickWorld for Probe {
        fn relay(&mut self) -> Result<bool> {
            Ok(true)
        }
        fn suspended(&mut self) -> Result<bool> {
            Ok(false)
        }
        fn clear_relay(&mut self) -> Result<()> {
            self.relay = 0;
            Ok(())
        }
        fn search_relays(&mut self) -> Result<()> {
            required("actual relay collector")
        }
        fn update_supply_state(&mut self) -> Result<()> {
            unreachable!()
        }
        fn out_of_supply(&mut self) -> Result<bool> {
            unreachable!()
        }
        fn stockpile(&mut self) -> Result<u32> {
            unreachable!()
        }
        fn maximum(&mut self) -> Result<u32> {
            unreachable!()
        }
        fn recharge(&mut self) -> Result<u32> {
            unreachable!()
        }
        fn store_stockpile(&mut self, _: u32) -> Result<()> {
            unreachable!()
        }
        fn refresh_economy(&mut self) -> Result<()> {
            unreachable!()
        }
        fn provides(&mut self) -> Result<bool> {
            unreachable!()
        }
        fn zone_active(&mut self) -> Result<bool> {
            unreachable!()
        }
        fn clear_zone(&mut self) -> Result<()> {
            unreachable!()
        }
        fn dirty(&mut self) -> Result<bool> {
            unreachable!()
        }
        fn radius(&mut self) -> Result<u32> {
            unreachable!()
        }
        fn position(&mut self) -> Result<[u32; 2]> {
            unreachable!()
        }
        fn move_zone(&mut self, _: [u32; 2], _: u32) -> Result<()> {
            unreachable!()
        }
        fn clear_dirty(&mut self) -> Result<()> {
            unreachable!()
        }
    }
    let mut probe = Probe { relay: 42 };
    assert!(tick(&mut probe, 0, Precision::Double53).is_err());
    assert_eq!(probe.relay, 0);
}

use serde_json::{Value, json};
use std::collections::BTreeMap;
struct World {
    input: Value,
    stop: Option<String>,
    events: Vec<Value>,
    counts: BTreeMap<String, usize>,
    stock: u32,
    relay_id: u32,
    dirty: u8,
    zone: Option<aa_zones::zone::Zone>,
    zone_count: u32,
}
impl World {
    fn word(&self, key: &str) -> u32 {
        self.input[key].as_u64().unwrap() as u32
    }
    fn flag(&self, key: &str) -> bool {
        self.input[key].as_bool().unwrap()
    }
    fn event(&mut self, label: &str, payload: Value) -> Result<()> {
        let count = self.counts.entry(label.to_owned()).or_default();
        let key = format!("{label}:{count}");
        *count += 1;
        if self.stop.as_deref() == Some(key.as_str()) {
            return Err(Error::World(key));
        }
        self.events.push(json!([key, payload]));
        Ok(())
    }
    fn read(&mut self, label: &str) -> Result<()> {
        self.event(label, Value::Null)
    }
}
impl InitWorld for World {
    fn schedule(&mut self) -> Result<()> {
        self.read("pending")?;
        if !self.flag("pending") {
            self.event("queue", json!([1, 0]))?;
        }
        Ok(())
    }
    fn zone_present(&mut self) -> Result<bool> {
        self.read("zone")?;
        Ok(self.zone.is_some())
    }
    fn create_zone(&mut self) -> Result<()> {
        self.read("create")?;
        self.zone_count = 9;
        self.zone = Some(aa_zones::zone::Zone::new(1, 1, 1));
        Ok(())
    }
    fn tick(&mut self) -> Result<()> {
        self.event("tick", json!(0))
    }
}
impl TickWorld for World {
    fn relay(&mut self) -> Result<bool> {
        self.read("relay")?;
        Ok(self.flag("relay"))
    }
    fn suspended(&mut self) -> Result<bool> {
        self.read("suspended")?;
        Ok(self.flag("suspended"))
    }
    fn clear_relay(&mut self) -> Result<()> {
        self.read("clear_relay")?;
        self.relay_id = 0;
        Ok(())
    }
    fn search_relays(&mut self) -> Result<()> {
        self.read("search")?;
        required("required_search")
    }
    fn update_supply_state(&mut self) -> Result<()> {
        self.read("update")
    }
    fn out_of_supply(&mut self) -> Result<bool> {
        self.read("out")?;
        Ok(self.word("state") & 0x80000 != 0)
    }
    fn stockpile(&mut self) -> Result<u32> {
        self.read("stock")?;
        Ok(self.stock)
    }
    fn maximum(&mut self) -> Result<u32> {
        self.read("maximum")?;
        Ok(self.word("maximum"))
    }
    fn recharge(&mut self) -> Result<u32> {
        self.read("rate")?;
        Ok(self.word("rate"))
    }
    fn store_stockpile(&mut self, bits: u32) -> Result<()> {
        self.read("store")?;
        self.stock = bits;
        Ok(())
    }
    fn refresh_economy(&mut self) -> Result<()> {
        self.read("economy")
    }
    fn provides(&mut self) -> Result<bool> {
        self.read("provides")?;
        Ok(self.flag("provides"))
    }
    fn zone_active(&mut self) -> Result<bool> {
        self.read("active")?;
        Ok(self.zone.as_ref().is_some_and(|z| z.active()))
    }
    fn clear_zone(&mut self) -> Result<()> {
        self.read("clear")?;
        self.zone.as_mut().unwrap().radius_bits = 0xbf800000;
        Ok(())
    }
    fn dirty(&mut self) -> Result<bool> {
        self.read("dirty")?;
        Ok(self.dirty != 0)
    }
    fn radius(&mut self) -> Result<u32> {
        self.read("radius")?;
        Ok(self.word("radius"))
    }
    fn position(&mut self) -> Result<[u32; 2]> {
        self.read("position")?;
        Ok(serde_json::from_value(self.input["xy"].clone()).unwrap())
    }
    fn move_zone(&mut self, xy: [u32; 2], radius: u32) -> Result<()> {
        self.event("move", json!([xy[0], xy[1], radius]))?;
        self.zone.as_mut().unwrap().radius_bits = radius;
        Ok(())
    }
    fn clear_dirty(&mut self) -> Result<()> {
        self.read("clear_dirty")?;
        self.dirty = 0;
        Ok(())
    }
}
impl ProvideWorld for World {
    fn owner(&mut self) -> Result<u32> {
        self.read("owner")?;
        Ok(u32::from(self.flag("owner")))
    }
    fn nonmajor(&mut self, _: u32) -> Result<u32> {
        self.read("nonmajor")?;
        Ok(self.word("nonmajor"))
    }
    fn high(&mut self) -> Result<u32> {
        self.read("high")?;
        Ok(self.word("high"))
    }
    fn low(&mut self) -> Result<u32> {
        self.read("low")?;
        Ok(self.word("state"))
    }
    fn destroyed(&mut self) -> Result<bool> {
        self.read("dead")?;
        Ok(self.word("flags") & 1 != 0)
    }
    fn under_siege_enabled(&mut self) -> Result<bool> {
        self.read("under")?;
        Ok(self.word("under") as u8 != 0)
    }
    fn under_siege(&mut self) -> Result<bool> {
        self.read("siege")?;
        Ok(self.word("state") & 0x100000 != 0)
    }
    fn is_relay(&mut self) -> Result<bool> {
        self.read("relay")?;
        Ok(self.flag("relay"))
    }
    fn stockpile_supplied(&mut self) -> Result<bool> {
        self.read("stockpile_supplied")?;
        Ok(self.word("state") & 0x80000 == 0 || f32::from_bits(self.stock) > 0.0)
    }
    fn range(&mut self) -> Result<u32> {
        self.read("range")?;
        Ok(self.word("radius"))
    }
    fn city(&mut self) -> Result<bool> {
        self.read("city")?;
        Ok(self.flag("city") && self.word("city_byte") as u8 != 0)
    }
}
impl PrimaryWorld for World {
    fn out(&mut self) -> Result<bool> {
        self.read("primary_out")?;
        Ok(self.word("state") & 0x80000 != 0)
    }
    fn supplied_state(&mut self) -> Result<()> {
        self.event("state", json!([0x13, 0]))?;
        required("required_state_effects")
    }
    fn refresh_actions(&mut self) -> Result<()> {
        unreachable!()
    }
}
#[test]
fn original_v14_all_pc_init_tick_provides_and_error_prefixes() {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../research/skirmish-20261017-supply-init/native-supply-init.json"
    ))
    .unwrap();
    let rows = fixture["cases"].as_array().unwrap();
    assert_eq!(rows.len(), 1506);
    for row in rows {
        let c = &row["input"];
        let n = &row["output"];
        let mut world = World {
            input: c.clone(),
            stop: row["stop"].as_str().map(str::to_owned),
            events: Vec::new(),
            counts: BTreeMap::new(),
            stock: c["stock"].as_u64().unwrap() as u32,
            relay_id: c["relay_id"].as_u64().unwrap() as u32,
            dirty: c["dirty"].as_u64().unwrap() as u8,
            zone: None,
            zone_count: if c["zone"].as_bool().unwrap() { 9 } else { 0 },
        };
        if world.flag("zone") {
            let mut zone = aa_zones::zone::Zone::new(1, 1, 1);
            zone.radius_bits = world.word("active_radius");
            zone.state = 0;
            world.zone = Some(zone);
        }
        let pc = match row["pc"].as_str().unwrap() {
            "Single24" => Precision::Single24,
            "Double53" => Precision::Double53,
            _ => Precision::Extended64,
        };
        let phase = c["phase"].as_str().unwrap();
        let result = match phase {
            "init" => initialize(&mut world).map(Some),
            "tick" => tick(&mut world, c["seconds"].as_u64().unwrap() as u32, pc).map(|_| None),
            "provides" => provides(&mut world).map(Some),
            "primary" => primary_update(&mut world).map(Some),
            "after" => Ok(None),
            _ => unreachable!(),
        };
        let stopped = match &result {
            Ok(_) => Value::Null,
            Err(Error::World(s)) => json!(s),
            Err(Error::Unsupported(s)) => json!(s),
            e => panic!("{row}: {e:?}"),
        };
        let actual = json!({"events":world.events,"stock":world.stock,"relay_id":world.relay_id,"dirty":world.dirty,
            "zone_count":world.zone_count,"zone_radius":world.zone.as_ref().map(|z|z.radius_bits),
            "zone_flags":world.zone.as_ref().map(|z|z.flags),"zone_state":world.zone.as_ref().map(|z|z.state),
            "stopped":stopped,"result":result.ok().flatten().map(u32::from)});
        assert_eq!(actual, *n, "{row}");
    }
}
