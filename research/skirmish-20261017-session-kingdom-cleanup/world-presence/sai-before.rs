//! Session+4/+8/+C is the actual growable SetupAI triple queue. Session+108
//! is an unrelated CString. Native778F20(false) calls778730 on Session+4.
//! Successful allocation/string copies are proved; arbitrary allocator
//! reentry/SEH is outside this owner. There is no four-entry gameplay cap.
use super::*;

#[derive(Debug, PartialEq, Eq)]
pub struct SetupAi {
    kingdom: Vec<u16>,
    display: Vec<u16>,
    profile: Vec<u16>,
}
impl SetupAi {
    pub fn kingdom(&self) -> &[u16] {
        &self.kingdom
    }
    pub fn display(&self) -> &[u16] {
        &self.display
    }
    pub fn profile(&self) -> &[u16] {
        &self.profile
    }
}
pub(super) struct Queue {
    entries: Vec<SetupAi>,
}
impl Queue {
    pub(super) fn entries(&self)->&[SetupAi] {&self.entries}
    ///76F890 vector constructor: ptr allocation for four76DA40 triples,
    ///count zero, growable capacity four. Capacity slack is not live entries.
    pub(super) fn from_native_constructor() -> Self {
        Self {
            entries: Vec::with_capacity(4),
        }
    }
    pub(super) fn clear_count(&mut self) {
        self.entries.clear();
    }
    fn append(&mut self, kingdom: &[u16], display: &[u16], profile: &[u16]) -> Result<()> {
        self.entries
            .try_reserve(1)
            .map_err(|e| missing(e.to_string()))?;
        self.entries.push(SetupAi {
            kingdom: kingdom.to_vec(),
            display: display.to_vec(),
            profile: profile.to_vec(),
        });
        Ok(())
    }
}
impl LocalBootstrap {
    /// Actual queuedfalse branch of778F20. Applyingtrue/773480 is separate;
    /// callers must not substitute an immediate controller rewrite here.
    pub fn queue_setup_ai_778f20(
        &mut self,
        kingdom: &[u16],
        display: &[u16],
        profile: &[u16],
    ) -> Result<()> {
        self.session.setup_ai.append(kingdom, display, profile)
    }
    pub fn setup_ai_entries(&self) -> &[SetupAi] {
        &self.session.setup_ai.entries
    }

    /// Actual766960(false), borrowing the real source/controller service and
    /// the SAME Sim/SessionPlayer/ActorStore. Applyingtrue is a distinct path.
    /// The source Find occurs even when the current World/Kingdom is null.
    pub fn create_sai_766960(
        &mut self,
        player: &PlayerHandle,
        actors: &aa_runtime::actor_world::ActorStore,
        application: &aa_runtime::application_mode::GameAppState,
        service: &mut impl aa_runtime::load_sai_players::AddService,
        effects: &mut impl Effects,
    ) -> Result<()> {
        create(&mut Live {
            owner: self,
            player,
            actors,
            application,
            service,
            effects,
        })
    }
}

