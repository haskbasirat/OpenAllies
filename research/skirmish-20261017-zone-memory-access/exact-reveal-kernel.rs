use aa_actors::actor_creation::{Error,Result};
pub(crate) trait World {
    fn shown(&mut self) -> Result<bool>;
    fn seen(&mut self) -> Result<bool>;
    fn show(&mut self) -> Result<()>;
    fn explore(&mut self) -> Result<()>;
    fn actor_b4(&mut self) -> Result<bool>;
    fn container(&mut self) -> Result<bool>;
    fn reveal_container(&mut self) -> Result<()>;
    fn memory(&mut self) -> Result<()>;
    fn announce(&mut self) -> Result<u8>;
    fn first_sighting(&mut self) -> Result<()>;
}
pub(crate) fn reveal_inner(world: &mut impl World, explored: bool, visible: bool) -> Result<()> {
    let update = if visible && !world.shown()? {
        true
    } else if !explored {
        false
    } else {
        !world.seen()?
    };
    if !update {
        return Ok(());
    }
    world.shown()?;
    let was_seen = world.seen()?;
    if visible {
        world.show()?;
    }
    if explored {
        world.explore()?;
        if world.actor_b4()? && world.container()? {
            world.reveal_container()?;
        }
    }
    world.memory()?;
    if world.announce()? != 0 && explored && !was_seen {
        world.first_sighting()?;
    }
    Ok(())
}

mod tests{
use super::*;
use serde_json::{Value, json};
struct Probe<'a> {
    c: &'a Value,
    word: u32,
    trace: Vec<&'static str>,
    stop: Option<&'static str>,
}
impl Probe<'_> {
    fn bit(&self, o: u32) -> u32 {
        1u32.wrapping_shl(self.c["index"].as_u64().unwrap() as u32 + o)
    }
    fn major(&mut self) -> Result<bool> {
        if self.c["null"].as_bool().unwrap() {
            self.stop = Some("null");
            return Err(Error::NullDereference("explicit null Kingdom probe"));
        }
        Ok(self.c["nonmajor"] == 0)
    }
    fn stop(&mut self, s: &'static str) -> Result<()> {
        self.stop = Some(s);
        Err(Error::Unsupported(s))
    }
}
impl World for Probe<'_> {
    fn shown(&mut self) -> Result<bool> {
        self.trace.push("shown");
        Ok(!self.major()? || self.word & self.bit(16) != 0)
    }
    fn seen(&mut self) -> Result<bool> {
        self.trace.push("seen");
        Ok(!self.major()? || self.word & self.bit(0) != 0)
    }
    fn show(&mut self) -> Result<()> {
        self.trace.push("show");
        if self.major()? {
            self.word |= self.bit(16);
        }
        Ok(())
    }
    fn explore(&mut self) -> Result<()> {
        self.trace.push("explore");
        if self.major()? {
            self.word |= self.bit(0);
        }
        Ok(())
    }
    fn actor_b4(&mut self) -> Result<bool> {
        self.trace.push("b4");
        Ok(true)
    }
    fn container(&mut self) -> Result<bool> {
        self.trace.push("container");
        if self.c["structure"].as_bool().unwrap() && self.c["container"].as_bool().unwrap() {
            return Ok(true);
        }
        if self.c["wall"].as_bool().unwrap() {
            self.trace.push("wall");
            self.stop("wall")?;
        }
        Ok(false)
    }
    fn reveal_container(&mut self) -> Result<()> {
        self.trace.push("recursive");
        self.stop("recursive")
    }
    fn memory(&mut self) -> Result<()> {
        self.trace.push("memory");
        if self.c["memory"] == "stop" {
            self.stop("memory")?;
        }
        Ok(())
    }
    fn announce(&mut self) -> Result<u8> {
        self.trace.push("announce");
        Ok(self.c["announce"].as_u64().unwrap() as u8)
    }
    fn first_sighting(&mut self) -> Result<()> {
        self.trace.push("first");
        self.stop("first")
    }
}
#[test]
fn exact_actor64_calls_and_partial_word_stores_match_original_v14_all_pc() {
    let f: Value = serde_json::from_slice(include_bytes!(
        "E:/Games/Axis and Alies/rust-rebuild/research/skirmish-20261016-load-actor-reveal/native-actor-reveal.json"
    ))
    .unwrap();
    let rows = f["rows"].as_array().unwrap();
    assert_eq!(rows.len(), 117);
    for r in rows {
        let c = &r["input"];
        let n = &r["native"];
        let mut p = Probe {
            c,
            word: c["word"].as_u64().unwrap() as u32,
            trace: vec![],
            stop: None,
        };
        let result = reveal_inner(
            &mut p,
            c["explore"].as_bool().unwrap(),
            c["show"].as_bool().unwrap(),
        );
        assert_eq!(json!(p.word), n["word"], "{r}");
        assert_eq!(json!(p.trace), n["trace"], "{r}");
        assert_eq!(json!(p.stop), n["stop"], "{r}");
        assert_eq!(result.is_ok(), p.stop.is_none(), "{r}");
    }
}

}
