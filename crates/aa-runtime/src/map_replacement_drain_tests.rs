use super::*;
use serde_json::{Value, json};
fn laboratory_saved_chunk(count: i32) -> aa_mapworld::Chunk {
    let bytes = count.to_le_bytes().to_vec();
    let identity = aa_mapworld::SourceIdentity::of(&bytes);
    aa_mapworld::Chunk {
        map_source: identity.clone(),
        payload_source: identity,
        directory: aa_mapworld::ChunkDirectory {
            name: "UPDATABLES".into(),
            directory_ordinal: 0,
            packed_offset: 12,
            packed_bytes: 4,
            source_timestamp: 0,
            decoded_bytes: 4,
        },
        encoding: aa_mapworld::archive::Encoding::Raw,
        bytes,
    }
}
#[test]
fn properties_cleanup_uses_actual_queues_and_retired_table_lifetime() {
    let doc: Value = serde_json::from_str(include_str!(
        "../../../research/skirmish-20261017-map-replacement-drain/native-properties-cleanup.json"
    ))
    .unwrap();
    assert_eq!(doc["rows"].as_array().unwrap().len(), 36);
    for row in doc["rows"].as_array().unwrap() {
        let pc = match row["precision"].as_str().unwrap() {
            "Single" => Precision::Single24,
            "Double" => Precision::Double53,
            "Extended" => Precision::Extended64,
            _ => panic!(),
        };
        let mut properties = aa_body::properties::PropertiesState::new(
            aa_body::credit::Object(99),
            row["input"]["definition_count"].as_u64().unwrap() as u32,
        )
        .unwrap();
        let mut owners = crate::scheduler_owners::Owners::new();
        //Explicit lab Updatable label; no native Sun getter or gameplay use.
        let target = crate::scheduler_owners::Target::Sun;
        let owner = owners.register(target).unwrap();
        let mut queues = aa_world::queues::Queues::new(
            aa_world::queues::Clock {
                begin_bits: 0,
                end_bits: 0,
                step_bits: 0x3c23d70a,
            },
            pc,
        )
        .unwrap();
        let mut table = crate::updatables::Table::new(3).unwrap();
        table.register(0, owner).unwrap();
        table.register(1, owner).unwrap();
        let mut lifetime = crate::updatable_lifetime::State::from_world_constructor();
        lifetime
            .bind_saved_table(&laboratory_saved_chunk(3), table.len())
            .unwrap();
        let returned = row["input"]["returned"].as_bool().unwrap();
        if returned {
            lifetime.finish_map_load_50b500().unwrap();
        }
        properties_fields_cleanup(
            &mut properties,
            Some(owner),
            target,
            &mut owners,
            &mut queues,
            &mut table,
            &lifetime,
        )
        .unwrap();
        assert!(owners.get(target).is_err());
        assert!(properties.effects.is_empty());
        assert_eq!(properties.scheduler_link, None);
        assert_eq!(
            json!({"head":0,"effects_count":properties.effects.len()}),
            row["output"]["after_uninit"]
        );
        assert_eq!(
            lifetime.saved_table_present(),
            row["output"]["world284_present"].as_bool().unwrap()
        );
        assert_eq!(
            lifetime.save_hash_present(),
            row["output"]["world280_present"].as_bool().unwrap()
        );
        if returned {
            assert_eq!(table.slots(), &[Some(owner), Some(owner), None]);
        } else {
            let slots = table
                .slots()
                .iter()
                .map(|v| if v.is_some() { json!("self") } else { json!(0) })
                .collect::<Vec<_>>();
            assert_eq!(json!(slots), row["output"]["saved_slots"]);
        }
    }
    let mut lifetime = crate::updatable_lifetime::State::from_world_constructor();
    assert!(
        lifetime
            .bind_saved_table(&laboratory_saved_chunk(3), 2)
            .is_err()
    );
    assert!(!lifetime.saved_table_present());
    let mut stale = laboratory_saved_chunk(3);
    stale.bytes[0] = 4;
    assert!(lifetime.bind_saved_table(&stale, 4).is_err());
    assert!(
        lifetime
            .bind_saved_table(&laboratory_saved_chunk(-1), 0)
            .is_err()
    );
}
struct Probe<'a> {
    input: &'a Value,
    trace: Vec<Value>,
}
impl Probe<'_> {
    fn event(&mut self, name: &str, value: Value) -> Result<()> {
        self.trace.push(value);
        if self.input["cutoff"].as_str() == Some(name) {
            Err(name.into())
        } else {
            Ok(())
        }
    }
    fn source(&self) -> u32 {
        self.input["source"].as_u64().unwrap_or(0x10007) as u32
    }
}
impl World for Probe<'_> {
    type Key = u32;
    fn resolve(&mut self, id: ActorId) -> Result<Option<u32>> {
        Ok((!self.input["stale"].as_bool().unwrap_or(false)
            && !self.input["missing"].as_bool().unwrap_or(false))
        .then_some(id.raw()))
    }
    fn category(&mut self, _: u32) -> Result<u32> {
        Ok(self.input["category"].as_u64().unwrap_or(31) as u32)
    }
    fn b4(&mut self, _: u32) -> Result<bool> {
        let value = self.input["b4"].as_bool().unwrap_or(true);
        self.event("b4", json!({"call":"b4","value":value}))?;
        Ok(value)
    }
    fn announce(&mut self, _: u32) -> Result<u8> {
        let value = self.input["announce"].as_u64().unwrap_or(0) as u8;
        self.event("announce", json!({"call":"announce","value":value}))?;
        Ok(value)
    }
    fn remove(&mut self, _: u32) -> Result<()> {
        self.event(
            "remove",
            json!({"call":"remove","source":self.source(),"argument":0}),
        )
    }
    fn virtual_160(&mut self, _: u32) -> Result<()> {
        self.event("virtual160", json!({"call":"virtual160","argument":0}))
    }
    fn create(&mut self, r: RequestMut<'_>) -> Result<Option<u32>> {
        let name = match &r {
            RequestMut::Actor(_) => "create_actor",
            RequestMut::Object(_) => "create_object",
        };
        self.event(name, json!({"call":name,"requested":source(r.as_ref()).raw()}))?;
        Ok(self.input["success"]
            .as_bool()
            .unwrap_or(true)
            .then_some(77))
    }
    fn property(&mut self, _: u32, d: Option<&DefinitionRef>) -> Result<()> {
        self.event(
            "property",
            json!({"call":"property","definition":d.map(|d|d.token),"kind":2,"source":"new_actor"}),
        )
    }
    fn created_log(&mut self, _: RequestRef<'_>) -> Result<()> {
        self.event("created_log", json!({"call":"created_log"}))
    }
    fn failure(&mut self, _: RequestRef<'_>) -> Result<()> {
        for name in [
            "failure_log",
            "failure_text",
            "failure_format",
            "failure_ui",
        ] {
            self.event(name, json!({"call":name}))?;
        }
        Ok(())
    }
}
fn request(input: &Value) -> crate::map_replacement_queue::Request {
    use crate::map_replacement_queue::{Request, Snapshot, object_payload};
    let snapshot = Snapshot {
        id: ActorId::from_raw(input["source"].as_u64().unwrap_or(0x10007) as u32),
        position_bits: [0x3f800000, 0xc0000000, 0],
        direction_bits: [0, 0],
    };
    let definition = DefinitionRef {
        token: 99,
        canonical_ids: "explicit-laboratory-definition".into(),
    };
    if !input["actor"].as_bool().unwrap_or(true) {
        return Request::Object(object_payload(snapshot, Some(definition)));
    }
    let mut params = aa_actors::actor_creation::ActorParams::new(definition);
    params.requested_id = snapshot.id.raw();
    let default = json!([11, null, 11]);
    let properties = input
        .get("properties")
        .unwrap_or(&default)
        .as_array()
        .unwrap()
        .iter()
        .map(|v| {
            v.as_u64().map(|v| DefinitionRef {
                token: v as u32,
                canonical_ids: format!("property{v}"),
            })
        })
        .collect();
    Request::Actor(ActorRequest {
        source: snapshot.id,
        params,
        properties,
    })
}
#[test]
fn full_native_replacement_body_order_false_returns_and_cutoffs() {
    let doc: Value = serde_json::from_str(include_str!(
        "../../../research/skirmish-20261017-map-replacement-drain/native-replacement-drain.json"
    ))
    .unwrap();
    assert_eq!(doc["rows"].as_array().unwrap().len(), 168);
    for row in doc["rows"].as_array().unwrap() {
        let input = &row["input"];
        let mut request = request(input);
        let borrowed = match &mut request {
            crate::map_replacement_queue::Request::Object(v) => RequestMut::Object(v),
            crate::map_replacement_queue::Request::Actor(v) => RequestMut::Actor(v),
        };
        let mut probe = Probe {
            input,
            trace: vec![],
        };
        let result = replace(&mut probe, borrowed);
        let (returned, error) = match result {
            Ok(value) => (Some(u32::from(value)), None),
            Err(error) => (None, Some(error)),
        };
        assert_eq!(json!(returned), row["output"]["returned"], "{row}");
        assert_eq!(json!(error), row["output"]["error"], "{row}");
        assert_eq!(json!(probe.trace), row["output"]["trace"], "{row}");
    }
}

