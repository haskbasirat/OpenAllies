Texture load field kernel checkpoint

New production module texture_load_kernel.rs + tests is exported, with no ResourceEntry
reconstruction, Native payload copy, readiness bool or alternate Resource facade.
State agent owns the actual growable SourceTexture/Pixel arena and SAME Entry bridge.
Root owns the real selected TextureInput -> decoded PixelData client operation.

Native5A8CD0/5A8B40/5A6A10 and actual pointer/quality/dimension/rounding helpers execute
351 rows /702 separate original-v14/all-PC runs /732 instruction byte pins. Explicit
Source-create/default, current-policy, stream/decode, renderer dimensions, resize and
diagnostic/CString/destruction transports are child seams. This is not full native loading.
Fresh immutable replay passed. The exact production Rust source is compiled by rustc --test
and replays every row/output/refcount/error prefix, 1/1 passed. Current formatted runtime
production check passed16.07s. No full build/release or actual decoder execution by this task.

API: Fields mandatory borrowed operations; State<Source,Pixel>::new_call().load(fields,PC);
normalize(fields,&mut actualLocalSource,PC). Source/Pixel values identify actual arena
allocations; only the concrete bridge validates same Manager/generation and owns payloads.
Failed call state retains local identities/stage. Caller must retain its actual in-flight
owner and cannot silently reuse that State to erase a failed prefix.

Output retain precedes stream destruction. Source3C publication follows post-load dimensions,
accounting and format gates; decoder success alone cannot publish it. Zero dimensions use
native16 fallback; nonpow2 resize keeps original54/58 and sets UV4C/50 after actual resize.
Native1024 warning is diagnostic only. Native resize, true pixel-format metadata and current
application policies remain reached concrete services; no guessed defaults are supplied.

No original files, releases, Cargo manifest/lock or vendor SDK sources were edited/integrated.
Independent review is pending; see findings.json for exact pins and scope boundaries.
