//! The World+130 broadcast array (`+10` base, `+14` count, `+18` capacity)
//! of fx_manager: its producers, the 50D070/50CF30 slot append and the
//! 50C750/50C600 walk that 43EC60 runs first (43ECE9).
//!
//! A slot is the native 0x4C-byte entry. 50CA40 builds it (+4..+44 zero
//! except +20 = 0xA, +48 an empty CString; +0 is left unwritten) and every
//! producer stores only its own words, so a slot reused after the walk (the
//! walk only resets the count) keeps the stale words of its earlier entry.
//! 50CF30 copies every old slot word for word when the array grows. The
//! stale words are observable: 50D280 never stores +28, which 50C600 case 4
//! reads as the attach target, so a detached request in a slot that held an
//! attached one is attached to that old target (or dropped once it is gone).
//!
//! Proof: research/phase8-vworld-update/native_world_broadcasts.py
//! (native-world-broadcasts.json): 50CE00, 50D120, 50D0B0, 50D230, 50D280,
//! 50D330, 50D370, 50D070/50CF30 growth and 50C750/50C600, every slot word.
use super::Result;

/// One 0x4C-byte slot.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Broadcast {
    /// +0. 50CA40 does not write it; every producer does, before the count
    /// covers the slot.
    kind: Option<u32>,
    /// +4: kind 0's thing handle word (4CF1B0: actor+14).
    handle_04: u32,
    /// +8..+20: kind 0's seven damage record words.
    record_08: [u32; 7],
    /// +24: kind 4's VFX definition. Natively a pointer; here the opaque
    /// handle of that ActorData definition (0 is the 50CA40 value only: no
    /// producer stores a null definition).
    definition_24: u64,
    /// +28: kind 4's attach target handle word (50D230: thing+14).
    target_28: u32,
    /// +2C..+34 position and +38..+40 direction (50D280).
    position_2c: [u32; 3],
    direction_38: [u32; 3],
    /// +44: GWorld+E0 when the request was made.
    time_44: u32,
    /// +48 CString (kind 1).
    text_48: Vec<u16>,
}
impl Broadcast {
    /// 50CA40.
    fn from_constructor_50ca40() -> Self {
        Self { kind: None, handle_04: 0, record_08: [0, 0, 0, 0, 0, 0, 0xa], definition_24: 0, target_28: 0,
            position_2c: [0; 3], direction_38: [0; 3], time_44: 0, text_48: Vec::new() }
    }
    pub fn kind(&self) -> Option<u32> {
        self.kind
    }
    pub fn text(&self) -> &[u16] {
        &self.text_48
    }
    pub fn handle_04(&self) -> u32 {
        self.handle_04
    }
    pub fn record_08(&self) -> [u32; 7] {
        self.record_08
    }
    pub fn definition_24(&self) -> u64 {
        self.definition_24
    }
    pub fn target_28(&self) -> u32 {
        self.target_28
    }
    pub fn position_2c(&self) -> [u32; 3] {
        self.position_2c
    }
    pub fn direction_38(&self) -> [u32; 3] {
        self.direction_38
    }
    pub fn time_44(&self) -> u32 {
        self.time_44
    }
}

