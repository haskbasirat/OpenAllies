5A94F0 ComputeSelectionCuboid: no-SelectionBox BoundingSphere fallback
=======================================================================

Blocker
-------
Berlin's ResourceTask queue stopped at the first model without a node named
by the Bone "selection_box" CString (TemperateTree04.nif):
"5A94F0 actual BoundingSphere selection fallback/diagnostic child is required"
(crates/aa-client/src/model_payload_backend.rs selection_cuboid).

Native 5A94F0 (original = v14 bytes)
------------------------------------
5A9524 42C070([B36530]+580, "selection_box") -> entry+1C name CString (an
absent entry uses the empty CString 9F611E). 5A955C root vf40
GetObjectByName(name). 5A95A0 keeps it only if its RTTI chain reaches
B43CF0 "NiGeometry" (base B43E10). 5A95C4 root vf48 Update(0,0).
Kept NiGeometry: 5A96B2 vertex extents (already ported, earlier oracle) and
6FF850 unlink. Otherwise (5A95CF):
  if Resource flags+8 bit11: 404ED0(B38C28, NiResource.cpp, 0x56F,
    "KKC_NiAVObjResource::ComputeSelectionCuboid",
    "Object model has no selection box: %s", +38 name) (404ED0 = level 1)
  r' = root world bound radius (+34) * [A4635C] 0.4f, extended
  +4C/+50/+54 = c - r', +58/+5C/+60 = r' + c (fstp m32)
  if (cx-r')(r'+cx) > [9F32DC] +0.0, or (only then) (cy-r')(r'+cy) > +0.0:
    404ED0 0x57D "Bounding sphere for art is incorrect: %s"
  (both comparisons at the current PC, unrounded; the cuboid is stored
  before the check and kept either way).
The root world bound (+28..+34) is the one produced by that Update(0,0).

Port
----
aa-runtime resource_model_payload.rs: Selection {Geometry, RootSphere} and
  Model::selection_5a94f0 (default: the old geometry-only selection_cuboid).
aa-runtime resource_model_bounds.rs: selection_from_sphere (exact x87 order).
aa-runtime model_resource_load.rs: 5A94F0 fallback/diagnostic order,
  Backend::selection_diagnostic_404ed0, SelectionDiagnostic constants.
aa-runtime resource_depot.rs: Resources::model_selection_cuboid_4c (read).
aa-client model_selection.rs (new, child of model_payload_backend): lookup,
  NiGeometry RTTI classification (NiTriShape/NiTriStrips yes; *Node kinds
  no; other kinds explicit), Update(0,0), RootSphere or geometry + unlink.
  model_payload_backend.rs: two lines (module + trait method).
aa-client battle_resource_tasks.rs: 404ED0 sink (log::warn, native text).
The root bound is the port's 86E8D0 update (model developer's code); it
refuses skinned graphs and node kinds other than NiNode/NiTriShape/
NiTriStrips, so those models stop there explicitly.

Proof
-----
native_selection_fallback.py (default: immutable replay; --write):
Berlin's 30 queued no-SelectionBox models (berlin-no-selection-models.json);
15 are in the port update domain (TemperateTree04, TemperateRock1-5,
TemperateLog05, Boat01-04, StreetLight01, WesternEuropeCity01, LargeDoor,
ThingStartLocation). Their root bounds are computed natively (45CAB0
compose, 89FBF0 sphere transform, 89FAD0 merge; traversal is transport,
the same rule the port update uses), then 5A94F0 executes from its entry with
those words; 11 scalar edge spheres add the incorrect-sphere, zero, signed-
zero, tiny, rounding and overflow cases, flags bit11 on/off and a named
non-NiGeometry object (real RTTI walk). 189 rows x 2 images = 378 machines.
Rust: resource_model_bounds::tests::original_selection_sphere_fallback_and_
diagnostics_all_precision_modes (all rows; overflow-to-infinity rows refuse
explicitly) and aa-client model_selection::tests::berlin_no_selection_models_
root_bound_and_fallback_match_native_all_precisions (AA_GAME_ROOT: real
source -> port graph -> port root bound == native composition at all PCs,
fallback cuboid == native, whole 5A99C0 kernel stores the same words).

Berlin GUI census (selection01, exe 21ea468e..., 900 s)
---------------------------------------------------------
TemperateTree04 and the following loads now complete: 18 tasks loaded
(TemperatePineTree01-03, TemperateTree01-13, both tree shadow DDS). No
5A94F0 diagnostic was logged. The next stop is the 19th task,
TemperateHighDryGrass.kf: "required native service:5A5520 actual same
Sequence payload/KF decoder" (Sequence vf4C, battle_resource_tasks.rs).

Real-model test result: 12 of the 15 in-domain models reproduce the native
root bound and fallback at all PCs and through the whole 5A99C0 kernel; 3
refuse earlier in the model backend (TemperateLog05: "NIF descriptor
filename differs from its shared SourceTexture owner"; WesternEuropeCity01
and LargeDoor: "8790B0 nonempty source StringExtraData conversion is
required").

Open gaps
---------
- Skinned and NiBillboardNode/particle graphs (15 of the 30) depend on the
  model developer's update domain; they refuse there, not here.
- The 42C070-absent (empty CString) branch stays an explicit error.
- NiGeometry kinds other than NiTriShape/NiTriStrips named as selection
  nodes stay explicit.
