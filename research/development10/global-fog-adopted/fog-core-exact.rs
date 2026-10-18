use aa_actors::{ActorId,actor_creation::{Error,Result}};
fn error(e: impl std::fmt::Display) -> Error {
    Error::World(e.to_string())
}
trait World {
    type Node: Copy + Ord;
    fn tile_count(&mut self) -> Result<i32>;
    fn tile_explored(&mut self, tile: i32) -> Result<()>;
    fn tile_visible(&mut self, tile: i32) -> Result<()>;
    fn first(&mut self, tile: i32) -> Result<Option<Self::Node>>;
    fn next(&mut self, node: Self::Node) -> Result<Option<Self::Node>>;
    fn actor(&mut self, node: Self::Node) -> Result<ActorId>;
    fn flags(&mut self, actor: ActorId) -> Result<u32>;
    fn set_flags(&mut self, actor: ActorId, flags: u32) -> Result<()>;
    fn is_object(&mut self, actor: ActorId) -> Result<bool>;
    fn associated_current(&mut self, actor: ActorId) -> Result<bool>;
    fn explore(&mut self, actor: ActorId) -> Result<()>;
    fn show(&mut self, actor: ActorId) -> Result<()>;
}

///54F400 and54F4B0 reread node.next after each callback. A Rust refusal at a
///callback does not run the later native bit30 restoration or next-node read.
fn tile_explore(w: &mut impl World, tile: i32) -> Result<()> {
    w.tile_explored(tile)?;
    let mut node = w.first(tile)?;
    let mut visited = std::collections::BTreeSet::new();
    while let Some(n) = node {
        if !visited.insert(n) {
            return Err(error("54F400 corrupt cyclic native node list"));
        }
        let actor = w.actor(n)?;
        let flags = w.flags(actor)?;
        if flags & (1 << 20) != 0 || flags & (1 << 11) != 0 && flags & (1 << 23) == 0 {
            let temporary_clear = w.is_object(actor)?
                && w.associated_current(actor)?
                && w.flags(actor)? & (1 << 30) != 0;
            if temporary_clear {
                let flags = w.flags(actor)?;
                w.set_flags(actor, flags & !(1 << 30))?;
            }
            w.explore(actor)?;
            if temporary_clear {
                let flags = w.flags(actor)?;
                w.set_flags(actor, flags | (1 << 30))?;
            }
        }
        node = w.next(n)?;
    }
    Ok(())
}
fn tile_show(w: &mut impl World, tile: i32) -> Result<()> {
    w.tile_visible(tile)?;
    let mut node = w.first(tile)?;
    let mut visited = std::collections::BTreeSet::new();
    while let Some(n) = node {
        if !visited.insert(n) {
            return Err(error("54F4B0 corrupt cyclic native node list"));
        }
        let actor = w.actor(n)?;
        w.show(actor)?;
        node = w.next(n)?;
    }
    Ok(())
}
fn all(w: &mut impl World, visible: bool) -> Result<()> {
    let mut tile = 0_i32;
    while tile < w.tile_count()? {
        tile_explore(w, tile)?;
        if visible {
            tile_show(w, tile)?;
        }
        tile = tile.wrapping_add(1);
    }
    Ok(())
}


#[cfg(test)] mod tests {
use super::*;
use serde_json::{Value, json};

struct Probe {
    input: Value,
    width: i32,
    tiles: [u32; 2],
    flags: [u32; 3],
    words: [u32; 3],
    next: [Option<usize>; 3],
    events: Vec<Value>,
    outcome: String,
}
impl Probe {
    fn new(input: &Value) -> Self {
        let flags = input["flags"].as_u64().unwrap() as u32;
        Self {
            input: input.clone(),
            width: 2,
            tiles: [0x12345678, 0xabcdef01],
            flags: [flags, 0, flags],
            words: [0x10203040, 0x10203041, 0x10203042],
            next: [Some(1), Some(2), None],
            events: Vec::new(),
            outcome: "complete".into(),
        }
    }
    fn index(actor: ActorId) -> usize {
        actor.raw() as usize - 1
    }
    fn memory(&mut self, actor: ActorId) -> Result<()> {
        let i = Self::index(actor);
        let mode = if self.words[i] & 0xffff0000 == 0xffff0000 {
            "visible"
        } else {
            "explored"
        };
        self.events
            .push(json!({"actor":i,"mode":mode,"flags":self.flags[i],"word":self.words[i]}));
        if self.input["stop"] == mode {
            self.outcome = format!("4EBC70:{mode}");
            return Err(Error::Unsupported("test required native4EBC70 entry"));
        }
        if self.input["mutate"] == true && self.events.len() == 1 {
            self.flags[i] ^= 0x10;
            self.next[0] = Some(2);
            self.width = 1;
        }
        Ok(())
    }
    fn result(&self) -> Value {
        json!({"outcome":self.outcome,"tiles":self.tiles,
            "actors":(0..3).map(|i|json!({"flags":self.flags[i],"word":self.words[i]})).collect::<Vec<_>>(),
            "events":self.events,"width":self.width,"next_first":self.next[0].unwrap()})
    }
}
impl World for Probe {
    type Node = usize;
    fn tile_count(&mut self) -> Result<i32> {
        Ok(self.width)
    }
    fn tile_explored(&mut self, tile: i32) -> Result<()> {
        self.tiles[tile as usize] = (self.tiles[tile as usize] & 0xffff0000) | 0xffff;
        Ok(())
    }
    fn tile_visible(&mut self, tile: i32) -> Result<()> {
        self.tiles[tile as usize] = (self.tiles[tile as usize] & 0xffff) | 0xffff0000;
        Ok(())
    }
    fn first(&mut self, tile: i32) -> Result<Option<usize>> {
        Ok((tile == 0).then_some(0))
    }
    fn next(&mut self, node: usize) -> Result<Option<usize>> {
        Ok(self.next[node])
    }
    fn actor(&mut self, node: usize) -> Result<ActorId> {
        Ok(ActorId::from_raw(node as u32 + 1))
    }
    fn flags(&mut self, actor: ActorId) -> Result<u32> {
        Ok(self.flags[Self::index(actor)])
    }
    fn set_flags(&mut self, actor: ActorId, flags: u32) -> Result<()> {
        self.flags[Self::index(actor)] = flags;
        Ok(())
    }
    fn is_object(&mut self, _: ActorId) -> Result<bool> {
        Ok(true)
    }
    fn associated_current(&mut self, _: ActorId) -> Result<bool> {
        Ok(self.input["associated"].as_bool().unwrap())
    }
    fn explore(&mut self, actor: ActorId) -> Result<()> {
        let i = Self::index(actor);
        self.words[i] = (self.words[i] & 0xffff0000) | 0xffff;
        self.memory(actor)
    }
    fn show(&mut self, actor: ActorId) -> Result<()> {
        let i = Self::index(actor);
        self.words[i] = (self.words[i] & 0xffff) | 0xffff0000;
        self.memory(actor)
    }
}

#[test]
fn all_native_global_fog_rows_preserve_live_nodes_flags_and_failure_prefixes() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../research/skirmish-20261016-load-store-fog/native-store-fog.json");
    let fixture: Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    let rows = fixture["rows"].as_array().unwrap();
    assert_eq!(rows.len(), 1728);
    for row in rows {
        let mut world = Probe::new(&row["input"]);
        let result = all(&mut world, row["input"]["visible"].as_bool().unwrap());
        assert_eq!(
            result.is_ok(),
            row["native"]["outcome"] == "complete",
            "{row}"
        );
        assert_eq!(world.result(), row["native"], "{row}");
    }
}


}
