//! Lazy, read-only original/v14 definitions. Never used as live simulation state.
use aa_resources::{MemoryEstimate, MemoryPolicy, count_bytes, preflight};
use aa_rules::{
    actors::{ActorCatalog, FieldResolution},
    sources::{SourceLimits, V14Sources},
};
use anyhow::{Context, Result};
use eframe::egui;
use std::{path::Path, sync::mpsc};

pub fn load(reconstruction: &Path, policy: MemoryPolicy) -> Result<ActorCatalog> {
    let budget = aa_resources::assess(&aa_resources::query_memory()?, policy, Default::default())?;
    let install = reconstruction
        .parent()
        .context("Missing original installation directory")?;
    let mut sources = V14Sources::open(
        install.join("Data.rwd"),
        reconstruction.join("native-game/runtime-expanded-v14"),
        SourceLimits {
            max_member_bytes: budget.effective_cpu_budget_bytes / 48,
        },
    )?;
    let source_bytes = sources
        .catalog()
        .iter()
        .filter(|row| row.logical_path.to_ascii_lowercase().ends_with(".tgi"))
        .try_fold(0u64, |sum, row| {
            sum.checked_add(row.size)
                .context("Rule source size overflow")
        })?;
    preflight(
        MemoryEstimate {
            resident_cpu_bytes: count_bytes(source_bytes, 48)?,
            ..Default::default()
        },
        policy,
    )?;
    Ok(ActorCatalog::load_v14(&mut sources)?)
}

#[derive(Default)]
pub struct AuthoredRules {
    pending: Option<mpsc::Receiver<Result<ActorCatalog>>>,
    catalog: Option<ActorCatalog>,
    error: Option<String>,
}
impl AuthoredRules {
    pub fn poll(&mut self) {
        let result = self
            .pending
            .as_ref()
            .and_then(|receive| match receive.try_recv() {
                Ok(result) => Some(result),
                Err(mpsc::TryRecvError::Disconnected) => {
                    Some(Err(anyhow::anyhow!("Rule reader stopped")))
                }
                Err(mpsc::TryRecvError::Empty) => None,
            });
        if let Some(result) = result {
            self.pending = None;
            match result {
                Ok(catalog) => self.catalog = Some(catalog),
                Err(error) => self.error = Some(format!("{error:#}")),
            }
        }
    }
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        thing_id: &str,
        reconstruction: &Path,
        policy: MemoryPolicy,
    ) {
        if self.pending.is_none() && self.catalog.is_none() && self.error.is_none() {
            let path = reconstruction.to_owned();
            let ctx = ui.ctx().clone();
            let (send, receive) = mpsc::sync_channel(1);
            self.pending = Some(receive);
            std::thread::spawn(move || {
                let result =
                    std::panic::catch_unwind(|| load(&path, policy)).unwrap_or_else(|_| {
                        Err(anyhow::anyhow!("Authored-rule loading failed unexpectedly"))
                    });
                let _ = send.send(result);
                ctx.request_repaint();
            });
        }
        ui.collapsing("Authored unit / object definition", |ui| {
            ui.small("Original/v14 source values, not live unit state. Expressions are shown without evaluation.");
            if self.pending.is_some() { ui.horizontal(|ui| { ui.spinner(); ui.label("Reading original rules…"); }); }
            if let Some(error) = &self.error {
                ui.colored_label(egui::Color32::LIGHT_RED, error);
                if ui.button("Retry rule loading").clicked() { self.error = None; }
            }
            if let Some(catalog) = &self.catalog {
                if let Some(actor) = catalog.find(thing_id) {
                    for (label, path) in [("Health", "BodyComponent.health"), ("Defense", "BodyComponent.defense"), ("Maximum movement speed", "MoverComponent.speed_max"), ("Category", "category"), ("Model resource prefix", "View.resource_prefix")] {
                        if let Some(field) = actor.field(path) {
                            match field {
                                FieldResolution::Value { raw, source } => {
                                    ui.label(format!("{label}: {}", String::from_utf8_lossy(raw)))
                                        .on_hover_text(format!("{}\nSHA256 {}\nSource bytes {}..{}", source.path, source.sha256, source.span.start, source.span.end));
                                }
                                FieldResolution::Unsupported { reason } => { ui.small(format!("{label}: unresolved")).on_hover_text(reason); }
                                FieldResolution::Declared { .. } | FieldResolution::Missing => {},
                            }
                        }
                    }
                    ui.small(format!("{} source definitions in this template chain", actor.template_chain.len()));
                } else { ui.label("This placed object has no resolved Thing definition in the current catalog."); }
                ui.small(format!("{} definitions · {} source notices", catalog.actors.len(), catalog.diagnostics.len()));
            }
        });
    }
}
