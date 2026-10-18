Native ability payment4E2590/4CDD90/4BEB50/4E1690

Implemented in aa-combat::payment with owned aa-economy::Economy and faction
experience balance. This is actual state mutation, not an accepted payment log.
No local definition means no payment. Faction cost is signed i32 converted to
f32 before subtraction. Positive cost and a statistics object call stat45(amount)
then46(1); the original captured statistics identity is reused. Only afterwards
is the current dense resource cost vector copied and actual stockpiles debited.
No affordability check or zero clamp belongs to this debit path.

366 native cases execute original payment/copy/debit instructions; only allocation,
free and statistics callback leaves are intercepted. Mutation fixtures alter
faction XP/statistics identity and costs inside the first statistics callback.
Native growth/copy instructions execute. Each fixture is replayed in PC24/53/64.
Two Rust tests include failed statistics after XP debit: it does not debit resources
or hide the earlier XP mutation. Finite source balances/costs with matching resource
registry dimensions are the supported domain. Numeric failures in aa-economy's
validated debit are Rust errors, not native invalid-input emulation.

Runtime must bind shared native kingdom+230 identity to the actual faction balance;
it must not keep divergent copies across ownership changes. Player/statistics/FX,
actor lifecycle, resource catalog resolution, and admission authorization are
external operations. This proof is not a live gameplay or network integration.
Commands: python research/phase7-payment/native_payment.py
cargo test -p aa-combat --test native_payment
cargo clippy -p aa-combat --all-targets -- -D warnings
