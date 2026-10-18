//! Retained current cells read by the native frame timeline/heartbeat gates.
//! Constructor values are from real registrations, then root wildcard source
//! assignments are applied once in native order. These four cells are disjoint
//! from aa-ui's eight view-reset cells; no per-frame Gates snapshot is built.
use crate::{catalog::Catalog, load_view_variables::NativeOverrides};
use aa_rules::{
    native::index::native_enumerate,
    resources::{Provenance, SourceIdentity},
    sources::V14Sources,
    tgi::{Document, Item, Limits},
};

pub struct State {
    auto_pause: u8,
    dump_avi: u8,
    avi_frames: i32,
    async_rate: u32,
    sources: Vec<SourceIdentity>,
    files: Vec<(String, String)>,
    assignments: Vec<Provenance>,
}
impl State {
    fn from_native_constructor_fields(sources: Vec<SourceIdentity>) -> Self {
        Self {
            auto_pause: 1,
            dump_avi: 0,
            avi_frames: 2,
            async_rate: 0x41800000,
            sources,
            files: Vec::new(),
            assignments: Vec::new(),
        }
    }
    pub fn from_fresh_sources(
        catalog: &Catalog<'_>,
        files: &mut V14Sources,
        scope: NativeOverrides,
    ) -> Result<Self, String> {
        catalog.require_source_owner()?;
        if !matches!(scope, NativeOverrides::ControlledNone) {
            return Err("frame variable profile/console owner is unbound".into());
        }
        let source_pc = {
            let bank = catalog
                .definition_bank
                .try_borrow()
                .map_err(|e| e.to_string())?;
            let source = bank.combat_source()?;
            if source.source_identities() != catalog.actors.sources {
                return Err("frame variable source Bank differs".into());
            }
            source.source_precision()
        };
        let mut this = Self::from_native_constructor_fields(catalog.actors.sources.clone());
        let paths: Vec<_> = files
            .catalog()
            .into_iter()
            .map(|e| e.logical_path)
            .collect();
        for pattern in ["AVars*.tgi", "UVars*.tgi"] {
            for path in native_enumerate(pattern, &paths) {
                let file = files.read(&path).map_err(|e| e.to_string())?;
                if aa_rules::sources::sha256(&file.bytes) != file.sha256 {
                    return Err("frame variable source bytes/hash differ".into());
                }
                let doc = Document::parse_native(file.bytes, Limits::default())
                    .map_err(|e| e.to_string())?;
                this.files
                    .push((file.logical_path.clone(), file.sha256.clone()));
                for item in &doc.items {
                    let Item::Node(node) = item else { continue };
                    if !doc
                        .header_name(&node.header)
                        .is_some_and(|n| n.eq_ignore_ascii_case("Vars"))
                    {
                        continue;
                    }
                    if node.header.tokens.len() != 1 {
                        return Err("frame variable Vars inheritance is unbound".into());
                    }
                    for item in &node.items {
                        let Item::Property(p) = item else { continue };
                        let Some(name) = doc.property_name(p) else {
                            continue;
                        };
                        let name = name.to_ascii_lowercase();
                        if !matches!(
                            name.as_str(),
                            "singleplayerautopause"
                                | "renderdumpavi"
                                | "renderaviframespertick"
                                | "networkadminasynctickspersecond"
                        ) {
                            continue;
                        }
                        if p.left.len() != 2 {
                            return Err(
                                "frame variable assignment needs the real typed declaration".into(),
                            );
                        }
                        let kind = doc
                            .text(p.left[0].span)
                            .ok_or("frame variable type absent")?;
                        let raw = doc
                            .property_value(p)
                            .ok_or("frame variable value absent")?
                            .trim();
                        let is_avar = pattern.starts_with('A');
                        match name.as_str() {
                            "singleplayerautopause" | "renderdumpavi" => {
                                if is_avar
                                    || !(kind.eq_ignore_ascii_case("flag")
                                        || kind.eq_ignore_ascii_case("bool"))
                                {
                                    return Err("frame Bool source class/type differs".into());
                                }
                                let v = if raw.eq_ignore_ascii_case("true") {
                                    1
                                } else if raw.eq_ignore_ascii_case("false") {
                                    0
                                } else {
                                    return Err("frame Bool source expression unbound".into());
                                };
                                if name == "singleplayerautopause" {
                                    this.auto_pause = v
                                } else {
                                    this.dump_avi = v
                                }
                            }
                            "renderaviframespertick" => {
                                if is_avar || !kind.eq_ignore_ascii_case("int") {
                                    return Err("frame Int source class/type differs".into());
                                }
                                this.avi_frames = raw
                                    .parse::<i32>()
                                    .map_err(|_| "frame Int literal unsupported")?;
                            }
                            "networkadminasynctickspersecond" => {
                                if !is_avar || !kind.eq_ignore_ascii_case("float") {
                                    return Err("frame Float source class/type differs".into());
                                }
                                this.async_rate = aa_rules::native::scalar_literal(raw, source_pc)
                                    .map_err(|e| e.to_string())?;
                            }
                            _ => unreachable!(),
                        }
                        this.assignments.push(Provenance {
                            path: file.logical_path.clone(),
                            sha256: file.sha256.clone(),
                            span: p.span,
                        });
                    }
                }
            }
        }
        Ok(this)
    }
    pub fn validate(&self, catalog: &Catalog<'_>) -> Result<(), String> {
        catalog.require_source_owner()?;
        if self.sources != catalog.actors.sources {
            return Err("frame variables belong to another source universe".into());
        }
        Ok(())
    }
    pub fn auto_pause_byte(&self) -> u8 {
        self.auto_pause
    }
    pub fn render_dump_avi_byte(&self) -> u8 {
        self.dump_avi
    }
    pub fn render_avi_frames_per_tick(&self) -> i32 {
        self.avi_frames
    }
    pub fn async_ticks_per_second_bits(&self) -> u32 {
        self.async_rate
    }
    pub fn source_files(&self) -> &[(String, String)] {
        &self.files
    }
    pub fn source_assignments(&self) -> &[Provenance] {
        &self.assignments
    }
    ///Both Bool registrations have flags0;4B29B0 preserves the actual byte.
    pub fn assign_auto_pause_byte(&mut self, value: u8) {
        self.auto_pause = value
    }
    pub fn assign_render_dump_avi_byte(&mut self, value: u8) {
        self.dump_avi = value
    }
    ///Native Int bounds span the whole signed32 domain.
    pub fn assign_render_avi_frames_per_tick(&mut self, value: i32) {
        self.avi_frames = value
    }
}

