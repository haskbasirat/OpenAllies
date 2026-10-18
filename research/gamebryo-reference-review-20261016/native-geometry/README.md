# Native geometry reference review, 2026-10-16

The evaluation headers help recognize Gamebryo objects, transform operations,
and separate model/world bounds. They do **not** directly supply the missing
Settlement `0x00460940` producer. The inspected native producer consumes
TimeGate gameplay definition shapes. Substituting a Gamebryo sphere radius for
those shape words or for the completed Settlement radius is unsupported.

This is a bounded, read-only reference review after reading
`docs/NATIVE-PORT-RULES.md` and `docs/DEVELOPMENT-09-CHECKPOINT.txt`. Only files
in this review directory were written. No SDK source was copied into the
report or rewrite, no engine binary was executed, and no runtime, oracle,
Cargo, release, original asset, or world-publication behavior was changed.

## Identity and evidence

`member-index.json` records exact ZIP member paths, SHA-256 hashes, byte
lengths, and 1-based line spans for 19 evaluation headers/inlines. Archive:

`E:\Games\Axis and Alies\reconstruction\references\gamebryo-archives-20261016\Gamebryo-1.1.2-Evaluation.zip`

Archive SHA-256:
`171e36a2d6ed8008b9d59711ebcf5ca3d906f7175d5843f821d6f15dc0feaa87`.
`SDK/Win32/Include/NiVersion.h:18-21` reports internal `10.1.0.0`.
This is a version-string match, not a proven ABI/build match or proof about
TimeGate changes. The SDK 1.2 source RAR was not inspected in this geometry
review; no compatibility with that later SDK is assumed.

`native-evidence.json` records both current native image identities, hashes of
seven inspected native function byte ranges and decompiler exports, exact
constant words, static instruction observations, and exported string xrefs.
All seven function ranges are byte-identical between current original AA.exe
and the accepted v14 image. This is static byte comparison, not a new oracle
or arithmetic parity proof. Direct PE disassembly corroborates the central
findings below. The decompiler files' embedded program hash differs from the
current original image; it was not treated as the current image identity.

The CHM member `Documentation/HTML/Gamebryo_1_1.chm` was inventoried and hashed,
but its compressed topic content was not decoded. No documentation-topic
claim is made. Headers/inlines were read directly from the ZIP without
extracting an SDK tree.

## What is established about Settlement

Native-instruction confidence is high for these field accesses and branches;
complete source completion order and numeric results remain unproved here.

* `0x00460A12..0x00460A54`: the first WallSet's `+0x18` and `+0x20`
  references select segment and gate definitions. Definition `+0x38` is
  tested against kinds 2 and 3; those branches read definition `+0x48`, then
  double it. The corner reference at WallSet `+0x1C` supplies definition
  `+0x3C`. These reads are not NiBound member accesses.
* `0x0049DC40` is the native ObjectData loader. Its Size rectangle path reads
  `rect_height` and `rect_width`; `0x0049DE04` puts definition `+0x38` in ECX
  and `0x0049DE0A` calls `0x0046E0E0`. That setter stores width times the
  native binary32 `0.5` at shape `+0x10` (`0x0046E10E..0x0046E114`). Thus
  the Settlement definition `+0x48` read is a gameplay half-width.
* The native circle setter `0x0046D8E0` stores kind 1 and its supplied radius
  at shape `+4`, explaining definition `+0x3C`. The rectangle setter also
  computes a separate radius from the two half-extents. That enclosing
  radius is not the rectangle half-width used by Settlement fitting.
* The WallSet loader region resolves ActorData references and stores segment
  at `0x0047DFBE` (WallSet `+0x18`), corner at `0x0047DFD1` (`+0x1C`), and
  gate at `0x0047DFEA` (`+0x20`). The exported function inventory does not
  contain a reliable enclosing function boundary for this region. Do not
  promote an arbitrary address in it to a function start.
* `0x004640F0` loads Level records and Shape settings, seeds `+0x478` from
  half the configured `wall_diameter`, then calls `0x00460940`. It also
  copies shape words between level wall definitions and placeholder
  definitions. Completion must preserve those shared-definition mutations
  and their order, rather than reread only untouched source templates.
* `0x00460940` executes FSIN/FCOS at `0x00460C18`, `0x00460C1C`,
  `0x00460DDB`, and `0x00460DE7`. Fitting includes binary32 spills, a
  binary64 argument spill at `0x00460C5C`, CRT calls `0x0096D4C0` and
  `0x0096CC4C`, and the completed radius store at `0x00460CE4` to `+0x478`.
  Neither SDK declarations nor ordinary host trig establishes exact x87
  results or exception behavior. The known Unicorn trig limitation still
  applies to future proof work.