struct WritebackProbe<'a> {
    base: Probe<'a>,
    actual_address: usize,
    observed: bool,
}
fn params_words(request: RequestRef<'_>) -> [u32; 4] {
    match request {
        RequestRef::Object(v) => [v.params.position_bits[0], v.params.direction_bits[0], v.params.variant, v.params.requested_id],
        RequestRef::Actor(v) => [v.params.position_bits[0], v.params.direction_bits[0], v.params.variant, v.params.requested_id],
    }
}
impl World for WritebackProbe<'_> {
    type Key = u32;
    fn resolve(&mut self, id: ActorId) -> Result<Option<u32>> { self.base.resolve(id) }
    fn category(&mut self, key: u32) -> Result<u32> { self.base.category(key) }
    fn b4(&mut self, key: u32) -> Result<bool> { self.base.b4(key) }
    fn announce(&mut self, key: u32) -> Result<u8> { self.base.announce(key) }
    fn remove(&mut self, key: u32) -> Result<()> { self.base.remove(key) }
    fn virtual_160(&mut self, key: u32) -> Result<()> { self.base.virtual_160(key) }
    fn create(&mut self, request: RequestMut<'_>) -> Result<Option<u32>> {
        //Chosen laboratory writes, matching the explicit native leaf. The
        //address check rejects a detached request or Params clone/copyback.
        let name = match request {
            RequestMut::Object(v) => {
                assert_eq!(v as *mut ObjectRequest as usize, self.actual_address);
                v.params.position_bits[0] = 0x41200000;
                v.params.direction_bits[0] = 0x3f800000;
                v.params.variant = 7;
                v.params.requested_id = 0x20009;
                "create_object"
            }
            RequestMut::Actor(v) => {
                assert_eq!(v as *mut ActorRequest as usize, self.actual_address);
                v.params.position_bits[0] = 0x41200000;
                v.params.direction_bits[0] = 0x3f800000;
                v.params.variant = 7;
                v.params.requested_id = 0x20009;
                "create_actor"
            }
        };
        self.base.event(name, json!({"call":name,"requested":0x20009}))?;
        Ok(self.base.input["success"].as_bool().unwrap_or(true).then_some(77))
    }
    fn property(&mut self, key: u32, definition: Option<&DefinitionRef>) -> Result<()> {
        self.base.property(key, definition)
    }
    fn created_log(&mut self, request: RequestRef<'_>) -> Result<()> {
        assert_eq!(params_words(request), [0x41200000, 0x3f800000, 7, 0x20009]);
        self.observed = true;
        self.base.created_log(request)
    }
    fn failure(&mut self, request: RequestRef<'_>) -> Result<()> {
        assert_eq!(params_words(request), [0x41200000, 0x3f800000, 7, 0x20009]);
        self.observed = true;
        self.base.failure(request)
    }
}
#[test]
fn queued_creation_writes_survive_false_returns_and_later_required_children() {
    let doc: Value = serde_json::from_str(include_str!(
        "../../../research/skirmish-20261017-map-replacement-drain/native-request-writeback.json"
    )).unwrap();
    assert_eq!(doc["rows"].as_array().unwrap().len(), 66);
    for row in doc["rows"].as_array().unwrap() {
        let input = &row["input"];
        let mut queue = Queue::default();
        queue.append(request(input));
        let actual_address = match queue.native_order().next().unwrap() {
            RequestRef::Object(v) => v as *const ObjectRequest as usize,
            RequestRef::Actor(v) => v as *const ActorRequest as usize,
        };
        let mut probe = WritebackProbe { base: Probe { input, trace: vec![] }, actual_address, observed: false };
        let result = drain_world(&mut probe, &mut queue);
        match (result, row["output"]["returned"].as_u64()) {
            (Ok(report), Some(returned)) => {
                assert_eq!(report, Report { visited: 1, created: returned as usize, failed: 1 - returned as usize });
                assert!(row["output"]["error"].is_null());
            }
            (Err(error), None) => assert_eq!(json!(error), row["output"]["error"], "{row}"),
            (result, returned) => panic!("{result:?} / {returned:?}: {row}"),
        }
        let after = queue.native_order().next().unwrap();
        assert_eq!(json!(params_words(after)), row["output"]["params_words"], "{row}");
        assert_eq!(source(after).raw(), 0x10007); //Source+80 is distinct from Params+24.
        assert_eq!(queue.native_order().count(), 1);
        assert_eq!(json!(probe.base.trace), row["output"]["trace"], "{row}");
        let cutoff = input["cutoff"].as_str();
        assert_eq!(probe.observed, !matches!(cutoff, Some("create_actor" | "create_object" | "property")));
    }
}

