# Saved actor with a removed Kingdom

The actual selected Firefight Host/resource-worker attempt10 completed
ConfigureGWorld, including removal of unused Kingdom1677. DynamicThing1682
loaded. DynamicThing1683 then referred to removed owner1677 and stopped the
whole Rust load. The plain error was incorrectly classified.

Native4D8D65 tests the real Thing bit23 before reading the saved owner. It
checks the GTID generation and current pointer table and stores the result at
Actor+E8. A nonzero handle whose pointer is null calls5A0910.5A0910 calls5A0300,
which constructs the TGC_LoadCancelException type described by B281A4. The
4FAE80 FuncInfo's states13..15 catch that same type with4FB410, which calls
4F5490, clears the output slot and returns continuation4FB4B1. It does not
restore the removed Kingdom or stop the entire map load.

The production fix only adds the already established `Map Load Canceled:`
marker to that exact missing-owner error. The existing caught exception,
partial-record cleanup and null-result path perform the rest. Unrelated
required-service errors retain their prior failure behavior.

`native_owner_cancel.py` independently executes54 owner-branch cases and6
catch-handler cases, original/v14 and all3 PCs. CString401B00 and destruction
4F5490 are explicit recorded leaves;5A0910 is a reached throw cutoff. The
throw/catch RTTI pointers are checked. `check_throw_bridge.py` separately pins
the actual direct calls and exception object construction bytes. Its static
pins are not counted as executed oracle instructions. Neither proof executes
the full SEH runtime, destructor or whole map load.

The2 Rust replay/regression tests exercise the actual private load_record
coordinator on explicitly synthetic, predecoded records and definitions.
They check every owner branch, canceled partial-record removal, true registry
generation increment, no warning/publication/component side effects, and
acceptance of the next registration in that same registry. Continued branches
must reach the deliberately missing Actor tail without being misclassified as
cancel. These fixtures do not certify complete source-created Worlds.

`rust-tests01.log` preserves a test-only formatting compile error: an internal
non-Debug result was formatted in an assertion. It was corrected without a
production change. `rust-tests02.log` passed2/2. An initial native-script
development assertion read only28 bytes of a29-byte RTTI name; corrected to32
before any fixture was created. No failed native fixture was overwritten.

Commands use locked/offline release, lto=false/codegen-units=16 overrides and
the exclusive target/s-local-bootstrap. This is not the final thin-LTO client
release. Native fixtures are create_new; ordinary producer invocations replay
them read-only. Original executables, maps and profiles remain untouched.

Attempt10 reports and log remain immutable. Actual11 tests this fix through
the genuine Host/selected resource worker; a later required gate cannot be
reported as a completed battle.
