//!The two current AVar Int cells reached at43FCF1, owned once by the application.
//!Native code defaults16/4 are overwritten by actual wildcard source assignments.
use crate::{catalog::Catalog, load_view_variables::NativeOverrides};
use aa_rules::{
    native::index::native_enumerate,
    resources::{Provenance, SourceIdentity},
    sources::V14Sources,
    tgi::{Document, Item, Limits},
};
type Result<T> = std::result::Result<T, String>;
pub struct State {
    terrain: i32,
    grid: i32,
    sources: Vec<SourceIdentity>,
    assignments: Vec<(String, i32, Provenance)>,
    source_loaded: bool,
}
///The same allocated cells and reached assignment prefix survive a source
///read/parse error. A rejected preflight has no allocated owner.
pub struct Failure {
    pub state: Option<State>,
    pub error: String,
}
impl std::fmt::Debug for Failure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VGridVariableFailure")
            .field("owner_retained", &self.state.is_some())
            .field("error", &self.error)
            .finish()
    }
}
#[derive(Debug, PartialEq, Eq)]
pub struct Metrics {
    pub grid_shift: u32,
    pub terrain_quad_size: i32,
}
impl State {
    pub fn from_fresh_sources(
        catalog: &Catalog<'_>,
        files: &mut V14Sources,
        scope: NativeOverrides,
    ) -> Result<Self> {
        Self::from_fresh_sources_retained(catalog, files, scope).map_err(|failure| failure.error)
    }
    pub fn from_fresh_sources_retained(
        catalog: &Catalog<'_>,
        files: &mut V14Sources,
        scope: NativeOverrides,
    ) -> std::result::Result<Self, Failure> {
        let allocation = (|| -> Result<Self> {
            catalog.require_source_owner()?;
            if !matches!(scope, NativeOverrides::ControlledNone) {
                return Err("VGrid AVar profile/console current owners are unbound".into());
            }
            Ok(Self {
                terrain: 16,
                grid: 4,
                sources: catalog.actors.sources.clone(),
                assignments: Vec::new(),
                source_loaded: false,
            })
        })();
        let mut this = allocation.map_err(|error| Failure { state: None, error })?;
        match this.load_sources(catalog, files) {
            Ok(()) => Ok(this),
            Err(error) => Err(Failure {
                state: Some(this),
                error,
            }),
        }
    }
    fn load_sources(&mut self, catalog: &Catalog<'_>, files: &mut V14Sources) -> Result<()> {
        let paths: Vec<_> = files
            .catalog()
            .into_iter()
            .map(|e| e.logical_path)
            .collect();
        for path in native_enumerate("AVars*.tgi", &paths) {
            let file = files.read(&path).map_err(|e| e.to_string())?;
            if aa_rules::sources::sha256(&file.bytes) != file.sha256 {
                return Err("VGrid AVar file bytes/hash differ".into());
            }
            let doc =
                Document::parse_native(file.bytes, Limits::default()).map_err(|e| e.to_string())?;
            for item in &doc.items {
                let Item::Node(node) = item else { continue };
                if !doc
                    .header_name(&node.header)
                    .is_some_and(|s| s.eq_ignore_ascii_case("Vars"))
                {
                    continue;
                }
                if node.header.tokens.len() != 1 {
                    return Err("VGrid AVar Vars inheritance requires native lookup owner".into());
                }
                for item in &node.items {
                    let Item::Property(p) = item else { continue };
                    let Some(name) = doc.property_name(p) else {
                        continue;
                    };
                    if !["TerrainBlockSize", "VGridCellSize"]
                        .iter()
                        .any(|n| name.eq_ignore_ascii_case(n))
                    {
                        continue;
                    }
                    if p.left.len() != 2
                        || !doc
                            .text(p.left[0].span)
                            .is_some_and(|s| s.eq_ignore_ascii_case("int"))
                    {
                        return Err(
                            "VGrid AVar reached assignment requires its native Int type".into()
                        );
                    }
                    let raw = doc
                        .property_value(p)
                        .ok_or("VGrid AVar Int value absent")?
                        .trim();
                    let value = raw
                        .parse::<i32>()
                        .map_err(|_| "VGrid AVar Int expression unbound")?;
                    let source = Provenance {
                        path: file.logical_path.clone(),
                        sha256: file.sha256.clone(),
                        span: p.span,
                    };
                    let public = catalog
                        .settings
                        .as_ref()
                        .ok_or("VGrid AVar Settings projection absent")?
                        .app_variables
                        .get(&name.to_ascii_lowercase())
                        .ok_or("VGrid AVar public declaration absent")?;
                    if public.len() != 1
                        || public[0].source != source
                        || public[0].raw.trim() != raw
                        || !public[0]
                            .field_type
                            .as_deref()
                            .is_some_and(|s| s.eq_ignore_ascii_case("int"))
                    {
                        return Err("VGrid AVar public/private file value/span differs".into());
                    }
                    if self
                        .assignments
                        .iter()
                        .any(|(n, _, _)| n.eq_ignore_ascii_case(&name))
                    {
                        return Err("VGrid AVar additional override producer is unbound".into());
                    }
                    if name.eq_ignore_ascii_case("TerrainBlockSize") {
                        self.terrain = value
                    } else {
                        self.grid = value
                    }
                    self.assignments.push((name.to_owned(), value, source));
                }
            }
        }
        if self.assignments.len() != 2 {
            return Err("VGrid AVar exact source assignments absent".into());
        }
        self.source_loaded = true;
        Ok(())
    }
    pub fn validate(&self, catalog: &Catalog<'_>) -> Result<()> {
        if !self.source_loaded {
            return Err("VGrid AVar actual source assignment pass is incomplete".into());
        }
        catalog.require_source_owner()?;
        if self.sources != catalog.actors.sources {
            return Err("VGrid AVar source owner differs".into());
        }
        Ok(())
    }
    pub fn terrain_block_size(&self) -> i32 {
        self.terrain
    }
    pub fn vgrid_cell_size(&self) -> i32 {
        self.grid
    }
    ///Native512160 flags0 stores the exact signed DWORD, without a clamp.
    pub fn assign_terrain_block_size(&mut self, value: i32) {
        self.terrain = value;
    }
    pub fn assign_vgrid_cell_size(&mut self, value: i32) {
        self.grid = value;
    }
    pub fn metrics(
        &self,
        catalog: &Catalog<'_>,
        quad: impl FnOnce() -> Result<i32>,
    ) -> Result<Metrics> {
        self.validate(catalog)?;
        struct Current<'a, F> {
            state: &'a State,
            quad: Option<F>,
        }
        impl<F: FnOnce() -> Result<i32>> World for Current<'_, F> {
            fn grid(&mut self) -> Result<i32> {
                Ok(self.state.grid)
            }
            fn terrain(&mut self) -> Result<i32> {
                Ok(self.state.terrain)
            }
            fn quad(&mut self) -> Result<i32> {
                self.quad.take().ok_or("VGrid duplicate UVar read")?()
            }
        }
        metrics(&mut Current {
            state: self,
            quad: Some(quad),
        })
    }
}
pub(crate) trait World {
    fn grid(&mut self) -> Result<i32>;
    fn terrain(&mut self) -> Result<i32>;
    fn quad(&mut self) -> Result<i32>;
}
///43FCF1..FD35: both signed mins and the exact unsigned highest-bit loop.
///The unrelated terrain UVar read remains mandatory before computing shift.
pub(crate) fn metrics(world: &mut impl World) -> Result<Metrics> {
    let grid = world.grid()?;
    let terrain = world.terrain()?;
    let selected = if grid > terrain { terrain } else { grid };
    let quad = world.quad()?;
    let terrain_quad_size = if quad > terrain { terrain } else { quad };
    let mut grid_shift = 0u32;
    let selected = selected as u32;
    if selected != 0 {
        while selected
            & !((1u32.wrapping_shl(grid_shift & 31))
                .wrapping_mul(2)
                .wrapping_sub(1))
            != 0
        {
            grid_shift = grid_shift.wrapping_add(1);
        }
    }
    Ok(Metrics {
        grid_shift,
        terrain_quad_size,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};
    #[test]
    fn original_int_constructors_raw_setters_and_metric_read_prefixes() {
        let fixture: Value = serde_json::from_str(include_str!(
            "../../../research/skirmish-20261017-view-grid/native-grid-variables.json"
        ))
        .unwrap();
        let rows = fixture["cases"].as_array().unwrap();
        assert_eq!(rows.len(), 168);
        struct Probe<'a> {
            input: &'a Value,
            stop: Option<&'a str>,
            events: Vec<&'static str>,
        }
        impl Probe<'_> {
            fn get(&mut self, key: &'static str) -> Result<i32> {
                if self.stop == Some(key) {
                    return Err(key.into());
                }
                self.events.push(key);
                Ok(self.input[key].as_u64().unwrap() as u32 as i32)
            }
        }
        impl World for Probe<'_> {
            fn grid(&mut self) -> Result<i32> {
                self.get("grid")
            }
            fn terrain(&mut self) -> Result<i32> {
                self.get("terrain")
            }
            fn quad(&mut self) -> Result<i32> {
                self.get("quad")
            }
        }
        for row in rows {
            let c = &row["input"];
            let output = &row["output"];
            if row["phase"] == "constructors" {
                assert_eq!(
                    output["constructors"],
                    json!([{"index":0,"name":"TerrainBlockSize","current":16,"kind":3,"flags":0},{"index":6,"name":"VGridCellSize","current":4,"kind":3,"flags":0}])
                );
                let mut state = State {
                    terrain: 16,
                    grid: 4,
                    sources: Vec::new(),
                    assignments: Vec::new(),
                    source_loaded: true,
                };
                state.assign_terrain_block_size(c[0].as_u64().unwrap() as u32 as i32);
                state.assign_vgrid_cell_size(c[1].as_u64().unwrap() as u32 as i32);
                assert_eq!(
                    json!([state.terrain as u32, state.grid as u32]),
                    output["current"]
                );
            } else {
                let mut probe = Probe {
                    input: c,
                    stop: row["stop"].as_str(),
                    events: Vec::new(),
                };
                let result = metrics(&mut probe);
                let stopped = result.as_ref().err().cloned();
                let result=result.ok().map(|m|json!({"grid_shift":m.grid_shift,"terrain_quad_size":m.terrain_quad_size as u32}));
                assert_eq!(
                    json!({"events":probe.events,"stopped":stopped,"result":result}),
                    *output,
                    "{row}"
                );
            }
        }
    }
}
