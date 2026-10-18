//!7757F0 caller ordering. Native successful film construction/open/preamble
//!is supported by the kernel; C++ exception retry/notification is a separate
//!required producer, never an arbitrary Rust-error suppression. A private
//!kernel replay is not an application activation receipt.
use super::*;
#[path = "local_activated_session.rs"]
mod activated;
#[path = "local_world_ready_actual.rs"]
pub(crate) mod actual;
#[path = "local_world_success.rs"]
pub(crate) mod success;
#[path = "local_task_protocol.rs"]
pub(crate) mod tasks;
pub use activated::{ActivationFailure, ActualActivatedSession};
pub use actual::Diagnostic;
pub use success::SuccessDiagnostic;

impl LocalBootstrap {
    pub fn current_film_name_150(&self) -> &[u16] {
        &self.session.film_name_150
    }
    ///775813..21 precedes every state test, including native skipped states.
    pub(crate) fn reset_world_ready_film_name(
        &mut self,
        application: &aa_runtime::application_mode::GameAppState,
    ) {
        self.session.film_name_150 = application.strings().default_b43b40().to_vec();
    }
}
trait World {
    type Film;
    fn reset_name(&mut self) -> Result<()>;
    fn state(&mut self) -> Result<u32>;
    fn phase(&mut self) -> Result<u32>;
    fn old_film(&mut self) -> Result<bool>;
    fn detach(&mut self) -> Result<()>;
    fn close(&mut self) -> Result<()>;
    fn clear_film(&mut self) -> Result<()>;
    fn prepare(&mut self) -> Result<()>;
    fn rotation(&mut self) -> Result<i32>;
    fn filename(&mut self) -> Result<Vec<u16>>;
    fn opening(&mut self, name: &[u16]) -> Result<()>;
    fn extension(&mut self, name: &mut Vec<u16>) -> Result<()>;
    fn path(&mut self) -> Result<Vec<u16>>;
    fn join(&mut self, path: &[u16], name: &[u16]) -> Result<Vec<u16>>;
    fn construct(&mut self, path: &[u16]) -> Result<Self::Film>;
    fn publish(&mut self, film: Self::Film) -> Result<()>;
    fn open(&mut self) -> Result<()>;
    fn record(&mut self) -> Result<()>;
    fn sai(&mut self) -> Result<()>;
    fn playback(&mut self) -> Result<()>;
    fn playback_name_present(&mut self) -> Result<bool>;
    fn store_playback_flag(&mut self, value: bool) -> Result<()>;
}
///Private control kernel. Only a future fully concrete owned caller may use
///its successful return as one prerequisite of ActualActivatedSession.
fn run<W: World>(w: &mut W) -> Result<()> {
    w.reset_name()?;
    if matches!(w.state()?, 2 | 5) {
        //Captured before old film/Prepare callbacks; don't reread F8 later.
        let recording = !matches!(w.phase()?, 7 | 14);
        if w.old_film()? {
            w.detach()?;
            w.close()?;
        }
        w.clear_film()?;
        w.prepare()?;
        if recording {
            //The loop bound is captured once. On a successful native attempt
            //the loop immediately terminates. A reached real film exception
            //requires native removal/retry/last-attempt notification handling.
            let rotation = w.rotation()?;
            if rotation > 0 {
                let mut name = w.filename()?;
                w.opening(&name)?;
                w.extension(&mut name)?;
                let path = w.path()?;
                let path = w.join(&path, &name)?;
                let film = w.construct(&path)?;
                w.publish(film)?;
                w.open()?;
                w.record()?;
            }
        }
        w.sai()?;
    }
    //This is an independent CURRENT E4 test, not else-if over a snapshot.
    if w.state()? == 4 {
        w.playback()?;
        w.sai()?;
        let value = w.playback_name_present()?;
        w.store_playback_flag(value)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};
    fn words(s: &str) -> Vec<u16> {
        s.encode_utf16().collect()
    }
    struct Current {
        input: Value,
        events: Vec<Value>,
        name: Vec<u16>,
        state: u32,
        phase: u32,
        film: bool,
        flag: u8,
    }
    impl Current {
        fn event(&mut self, kind: &str, fields: Value) -> Result<()> {
            if self.input["stop"].as_str() == Some(kind) {
                return Err(missing(kind));
            }
            let mut value = json!({"kind":kind});
            for (k, v) in fields.as_object().unwrap() {
                value[k] = v.clone();
            }
            self.events.push(value);
            Ok(())
        }
    }
    impl World for Current {
        type Film = ();
        fn reset_name(&mut self) -> Result<()> {
            self.name.clear();
            Ok(())
        }
        fn state(&mut self) -> Result<u32> {
            Ok(self.state)
        }
        fn phase(&mut self) -> Result<u32> {
            Ok(self.phase)
        }
        fn old_film(&mut self) -> Result<bool> {
            Ok(self.film)
        }
        fn detach(&mut self) -> Result<()> {
            self.event("detach", json!({}))
        }
        fn close(&mut self) -> Result<()> {
            self.event("close", json!({}))
        }
        fn clear_film(&mut self) -> Result<()> {
            self.film = false;
            Ok(())
        }
        fn prepare(&mut self) -> Result<()> {
            self.event("prepare", json!({}))?;
            if let Some(v) = self.input["prepare_state"].as_u64() {
                self.state = v as u32;
            }
            if let Some(v) = self.input["prepare_phase"].as_u64() {
                self.phase = v as u32;
            }
            Ok(())
        }
        fn rotation(&mut self) -> Result<i32> {
            let v = self.input["rotation"].as_i64().unwrap() as i32;
            self.event("rotation", json!({"value":v as u32}))?;
            Ok(v)
        }
        fn filename(&mut self) -> Result<Vec<u16>> {
            self.event("filename", json!({}))?;
            self.name = words("chosen_name");
            if let Some(v) = self.input["filename_rotation"].as_i64() {
                self.input["rotation"] = json!(v);
            }
            Ok(self.name.clone())
        }
        fn opening(&mut self, name: &[u16]) -> Result<()> {
            self.event("opening", json!({"name":name}))
        }
        fn extension(&mut self, name: &mut Vec<u16>) -> Result<()> {
            let extension = words("RFM");
            self.event("extension", json!({"extension":extension}))?;
            aa_runtime::film_paths::append_extension(name, &extension);
            Ok(())
        }
        fn path(&mut self) -> Result<Vec<u16>> {
            self.event("path", json!({}))?;
            Ok(words("films/"))
        }
        fn join(&mut self, a: &[u16], b: &[u16]) -> Result<Vec<u16>> {
            let value = aa_runtime::film_paths::join(a, b);
            self.event("join", json!({"value":value}))?;
            Ok(value)
        }
        fn construct(&mut self, path: &[u16]) -> Result<()> {
            self.event("construct", json!({"ids":path,"creator":true,"rules":true}))
        }
        fn publish(&mut self, _: ()) -> Result<()> {
            self.film = true;
            Ok(())
        }
        fn open(&mut self) -> Result<()> {
            self.event("open", json!({"published":self.film}))
        }
        fn record(&mut self) -> Result<()> {
            self.event("record", json!({}))
        }
        fn sai(&mut self) -> Result<()> {
            self.event("sai", json!({}))?;
            if let Some(v) = self.input["sai_state"].as_u64() {
                self.state = v as u32;
            }
            Ok(())
        }
        fn playback(&mut self) -> Result<()> {
            self.event("playback", json!({}))
        }
        fn playback_name_present(&mut self) -> Result<bool> {
            Ok(self.input["playback_name"] != false)
        }
        fn store_playback_flag(&mut self, v: bool) -> Result<()> {
            self.flag = u8::from(v);
            Ok(())
        }
    }
    #[test]
    fn original_v14_world_ready_native_order_and_required_prefixes() {
        let fixture: Value = serde_json::from_str(include_str!(
            "../../../research/development10/world-ready-7757f0/native-world-ready.json"
        ))
        .unwrap();
        assert_eq!(fixture["passed"], true);
        assert_eq!(fixture["cases"].as_array().unwrap().len(), 516);
        for row in fixture["cases"].as_array().unwrap() {
            let input = row["input"].clone();
            let mut current = Current {
                state: input["state"].as_u64().unwrap() as u32,
                phase: input["phase"].as_u64().unwrap() as u32,
                film: input["old"].as_bool().unwrap(),
                input,
                name: words("old_name"),
                events: Vec::new(),
                flag: 0xa5,
            };
            let result = run(&mut current);
            assert_eq!(result.is_err(), !row["stop"].is_null(), "{row}");
            assert_eq!(json!(current.events), row["events"], "{row}");
            assert_eq!(json!(current.name), row["name"], "{row}");
            assert_eq!(current.state, row["state"].as_u64().unwrap() as u32);
            assert_eq!(current.phase, row["phase"].as_u64().unwrap() as u32);
            assert_eq!(current.film, row["film"].as_bool().unwrap());
            assert_eq!(current.flag, row["flag168"].as_u64().unwrap() as u8);
        }
        let owner = LocalBootstrap::from_native_constructors();
        assert!(owner.current_film_name_150().is_empty());
    }
}
