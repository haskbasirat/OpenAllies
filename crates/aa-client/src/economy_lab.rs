//! Diagnostic scenario built from the accepted rule sources. It does not spawn
//! a kingdom, authorize an order, select a native registry order or run a match.
use aa_economy::{
    Catalog, Economy, F32Bits, Limits, ResourceDefinition, ResourceKind, State, X87Precision,
};
use aa_rules::{
    resources::{ResourceCatalog, ResourceCategory},
    sources::{SourceLimits, V14Sources},
};
use anyhow::{Context, Result, bail};
use serde::Serialize;
use std::path::Path;

#[derive(Serialize)]
pub struct Row {
    pub id: String,
    pub raw_provided: f32,
    pub effective_provided: f32,
    pub consumed: f32,
    pub stockpiled: f32,
}
#[derive(Serialize)]
pub struct LabReport {
    pub scope: String,
    pub registry_order: String,
    pub precision: String,
    pub provider_limit: i32,
    pub stockpile_limit: f32,
    pub shortage_money_per_minute: f32,
    pub money_after_60_seconds: f32,
    pub shortage_helper_calls: usize,
    pub rows: Vec<Row>,
    pub source_files: Vec<aa_rules::resources::SourceIdentity>,
}
fn vector(catalog: &Catalog, values: &[(&str, f32)]) -> Result<Vec<F32Bits>> {
    let mut result = vec![F32Bits::ZERO; catalog.resources.len()];
    for (id, value) in values {
        let index = catalog
            .resources
            .iter()
            .position(|r| r.id == *id)
            .with_context(|| format!("Missing original resource {id}"))?;
        result[index] = F32Bits::from_f32(*value);
    }
    Ok(result)
}
pub fn run(reconstruction: &Path, policy: aa_resources::MemoryPolicy) -> Result<LabReport> {
    let memory = aa_resources::assess(&aa_resources::query_memory()?, policy, Default::default())?;
    let install = reconstruction
        .parent()
        .context("Missing game installation parent")?;
    let mut sources = V14Sources::open(
        install.join("Data.rwd"),
        reconstruction.join("native-game/runtime-expanded-v14"),
        SourceLimits {
            max_member_bytes: memory.effective_cpu_budget_bytes / 12,
        },
    )?;
    let source = ResourceCatalog::load_v14(&mut sources)?;
    let ids: Vec<_> = source.resources.iter().map(|r| r.id.as_str()).collect();
    let mut resources = Vec::new();
    for resource in &source.resources {
        let mut costs = vec![F32Bits::ZERO; ids.len()];
        // Only use a zero where the original CostList explicitly supplied it.
        for (i, id) in ids.iter().enumerate() {
            // 0x4E1F60 never reads coefficients whose destination is not
            // STOCKPILED. Keep those unused matrix entries zero, without
            // inventing an authored purchase-cost row for ammo/oil/capacity.
            if source
                .find(id)
                .context("Resource definition missing")?
                .category
                != ResourceCategory::Stockpiled
            {
                continue;
            }
            let row = resource
                .purchase_cost
                .iter()
                .find(|r| r.resource_id == *id)
                .or_else(|| source.cost_defaults.iter().find(|r| r.resource_id == *id))
                .with_context(|| format!("No authored cost/default for {id}"))?;
            costs[i] = F32Bits(row.value.f32_bits);
        }
        resources.push(ResourceDefinition {
            id: resource.id.clone(),
            limited_provider: resource.limited_provider,
            purchase_cost: costs,
            kind: match resource.category {
                ResourceCategory::Stockpiled => ResourceKind::Stockpiled,
                ResourceCategory::Upkeep => ResourceKind::Upkeep,
                ResourceCategory::LimitedConsumed => ResourceKind::LimitedConsumed,
            },
        });
    }
    let catalog = Catalog { resources };
    let production = vector(
        &catalog,
        &[
            ("money", 100.0),
            ("ammo", 2.0),
            ("oil", 1.0),
            ("unit_limit_provided", 150.0),
            ("building_limit_provided", 150.0),
        ],
    )?;
    let upkeep = vector(&catalog, &[("ammo", 4.0), ("oil", 2.0)])?;
    let credit = vector(&catalog, &[("money", 1000.0)])?;
    let money = ids
        .iter()
        .position(|id| *id == "money")
        .context("Original money resource missing")?;
    let limits = Limits {
        provided_maximum: source.limited_resource_max,
        stockpiled_maximum: F32Bits(source.stockpiled_resource_max.f32_bits),
    };
    let mut economy = Economy::new(
        catalog.clone(),
        State::zeroed(ids.len()),
        limits,
        X87Precision::Extended64,
    )?;
    // Deliberately labelled diagnostic inputs, not guessed map starting values.
    let provider_trace = economy.adjust_providers(&production, true)?;
    economy.credit_stockpiles(&credit)?;
    let consumption_trace = economy.adjust_consumption(&upkeep, true)?;
    let shortage = economy.state().consumed[money].as_f32();
    if shortage != 7.0 {
        bail!("Original shortage diagnostic diverged: expected 7 money/minute, got {shortage}");
    }
    economy.flow_arithmetic(F32Bits::from_f32(60.0), false)?;
    let state = economy.state();
    if state.stockpiled[money].as_f32() != 1093.0 {
        bail!("Stock flow diagnostic diverged");
    }
    for id in ["unit_limit_provided", "building_limit_provided"] {
        let i = ids
            .iter()
            .position(|v| *v == id)
            .context("Capacity resource missing")?;
        if state.effective_provided[i].as_f32() != source.limited_resource_max as f32 {
            bail!("Expanded capacity diagnostic diverged");
        }
    }
    Ok(LabReport{scope:"Native arithmetic diagnostic only; no kingdom initialization, bankruptcy callbacks or gameplay execution".into(),
        registry_order:"Explicit resources.tgi declaration order for this diagnostic; native registry construction order remains unverified".into(),
        precision:"Explicit x87 Extended64 / round-to-nearest fixture mode; whole-process FPU lifecycle remains unverified".into(),
        provider_limit:source.limited_resource_max,stockpile_limit:f32::from_bits(source.stockpiled_resource_max.f32_bits),
        shortage_money_per_minute:shortage,money_after_60_seconds:state.stockpiled[money].as_f32(),
        shortage_helper_calls:provider_trace.shortage_steps.len()+consumption_trace.shortage_steps.len(),
        rows:ids.iter().enumerate().map(|(i,id)|Row{id:(*id).into(),raw_provided:state.raw_provided[i].as_f32(),
            effective_provided:state.effective_provided[i].as_f32(),consumed:state.consumed[i].as_f32(),stockpiled:state.stockpiled[i].as_f32()}).collect(),
        source_files:source.sources})
}