/// The children 50C600 reaches, owned elsewhere. None of them can reach this
/// array (natively a child could append and the walk would reread the count;
/// the Rust owners have no such access).
pub trait BroadcastDelivery {
    /// The view-side object 432E40 returns.
    type Thing;
    /// The VFX view 43CD00/43CC50 return.
    type Vfx;
    /// 432E40 on a handle word (kind 0: +4, kind 4: +28): the object
    /// registered under it in the `[B364C0]` table (handle low 16 bits index
    /// it, the high 16 bits must match its generation), or None.
    fn thing_432e40(&mut self, handle: u32) -> Result<Option<Self::Thing>>;
    /// Kind 0: 43B270(thing, &record) on the attached view (manager+4).
    fn thing_record_43b270(&mut self, view: u64, thing: Option<Self::Thing>, record: &[u32; 7]) -> Result<()>;
    /// Kind 1: 427950(&text, 0) on the script processor `[B36500]`, which
    /// evaluates the text immediately (426C20 TGC_ScriptProcessor::Eval).
    fn script_eval_427950(&mut self, text: &[u16], flag: u8) -> Result<()>;
    /// Kind 2 (50D370, from 4E9720 SetMissionResult and 938E10): 440190 on
    /// the VWorld `[B36538]` (the mission result messages).
    fn mission_result_440190(&mut self) -> Result<()>;
    /// Kind 3 (50D330, from 4F4DA0 TeamVictory): 4408A0 on the VWorld.
    fn team_victory_4408a0(&mut self) -> Result<()>;
    /// Kind 4 with a registered target: 43CD00(definition, thing+C, 0,
    /// &B43CF8, 0, 0) on the VWorld (the implementation reads thing+C).
    fn attached_vfx_43cd00(&mut self, definition: u64, thing: Self::Thing) -> Result<Option<Self::Vfx>>;
    /// Kind 4 without a target (+28 == 0): 43CC50(definition, &+2C, &+38, 0,
    /// 0) on the VWorld.
    fn detached_vfx_43cc50(&mut self, definition: u64, position: [u32; 3], direction: [u32; 3])
        -> Result<Option<Self::Vfx>>;
    /// Kind 4 after a non-null create: VFX +50 = +44.
    fn vfx_time_50(&mut self, vfx: Self::Vfx, time: u32) -> Result<()>;
}

/// The GWorld `[B36534]` words a VFX request reads, each at its native
/// position.
pub trait RequestWorld {
    /// GWorld+D8 (6 refuses the request), read first.
    fn world_d8(&mut self) -> Result<u32>;
    /// GWorld+E0, read after the slot is appended.
    fn world_e0(&mut self) -> Result<u32>;
}

/// The +10/+14/+18 array. `slots.len()` is the native capacity +18.
pub(super) struct Broadcasts {
    pub(super) slots: Vec<Broadcast>,
    pub(super) count: usize,
}
impl Broadcasts {
    /// 50CE00: four 50CA40 slots, count 0.
    pub(super) fn from_native_constructor() -> Self {
        Self { slots: (0..4).map(|_| Broadcast::from_constructor_50ca40()).collect(), count: 0 }
    }
    pub(super) fn live(&self) -> &[Broadcast] {
        &self.slots[..self.count]
    }
    /// 50D070: count + 1 over the capacity grows it through 50CF30 (to
    /// count + 1 when that exceeds twice the capacity, else to twice the
    /// capacity; at least 4); new slots are 50CA40's, old slots keep every
    /// word. The base is never null here (an allocation failure is an
    /// error), so 50D070's null-base regrow edge is not reached.
    fn append_50d070(&mut self) -> Result<&mut Broadcast> {
        let capacity = self.slots.len();
        let n = self.count + 1;
        let grow = if n > capacity * 2 { Some(n) } else if n > capacity { Some(capacity * 2) } else { None };
        if let Some(target) = grow {
            let target = target.max(4);
            self.slots.try_reserve(target - capacity).map_err(|e| format!("50CF30 broadcast array growth: {e}"))?;
            self.slots.resize_with(target, Broadcast::from_constructor_50ca40);
        }
        self.count = n;
        Ok(&mut self.slots[n - 1])
    }
    /// 50D120's append (and its WaitWorld append): +0 = 1, then the +48
    /// CString assignment. The other words stay.
    pub(super) fn append_text(&mut self, text: &[u16]) -> Result<()> {
        let slot = self.append_50d070()?;
        slot.kind = Some(1);
        let mut value = Vec::new();
        value.try_reserve_exact(text.len()).map_err(|e| e.to_string())?;
        value.extend_from_slice(text);
        slot.text_48 = value;
        Ok(())
    }
    /// 50D0B0 after its gates: +0 = 0, +4 = actor+14, +8..+20 the record.
    pub(super) fn append_thing_record(&mut self, handle: u32, record: [u32; 7]) -> Result<()> {
        let slot = self.append_50d070()?;
        slot.kind = Some(0);
        slot.handle_04 = handle;
        slot.record_08 = record;
        Ok(())
    }
    /// 50D230.
    pub(super) fn append_attached_vfx_50d230(&mut self, world: &mut impl RequestWorld, definition: Option<u64>,
        thing_14: Option<u32>) -> Result<()> {
        if world.world_d8()? == 6 {
            return Ok(());
        }
        let (Some(definition), Some(target)) = (definition, thing_14) else { return Ok(()) };
        if definition == 0 {
            return Err("50D230 VFX definition handle 0 is not a native definition".into());
        }
        let slot = self.append_50d070()?;
        slot.kind = Some(4);
        slot.definition_24 = definition;
        slot.target_28 = target;
        let time = world.world_e0()?;
        self.slots[self.count - 1].time_44 = time;
        Ok(())
    }
    /// 50D280: +28 is not written.
    pub(super) fn append_detached_vfx_50d280(&mut self, world: &mut impl RequestWorld, definition: Option<u64>,
        position: [u32; 3], direction: [u32; 2]) -> Result<()> {
        if world.world_d8()? == 6 {
            return Ok(());
        }
        let Some(definition) = definition else { return Ok(()) };
        if definition == 0 {
            return Err("50D280 VFX definition handle 0 is not a native definition".into());
        }
        let slot = self.append_50d070()?;
        slot.kind = Some(4);
        slot.definition_24 = definition;
        slot.position_2c = position;
        slot.direction_38 = [direction[0], direction[1], 0];
        let time = world.world_e0()?;
        self.slots[self.count - 1].time_44 = time;
        Ok(())
    }
    /// 50D370 (kind 2) and 50D330 (kind 3): +0 only.
    pub(super) fn append_kind(&mut self, kind: u32) -> Result<()> {
        self.append_50d070()?.kind = Some(kind);
        Ok(())
    }
    /// 50C750: every slot below the count (reread each iteration) through
    /// 50C600 in index order, then count = 0. A refused child keeps the
    /// count (native has no failure edge); the delivered prefix stays.
    pub(super) fn walk_50c750<D: BroadcastDelivery>(&mut self, view: Option<u64>, delivery: &mut D) -> Result<usize> {
        let mut index = 0;
        while index != self.count {
            dispatch_50c600(&self.slots[index], view, delivery)?;
            index += 1;
        }
        self.count = 0;
        Ok(index)
    }
}

