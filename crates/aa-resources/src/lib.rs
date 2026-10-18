//! Read-only machine-memory observations and explicit, checked load planning.
//! A successful assessment is not a reservation or an allocation guarantee.
use serde::{Deserialize, Serialize};

pub type Result<T> = std::result::Result<T, ResourceError>;

#[derive(Debug, thiserror::Error)]
pub enum ResourceError {
    #[error("memory query {api} failed: {source}")]
    Query {
        api: &'static str,
        source: std::io::Error,
    },
    #[error("memory query is unsupported on this platform")]
    UnsupportedPlatform,
    #[error("memory estimate arithmetic overflow: {0}")]
    Overflow(&'static str),
    #[error("invalid memory policy or observation: {0}")]
    Invalid(&'static str),
    #[error(
        "load requires an estimated {requested_bytes} additional CPU bytes, but the observed headroom allows {budget_bytes}: {reason}"
    )]
    Insufficient {
        requested_bytes: u64,
        budget_bytes: u64,
        reason: String,
    },
}

/// Windows values are a short-lived observation, not a transactionally atomic
/// snapshot across APIs. Values include the OS and other applications.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemorySnapshot {
    pub sampled_unix_ms: u64,
    pub physical_total_bytes: u64,
    pub physical_available_bytes: u64,
    pub system_commit_limit_bytes: u64,
    pub system_committed_bytes: u64,
    pub process_available_commit_bytes: u64,
    pub process_virtual_total_bytes: u64,
    pub process_virtual_available_bytes: u64,
    pub page_size_bytes: u64,
}
impl MemorySnapshot {
    pub fn system_available_commit_bytes(&self) -> u64 {
        self.system_commit_limit_bytes
            .saturating_sub(self.system_committed_bytes)
    }
    pub fn validate(&self) -> Result<()> {
        if self.physical_total_bytes == 0
            || self.physical_available_bytes > self.physical_total_bytes
            || self.system_commit_limit_bytes == 0
            || self.page_size_bytes == 0
            || self.process_virtual_available_bytes > self.process_virtual_total_bytes
        {
            return Err(ResourceError::Invalid(
                "invalid physical, commit, page or virtual-memory observation",
            ));
        }
        Ok(())
    }
}

/// Basis points: 100=1%, 1000=10%. Reserves scale with the machine rather
/// than imposing a fixed RAM ceiling. Fractions and overhead are user policy,
/// not Windows requirements. An exhausted reserve yields a zero-byte budget.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryPolicy {
    pub physical_headroom_bps: u32,
    pub commit_headroom_bps: u32,
    pub estimation_overhead_bps: u32,
}
impl Default for MemoryPolicy {
    fn default() -> Self {
        Self {
            physical_headroom_bps: 1000,
            commit_headroom_bps: 1000,
            estimation_overhead_bps: 2000,
        }
    }
}

