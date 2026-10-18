use aa_triggers::{
    decode::{Element, SavedManager, Span, Trigger},
    manager::{DeferredPayload, Elements, Error, Manager, Result, TriggerState, UpdateTrace},
};
use serde::Deserialize;
use std::collections::BTreeMap;
#[derive(Clone, Deserialize)]
struct Mutation {
    #[serde(default)]
    call: usize,
    trigger: Option<usize>,
    mode: Option<u32>,
    enabled: Option<u8>,
    repeat: Option<u8>,
    activation_bits: Option<u32>,
    subject: Option<u32>,
    event: Option<u64>,
    context_word_10: Option<u32>,
    time_bits: Option<u32>,
}
#[derive(Clone, Deserialize)]
struct Leaf {
    id: u32,
    returns: Vec<bool>,
    mutations: Vec<Mutation>,
}
#[derive(Deserialize)]
struct InputTrigger {
    id: u32,
    mode: u32,
    enabled: u8,
    repeat: u8,
    activation_bits: u32,
    events: Vec<Leaf>,
    conditions: Vec<Leaf>,
    actions: Vec<Leaf>,
}
#[derive(Deserialize)]
struct Input {
    name: String,
    time_bits: u32,
    application_mode: u32,
    context_word_10: u32,
    subjects: Vec<u32>,
    deferred: Vec<Option<u64>>,
    triggers: Vec<InputTrigger>,
}
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
struct Call {
    trigger: usize,
    element: u32,
    subject: Option<u32>,
    context_word_10: u32,
}
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
struct Diagnostic {
    trigger: usize,
    element: u32,
}
#[derive(Deserialize)]
struct Output {
    triggers: Vec<TriggerState>,
    calls: Vec<Call>,
    diagnostics: Vec<Diagnostic>,
    trace: UpdateTrace,
    subjects: Vec<u32>,
    context_word_10: u32,
    cursor_bound_to_subject_list: bool,
    cursor: Option<u32>,
    deferred: Vec<Option<u64>>,
    time_bits: u32,
}
#[derive(Deserialize)]
struct Case {
    input: Input,
    output: Output,
}
#[derive(Deserialize)]
struct Fixture {
    cases: Vec<Case>,
}
struct Script {
    leaves: BTreeMap<(usize, u32), Leaf>,
    steps: BTreeMap<(usize, u32), usize>,
    calls: Vec<Call>,
    diagnostics: Vec<Diagnostic>,
    time_bits: u32,
    mode: u32,
    fail: bool,
}
impl Elements for Script {
    fn evaluate(
        &mut self,
        manager: &mut Manager,
        ordinal: usize,
        element: &Element,
    ) -> Result<bool> {
        if self.fail {
            return Err(Error::Callback("script failure".into()));
        }
        self.calls.push(Call {
            trigger: ordinal,
            element: element.word_10,
            subject: manager.current_subject(),
            context_word_10: manager.snapshot().context_word_10,
        });
        let key = (ordinal, element.word_10);
        let leaf = self.leaves[&key].clone();
        let step = *self.steps.entry(key).or_default();
        self.steps.insert(key, step + 1);
        for m in leaf.mutations.iter().filter(|m| m.call == step) {
            if let Some(i) = m.trigger {
                let t = manager.trigger_mut(i)?;
                if let Some(v) = m.mode {
                    t.mode = v;
                }
                if let Some(v) = m.enabled {
                    t.enabled = v;
                }
                if let Some(v) = m.repeat {
                    t.repeat = v;
                }
                if let Some(v) = m.activation_bits {
                    t.activation_bits = v;
                }
            }
            if let Some(v) = m.subject {
                manager.add_subject(v);
            }
            if let Some(v) = m.event {
                manager.push_event(Some(DeferredPayload {
                    identity: v,
                    data: aa_triggers::predicates::Event::UnclassifiedWords(vec![]),
                }));
            }
            if let Some(v) = m.context_word_10 {
                manager.set_context_word(v);
            }
            if let Some(v) = m.time_bits {
                self.time_bits = v;
            }
        }
        Ok(leaf.returns[step.min(leaf.returns.len() - 1)])
    }
    fn world_time_bits(&self) -> u32 {
        self.time_bits
    }
    fn action_failed(&mut self, _: &mut Manager, ordinal: usize, element: &Element) -> Result<()> {
        if self.mode == 7 {
            self.diagnostics.push(Diagnostic {
                trigger: ordinal,
                element: element.word_10,
            });
        }
        Ok(())
    }
}
fn prepare(c: &Input) -> (Manager, Script) {
    let mut leaves = BTreeMap::new();
    let mut triggers = vec![];
    for (i, t) in c.triggers.iter().enumerate() {
        let mut list = |entries: &[Leaf]| {
            entries
                .iter()
                .map(|e| {
                    leaves.insert((i, e.id), e.clone());
                    Element {
                        source: Span {
                            offset: 0,
                            length: 0,
                        },
                        name: "fixture leaf only".into(),
                        opcode: 0,
                        word_10: e.id,
                        parameters: vec![],
                    }
                })
                .collect()
        };
        triggers.push(Trigger {
            source: Span {
                offset: 0,
                length: 0,
            },
            id: t.id,
            mode: t.mode,
            enabled: t.enabled,
            repeat: t.repeat,
            activation_bits: t.activation_bits,
            name: vec![],
            description: vec![],
            word_2c: 0,
            events: list(&t.events),
            conditions: list(&t.conditions),
            actions: list(&t.actions),
        });
    }
    let mut manager = Manager::from_saved(SavedManager {
        next_id: 1,
        triggers,
        variables: vec![],
        strings: vec![],
    });
    manager.set_context_word(c.context_word_10);
    for subject in c.subjects.iter().rev() {
        manager.add_subject(*subject);
    }
    for event in c.deferred.iter().rev() {
        manager.push_event(event.map(|identity| DeferredPayload {
            identity,
            data: aa_triggers::predicates::Event::UnclassifiedWords(vec![]),
        }));
    }
    (
        manager,
        Script {
            leaves,
            steps: BTreeMap::new(),
            calls: vec![],
            diagnostics: vec![],
            time_bits: c.time_bits,
            mode: c.application_mode,
            fail: false,
        },
    )
}
#[test]
fn native_trigger_manager_order_mutations_and_cleanup() {
    let fixture: Fixture = serde_json::from_str(include_str!(
        "../../../research/phase7-triggers/native-manager-fixtures.json"
    ))
    .unwrap();
    assert_eq!(fixture.cases.len(), 67);
    for case in fixture.cases {
        let (mut manager, mut script) = prepare(&case.input);
        let expected = case.output;
        manager.update(&mut script).unwrap();
        let state = manager.snapshot();
        assert_eq!(state.triggers, expected.triggers, "{}", case.input.name);
        assert_eq!(script.calls, expected.calls, "{}", case.input.name);
        assert_eq!(
            script.diagnostics, expected.diagnostics,
            "{}",
            case.input.name
        );
        assert_eq!(*manager.trace(), expected.trace, "{}", case.input.name);
        assert_eq!(
            state.subjects.into_iter().collect::<Vec<_>>(),
            expected.subjects,
            "{}",
            case.input.name
        );
        assert_eq!(state.context_word_10, expected.context_word_10);
        assert_eq!(
            state.cursor_bound_to_subject_list,
            expected.cursor_bound_to_subject_list
        );
        assert_eq!(manager.current_subject(), expected.cursor);
        assert_eq!(
            state
                .deferred
                .into_iter()
                .map(|p| p.map(|p| p.identity))
                .collect::<Vec<_>>(),
            expected.deferred
        );
        assert_eq!(script.time_bits, expected.time_bits);
    }
}
#[test]
fn failed_step_is_sticky_and_activation_time_only_changes_on_enable() {
    let fixture: Fixture = serde_json::from_str(include_str!(
        "../../../research/phase7-triggers/native-manager-fixtures.json"
    ))
    .unwrap();
    let case = fixture
        .cases
        .into_iter()
        .find(|c| c.input.name == "unconditional")
        .unwrap();
    let (mut manager, mut script) = prepare(&case.input);
    manager.set_enabled(0, 0, 1).unwrap();
    manager.set_enabled(0, 1, 2).unwrap();
    manager.set_enabled(0, 1, 3).unwrap();
    assert_eq!(manager.trigger(0).unwrap().activation_bits, 2);
    manager.add_subject(5);
    script.fail = true;
    assert!(manager.update(&mut script).is_err());
    assert!(manager.stopped());
    assert_eq!(manager.current_subject(), Some(5));
    assert_eq!(manager.update(&mut script).unwrap_err(), Error::Stopped);
}
