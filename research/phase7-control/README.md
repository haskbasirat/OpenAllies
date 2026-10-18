# Tutorial SAI control and actor Stop dispatch

`aa_navigation::control` implements native trigger wrappers542080/5432C0/543380,
flag leaves79E2E0/79E2F0, actor disable79E980, command construction4D9E20,
actor routing4D9B60, and temporary strong-reference cleanup4CA2D0.

- `enable(None, world)` returns false without touching world state. A real object
  clears flag0x20, identically for Kingdoms and actors.
- `disable_kingdom` sets flag0x20, takes the actual Kingdom+284 head, processes a
  fresh registry-ordinal6 command for each node and rereads that node's next link
  only after processing and reference cleanup. Null actor nodes still construct
  a command and return false from processing; the wrapper ignores that result.
- `disable_actor_state` sets the actor flag, caches and retains actor+154, reads
  that shared object's actual+10 target, calls target virtual+68 with the held
  handle and four zeros, then releases the cached handle. Callback replacement
  of actor+154 does not change the reference that must be released.
- `disable_actor` performs that operation then constructs/processes/releases a
  fresh Stop. It returns true for a nonnull actor even when processing returns
  false; callback errors still stop the operation.
- `stop` exposes the actual process return, with temporary command+34 cleanup.
  Mind's deliver_stop can bind to this without bypassing actor authorization.

`process` resolves the definition's real component index, captures that actor
component before admission, and calls actual actor virtual+38. Rejection returns
false without parent fallback. If the component is missing, native virtual+158
chooses virtual+15C versus the actor+7C organization+44 parent. The loop has no
invented hop limit or route shortcut. Host hierarchy/lifetimes must remain valid.

All required World fields and callbacks are fallible, with no successful default.
Actual component+24 dispatch must bind to the real component command handler.
For Element command6 this is the independently developed aa-mind handler, reached
through the existing Mind command processing path. This module does not implement
that handler or substitute a direct movement cancellation.

Reference increments/decrements wrap like original native words. Reaching zero
calls the required actual destructor service. On an update error the reference
word must remain unchanged; if destruction fails after reaching zero, the zero
and prior effects remain. The runtime must stop and must not automatically repeat
the release. Service errors release successfully held temporary references and
prevent later node/command submission. Native callbacks' normal false return can
still have committed changes and is not treated as an error.

Evidence: native_control.py executes31 complete original cases with stack and
nonvolatile-register checks. Cases cover null wrappers, both flags, real command
construction, direct/organization/virtual parent routing, admission rejection,
false command result, alternate component slots, empty/null-node lists, mutation
of next links and captured components, actor-handle replacement, refcount zero/
wrap and a command parameter assigned during dispatch. Actor/component/SAI
virtuals and destructors are explicit mutable fixture services. The null-actor
diagnostic is suppressed. No original executable bytes are patched.

Two Rust tests compare observable callback order, final flags/reference words,
return values and every initialized command field, then inject callback errors
to verify cleanup and stop behavior. Constructor-only observations and suppressed
logging are excluded from the callback trace comparison. Full original/v14
routines and visited instruction bytes are pinned. Independent peer review is a
separate gate. The wrappers are library implementations; complete real tutorial
initialization still requires live world/definition and component bindings.