#[cfg(test)]
mod tests {
    use super::State;
    #[test]
    fn actual_native_registrations_have_expected_types_flags_and_constructor_words() {
        let d: serde_json::Value = serde_json::from_str(include_str!(
            "../../../research/development10/live-frame-host/native-frame-variables.json"
        ))
        .unwrap();
        assert_eq!(d["cases"].as_array().unwrap().len(), 12);
        for c in d["cases"].as_array().unwrap() {
            let state = State::from_native_constructor_fields(Vec::new());
            let expected: Vec<(u32, &str, u32, u32)> = if c["class"] == "AVar" {
                vec![(71, "NetworkAdminAsyncTicksPerSecond", 2, 0x41800000)]
            } else {
                vec![
                    (114, "RenderDumpAVI", 1, 0),
                    (115, "RenderAVIFramesPerTick", 3, 2),
                    (212, "SinglePlayerAutoPause", 1, 1),
                ]
            };
            for (v, (index, name, kind, current)) in
                c["values"].as_array().unwrap().iter().zip(expected)
            {
                assert_eq!(v["index"], index);
                assert_eq!(v["kind"], kind);
                assert_eq!(v["flags"], 0);
                assert_eq!(v["current"], current);
                let actual = match name {
                    "RenderDumpAVI" => state.render_dump_avi_byte() as u32,
                    "RenderAVIFramesPerTick" => state.render_avi_frames_per_tick() as u32,
                    "SinglePlayerAutoPause" => state.auto_pause_byte() as u32,
                    "NetworkAdminAsyncTicksPerSecond" => state.async_ticks_per_second_bits(),
                    _ => unreachable!(),
                };
                assert_eq!(v["current"], actual);
                assert_eq!(
                    v["name"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .map(|v| v.as_u64().unwrap() as u16)
                        .collect::<Vec<_>>(),
                    name.encode_utf16().collect::<Vec<_>>()
                );
            }
        }
    }
}
