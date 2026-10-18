# Concrete local frame owner binding

This work connects the existing borrowed simulation/order kernels to the retained local `Session`, `Node`, `Admin`, `Sim`, clocks and A/S synchronizers. It does not create a mission, view-completion receipt or playable-battle certificate.

`Cells::from_runtime` consumes the actual `LocalBootstrap::runtime_borrow()` sibling loans. The private `Host` is built from those cells, validates the current Session/gate provenance, and borrows the same fields during callbacks. Nested order effects use disjoint cells rather than extracting and replacing the Admin or borrowing the whole Host. The ordinary producer/caller is trusted: public `RuntimeBorrow` reference fields are not protected against a deliberately hostile caller swapping unrelated sibling fields before consumption.

The concrete adapter reads current focus and four retained frame variables lazily. Session24 and Node2E5 have one byte owner each; weak gate capabilities observe raw nonzero bytes and expire with the owning allocations. The order dispatcher reads Session24 first and reaches Node2E5 only when needed. The old laboratory `Gate` delegates the same dispatcher. Admin request/decree sealing and client execution retain the existing streams and canonical ordering-player slot.

The actual Node command queue has its native constructor-empty state. Empty `75C9A0` returns before diagnostics, temporary commands or handlers. Positive queues retain their contents and require the real handler. No successful substitute drains them. Recurring Session work calls the separately proved `session_frame_update::run` over the same loans.

`frame_task_effects` routes decree315 through the actual task-history/775BF0 kernel in the same order interval. Its concrete services use the same published World identity, authoritative clock, RNG, current SVar owner and definition Bank. It grants no task/view success by itself. Application timing and heartbeat counters are retained by their separate application owners and loaned through `Frame::with_process_effects`.

## Validation

- `netcode-tests-v1.log`: 56 unit tests passed, including same-owner frame loans, foreign gate refusal, byte lifetime, native queue prefixes and existing Host/task/Session cases. This run predates the additive positive SVar binding and timing/heartbeat loans.
- `orders-gate-tests-v1.log`: 11 selected compatibility tests passed. The later `orders-gate-tests-v2.log` replays all three lazy-gate tests, including 96 native raw-byte cases; these runs overlap.
- `runtime-frame-tests-v1.log`: the actual-source frame-variable test and eight existing Sim tests passed. The actual-source test used the verified source Bank and explicit controlled fresh profile scope; it did not import the user's active preferences.
- Immutable native replays: four variable registrations across 12 original/v14 and x87-precision runs; queue constructor/empty/positive cutoffs across18 runs; 96 lazy-gate cases plus96 bounded constructor-store projections. Byte and instruction-boundary audits are recorded in `native-pin-audit-v3.json` (and superseded earlier audits).
- Variable source precision comes from the actual completed source Bank, independently of live simulation precision. Source inputs, flags and provenance remain retained. The UVar oracle stops after all213 registration calls, before later snapshot-manager work; the AVar oracle is an explicit constructor slice.

The initially overstated UVar scope is retained in `native-frame-variables-v1-scope-overstatement.json`. The corrected fixture has identical registration values and instruction pins; its scope/producer hash changed. Failed development attempts and their logs are retained.

## Remaining boundaries

Real loaded-mission/application/view completion has not reached this adapter end to end. Positive AVI output, command handlers, remote peers, replay-source ownership, timing/end-condition/heartbeat services and the outer `752AE0` two-QPC duration-history wrapper must be supplied at their reached boundaries. The typed Node238 DWORD ring is shared with that outer wrapper; it must not gain a second timing-history owner. A rendered GPU frame alone cannot substitute for native view initialization or `74FF50`.

The positive current-SVar traversal is documented separately in `../current-svar-sync/`; its source-store proof and actual-map integration are separate from the registration-only diagnostic projection.
