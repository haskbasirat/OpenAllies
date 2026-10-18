# aa-economy

Native-proven kingdom economy for the Rust rebuild: resource mutation functions,
the complete GKingdom economy Tick (`tick` module, phase 8) and kingdom resource
transfers (`transfers` module, phase 8). It is not a scheduler, a world, or a
presentation layer: events, victory checks and actor/kingdom resolution are
explicit callbacks or caller responsibilities.

`Economy::new(Catalog, State, Limits, X87Precision)` takes a caller-provided resource
registry order, four exact binary32 arrays, the provided-resource cap (100 in the
expanded build), and the stock maximum resolved from rules (99999 in v14).
`F32Bits` preserves raw bits, including signed zero and subnormals. The constructor
does not invent balances, actor production, map starting resources, or an initial
tick. In a snapshot, `State::consumed` already contains the native shortage charges.

The implemented operations are:

| API | Original code | Scope |
| --- | --- | --- |
| `adjust_provider` | `0x4E3940` + complete `0x4E1F60` | Remove old shortage charges, update raw/effective capacity, apply new charges. |
| `adjust_providers` | `0x4E4930` | Complete ascending vector loop, including a full rebalance for every zero delta. |
| `adjust_consumption` | `0x4E48C0` | Remove/mutate/apply for each ascending resource, rather than one rebalance around the vector. |
| `credit_stockpiles` | `0x4E39D0` | Add and clamp each row to zero/stock maximum. |
| `debit_stockpiles` | `0x4E1690` | Subtract all rows without an affordability check or zero clamp. |
| `can_afford` | `0x4E20A0` | Compare only STOCKPILED rows; no purchase or debit. |
| `flow_arithmetic` | `0x4E9C40..0x4E9DA6` | Seconds × stored float32 `1/60`, gross/net vectors, then reverse-order STOCKPILED updates/clamps and the negative-stock flag. |
| `tick` (phase 8) | complete `0x4E9C40` + `0x4E12D0`, `0x4CDFD0`, `0x4E37D0`, `0x58ED00` | Flow, upkeep-shortage ratio events 3/4, recruit-single regeneration, Commander XP income, victory-check seams, statistics. |
| `transfers::*` (phase 8) | `0x4E2080`/`0x4E1630`, `0x4E16D0`, `0x4CA680`, `0x509840` types 3/4 (`0x509440` tribute), `0x4E24B0`, `0x4E0FD0` | Row add/subtract, steal resources, give primary resource, faction ability affordability, Commander XP with multiplier. |
| `statistics` (phase 8) | `0x58E870`, `0x58D950`, `0x58D8C0`, `0x564ED0`, `0x564E10`, `0x58EAB0`, `0x58EE00` | Authoritative kingdom statistics (+0x32C scores/production, +0x340 ten keyed relation slots) and construction/upgrade completion counters. |
| `supply` (phase 8) | `0x58AE10`, `0x58ADB0`, `0x58ACE0`, `0x58AC70`, `0x4D0A80`, `0x4D0C90`, `0x4D7CE0`, `0x56F2B0` | Supply component tick (relays, stockpile drain/recharge, zones) and EconomyComponent::Tick. |
| `kingdom_registry` (phase 8) | `0x4E6180`, `0x4E44E0`, `0x4E2A60`, `0x4E2CB0`, `0x4E4C30` | Kingdom actor membership and recruit item table used by ChangeKingdom `0x4D8450`. |
| `kingdom_start` (phase 8) | `0x4E6DB0`, `0x4CDF10`, `0x4730D0` | Start-mission stockpile reset, credits, technology grants (callback), faction check, Commander starting experience. |
| `restrictions` (phase 8) | `0x4E1910`, `0x4E1810`, `0x4E1890` | Kingdom/team actor and technology restriction profiles (ASCII case-insensitive, last entry wins). |
| `kingdom_load` + `statistics` load (phase 8) | `0x58EE80`, `0x58DBE0`, `0x4E3690`, `0x58EA00`, `0x4E8A09` loop | Economy semantics of GKingdom::Load not applied by the aa-mapworld wire decoders, statistics promotion at world start. |
| `world_start` (phase 8) | `0x4F1E10` | World-start statistics promotion/reset, owner statistics tracking walk, forced initial snapshots. |
| `x87::Register` (phase 8) | n/a | Public finite x87 register (load/add/sub/mul/compare/round_f64/floor/store_f32) for ports in other crates. |

Phase 8 evidence, cases and gaps: `research/phase8-economy/README.txt`
(fixtures `tests/fixtures/native-tick-fixtures.json`,
`tests/fixtures/native-transfer-fixtures.json`).

Shortage handling visits definitions in registry order. For an UPKEEP deficit it
updates only STOCKPILED consumption rows using that upkeep's cost coefficient.
The difference spills to binary32 before multiplication. Multiplication and the
destination add/subtract stay at the selected x87 precision until their final
binary32 store. Combining charges, skipping zeros, using unordered parallel
updates, or rounding every expression to `f32` changes some native results.

The implementation therefore uses finite dyadic integer arithmetic, explicitly
rounding at each x87 instruction/store boundary. Select `Single24` (`0x007f`),
`Double53` (`0x027f`), or `Extended64` (`0x037f`), all round-to-nearest/ties-even.
There is **no default assertion that the complete original process always uses
one control word**: graphics initialization and other native code can change it.
The simulation thread itself runs at `Double53` (Windows initial control word on
its own TGC_Thread; see research/phase8-tick/fpu-precision.md), so game callers
should pass `Double53`; the oracles still prove all three modes.
This implementation neither reads nor changes the host process's FPU mode.

Nonfinite inputs, nonfinite binary32 results and other rounding modes are outside
the supported domain. Invalid operations return errors without committing a
partial Rust state; that transactional error behavior is a laboratory safety
contract, not a claim about the native engine's NaN/exception handling.

`flow_arithmetic` intentionally stops before bankruptcy ratio calculation/event
dispatch, attrition, experience and actor effects later in `0x4E9C40`. Its
`suppress_overdrawn` argument corresponds only to the observed economy+0x10
nonzero guard; its wider gameplay meaning is not guessed. The function does not
schedule itself or claim to implement the native economy tick. Returned shortage
snapshots are diagnostic observations, not fabricated gameplay events.

Resource order is explicit. `aa-rules` exposes resource declarations, ResourceList
positions and cost rows by ID separately. Native insertion assigns an incrementing
index, but the caller must establish the loader's visitation order before treating
an authored-order projection as the actual native registry. Synthetic oracle
fixtures also exercise layouts beyond the stock catalog; they do not validate
general TGI loading or arbitrary mod registration.

Validation uses unchanged original x86 instructions under Unicorn, checking every
function range against the accepted v14 image. There are 150 scenarios / 960
operations over the three precision modes, including coupled ammo/oil charges,
expanded capacity transitions, vector ordering, two stock destinations, fractional
costs, subnormal/signed-zero values, a 53-vs-64-bit double-rounding boundary, and a
signed integer cap above the binary32 exact-integer range. Every resulting array
and every complete shortage-helper return snapshot is compared bit for bit.
Only the flow prefix's two temporary allocations are mocked; no arithmetic or
branch decision is replaced. No native game is launched or modified.

```powershell
python rust-rebuild/research/evidence/economy-phase2-oracle.py
cargo test --manifest-path rust-rebuild/Cargo.toml -p aa-economy
cargo clippy --manifest-path rust-rebuild/Cargo.toml -p aa-economy --all-targets -- -D warnings
```
