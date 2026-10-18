//!511297..51145E: enabled, condition-free triggers expose opcode50
//!kingdom_gains_objective choices. Two native54DE30 scratch registers are
//!reused across actions; only storage-owned words are overwritten.
use crate::{
    actor_world::{ActorStore, ObjectKey},
    localization::Localization,
    names,
};
use aa_triggers::{
    decode::{Parameter, Value},
    manager::Manager,
};
pub type Result<T> = std::result::Result<T, String>;
pub struct Register {
    pub word: u32,
    pub text: Vec<u16>,
    pub key: Vec<u16>,
    pub localized_text: Vec<u16>,
    pub position: [u32; 3],
    pub kind: u32,
    pub item: u32,
}
impl Register {
    pub fn from_native_constructor() -> Self {
        Self {
            word: 0,
            text: Vec::new(),
            key: Vec::new(),
            localized_text: Vec::new(),
            position: [0; 3],
            kind: u32::MAX,
            item: 0,
        }
    }
}
///5422F0/54A5A0. Extra scratch has two records; more parameters would overwrite
///the native caller stack and remain an explicit unsupported input domain.
pub fn convert(parameters: &[Parameter], registers: &mut [Register; 2]) -> Result<()> {
    if parameters.len() > 2 {
        return Err("film objective extra has more than two native scratch parameters".into());
    }
    for (p, r) in parameters.iter().zip(registers) {
        r.kind = 2;
        r.item = p.item_index;
        match p.storage {
            1 | 2 => {
                let Value::Word(w) = p.value else {
                    return Err("film extra word parameter has no actual storage".into());
                };
                r.word = w;
            }
            5 => {
                let Value::Position(v) = p.value else {
                    return Err("film extra position parameter has no actual storage".into());
                };
                r.position = v;
            }
            6 => {
                let Value::Text(v) = &p.value else {
                    return Err("film extra string parameter has no actual storage".into());
                };
                r.text = v.clone();
            }
            7 => {
                let Value::Localized {
                    key,
                    inline: Some(text),
                } = &p.value
                else {
                    return Err("film extra localized parameter companion is unbound".into());
                };
                r.key = key.clone();
                r.localized_text = text.clone();
            }
            _ => r.word = 0,
        }
    }
    Ok(())
}
pub trait Target {
    fn find_50dc00(&self, name: &[u16]) -> Option<usize>;
    fn append_510150(&mut self, index: usize, key: Vec<u16>, text: Vec<u16>) -> Result<()>;
}
pub trait Fields {
    type Trigger: Copy;
    type Action: Copy;
    type Actor;
    type Objective;
    fn first_trigger(&mut self) -> Result<Option<Self::Trigger>>;
    fn next_trigger(&mut self, t: Self::Trigger) -> Result<Option<Self::Trigger>>;
    fn enabled(&mut self, t: Self::Trigger) -> Result<u8>;
    fn conditions_present(&mut self, t: Self::Trigger) -> Result<bool>;
    fn first_action(&mut self, t: Self::Trigger) -> Result<Option<Self::Action>>;
    fn next_action(&mut self, a: Self::Action) -> Result<Option<Self::Action>>;
    fn opcode(&mut self, a: Self::Action) -> Result<u32>;
    fn convert_5422f0(&mut self, a: Self::Action, r: &mut [Register; 2]) -> Result<()>;
    fn kingdom_54dac0(&mut self, r: &Register) -> Result<Option<Self::Actor>>;
    fn objective_54da70(&mut self, r: &Register) -> Result<Option<Self::Objective>>;
    fn name_18(&mut self, actor: &Self::Actor) -> Result<Vec<u16>>;
    fn localized_49ad00(&mut self, objective: &Self::Objective) -> Result<(Vec<u16>, Vec<u16>)>;
}
pub fn capture_fields(target: &mut impl Target, source: &mut impl Fields) -> Result<()> {
    let mut registers = [
        Register::from_native_constructor(),
        Register::from_native_constructor(),
    ];
    let mut next = source.first_trigger()?;
    while let Some(t) = next {
        if source.enabled(t)? != 0 && !source.conditions_present(t)? {
            let mut action = source.first_action(t)?;
            while let Some(a) = action {
                if source.opcode(a)? == 50 {
                    source.convert_5422f0(a, &mut registers)?;
                    if let Some(actor) = source.kingdom_54dac0(&registers[0])? {
                        if let Some(objective) = source.objective_54da70(&registers[1])? {
                            let name = source.name_18(&actor)?;
                            let index = target.find_50dc00(&name);
                            //Native49AD00 runs even when50DC00 returned null.
                            //A later invalid destination cannot erase its prefix.
                            let (key, text) = source.localized_49ad00(&objective)?;
                            target.append_510150(index.ok_or("film objective extra would dereference null KingdomParameter+40")?,key,text)?;
                        }
                    }
                }
                action = source.next_action(a)?;
            }
        }
        next = source.next_trigger(t)?;
    }
    Ok(())
}
///Actual OBJECTIVES allocation/lookup owner, distinct from MAPSTRINGS and
///TriggerManager.strings. No default absent-list or manufactured entry exists.
pub trait Objectives {
    type Objective;
    fn find_4f1ad0(&mut self, name: &[u16]) -> Result<Option<Self::Objective>>;
    fn ids_4(&mut self, objective: &Self::Objective) -> Result<Vec<u16>>;
}
impl Objectives for crate::world_objectives::State {
    type Objective = crate::world_objectives::Handle;
    fn find_4f1ad0(&mut self, name: &[u16]) -> Result<Option<Self::Objective>> {
        self.find_handle(name)
    }
    fn ids_4(&mut self, objective: &Self::Objective) -> Result<Vec<u16>> {
        Ok(self.get(objective)?.key().to_vec())
    }
}
struct Live<'a, O> {
    actors: &'a ActorStore,
    names: &'a names::Table,
    manager: &'a Manager,
    objectives: &'a mut O,
    text: &'a Localization,
}
impl<O: Objectives> Fields for Live<'_, O> {
    type Trigger = usize;
    type Action = (usize, usize);
    type Actor = ObjectKey;
    type Objective = O::Objective;
    fn first_trigger(&mut self) -> Result<Option<usize>> {
        Ok((!self.manager.definitions().is_empty()).then_some(0))
    }
    fn next_trigger(&mut self, t: usize) -> Result<Option<usize>> {
        Ok((t + 1 < self.manager.definitions().len()).then_some(t + 1))
    }
    fn enabled(&mut self, t: usize) -> Result<u8> {
        Ok(self.manager.trigger(t).map_err(|e| e.to_string())?.enabled)
    }
    fn conditions_present(&mut self, t: usize) -> Result<bool> {
        Ok(!self
            .manager
            .definitions()
            .get(t)
            .ok_or("film trigger definition absent")?
            .conditions
            .is_empty())
    }
    fn first_action(&mut self, t: usize) -> Result<Option<(usize, usize)>> {
        Ok((!self.manager.definitions()[t].actions.is_empty()).then_some((t, 0)))
    }
    fn next_action(&mut self, a: (usize, usize)) -> Result<Option<(usize, usize)>> {
        Ok((a.1 + 1 < self.manager.definitions()[a.0].actions.len()).then_some((a.0, a.1 + 1)))
    }
    fn opcode(&mut self, a: (usize, usize)) -> Result<u32> {
        Ok(self.manager.definitions()[a.0].actions[a.1].opcode)
    }
    fn convert_5422f0(&mut self, a: (usize, usize), r: &mut [Register; 2]) -> Result<()> {
        convert(&self.manager.definitions()[a.0].actions[a.1].parameters, r)
    }
    fn kingdom_54dac0(&mut self, r: &Register) -> Result<Option<ObjectKey>> {
        let key = match r.item {
            18 => crate::trigger_world::named_kingdom(self.actors, self.names, &r.text)
                .map_err(|e| e.to_string())?
                .map(|o| o.0),
            19 => self
                .actors
                .key(aa_actors::ActorId::from_raw(r.word))
                .map(|k| k.raw()),
            23 => self.manager.condition_kingdom(),
            24 => self.manager.event_kingdom(),
            _ => None,
        };
        key.map(|k| {
            self.actors
                .checked_key(k)
                .ok_or("film current trigger actor pointer is not owned".into())
        })
        .transpose()
    }
    fn objective_54da70(&mut self, r: &Register) -> Result<Option<O::Objective>> {
        self.objectives.find_4f1ad0(&r.text)
    }
    fn name_18(&mut self, actor: &ObjectKey) -> Result<Vec<u16>> {
        Ok(self
            .actors
            .object(*actor)
            .map_err(|e| e.to_string())?
            .thing
            .name_utf16
            .clone())
    }
    fn localized_49ad00(&mut self, objective: &O::Objective) -> Result<(Vec<u16>, Vec<u16>)> {
        let key = self.objectives.ids_4(objective)?;
        let text = if key.is_empty() {
            Vec::new()
        } else {
            self.text.lookup(&key).map_err(|e|e.to_string())?.ok_or("film49AD00 missing objective MapString requires actual entry creation/fallback owner")?
        };
        Ok((key, text))
    }
}
///Actual same-owner trigger scan. Element definitions are immutably borrowed
///for this closed read-only action graph; opcode50 kind2 changes no manager
///condition/event selector pointer. Native live enabled bytes are read lazily.
pub fn capture_current<O: Objectives>(
    target: &mut crate::film_parameters::State,
    actors: &ActorStore,
    names: &names::Table,
    manager: &Manager,
    objectives: &mut O,
    text: &Localization,
) -> Result<()> {
    if !manager.saved_payload_installed() {
        return Err("film extras require the actual installed TRIGGERS owner".into());
    }
    capture_fields(
        target,
        &mut Live {
            actors,
            names,
            manager,
            objectives,
            text,
        },
    )
}