/// 50C600 on one slot; `view` is manager+4.
fn dispatch_50c600<D: BroadcastDelivery>(slot: &Broadcast, view: Option<u64>, delivery: &mut D) -> Result<()> {
    let kind = slot.kind.ok_or("50C600 slot below the count holds no producer kind")?;
    match kind {
        0 => {
            //50C616: nothing without an attached view.
            if let Some(view) = view {
                let thing = delivery.thing_432e40(slot.handle_04)?;
                delivery.thing_record_43b270(view, thing, &slot.record_08)?;
            }
            Ok(())
        }
        1 => delivery.script_eval_427950(&slot.text_48, 0),
        2 => delivery.mission_result_440190(),
        3 => delivery.team_victory_4408a0(),
        4 => {
            let vfx = match delivery.thing_432e40(slot.target_28)? {
                Some(thing) => delivery.attached_vfx_43cd00(slot.definition_24, thing)?,
                //50C6A2: a non-zero target that is gone drops the request.
                None if slot.target_28 != 0 => return Ok(()),
                None => delivery.detached_vfx_43cc50(slot.definition_24, slot.position_2c, slot.direction_38)?,
            };
            match vfx {
                Some(vfx) => delivery.vfx_time_50(vfx, slot.time_44),
                None => Ok(()),
            }
        }
        //50C608 ja 50C6CD: any other kind does nothing.
        _ => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{json, Value};
    use std::collections::{BTreeMap, VecDeque};

    fn u(v: &Value) -> u32 {
        v.as_u64().unwrap() as u32
    }
    fn words<const N: usize>(v: &Value) -> [u32; N] {
        let list: Vec<u32> = v.as_array().unwrap().iter().map(u).collect();
        list.try_into().unwrap()
    }
    struct World {
        d8: u32,
        e0: u32,
        show: bool,
    }
    impl RequestWorld for World {
        fn world_d8(&mut self) -> Result<u32> {
            Ok(self.d8)
        }
        fn world_e0(&mut self) -> Result<u32> {
            Ok(self.e0)
        }
    }
    struct Actor<'a> {
        world: &'a World,
        flags: Option<u32>,
        handle: u32,
    }
    impl aa_combat::presentation::World for Actor<'_> {
        fn session_mode(&self) -> aa_combat::Result<u32> {
            Ok(self.world.d8)
        }
        fn show_damage(&self) -> aa_combat::Result<bool> {
            Ok(self.world.show)
        }
        fn definition_flags54(&self, _: u32) -> aa_combat::Result<Option<u32>> {
            Ok(self.flags)
        }
        fn actor_id(&self, _: u32) -> aa_combat::Result<u32> {
            Ok(self.handle)
        }
    }
    /// The oracle seams: a handle-keyed thing table and scripted results.
    struct Sink {
        things: BTreeMap<u32, u32>,
        results: VecDeque<bool>,
        trace: Vec<Value>,
        times: Vec<u32>,
    }
    impl BroadcastDelivery for Sink {
        type Thing = (u32, u32);
        type Vfx = ();
        fn thing_432e40(&mut self, handle: u32) -> Result<Option<(u32, u32)>> {
            self.trace.push(json!(["432E40", handle]));
            Ok(self.things.get(&handle).map(|c| (handle, *c)))
        }
        fn thing_record_43b270(&mut self, view: u64, thing: Option<(u32, u32)>, record: &[u32; 7]) -> Result<()> {
            let label = thing.map_or(json!(0), |(h, _)| json!(format!("thing{h:#x}")));
            self.trace.push(json!(["43B270", view == 1, label, record]));
            Ok(())
        }
        fn script_eval_427950(&mut self, text: &[u16], flag: u8) -> Result<()> {
            self.trace.push(json!(["427950", String::from_utf16(text).unwrap(), flag]));
            Ok(())
        }
        fn mission_result_440190(&mut self) -> Result<()> {
            self.trace.push(json!(["440190", true]));
            Ok(())
        }
        fn team_victory_4408a0(&mut self) -> Result<()> {
            self.trace.push(json!(["4408A0", true]));
            Ok(())
        }
        fn attached_vfx_43cd00(&mut self, definition: u64, thing: (u32, u32)) -> Result<Option<()>> {
            self.trace.push(json!(["43CD00", true, definition, thing.1, 0, 0xb43cf8u32, 0, 0]));
            Ok(self.results.pop_front().unwrap().then_some(()))
        }
        fn detached_vfx_43cc50(&mut self, definition: u64, position: [u32; 3], direction: [u32; 3]) -> Result<Option<()>> {
            self.trace.push(json!(["43CC50", true, definition, position, direction, 0, 0]));
            Ok(self.results.pop_front().unwrap().then_some(()))
        }
        fn vfx_time_50(&mut self, _: (), time: u32) -> Result<()> {
            self.times.push(time);
            Ok(())
        }
    }
    fn definition(v: &Value) -> Option<u64> {
        Some(v.as_u64().unwrap()).filter(|d| *d != 0)
    }

    /// research/phase8-vworld-update/native-world-broadcasts.json: every
    /// producer, the slot reuse and growth, and the walk, slot word for word.
    #[test]
    fn native_world_broadcast_producers_slots_and_walk() {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../research/phase8-vworld-update/native-world-broadcasts.json");
        let doc: Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
        let rows = doc["cases"].as_array().unwrap();
        assert_eq!(rows.len(), 42);
        for row in rows {
            let c = &row["input"];
            let name = c["name"].as_str().unwrap();
            let mut state = crate::fx_manager::State::<u32>::from_native_constructor();
            let mut world = World { d8: 0, e0: 0, show: false };
            let mut sink = Sink { things: BTreeMap::new(), results: c["vfx_results"].as_array().unwrap().iter()
                .map(|v| v.as_bool().unwrap()).collect(), trace: Vec::new(), times: Vec::new() };
            let mut walks = Vec::new();
            for op in c["ops"].as_array().unwrap() {
                let o = op.as_array().unwrap();
                match o[0].as_str().unwrap() {
                    "world" => (world.d8, world.e0) = (u(&o[1]), u(&o[2])),
                    "show" => world.show = u(&o[1]) != 0,
                    //Laboratory pre-state, as in the fx_manager tests: the
                    //50CAB0 attachment is proven separately.
                    "view" => state.view = (u(&o[1]) != 0).then_some(1),
                    "thing" => {
                        sink.things.insert(u(&o[1]), u(&o[2]));
                    }
                    "forget" => {
                        sink.things.remove(&u(&o[1]));
                    }
                    "text" => state.broadcast(&o[1].as_str().unwrap().encode_utf16().collect::<Vec<_>>()).unwrap(),
                    "damage" => {
                        let w: [u32; 7] = words(&o[3]);
                        let record = aa_body::damage::DamageRecord { source_id: w[0], allegiance_id: w[1], damage_type: w[2],
                            amount: aa_body::F32(w[3]), time: aa_body::F32(w[4]), event_kind: w[5], reason: w[6] };
                        let actor = Actor { world: &world, flags: o[1].as_u64().map(|f| f as u32), handle: u(&o[2]) };
                        state.queue_damage_number_50d0b0(1, &record, &actor).unwrap();
                    }
                    "attached" => state.queue_attached_vfx_50d230(&mut world, definition(&o[1]),
                        o[2].as_u64().map(|t| t as u32)).unwrap(),
                    "detached" => state.queue_detached_vfx_50d280(&mut world, definition(&o[1]), words(&o[2]),
                        words(&o[3])).unwrap(),
                    "kind2" => state.queue_mission_result_50d370().unwrap(),
                    "kind3" => state.queue_team_victory_50d330().unwrap(),
                    "walk" => {
                        sink.trace.clear();
                        sink.times.clear();
                        state.deliver_broadcasts_50c750(&mut sink).unwrap();
                        walks.push(json!({"trace": sink.trace, "vfx_50": sink.times}));
                    }
                    other => panic!("{other}"),
                }
            }
            let slots: Vec<Value> = state.broadcasts.slots.iter().map(|s| {
                let mut w = vec![s.handle_04];
                w.extend(s.record_08);
                w.push(u32::try_from(s.definition_24).unwrap());
                w.push(s.target_28);
                w.extend(s.position_2c);
                w.extend(s.direction_38);
                w.push(s.time_44);
                json!({"kind": s.kind, "words": w, "text": String::from_utf16(&s.text_48).unwrap()})
            }).collect();
            let out = json!({"walks": walks, "count": state.broadcasts.count, "capacity": state.broadcasts.slots.len(),
                "enabled": u8::from(state.enabled_1c()), "slots": slots});
            assert_eq!(out, row["output"], "{name} {}", c["precision"]);
        }
    }

    /// A refused child keeps the count and the slots; the next walk starts
    /// again at index 0 (native has no failure edge).
    #[test]
    fn refused_child_keeps_the_count() {
        struct Refuse;
        impl BroadcastDelivery for Refuse {
            type Thing = ();
            type Vfx = ();
            fn thing_432e40(&mut self, _: u32) -> Result<Option<()>> {
                Err("432E40 refused".into())
            }
            fn thing_record_43b270(&mut self, _: u64, _: Option<()>, _: &[u32; 7]) -> Result<()> {
                unreachable!()
            }
            fn script_eval_427950(&mut self, _: &[u16], _: u8) -> Result<()> {
                Ok(())
            }
            fn mission_result_440190(&mut self) -> Result<()> {
                unreachable!()
            }
            fn team_victory_4408a0(&mut self) -> Result<()> {
                unreachable!()
            }
            fn attached_vfx_43cd00(&mut self, _: u64, _: ()) -> Result<Option<()>> {
                unreachable!()
            }
            fn detached_vfx_43cc50(&mut self, _: u64, _: [u32; 3], _: [u32; 3]) -> Result<Option<()>> {
                unreachable!()
            }
            fn vfx_time_50(&mut self, _: (), _: u32) -> Result<()> {
                unreachable!()
            }
        }
        let mut state = crate::fx_manager::State::<u32>::from_native_constructor();
        let mut world = World { d8: 0, e0: 0x4148_0000, show: false };
        state.broadcast(&[0x61]).unwrap();
        state.queue_detached_vfx_50d280(&mut world, Some(9), [1, 2, 3], [4, 5]).unwrap();
        assert!(state.deliver_broadcasts_50c750(&mut Refuse).unwrap_err().contains("432E40"));
        assert_eq!(state.broadcasts().len(), 3);
        assert_eq!(state.broadcasts()[2].direction_38(), [4, 5, 0]);
        assert_eq!(state.broadcasts()[2].target_28(), 0);
        assert_eq!(state.broadcasts()[2].time_44(), 0x4148_0000);
    }
}
