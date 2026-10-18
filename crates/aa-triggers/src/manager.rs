//! Owned54C280/5411D0 trigger sequencing. Loaded element lists and trigger order
//! remain immutable during an update; live enabled/repeat/mode/time, subjects,
//! variables and incoming event queues remain mutable at evaluation boundaries.
//! Opcode execution is a separate required implementation, never default success.
use crate::decode::{Element, SavedManager, Trigger, Value, Variable};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    #[error("trigger callback failed: {0}; preceding effects remain committed")]
    Callback(String),
    #[error("invalid trigger state: {0}")]
    State(String),
    #[error("trigger manager stopped after a failed update; restore before retrying")]
    Stopped,
}
pub type Result<T> = std::result::Result<T, Error>;
/// Registers in the manager+2C file (54CEE0 allocates 4 + 8 * 0x24 bytes).
pub const REGISTERS: usize = 8;
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TriggerState {
    pub id: u32,
    pub mode: u32,
    pub enabled: u8,
    pub repeat: u8,
    pub activation_bits: u32,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeferredPayload {
    /// Stable runtime event identity for ownership and diagnostics, not pointer.
    pub identity: u64,
    /// One owned payload; opaque sequencing fixtures cannot masquerade as a
    ///typed gameplay event when a predicate reads its native kind/fields.
    pub data: crate::predicates::Event,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Release {
    SubjectNode(u32),
    EventNode(Option<u64>),
    EventPayload(u64),
}
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct UpdateTrace {
    pub evaluated: Vec<u32>,
    pub fired: Vec<u32>,
    pub releases: Vec<Release>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Snapshot {
    pub next_id: u32,
    pub triggers: Vec<TriggerState>,
    pub context_word_10: u32,
    ///Native54CEE0 initializes+24/+28 null. Selectors retain canonical runtime
    ///object handles here; these are pointers in native, not raw actor GTIDs.
    pub condition_kingdom: Option<u64>,
    pub event_kingdom: Option<u64>,
    pub subjects: VecDeque<u32>,
    pub cursor: Option<usize>,
    pub cursor_bound_to_subject_list: bool,
    pub deferred: VecDeque<Option<DeferredPayload>>,
    pub variables: Vec<Variable>,
    pub strings: Vec<Vec<u16>>,
}
pub struct Manager {
    definitions: Vec<Trigger>,
    state: Snapshot,
    stopped: bool,
    trace: UpdateTrace,
    /// Manager+2C register file: 8 registers built by the constructor 54CEE0
    /// (54DE30: empty CStrings) and reused by every 546AF0 call for the
    /// manager's lifetime; never saved. Only the +4 string slot is modelled.
    register_text: Vec<Vec<u16>>,
    fresh_load_installed: bool,
}
/// Actual opcode evaluator. Implementations must execute the recovered element
/// semantics, including required world services, or return an explicit error.
/// Native test fixtures replace this exact546AF0 boundary to prove sequencing;
/// those tests alone do not establish gameplay support for any opcode.
pub trait Elements {
    fn evaluate(
        &mut self,
        manager: &mut Manager,
        ordinal: usize,
        element: &Element,
    ) -> Result<bool>;
    fn world_time_bits(&self) -> u32;
    /// Native5411D0 reports a false action only in application mode7. The real
    /// evaluator owns that mode and diagnostic localization; no default no-op.
    fn action_failed(
        &mut self,
        manager: &mut Manager,
        ordinal: usize,
        element: &Element,
    ) -> Result<()>;
}
impl Manager {
    ///54CEE0. Kept before record initialization so earlier subject/event
    /// callbacks and the subsequent54D3D0 Load share this same allocation.
    /// Constructor and register strings are covered by the original
    /// research/phase8-triggers/native_triggers.py loader oracle.
    pub fn from_native_constructor() -> Self {
        Self {
            definitions: Vec::new(),
            state: Snapshot {
                next_id: 1,
                triggers: Vec::new(),
                context_word_10: 0,
                condition_kingdom: None,
                event_kingdom: None,
                subjects: VecDeque::new(),
                cursor: None,
                cursor_bound_to_subject_list: false,
                deferred: VecDeque::new(),
                variables: Vec::new(),
                strings: Vec::new(),
            },
            stopped: false,
            trace: UpdateTrace::default(),
            register_text: vec![Vec::new(); REGISTERS],
            fresh_load_installed: false,
        }
    }
    pub fn from_saved(saved: SavedManager) -> Self {
        let mut owner=Self::from_native_constructor();
        owner.install_saved_fields(saved);
        owner
    }
    /// The empty-trigger branch of54BD60 followed by the decoded54D3D0 fields.
    /// Retains the same runtime context, subject/event lists and registers.
    /// This is field installation only: native resource diagnostics and the
    /// final TriggerListUpdate presentation broadcast are separate services.
    pub fn install_saved_on_fresh(&mut self,saved:SavedManager)->Result<()> {
        if self.fresh_load_installed || !self.definitions.is_empty()
            || self.state.next_id!=1 || !self.state.triggers.is_empty()
            || !self.state.variables.is_empty() || !self.state.strings.is_empty() {
            return Err(Error::State("saved installation requires the actual fresh trigger payload owner".into()));
        }
        self.install_saved_fields(saved);
        Ok(())
    }
    fn install_saved_fields(&mut self,saved:SavedManager) {
        self.state.next_id=saved.next_id;
        self.state.triggers=saved.triggers.iter().map(|t|TriggerState {
            id:t.id,mode:t.mode,enabled:t.enabled,repeat:t.repeat,activation_bits:t.activation_bits,
        }).collect();
        self.definitions=saved.triggers;
        self.state.variables=saved.variables;
        self.state.strings=saved.strings;
        self.fresh_load_installed=true;
    }
    pub fn saved_payload_installed(&self)->bool {self.fresh_load_installed}
    /// 5422F0 / 54A5A0 at 546AF0 entry (after 541900 validity): parameter i is
    /// converted into register i; only storage 6 assigns the +4 string, every
    /// other storage leaves the string of an earlier element in place.
    pub fn convert_registers(&mut self, element: &Element) {
        for (i, p) in element.parameters.iter().enumerate().take(REGISTERS) {
            if let (6, Value::Text(text)) = (p.storage, &p.value) {
                self.register_text[i] = text.clone();
            }
        }
    }
    /// Register `index` +4 string as a 546AF0 selector reads it (54DE00,
    /// 54E910, ...): this element's text, or the text an earlier element left.
    pub fn register_text(&self, index: usize) -> Result<&[u16]> {
        self.register_text
            .get(index)
            .map(Vec::as_slice)
            .ok_or_else(|| Error::State(format!("register {index} beyond the 8-register file")))
    }
    pub fn snapshot(&self) -> Snapshot {
        self.state.clone()
    }
    pub fn trace(&self) -> &UpdateTrace {
        &self.trace
    }
    pub fn stopped(&self) -> bool {
        self.stopped
    }
    pub fn trigger(&self, ordinal: usize) -> Result<&TriggerState> {
        self.state
            .triggers
            .get(ordinal)
            .ok_or_else(|| Error::State("missing trigger ordinal".into()))
    }
    pub fn trigger_mut(&mut self, ordinal: usize) -> Result<&mut TriggerState> {
        self.state
            .triggers
            .get_mut(ordinal)
            .ok_or_else(|| Error::State("missing trigger ordinal".into()))
    }
    pub fn definitions(&self) -> &[Trigger] {
        &self.definitions
    }
    pub fn set_context_word(&mut self, word: u32) {
        self.state.context_word_10 = word;
    }
    pub fn context_word(&self) -> u32 {
        self.state.context_word_10
    }
    pub fn condition_kingdom(&self) -> Option<u64> {
        self.state.condition_kingdom
    }
    pub fn event_kingdom(&self) -> Option<u64> {
        self.state.event_kingdom
    }
    pub fn set_condition_kingdom(&mut self, value: Option<u64>) {
        self.state.condition_kingdom = value;
    }
    pub fn set_event_kingdom(&mut self, value: Option<u64>) {
        self.state.event_kingdom = value;
    }
    pub fn variables(&self) -> &[Variable] {
        &self.state.variables
    }
    /// Manager+40 string array (54C240 lookup).
    pub fn strings(&self) -> &[Vec<u16>] {
        &self.state.strings
    }
    pub fn variables_mut(&mut self) -> &mut Vec<Variable> {
        &mut self.state.variables
    }
    /// Native540A30: only a disabled→enabled transition captures current time.
    pub fn set_enabled(&mut self, ordinal: usize, enabled: u8, time_bits: u32) -> Result<()> {
        let t = self.trigger_mut(ordinal)?;
        if t.enabled == 0 && enabled != 0 {
            t.activation_bits = time_bits;
        }
        t.enabled = enabled;
        Ok(())
    }
    /// Native54C380 deduplicates raw GTIDs and prepends a new subject. Adding an
    /// existing subject does not reset iteration. No registry identity guessed.
    pub fn add_subject(&mut self, identity: u32) {
        if self.state.subjects.contains(&identity) {
            return;
        }
        self.state.subjects.push_front(identity);
        self.state.cursor = Some(0);
        self.state.cursor_bound_to_subject_list = true;
    }
    pub fn current_subject(&self) -> Option<u32> {
        self.state
            .cursor
            .and_then(|i| self.state.subjects.get(i).copied())
    }
    /// Incoming native event queue54C050/54C0E0/54C170 prepends each event.
    /// Null payload nodes are retained:4DB300 pops one and stops at that node.
    pub fn push_event(&mut self, event: Option<DeferredPayload>) {
        self.state.deferred.push_front(event);
    }
    pub fn events(&self) -> &VecDeque<Option<DeferredPayload>> {
        &self.state.deferred
    }
    fn advance_subject(&mut self) -> bool {
        if let Some(current) = self.state.cursor {
            self.state.cursor = current
                .checked_add(1)
                .filter(|next| *next < self.state.subjects.len());
        }
        self.state.cursor.is_some()
    }
    fn clear_subjects(&mut self) {
        self.state.context_word_10 = 0;
        while let Some(subject) = self.state.subjects.pop_front() {
            self.trace.releases.push(Release::SubjectNode(subject));
        }
        self.state.cursor_bound_to_subject_list = true;
        self.state.cursor = None;
    }
    /// Native5411D0. Event and condition lists never short-circuit: later leaves
    /// may contribute subjects or world effects even after a true/false result.
    pub fn evaluate_trigger(
        &mut self,
        ordinal: usize,
        elements: &mut impl Elements,
    ) -> Result<bool> {
        let definition = self
            .definitions
            .get(ordinal)
            .ok_or_else(|| Error::State("missing trigger definition".into()))?
            .clone();
        if !definition.events.is_empty() {
            let mut any = false;
            for element in &definition.events {
                any |= elements.evaluate(self, ordinal, element)?;
            }
            if !any {
                return Ok(false);
            }
        }
        let mut fired = false;
        loop {
            let mut any = false;
            let mut all = true;
            for element in &definition.conditions {
                let value = elements.evaluate(self, ordinal, element)?;
                any |= value;
                all &= value;
            }
            if definition.conditions.is_empty()
                || (any && (all || self.trigger(ordinal)?.mode != 1))
            {
                fired = true;
                for element in &definition.actions {
                    if !elements.evaluate(self, ordinal, element)? {
                        elements.action_failed(self, ordinal, element)?;
                    }
                }
                let now = elements.world_time_bits();
                let state = self.trigger_mut(ordinal)?;
                if state.repeat == 0 {
                    state.enabled = 0;
                } else {
                    state.activation_bits = now;
                }
            }
            if !self.advance_subject() || self.trigger(ordinal)?.enabled == 0 {
                break;
            }
        }
        Ok(fired)
    }
    /// Native54C280. A failed call is deliberately non-retryable: native side
    /// effects already performed cannot be rolled back by replaying a tick.
    pub fn update(&mut self, elements: &mut impl Elements) -> Result<&UpdateTrace> {
        if self.stopped {
            return Err(Error::Stopped);
        }
        self.stopped = true;
        self.trace = UpdateTrace::default();
        for ordinal in 0..self.definitions.len() {
            if self.trigger(ordinal)?.enabled == 0 {
                continue;
            }
            let id = self.trigger(ordinal)?.id;
            self.trace.evaluated.push(id);
            if self.evaluate_trigger(ordinal, elements)? {
                self.trace.fired.push(id);
            }
            self.clear_subjects();
        }
        while let Some(payload) = self.state.deferred.pop_front() {
            self.trace
                .releases
                .push(Release::EventNode(payload.as_ref().map(|p| p.identity)));
            let Some(payload) = payload else { break };
            self.trace
                .releases
                .push(Release::EventPayload(payload.identity));
        }
        self.stopped = false;
        Ok(&self.trace)
    }
}
