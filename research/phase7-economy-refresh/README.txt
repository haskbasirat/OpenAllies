Economy contribution refresh: original 570020

This is an independent native-oracle acceptance gate for root's
crates/aa-runtime/src/economy_refresh.rs. No production code was changed by
the reviewer. Native execution reads the original AA.exe; full function and
visited instruction bytes are checked against accepted runtime-expanded-v14.

159 original executions cover 53 refresh scenarios at all three x87 precision
settings. Original resource constructors, copies, numeric equality, clearing,
56F310/56F340 removal, 4E48C0/4E4930 Kingdom changes and coupled shortage
arithmetic execute unchanged. Two independent Kingdom states are retained and
compared bit-for-bit. The corpus checks:
- flags/equal and changed vectors, provider true/false and a changing predicate;
- parent membership, destructed and nonpaying Organization upkeep gates;
- captured first owner for additions versus fresh removal owner, including null;
- live callback changes to flags, old vectors and provided member cache;
- signed zero equality, zero-size domain and finite rounding cases;
- parent appearance/disappearance during cost computation and final reread.

450 separate original 4CCE70 executions cover all pairs of twelve float words
including signed zero, subnormals, finite values, infinities and quiet/signaling
NaNs, plus empty/different-size/multi-element vectors, at all three PC settings.
This equality scope does not imply nonfinite economy arithmetic is supported.

Explicit native seams: allegiance virtual+108; source/Organization computations
56FB10, 56FC20, 56FA30; provider predicate 56F9A0; parent 4CF0F0; allocator/free.
All provided vectors and callback mutations are visible in the fixture. Actual
56FC20 call argument write_member_cache=true is asserted. Owner/component and
registry identity are coherent and stable for each call. Source vector helpers
are reviewed separately; this gate is not full actor/economy/world integration.

Events capture pre-mutation component snapshots at every world callback that
receives the component. Kingdom and RefreshParent events record ordering and
identity only; final Kingdom/component state is still compared in full. The
allocator returns distinct writable buffers and is not a game-memory model.
Visited byte pins include replaced seam-entry instructions; full range pins are
static identity checks and do not claim execution behind replaced calls.

Four Rust tests compare all native outputs and check two additional fallible
API boundaries: failure at cost computation retains previous resource changes;
nonfinite upkeep explicitly stops after old removal/new assignment rather than
pretending to reproduce unchecked native NaN arithmetic. No rollback is implied.
The ABI assertions in the producer check stack and all callee-saved registers;
the Rust output marker only associates a comparison with that native result.

Reproduce:
  python research/phase7-economy-refresh/native_refresh.py
  python research/phase7-economy-refresh/record_findings.py

No original files were written. No complete live match, AI, save replay, or
tutorial completion is claimed by this bounded gate.
