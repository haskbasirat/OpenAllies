use aa_actors::ActorId;
use aa_body::credit::Object;
type Result<T> = std::result::Result<T,String>;
/// Read services for the unchanged5784C0 coordinator. There is deliberately no
/// snapshot `Vec<OwnerProperty>`: an apply may change the captured list/count.
pub(crate) trait LookupWorld {
    type Properties: Copy;
    fn properties(&self, actor: ActorId) -> Result<Option<Self::Properties>>;
    fn count(&self, properties: Self::Properties) -> Result<i32>;
    fn slot(&self, properties: Self::Properties, index: i32) -> Result<Object>;
    fn instance_definition(&self, instance: Object) -> Result<Object>;
    fn instance_source(&self, instance: Object) -> Result<ActorId>;
}

pub(crate) trait World: LookupWorld {
    fn owner(&self) -> Result<ActorId>;
    fn pass_down_members(&self, definition: Object) -> Result<bool>;
    fn has_experience(&self, actor: ActorId) -> Result<bool>;
    fn instance_kind(&self, instance: Object) -> Result<u32>;
    fn instance_disabled(&self, instance: Object) -> Result<bool>;
    fn apply(&mut self, actor: ActorId, property: Object, kind: u32, source: ActorId)
    -> Result<()>;
}

fn lookup(
    world: &impl LookupWorld,
    actor: ActorId,
    property: Object,
    source: ActorId,
) -> Result<Option<Object>> {
    let Some(properties) = world.properties(actor)? else {
        return Ok(None);
    };
    //5032D6 captures the count and original vector base. Registry resolution
    //is a pure read; there is no callback which may replace that allocation.
    let count = world.count(properties)?;
    for index in 0..count {
        let instance = world.slot(properties, index)?;
        if world.instance_definition(instance)? == property
            && (source.is_null() || world.instance_source(instance)? == source)
        {
            return world.slot(properties, index).map(Some);
        }
    }
    Ok(None)
}

pub(crate) fn propagate(world: &mut impl World, member: ActorId) -> Result<()> {
    let owner = world.owner()?;
    let Some(properties) = world.properties(owner)? else {
        return Ok(());
    };
    let mut index = 0i32;
    if world.count(properties)? <= 0 {
        return Ok(());
    }
    loop {
        //5784E2/E5 fetches this live slot even for a null member. It does not
        //read the instance itself until after the member+78 presence gate.
        let instance = world.slot(properties, index)?;
        if !member.is_null() && world.properties(member)?.is_some() {
            let definition = world.instance_definition(instance)?;
            if world.pass_down_members(definition)?
                && !(world.has_experience(member)? && world.instance_kind(instance)? == 7)
                && !world.instance_disabled(instance)?
            {
                //578512 rereads org+4;578515 rereads instance+8 even when the
                //Experience gate has already read it. Property ECX is retained.
                let source = world.owner()?;
                let kind = world.instance_kind(instance)?;
                world.apply(member, definition, kind, source)?;
            }
        }
        let count = world.count(properties)?;
        index = index.wrapping_add(1);
        if index >= count {
            return Ok(());
        }
    }
}