* `0x0045E6D0` returns completed `+0x478` for nonpositive Level count;
  otherwise `0x0045E6E0..0x0045E6F5` adds the first WallSet corner
  definition's `+0x3C`, completed `+0x478`, and `0x00A03448` in x87 order.
  The last constant is binary32 `0x400001A3` (about 2.0000999).

Existing source/type leads are `crates/aa-rules/src/objects.rs:12` and `:20`
(`Shape` and native shape setters),
`crates/aa-rules/src/native_object_definition.rs:15` (definition `+0x38`,
`+0x50`, `+0x68` shape roles), and
`crates/aa-spatial/src/geometry.rs:70` (native gameplay placement bounds).
The latter is a separate 2D gameplay operation, not the SDK NiAVObject world
sphere. Existing ports/tests were only read, not revalidated by this review.

## Useful evaluation API leads

These are directly observed evaluation declarations/inline behavior. Their
native-game mappings, offsets, vtable slots, and arithmetic remain candidates
until checked against AA.exe instructions.

| Member and lines | Reference lead | Practical use and limit |
| --- | --- | --- |
| `NiBound.h:25-50,75-77`; `NiBound.inl:17-50` | Sphere with NiPoint3 center and float radius; fitting, transformed update, merge/minimal-bound APIs | Recognize a visual sphere. Fitting/merge bodies are absent here, so their algorithm cannot be inferred from their names. The evaluation default constructor body is empty; do not infer zero initialization. |
| `NiAVObject.h:55-88,229-245`; `NiAVObject.inl:77-187` | Parent, local transform, world transform, world bound, update passes | Distinguish local setters from world accessors. SetTranslate/SetRotate inline behavior writes local members. Evaluation local scale applies absolute value; this is only a candidate signature for native recognition. |
| `NiAVObject.inl:211-215`; `NiNode.h:63-69`; `NiGeometry.h:130` | Empty base world-bound virtual and derived overrides | A base stub cannot stand in for actual node/geometry native dispatch or certify initialized geometry. Full update ordering is not supplied by these declarations. |
| `NiNode.h:38-49,106-120`; `NiNode.inl:28-46` | Children container, attach/detach/index API, array/effective child counts, subtree bound | Useful for recognizing scene traversal and null slots. Do not presume compact children, visitation order, or copy these SDK container limits into Rust. |
| `NiGeometry.h:41-54,137`; `NiGeometry.inl:17-61`; `NiGeometryData.h:65-68,205-213` | Geometry owns a model-data pointer; model-bound access forwards into geometry data | Distinguish mesh model bound from inherited world bound. Vertex access/count are separate. Neither bound is established as Settlement input. |
| `NiTriBasedGeom.h:20,35-51`; `NiTriBasedGeom.inl:17-40` | Inherits NiGeometry; triangle APIs forward to triangle geometry data | Triangle geometry supplies no separate Settlement fitting contract. Its unsigned-short counts describe this legacy API, not new runtime caps. |
| `NiTransform.h:20-33`; `NiTransform.inl:15-28`; `NiPoint3.h:20-23` | Rotation matrix, three-float translation, uniform float scale; transform composition and point application | Useful to label a candidate transform sequence. Evaluation composition scales/rotates child translation then adds parent translation; point application rotates, scales, translates. Native precision, matrix convention, association, and spills still require instructions. |

The exported string-reference inventory gives bounded native searches for
`NiNode` (`0x00AB69E8`, xrefs `0x009E7545`, `0x008C44A5`, `0x008C40CD`),
`NiTriBasedGeom` (`0x00AB6AF8`, xref `0x009E7585`), `NiGeometry`
(`0x00AB6B84`, xref `0x009E75A5`), `NiAVObject` (`0x00AB75BC`, xref
`0x009E7925`), and `NiGeometryData` (`0x00ABCED0`, xref `0x009E8365`).
These are type-name/string xref leads, not identified engine methods or
verified vtables. The absolute inventories are under
`E:\Games\Axis and Alies\reconstruction\research\native-aa\`.

## Scoped next work

For Settlement, bind actual completed WallSet and ObjectData shape words,
then prove `0x004640F0`/`0x00460940` completion, shared mutations, fitting,
and numerical behavior. The SDK does not remove that task. For loaded visual
geometry, use the distinct Gamebryo type/API names to find native field and
virtual-call producers, and prove the reached native update path before
binding world-init services. The current immediate Init/PostCreate,
spatial/footprint/visibility, and publication gaps remain unchanged.
