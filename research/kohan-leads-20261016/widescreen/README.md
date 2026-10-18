# Kohan II widescreen lead review

Reviewed October 16, 2026. Eleven selected source/documentation/license files
(77,675 bytes) were downloaded into the separate reference directory. No DLL,
game binary or derivative game-data files were downloaded, and no downloaded
script/source was executed, installed, built, or applied.

## Provenance and version

The [repository](https://github.com/Ensrick/kohan2-widescreen/tree/312a8f3cd84d20708ad5ee555a46a66c1bdb9fb4)
was pinned at main commit `312a8f3cd84d20708ad5ee555a46a66c1bdb9fb4`
(August 15, 2026). Its stated target is **Kohan II classic/Steam 1.2.3**. Its
changelog has release 0.2.0 plus subsequent unreleased removals. The actual
Kohan executable/build compatibility and the author's live rendering claims
were not independently tested here.

The root LICENSE grants MIT terms for the repository's original scripts,
documentation, research and authored work, with an explicit exception for
derivative Kohan game-data content. That exception explains why the captured
GitHub API identifies the notice as `NOASSERTION` rather than a simple MIT
badge. Notices remain intact. No rights to TimeGate's underlying game assets
are inferred. `download-manifest.json` records every commit-specific URL,
Git blob SHA1, byte length and SHA256.

## What the source actually does

The C source targets Kohan **RVA `49545D`**, resumes at RVA `495465`, and
requires the displaced bytes `8b028d8e20030000`. It modifies the frustum
instead of only one projection-matrix entry, with perspective and near-4:3
guards. The repository attributes its former terrain-edge clipping to
patching only the displayed matrix; this is an investigation lead for matching
projection and visibility, not an AA result. See the pinned
[aspect research](https://github.com/Ensrick/kohan2-widescreen/blob/312a8f3cd84d20708ad5ee555a46a66c1bdb9fb4/docs/ASPECT_PATCH.md).

There is a source/documentation difference: `dll/avifil32.c` widens the horizontal
bounds using aspect and a height-based zoom multiplier, widens the vertical
bounds by zoom, and scales far distance. Its zoom is `max(1, clientHeight/1024)`.
`Kohan2Widescreen.ps1` and `tools/frustumpatch.py` contain the earlier horizontal
aspect adjustment, without that vertical/far scaling. They are not literally
identical implementations despite an old source comment. No AA constants,
baseline height or zoom policy should be copied from these files.

The implementation waits for a known byte sequence, installs a detour and
forwards imported AVI functions; those are Kohan binary-patching mechanisms.
The independent Rust rewrite needs none of that injection, thread suspension,
DLL proxy or Steam-wrapper machinery. The source chooses one initial window
size; stale splash/resolution/menu framing concerns in its documents remain
useful test cases. Its native scroll-out rule is separate from this default
projection change. No Kohan memory address is an AA address.

## A useful original-AA candidate was independently found

Searching the **original AA bytes**, not assuming equal addresses, found that
the eight displaced bytes occur once at **AA VA `00892B4D`**. Surrounding AA
instructions independently establish a camera/projection builder:

* The function candidate starts at `00892970`, takes six stack arguments and
  ends with `ret 18` at `00892E81`. The old exported function list did not
  identify this region as a separate function; its label remains a candidate.
* At `00892B44`, the real frustum pointer is loaded from argument 5; the earlier
  EDI value belongs to camera basis data. This is confirmed from AA's own
  stack/register instructions rather than importing the Kohan register claim.
* `00892B7A..00892D5E` reads near/far, horizontal bounds, vertical bounds and
  mode `+18`, constructs perspective/orthographic projection words, and
  prepares submission of the projection matrix at renderer `+6E0`.
* Original AA constants at `009F32E0` and `00A0083C` are raw 1.0 and -2.0.
  The original and accepted v14 bytes in the bounded region match.

There is also a naming trap in the Kohan notes: they call frustum `+8` bottom
and `+C` top. The matching evaluation **header declaration only** identifies
the order as left, right, top, bottom, near, far, orthographic. The actual AA
instructions use `+8 minus +C`; the new Rust API therefore uses top-minus-bottom.
No restricted vendor implementation was consulted or integrated.

The lead authorized a separately documented, bounded **AA-native** projection
producer after this discovery. Its code, oracle and limits are in
`../projection/`. It does not implement the Kohan widescreen patch, mutate
frustum inputs, set camera FOV/defaults, install a DLL, change the client camera
or claim complete SetCameraData/culling/picking/rendering compatibility.

AA also has its own `CameraFOV` registration at `00402820`, followed by
`004B3470` at `00402843`, with binary32 arguments 15, 5 and 180 in the nearby
pushes. That establishes an independent AA source lead; it is not evidence of
the current player's loaded FOV or the full game camera's projection policy.
The current Rust `aa-client::Camera` is explicitly an asset-inspection camera
with unproven projection parity.

## DPI and deterministic integration lessons

The selected patch source reads client dimensions but does not implement a DPI
policy, text layout or input-coordinate reconciliation. It is not a solution
to the reported Windows 150% text/hitbox problem. Microsoft's
[GetClientRect contract](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-getclientrect)
describes client-area dimensions; window DPI is a separate query whose result
depends on awareness, as described for
[GetDpiForWindow](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-getdpiforwindow).
Test actual drawable size, UI scaling, mouse conversion, live resize and monitor
changes independently. No DPI or visual test was performed here.

Keep projection/viewport/FOV state separate from authoritative simulation
state and RNG. Camera changes can still affect picking and visibility, so
matching matrix arithmetic alone cannot certify gameplay equivalence. Preserve
native x87 rounding order and precision explicitly; do not change the simulation
thread's precision to obtain a graphics fix. The existing AA precision audit
distinguishes main/UI PC24 from simulation PC53 with stated OS assumptions;
the new bounded projection oracle covers PC24, PC53 and PC64 separately.

Next integration evidence would need AA's actual frustum/FOV producer, real
viewport and culling consumers, screen-to-world picking, terrain-edge and
depth behavior, aspect changes, orthographic/minimap/shadow cameras, menu
framing, and genuine DPI transition checks. None is supplied by merely copying
Kohan's address or height factor.