#[cfg(test)] mod tests {
use super::*;
use std::{cell::RefCell,path::PathBuf};
use serde_json::{Value,json};
fn evidence(name:&str)->PathBuf { PathBuf::from("E:\\Games\\Axis and Alies\\rust-rebuild\\research\\skirmish-20261016-member-properties").join(name) }
struct Probe {
    input: Value,
    entries: Vec<Value>,
    trace: RefCell<Vec<Value>>,
    applies: usize,
    owner: u32,
}
impl Probe {
    fn event(&self, value: Value) {
        self.trace.borrow_mut().push(value);
    }
    fn instance(&self, id: Object) -> Result<&Value> {
        self.entries
            .iter()
            .find(|row| row["handle"] == id.0)
            .ok_or_else(|| "instance_required".into())
    }
}
impl LookupWorld for Probe {
    type Properties = u32;
    fn properties(&self, actor: ActorId) -> Result<Option<u32>> {
        let present = if actor.raw() == 4099 {
            !self.input["properties"].is_null()
        } else {
            self.input["member"] != "absent"
        };
        self.event(json!(["properties", actor.raw(), present]));
        Ok(present.then_some(actor.raw()))
    }
    fn count(&self, _: u32) -> Result<i32> {
        let count = self.entries.len() as i32;
        self.event(json!(["count", count]));
        Ok(count)
    }
    fn slot(&self, _: u32, index: i32) -> Result<Object> {
        let handle = self.entries[index as usize]["handle"].as_u64().unwrap() as u32;
        self.event(json!(["slot", index, handle]));
        Ok(Object(handle))
    }
    fn instance_definition(&self, id: Object) -> Result<Object> {
        let def = self.instance(id)?["definition"].as_u64().unwrap() as u32;
        self.event(json!(["definition", id.0, def]));
        Ok(Object(def))
    }
    fn instance_source(&self, id: Object) -> Result<ActorId> {
        let value = self.instance(id)?["resolved_source"].as_u64().unwrap() as u32;
        self.event(json!(["source", id.0, value]));
        Ok(ActorId::from_raw(value))
    }
}
impl World for Probe {
    fn owner(&self) -> Result<ActorId> {
        self.event(json!(["owner", self.owner]));
        Ok(ActorId::from_raw(self.owner))
    }
    fn pass_down_members(&self, def: Object) -> Result<bool> {
        let value = self
            .entries
            .iter()
            .find(|row| row["definition"] == def.0)
            .unwrap()["pass_down"]
            .as_bool()
            .unwrap();
        self.event(json!(["pass_down", def.0, value]));
        Ok(value)
    }
    fn has_experience(&self, actor: ActorId) -> Result<bool> {
        let value = self.input["member"] == "experience";
        self.event(json!(["experience", actor.raw(), value]));
        Ok(value)
    }
    fn instance_kind(&self, id: Object) -> Result<u32> {
        let value = self.instance(id)?["kind"].as_u64().unwrap() as u32;
        self.event(json!(["kind", id.0, value]));
        Ok(value)
    }
    fn instance_disabled(&self, id: Object) -> Result<bool> {
        let value = self.instance(id)?["disabled"].as_bool().unwrap();
        self.event(json!(["disabled", id.0, value]));
        Ok(value)
    }
    fn apply(
        &mut self,
        actor: ActorId,
        property: Object,
        kind: u32,
        source: ActorId,
    ) -> Result<()> {
        self.event(json!([
            "apply",
            actor.raw(),
            property.0,
            kind,
            source.raw()
        ]));
        self.applies += 1;
        match self.input["callback"].as_str().unwrap() {
            "required" => return Err("apply_required".into()),
            "truncate" => self.entries.truncate(1),
            "append" if self.applies == 1 => {
                let extra = self.input["extra"].clone();
                self.entries.push(extra);
            }
            "replace_next" if self.applies == 1 && self.entries.len() > 1 => {
                self.entries[1] = self.input["extra"].clone();
            }
            "replace_owner" if self.applies == 1 => self.owner = 4101,
            _ => {}
        }
        Ok(())
    }
}

#[test]
fn coordinator_live_reads_and_guard_order_match_original_and_v14() {
    let fixture: Value =
        serde_json::from_slice(&std::fs::read(evidence("native-member-properties.json")).unwrap())
            .unwrap();
    let mut cases = 0;
    for row in fixture["cases"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|row| row["scope"] == "coordinator")
    {
        let input = row["input"].clone();
        let entries = input["properties"].as_array().cloned().unwrap_or_default();
        let member = if input["member"] == "null" {
            ActorId::NULL
        } else {
            ActorId::from_raw(4100)
        };
        let mut world = Probe {
            input,
            entries,
            trace: RefCell::new(vec![]),
            applies: 0,
            owner: 4099,
        };
        let result = propagate(&mut world, member);
        let outcome = result.err().unwrap_or_else(|| "ok".into());
        assert_eq!(
            json!({"outcome":outcome,"trace":world.trace.into_inner(),"entries":world.entries}),
            row["output"],
            "{}",
            row["name"]
        );
        cases += 1;
    }
    assert!(
        cases >= 90,
        "replay both images/all PCs and all guard shapes"
    );
}

#[test]
fn lookup_guards_disabled_records_and_source_generation_match_original_and_v14() {
    let fixture: Value =
        serde_json::from_slice(&std::fs::read(evidence("native-member-properties.json")).unwrap())
            .unwrap();
    let mut count = 0;
    for row in fixture["cases"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|row| row["scope"] == "lookup")
    {
        let input = row["input"].clone();
        let entries = input["properties"].as_array().cloned().unwrap_or_default();
        let world = Probe {
            input: input.clone(),
            entries,
            trace: RefCell::new(vec![]),
            applies: 0,
            owner: 4099,
        };
        let found = lookup(
            &world,
            ActorId::from_raw(4099),
            Object(input["property"].as_u64().unwrap() as u32),
            ActorId::from_raw(input["source"].as_u64().unwrap() as u32),
        )
        .unwrap();
        assert_eq!(
            json!({"found":found.map(|id|id.0),"trace":world.trace.into_inner()}),
            row["output"],
            "{}",
            row["name"]
        );
        count += 1;
    }
    assert!(count >= 48);
}


}
