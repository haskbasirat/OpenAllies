# Memory planning

This crate observes memory and evaluates a caller-supplied load estimate. It
does not change Windows settings, reserve memory, open another process or
discard assets. An insufficient budget produces a report/error before the
caller starts the planned allocation.

```rust,no_run
use aa_resources::{query_memory, assess, MemoryEstimate, MemoryPolicy};
let snapshot = query_memory()?;
let estimate = MemoryEstimate {
    resident_cpu_bytes: 100_000_000,
    temporary_cpu_bytes: 20_000_000,
    upload_staging_bytes: 100_000_000,
    gpu_bytes: 100_000_000,
};
let report = assess(&snapshot, MemoryPolicy::default(), estimate)?;
report.ensure_fits()?;
# Ok::<(), aa_resources::ResourceError>(())
```

The default policy leaves 10% of observed total physical RAM and 10% of the
current system commit limit unused, and adds 20% to the supplied CPU estimate.
These explicit configurable fractions are safety policy, not operating-system
requirements. There is no fixed byte cap. Available process commit and virtual
address space are additional constraints. All arithmetic is checked; fractional
reserves round upward without floating-point rounding.

CPU estimates mean **additional** memory after the observation. Include all
simultaneously retained arrays, copies of old/new scenes, parallel decoder
scratch, mipmap intermediates and upload staging that will be allocated later.
Already allocated data is reflected in available memory and must not be charged
twice. `rgba8_mip_chain_bytes` counts exact RGBA8 texel bytes through 1x1; it
does not include row padding or decoder/driver overhead. The estimate must not
be confused with the size of compressed PNG files.

`gpu_bytes` is reported separately. CPU RAM does not establish dedicated GPU
capacity. The client must still query adapter limits/budgets where supported,
use checked/fallible allocations, handle device allocation errors, and commit
a new scene only after it is ready.

Windows observations are volatile and the two calls are not an atomic snapshot.
Preflight cannot guarantee a later allocation, prevent other programs from
using RAM, or prove availability of a contiguous virtual range. Refresh the
observation before each large phase. Failure to query is an error, never a
pretend unlimited budget. Other operating systems currently return Unsupported.

Sources checked October 13, 2026:

- [GlobalMemoryStatusEx](https://learn.microsoft.com/en-us/windows/win32/api/sysinfoapi/nf-sysinfoapi-globalmemorystatusex)
  supplies physical and process-available values; results are volatile.
- [MEMORYSTATUSEX](https://learn.microsoft.com/en-us/windows/win32/api/sysinfoapi/ns-sysinfoapi-memorystatusex)
  defines `ullAvailPageFile` as available process commit, not page-file bytes.
- [GetPerformanceInfo](https://learn.microsoft.com/en-us/windows/win32/api/psapi/nf-psapi-getperformanceinfo)
  supplies system commit information; the crate checks its return value.
- [PERFORMANCE_INFORMATION](https://learn.microsoft.com/en-us/windows/win32/api/psapi/ns-psapi-performance_information)
  reports commit counts in pages. The crate multiplies by PageSize with overflow
  checking, rather than confusing page counts with bytes.

`cargo test -p aa-resources` runs pure arithmetic checks. The `memory_report`
example prints one read-only observation with default policy and a zero-byte
plan; it does not allocate a synthetic workload or launch the game.