/// All CPU fields are *additional* bytes not already allocated when the
/// snapshot was taken. Caller accounts for simultaneous copies/parallel work.
/// GPU bytes are reported separately, not treated as a VRAM budget or guarantee.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryEstimate {
    pub resident_cpu_bytes: u64,
    pub temporary_cpu_bytes: u64,
    pub upload_staging_bytes: u64,
    pub gpu_bytes: u64,
}
impl MemoryEstimate {
    pub fn peak_cpu_bytes(&self) -> Result<u64> {
        self.resident_cpu_bytes
            .checked_add(self.temporary_cpu_bytes)
            .and_then(|n| n.checked_add(self.upload_staging_bytes))
            .ok_or(ResourceError::Overflow(
                "CPU resident + temporary + upload staging",
            ))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MemoryConstraint {
    PhysicalHeadroom,
    SystemCommitHeadroom,
    ProcessCommit,
    ProcessAddressSpace,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BudgetAssessment {
    pub snapshot: MemorySnapshot,
    pub policy: MemoryPolicy,
    pub estimate: MemoryEstimate,
    pub estimated_cpu_peak_bytes: u64,
    pub adjusted_cpu_peak_bytes: u64,
    pub physical_reserved_bytes: u64,
    pub commit_reserved_bytes: u64,
    pub physical_budget_bytes: u64,
    pub system_commit_budget_bytes: u64,
    pub effective_cpu_budget_bytes: u64,
    pub exceeded: Vec<MemoryConstraint>,
}
impl BudgetAssessment {
    pub fn fits(&self) -> bool {
        self.exceeded.is_empty()
    }
    pub fn ensure_fits(&self) -> Result<()> {
        if self.fits() {
            Ok(())
        } else {
            Err(ResourceError::Insufficient {
                requested_bytes: self.adjusted_cpu_peak_bytes,
                budget_bytes: self.effective_cpu_budget_bytes,
                reason: self
                    .exceeded
                    .iter()
                    .map(|v| format!("{v:?}"))
                    .collect::<Vec<_>>()
                    .join(", "),
            })
        }
    }
}

fn ceil_basis_points(value: u64, bps: u32) -> Result<u64> {
    let numerator = u128::from(value) * u128::from(bps);
    u64::try_from(numerator.div_ceil(10_000))
        .map_err(|_| ResourceError::Overflow("basis-point scaling"))
}

/// Pure preflight decision. No files, settings, processes or planned asset
/// allocations are changed; only the small returned report is constructed.
pub fn assess(
    snapshot: &MemorySnapshot,
    policy: MemoryPolicy,
    estimate: MemoryEstimate,
) -> Result<BudgetAssessment> {
    snapshot.validate()?;
    if policy.physical_headroom_bps > 10_000 || policy.commit_headroom_bps > 10_000 {
        return Err(ResourceError::Invalid("headroom fraction exceeds 100%"));
    }
    let estimated_cpu_peak_bytes = estimate.peak_cpu_bytes()?;
    let overhead = ceil_basis_points(estimated_cpu_peak_bytes, policy.estimation_overhead_bps)?;
    let adjusted_cpu_peak_bytes = estimated_cpu_peak_bytes
        .checked_add(overhead)
        .ok_or(ResourceError::Overflow("peak plus estimation overhead"))?;
    let physical_reserved_bytes =
        ceil_basis_points(snapshot.physical_total_bytes, policy.physical_headroom_bps)?;
    let commit_reserved_bytes = ceil_basis_points(
        snapshot.system_commit_limit_bytes,
        policy.commit_headroom_bps,
    )?;
    let physical_budget_bytes = snapshot
        .physical_available_bytes
        .saturating_sub(physical_reserved_bytes);
    let system_commit_budget_bytes = snapshot
        .system_available_commit_bytes()
        .saturating_sub(commit_reserved_bytes);
    let constraints = [
        (MemoryConstraint::PhysicalHeadroom, physical_budget_bytes),
        (
            MemoryConstraint::SystemCommitHeadroom,
            system_commit_budget_bytes,
        ),
        (
            MemoryConstraint::ProcessCommit,
            snapshot.process_available_commit_bytes,
        ),
        (
            MemoryConstraint::ProcessAddressSpace,
            snapshot.process_virtual_available_bytes,
        ),
    ];
    let effective_cpu_budget_bytes = constraints.iter().map(|(_, n)| *n).min().unwrap();
    let exceeded = constraints
        .into_iter()
        .filter_map(|(kind, bytes)| (adjusted_cpu_peak_bytes > bytes).then_some(kind))
        .collect();
    Ok(BudgetAssessment {
        snapshot: snapshot.clone(),
        policy,
        estimate,
        estimated_cpu_peak_bytes,
        adjusted_cpu_peak_bytes,
        physical_reserved_bytes,
        commit_reserved_bytes,
        physical_budget_bytes,
        system_commit_budget_bytes,
        effective_cpu_budget_bytes,
        exceeded,
    })
}

pub fn preflight(estimate: MemoryEstimate, policy: MemoryPolicy) -> Result<BudgetAssessment> {
    let report = assess(&query_memory()?, policy, estimate)?;
    report.ensure_fits()?;
    Ok(report)
}

pub fn count_bytes(count: u64, stride_bytes: u64) -> Result<u64> {
    count
        .checked_mul(stride_bytes)
        .ok_or(ResourceError::Overflow("element count * stride"))
}

/// RGBA8 byte estimate including all floor-halved mip levels through1x1.
/// This counts texels only, excluding decoder, row-alignment and driver overhead.
pub fn rgba8_mip_chain_bytes(mut width: u32, mut height: u32) -> Result<u64> {
    if width == 0 || height == 0 {
        return Err(ResourceError::Invalid("zero texture dimension"));
    }
    let mut bytes = 0_u64;
    loop {
        let level = count_bytes(u64::from(width) * u64::from(height), 4)?;
        bytes = bytes
            .checked_add(level)
            .ok_or(ResourceError::Overflow("RGBA8 mip chain"))?;
        if width == 1 && height == 1 {
            return Ok(bytes);
        }
        width = (width / 2).max(1);
        height = (height / 2).max(1);
    }
}

#[cfg(windows)]
pub fn query_memory() -> Result<MemorySnapshot> {
    use windows_sys::Win32::System::{
        ProcessStatus::{GetPerformanceInfo, PERFORMANCE_INFORMATION},
        SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX},
    };
    let mut status = MEMORYSTATUSEX {
        dwLength: std::mem::size_of::<MEMORYSTATUSEX>() as u32,
        ..Default::default()
    };
    // SAFETY: pointers refer to initialized writable structures with their exact
    // API-declared sizes. APIs only observe system state; they retain no pointers.
    if unsafe { GlobalMemoryStatusEx(&mut status) } == 0 {
        return Err(ResourceError::Query {
            api: "GlobalMemoryStatusEx",
            source: std::io::Error::last_os_error(),
        });
    }
    let mut perf = PERFORMANCE_INFORMATION {
        cb: std::mem::size_of::<PERFORMANCE_INFORMATION>() as u32,
        ..Default::default()
    };
    if unsafe {
        GetPerformanceInfo(
            &mut perf,
            std::mem::size_of::<PERFORMANCE_INFORMATION>() as u32,
        )
    } == 0
    {
        return Err(ResourceError::Query {
            api: "GetPerformanceInfo",
            source: std::io::Error::last_os_error(),
        });
    }
    let page_size = perf.PageSize as u64;
    let snapshot = MemorySnapshot {
        sampled_unix_ms: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|_| ResourceError::Invalid("system clock precedes Unix epoch"))?
            .as_millis()
            .try_into()
            .map_err(|_| ResourceError::Overflow("sample timestamp"))?,
        physical_total_bytes: status.ullTotalPhys,
        physical_available_bytes: status.ullAvailPhys,
        system_commit_limit_bytes: count_bytes(perf.CommitLimit as u64, page_size)?,
        system_committed_bytes: count_bytes(perf.CommitTotal as u64, page_size)?,
        process_available_commit_bytes: status.ullAvailPageFile,
        process_virtual_total_bytes: status.ullTotalVirtual,
        process_virtual_available_bytes: status.ullAvailVirtual,
        page_size_bytes: page_size,
    };
    snapshot.validate()?;
    Ok(snapshot)
}

#[cfg(not(windows))]
pub fn query_memory() -> Result<MemorySnapshot> {
    Err(ResourceError::UnsupportedPlatform)
}