pub trait Effects {
    /// Read actual Kingdom+234 Nation+10 in editor mode. None certifies the
    /// real null Nation owner; an absent/unbound producer must return error.
    fn editor_nation_text(
        &mut self,
        kingdom: aa_runtime::load_tail_world::KingdomId,
    ) -> Result<Option<Vec<u16>>>;
    fn diagnostic(&mut self, profile_ids: &str, kingdom_name: &[u16]) -> Result<()>;
}
trait World {
    type Profile;
    type Kingdom: Copy;
    fn find_profile(&mut self) -> Result<Option<Self::Profile>>;
    fn world_present(&mut self) -> Result<bool>;
    fn kingdom(&mut self) -> Result<Option<Self::Kingdom>>;
    fn add(
        &mut self,
        kingdom: Self::Kingdom,
        profile: Option<&Self::Profile>,
    ) -> Result<Option<u32>>;
    fn mode(&mut self) -> Result<u32>;
    fn nation_name(&mut self, kingdom: Self::Kingdom) -> Result<Option<Vec<u16>>>;
    fn profile_name(&mut self, slot: u32) -> Result<Vec<u16>>;
    fn kingdom_name(&mut self, kingdom: Self::Kingdom) -> Result<Vec<u16>>;
    fn queue(&mut self, kingdom: &[u16], display: &[u16], profile: &[u16]) -> Result<()>;
    fn diagnostic(&mut self, slot: u32) -> Result<()>;
}
fn create<W: World>(w: &mut W) -> Result<()> {
    let profile = w.find_profile()?;
    if !w.world_present()? {
        return Ok(());
    }
    let Some(kingdom) = w.kingdom()? else {
        return Ok(());
    };
    let Some(slot) = w.add(kingdom, profile.as_ref())? else {
        return Ok(());
    };
    let display = if w.mode()? == 4 {
        if !w.world_present()? {
            return Err(missing("766960 editor Kingdom read reached null World"));
        }
        let k = w
            .kingdom()?
            .ok_or_else(|| missing("766960 editor Kingdom became null"))?;
        match w.nation_name(k)? {
            Some(name) => name,
            None => w.profile_name(slot)?,
        }
    } else {
        w.profile_name(slot)?
    };
    let profile_name = w.profile_name(slot)?;
    if !w.world_present()? {
        return Err(missing("766960 Thing18 read reached null World"));
    }
    let kingdom = w
        .kingdom()?
        .ok_or_else(|| missing("766960 Thing18 read reached null Kingdom"))?;
    let kingdom_name = w.kingdom_name(kingdom)?;
    w.queue(&kingdom_name, &display, &profile_name)?;
    w.diagnostic(slot)
}
struct Live<'a, A, E> {
    owner: &'a mut LocalBootstrap,
    player: &'a PlayerHandle,
    actors: &'a aa_runtime::actor_world::ActorStore,
    application: &'a aa_runtime::application_mode::GameAppState,
    service: &'a mut A,
    effects: &'a mut E,
}
impl<A: aa_runtime::load_sai_players::AddService, E: Effects> World for Live<'_, A, E> {
    type Profile = String;
    type Kingdom = aa_runtime::load_tail_world::KingdomId;
    fn find_profile(&mut self) -> Result<Option<String>> {
        self.service
            .find_profile_name(&self.owner.player(self.player)?.core.text_14)
            .map_err(missing)
    }
    fn world_present(&mut self) -> Result<bool> {
        Ok(!self.owner.sim.world_is_null())
    }
    fn kingdom(&mut self) -> Result<Option<Self::Kingdom>> {
        let p = self.owner.player(self.player)?;
        if !p.core.binding_complete {
            return Err(missing(
                "766960 actual765E10 Kingdom binding has not completed",
            ));
        }
        Ok(p.core.field_28)
    }
    fn add(&mut self, kingdom: Self::Kingdom, profile: Option<&String>) -> Result<Option<u32>> {
        self.service
            .add(
                kingdom,
                profile.map(String::as_str),
                Box::new(self.player.clone()),
            )
            .map_err(missing)
    }
    fn mode(&mut self) -> Result<u32> {
        Ok(self.application.mode())
    }
    fn nation_name(&mut self, k: Self::Kingdom) -> Result<Option<Vec<u16>>> {
        self.effects.editor_nation_text(k)
    }
    fn profile_name(&mut self, slot: u32) -> Result<Vec<u16>> {
        Ok(self
            .service
            .profile_name(slot)
            .map_err(missing)?
            .encode_utf16()
            .collect())
    }
    fn kingdom_name(&mut self, k: Self::Kingdom) -> Result<Vec<u16>> {
        let key = self
            .actors
            .key(k)
            .ok_or_else(|| missing("766960 Kingdom no longer live"))?;
        let e = self
            .actors
            .entity(key)
            .map_err(|e| missing(e.to_string()))?;
        if !matches!(&e.source,aa_runtime::actor_world::Source::Special(s) if matches!(&s.source,aa_runtime::actor_world::SpecialSource::Kingdom(_)))
        {
            return Err(missing("766960 positive Kingdom owner is not Kingdom"));
        }
        Ok(self
            .actors
            .object(key)
            .map_err(|e| missing(e.to_string()))?
            .thing
            .name_utf16
            .clone())
    }
    fn queue(&mut self, k: &[u16], d: &[u16], p: &[u16]) -> Result<()> {
        self.owner.queue_setup_ai_778f20(k, d, p)
    }
    fn diagnostic(&mut self, slot: u32) -> Result<()> {
        if !self.world_present()? {
            return Err(missing("766960 diagnostic reached null World"));
        }
        let k = self
            .kingdom()?
            .ok_or_else(|| missing("766960 diagnostic reached null Kingdom"))?;
        let name = self.kingdom_name(k)?;
        let ids = self.service.profile_ids(slot).map_err(missing)?;
        self.effects.diagnostic(ids, &name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;
    #[test]
    fn local_setup_ai_replays_native_growing_queue_and_embedded_nul() {
        let f: Value = serde_json::from_str(include_str!(
            "../../../research/skirmish-20261016-local-bootstrap/begin-playing/native-setup-ai.json"
        ))
        .unwrap();
        assert_eq!(f["passed"], true);
        assert_eq!(f["cases"].as_array().unwrap().len(), 54);
        let mut owner = LocalBootstrap::from_native_constructors();
        for row in f["cases"].as_array().unwrap() {
            if row["count"] == 1 {
                owner.session.setup_ai.clear_count();
            }
            let a = row["appended"]
                .as_array()
                .unwrap()
                .iter()
                .map(|a| {
                    a.as_array()
                        .unwrap()
                        .iter()
                        .map(|u| u.as_u64().unwrap() as u16)
                        .collect::<Vec<_>>()
                })
                .collect::<Vec<_>>();
            owner.queue_setup_ai_778f20(&a[0], &a[1], &a[2]).unwrap();
            let got = owner
                .setup_ai_entries()
                .iter()
                .map(|e| vec![e.kingdom(), e.display(), e.profile()])
                .collect::<Vec<_>>();
            assert_eq!(serde_json::to_value(got).unwrap(), row["triples"]);
        }
        assert_eq!(owner.setup_ai_entries().len(), 9);
        owner.session.setup_ai.clear_count();
        assert!(owner.setup_ai_entries().is_empty());
    }
    struct Probe {
        row: Value,
        trace: Vec<Value>,
        queued: Vec<Vec<Vec<u16>>>,
        fail: Option<&'static str>,
        names: usize,
    }
    impl Probe {
        fn new(row: Value) -> Self {
            Self {
                row,
                trace: Vec::new(),
                queued: Vec::new(),
                fail: None,
                names: 0,
            }
        }
        fn step(&self, s: &'static str) -> Result<()> {
            if self.fail == Some(s) {
                Err(missing(s))
            } else {
                Ok(())
            }
        }
    }
    fn units(s: &str) -> Vec<u16> {
        s.encode_utf16().collect()
    }
    impl World for Probe {
        type Profile = String;
        type Kingdom = u32;
        fn find_profile(&mut self) -> Result<Option<String>> {
            self.trace
                .push(serde_json::json!(["find", "ChosenProfile"]));
            self.step("find")?;
            Ok((self.row["known"] == true).then(|| "canonical-profile".into()))
        }
        fn world_present(&mut self) -> Result<bool> {
            self.step("world")?;
            Ok(self.row["world"] == true)
        }
        fn kingdom(&mut self) -> Result<Option<u32>> {
            self.step("kingdom")?;
            Ok((self.row["kingdom"] == true).then_some(1))
        }
        fn add(&mut self, _: u32, p: Option<&String>) -> Result<Option<u32>> {
            self.trace.push(serde_json::json!(["add", p.is_some()]));
            self.step("add")?;
            Ok((self.row["fail_slot"] == false).then_some(0))
        }
        fn mode(&mut self) -> Result<u32> {
            self.step("mode")?;
            Ok(self.row["mode"].as_u64().unwrap() as u32)
        }
        fn nation_name(&mut self, _: u32) -> Result<Option<Vec<u16>>> {
            self.step("nation")?;
            Ok((self.row["nation"] == true).then(|| units("ActualNationName")))
        }
        fn profile_name(&mut self, _: u32) -> Result<Vec<u16>> {
            self.names += 1;
            self.step("profile_name")?;
            Ok(units("ActualProfileName"))
        }
        fn kingdom_name(&mut self, _: u32) -> Result<Vec<u16>> {
            self.step("kingdom_name")?;
            Ok(units("KingdomThing"))
        }
        fn queue(&mut self, k: &[u16], d: &[u16], p: &[u16]) -> Result<()> {
            self.step("queue")?;
            self.trace.push(serde_json::json!([
                "queue",
                String::from_utf16(k).unwrap(),
                String::from_utf16(d).unwrap(),
                String::from_utf16(p).unwrap(),
                0
            ]));
            self.queued.push(vec![k.to_vec(), d.to_vec(), p.to_vec()]);
            Ok(())
        }
        fn diagnostic(&mut self, _: u32) -> Result<()> {
            self.trace.push(serde_json::json!(["diagnostic"]));
            self.step("diagnostic")
        }
    }
    #[test]
    fn local_create_sai_false_replays_all_native_caller_paths_and_partial_queue() {
        let f:Value=serde_json::from_str(include_str!("../../../research/skirmish-20261016-local-bootstrap/begin-playing/native-create-sai.json")).unwrap();
        assert_eq!(f["passed"], true);
        assert_eq!(f["cases"].as_array().unwrap().len(), 42);
        for row in f["cases"].as_array().unwrap() {
            let mut p = Probe::new(row.clone());
            create(&mut p).unwrap();
            assert_eq!(serde_json::to_value(&p.trace).unwrap(), row["trace"]);
            assert_eq!(serde_json::to_value(&p.queued).unwrap(), row["queued"]);
            if !p.queued.is_empty() {
                assert_eq!(
                    p.names,
                    if row["mode"] == 4 && row["nation"] == true {
                        1
                    } else {
                        2
                    }
                );
            }
        }
        let row = f["cases"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| !r["queued"].as_array().unwrap().is_empty())
            .unwrap();
        for failure in [
            "find",
            "world",
            "kingdom",
            "add",
            "mode",
            "profile_name",
            "kingdom_name",
            "queue",
            "diagnostic",
        ] {
            let mut p = Probe::new(row.clone());
            p.fail = Some(failure);
            assert!(create(&mut p).is_err());
            assert_eq!(p.queued.len(), usize::from(failure == "diagnostic"));
        }
    }
}
