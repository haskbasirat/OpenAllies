The real LocalBootstrap LoadGameTask now retains its native+C assignment from
the application's SAME ResourceManager TaskQueue counterB3DA08. This is a
sequence value and lifetime/source witness; it does not certify readiness.

Native order is preserved: construct the actual task/Inputs, publish that SAME
Arc in Session124, validate the current selected Creator/source against this
application depot, reserve the shared counter slot, retain the opaque slot on
the SAME task, then push the actual Job before Node BeginTask side effects.
The task field lock is acquired before the counter advances. A bridge refusal
therefore leaves Session124 published and does not insert an unissued Job.

The stored field is Mutex<Option<ExternalSequence>>, with None representing
the pre-writer constructor interval. No copied counter, public raw constructor,
Clone or zero fallback is exposed. The existing resource_task native proofs
belong to their author; this directory certifies only the concrete caller glue.
No heterogeneous ResourceManager queue execution or complete worker is inferred
from the shared counter assignment.

The initial production check was blocked by an unrelated transient visual-map
argument type and is preserved. The second aa-netcode library check passed in
28.08s using release with lto=false and codegen-units=16, locked/offline.
The updated actual-source integration test asserts the genuine first sequence
is0 in its explicit fresh-manager scope, which has no earlier task enqueues.
Its execution is recorded separately after the current integration build.

Actual21 deliberately predates this caller glue; its frozen binary and inputs
remain unchanged. Actual22 is the first selected-map startup build containing
the shared-counter integration. Existing task ownership/status/completion and
CallbacksTicket APIs are unchanged.