#[test]
fn actual_world_object_list_removal_matches_intrusive_native_stores() {
    use crate::world_object_list::{NodeId, PreviousLink, WorldObjectList};
    let doc: Value = serde_json::from_str(include_str!(
        "../../../research/skirmish-20261017-map-replacement-drain/native-removal.json"
    ))
    .unwrap();
    let mut rows = 0;
    for row in doc["rows"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|r| r["kind"] == "intrusive")
    {
        let input = &row["input"];
        let mut list = WorldObjectList::constructed();
        let mut ids = vec![];
        //Insertion is native head-first; construct the fixture's current order.
        for value in input["values"].as_array().unwrap().iter().rev() {
            ids.push(
                list.insert(if value == "old" { 7u32 } else { 8u32 }, 1 << 8)
                    .unwrap()
                    .unwrap(),
            );
        }
        ids.reverse();
        let before = ids.clone();
        let removed = list
            .remove_first(7, input["flags"].as_u64().unwrap() as u32)
            .unwrap();
        let mut values = vec![];
        let mut identities = vec![];
        let mut node = list.head();
        let mut previous = None::<NodeId>;
        while let Some(id) = node {
            let n = list.node(id).unwrap();
            assert_eq!(
                n.previous_link,
                previous
                    .map(PreviousLink::Next)
                    .unwrap_or(PreviousLink::Head)
            );
            values.push(if n.object == 7 { "old" } else { "other" });
            identities.push(before.iter().position(|old| *old == id).unwrap() + 1);
            previous = Some(id);
            node = n.next;
        }
        assert_eq!(json!(values), row["output"]["values"], "{row}");
        assert_eq!(json!(identities), row["output"]["identities"], "{row}");
        assert_eq!(
            removed,
            !row["output"]["trace"].as_array().unwrap().is_empty()
        );
        if removed {
            let i = input["values"]
                .as_array()
                .unwrap()
                .iter()
                .position(|v| v == "old")
                .unwrap();
            assert!(list.node(before[i]).is_none());
        }
        assert_eq!(list.nodes().len(), before.len()); //allocation history, not live membership
        rows += 1;
    }
    assert_eq!(rows, 72);
}
