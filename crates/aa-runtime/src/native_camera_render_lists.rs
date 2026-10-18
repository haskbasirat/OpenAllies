//! Actual712660 camera render-list constructor and fixed A87A80 descriptor table.
//! Current Bool references are retained opaquely; no per-descriptor Bool copy.
//! Native15 pass definitions remain pipeline data, not allocation limits.
use std::sync::{Arc, Weak};
type Result<T> = std::result::Result<T, String>;
///4B2C30(name,1) resolves a reference only; constructor does not read its Bool.
pub trait Variables {
    type Reference;
    fn bool_reference(&mut self, name: &'static str) -> Result<Self::Reference>;
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Command {
    pub kind: u32,
    pub argument: u32,
}
struct Spec {
    label: &'static str,
    sort: u32,
    variable: Option<&'static str>,
    commands: &'static [(u32, u32)],
}
const SPECS: &[Spec] = &[
    Spec {
        label: "Node based things",
        sort: 0,
        variable: None,
        commands: &[(2, 10)],
    },
    Spec {
        label: "Terrain",
        sort: 0,
        variable: Some("RenderlistDrawTerrain"),
        commands: &[(2, 16), (2, 23), (2, 11), (2, 9), (2, 13), (2, 17), (2, 21)],
    },
    Spec {
        label: "Terrain Replacement Objects",
        sort: 0,
        variable: Some("RenderlistDrawTerrain"),
        commands: &[(2, 24), (2, 15), (2, 11), (2, 9), (2, 13), (2, 17), (2, 21)],
    },
    Spec {
        label: "Terrain Decals",
        sort: 0,
        variable: Some("RenderlistDrawTerrainDecals"),
        commands: &[(2, 7)],
    },
    Spec {
        label: "NiLines",
        sort: 0,
        variable: Some("RenderlistDrawLines"),
        commands: &[(2, 20), (2, 11), (2, 21)],
    },
    Spec {
        label: "Skinned (grouped by vertex buffer)",
        sort: 0,
        variable: Some("RenderlistDrawSkinned"),
        commands: &[
            (1, 1),
            (1, 3),
            (1, 5),
            (2, 27),
            (2, 11),
            (2, 9),
            (2, 13),
            (2, 17),
            (2, 21),
        ],
    },
    Spec {
        label: "Non-skinned things (grouped by vertex buffer and texture buffer)",
        sort: 0,
        variable: Some("RenderlistDrawUnskinned"),
        commands: &[
            (1, 3),
            (1, 5),
            (2, 27),
            (2, 11),
            (2, 9),
            (2, 13),
            (2, 17),
            (2, 21),
        ],
    },
    Spec {
        label: "Stencil Shadows",
        sort: 0,
        variable: Some("RenderlistDrawStencilShadows"),
        commands: &[(1, 13), (2, 9), (2, 17), (2, 21)],
    },
    Spec {
        label: "Water",
        sort: 0,
        variable: Some("RenderlistDrawWater"),
        commands: &[(2, 26), (2, 13), (2, 17), (2, 21)],
    },
    Spec {
        label: "Alpha'd things (and some that are node-based)",
        sort: 1,
        variable: Some("RenderlistDrawAlphaBlended"),
        commands: &[(1, 11), (2, 27), (2, 13), (2, 17), (2, 21)],
    },
    Spec {
        label: "Particles",
        sort: 1,
        variable: Some("RenderlistDrawParticles"),
        commands: &[(2, 28), (2, 13), (2, 17), (2, 21)],
    },
    Spec {
        label: "Floating UI VFX",
        sort: 0,
        variable: Some("RenderlistDrawUIVFX"),
        commands: &[(1, 21), (2, 9), (2, 17)],
    },
    Spec {
        label: "World Catch-all",
        sort: 0,
        variable: Some("RenderlistDrawWorldCatchAll"),
        commands: &[(2, 17), (1, 7)],
    },
    Spec {
        label: "UI",
        sort: 0,
        variable: Some("RenderlistDrawUI"),
        commands: &[(1, 17), (2, 9)],
    },
    Spec {
        label: "UI Catch-all",
        sort: 0,
        variable: Some("RenderlistDrawUICatchAll"),
        commands: &[(1, 7)],
    },
];
///One retained712660 allocation. No Clone/default/raw-source constructor.
pub struct State<K> {
    identity: Arc<()>,
    enabled: Vec<u32>,
    descriptors: Vec<Descriptor<K>>,
}
pub struct Descriptor<K> {
    owner: Weak<()>,
    label: Vec<u16>,
    sort: u32,
    variable: Option<K>,
    reference_bound: bool,
    commands: Vec<Command>,
}
pub struct Failure<K> {
    pub state: State<K>,
    pub error: String,
}
impl<K> std::fmt::Debug for Failure<K> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NativeCameraRenderListsFailure")
            .field("error", &self.error)
            .field("retained_descriptors", &self.state.descriptors.len())
            .finish()
    }
}
impl<K> State<K> {
    pub fn from_native_constructor(
        vars: &mut impl Variables<Reference = K>,
    ) -> std::result::Result<Self, Failure<K>> {
        let mut state = Self {
            identity: Arc::new(()),
            enabled: vec![0; SPECS.len().div_ceil(32)],
            descriptors: Vec::new(),
        };
        for (index, spec) in SPECS.iter().enumerate() {
            //7126E8 sets this enabled bit BEFORE creating the descriptor.
            state.enabled[index / 32] |= 1 << (index & 31);
            let mut d = Descriptor {
                owner: Arc::downgrade(&state.identity),
                label: spec.label.encode_utf16().collect(),
                sort: spec.sort,
                variable: None,
                reference_bound: false,
                commands: Vec::new(),
            };
            if let Some(name) = spec.variable {
                match vars.bool_reference(name) {
                    Ok(key) => d.variable = Some(key),
                    Err(error) => {
                        state.descriptors.push(d);
                        return Err(Failure { state, error });
                    }
                }
            }
            d.reference_bound = true;
            //Native singly linked tail insertion preserves table pair order exactly.
            for &(kind, argument) in spec.commands {
                d.commands.push(Command { kind, argument })
            }
            state.descriptors.push(d);
        }
        Ok(state)
    }
    pub fn descriptors(&self) -> &[Descriptor<K>] {
        &self.descriptors
    }
    pub fn enabled(&self, index: usize) -> Result<bool> {
        if index >= SPECS.len() {
            return Err("native camera render-list index outside actual descriptor table".into());
        }
        Ok(self.enabled[index / 32] & (1 << (index & 31)) != 0)
    }
}
impl<K> Descriptor<K> {
    pub fn label_units(&self) -> &[u16] {
        &self.label
    }
    pub fn sort_word(&self) -> u32 {
        self.sort
    }
    pub fn bool_reference(&self) -> Result<Option<&K>> {
        if self.owner.upgrade().is_none() {
            return Err("native camera render-list owner destroyed".into());
        }
        if !self.reference_bound {
            return Err("native camera render-list lookup did not complete".into());
        }
        Ok(self.variable.as_ref())
    }
    pub fn commands(&self) -> &[Command] {
        &self.commands
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    struct Vars {
        names: Vec<&'static str>,
        fail: Option<usize>,
    }
    impl Variables for Vars {
        type Reference = &'static str;
        fn bool_reference(&mut self, name: &'static str) -> Result<Self::Reference> {
            let i = self.names.len();
            self.names.push(name);
            if self.fail == Some(i) {
                Err("required actual current UI Bool reference".into())
            } else {
                Ok(name)
            }
        }
    }
    #[test]
    fn actual_native_full_constructor_render_list_table() {
        let native: serde_json::Value = serde_json::from_str(include_str!(
            "../../../research/skirmish-20261017-native-camera/native-camera-constructor.json"
        ))
        .unwrap();
        for row in native["rows"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|r| r["render_lists"] == 1)
        {
            let mut vars = Vars {
                names: Vec::new(),
                fail: None,
            };
            let s = State::from_native_constructor(&mut vars).unwrap();
            assert_eq!(s.descriptors().len(), 15);
            for (i, (d, r)) in s
                .descriptors()
                .iter()
                .zip(row["output"]["descriptors"].as_array().unwrap())
                .enumerate()
            {
                assert!(s.enabled(i).unwrap());
                assert_eq!(
                    String::from_utf16(d.label_units()).unwrap(),
                    r["label"].as_str().unwrap()
                );
                assert_eq!(u64::from(d.sort_word()), r["sort"].as_u64().unwrap());
                assert_eq!(d.bool_reference().unwrap().copied(), r["varname"].as_str());
                assert_eq!(
                    d.commands()
                        .iter()
                        .map(|p| [p.kind, p.argument])
                        .collect::<Vec<_>>(),
                    serde_json::from_value::<Vec<[u32; 2]>>(r["pairs"].clone()).unwrap()
                )
            }
            assert_eq!(vars.names.len(), 14)
        }
    }
    #[test]
    fn native_lookup_stops_preserve_each_reached_descriptor() {
        let fixture:serde_json::Value=serde_json::from_str(include_str!("../../../research/skirmish-20261017-native-camera/native-camera-render-list-prefix.json")).unwrap();
        for row in fixture["rows"].as_array().unwrap() {
            let cut = row["cut"].as_u64().unwrap() as usize;
            let mut vars = Vars {
                names: Vec::new(),
                fail: Some(cut),
            };
            let failure = State::from_native_constructor(&mut vars).err().unwrap();
            let expected = row["output"]["descriptors"].as_array().unwrap();
            assert_eq!(failure.state.descriptors.len(), expected.len());
            for (index, (d, r)) in failure.state.descriptors.iter().zip(expected).enumerate() {
                assert!(failure.state.enabled(index).unwrap());
                assert_eq!(
                    String::from_utf16(d.label_units()).unwrap(),
                    r["label"].as_str().unwrap()
                );
                assert_eq!(u64::from(d.sort_word()), r["sort"].as_u64().unwrap());
                assert_eq!(
                    d.commands()
                        .iter()
                        .map(|c| [c.kind, c.argument])
                        .collect::<Vec<_>>(),
                    serde_json::from_value::<Vec<[u32; 2]>>(r["pairs"].clone()).unwrap()
                );
                if index + 1 == expected.len() {
                    assert!(d.bool_reference().is_err())
                } else {
                    assert_eq!(d.bool_reference().unwrap().copied(), r["varname"].as_str())
                }
            }
        }
    }
    #[test]
    fn required_lookup_preserves_same_descriptor_prefix() {
        for fail in [0, 3, 13] {
            let mut vars = Vars {
                names: Vec::new(),
                fail: Some(fail),
            };
            let failure = State::from_native_constructor(&mut vars).err().unwrap();
            assert_eq!(vars.names.len(), fail + 1);
            assert_eq!(failure.state.descriptors.len(), fail + 2);
            let d = failure.state.descriptors.last().unwrap();
            assert!(d.bool_reference().is_err());
            assert!(d.commands().is_empty());
            assert!(failure.state.enabled(fail + 1).unwrap());
            if fail + 2 < 15 {
                assert!(!failure.state.enabled(fail + 2).unwrap())
            }
        }
    }
}
