# Current GameSettings and selected LoadCreator

The Host now retains a source-backed `CreatorLoad` allocation before Reset.
After the actual local Host prefix it installs the selected filename, copies
the selected source Header90 values into a separate mutable Session68 owner,
and assigns that value to the same retained CreatorLoad. The copied source
header, mutable Session68, selected Creator and later target Resource88 are
separate objects. The selected Creator's source-resource50 remains null until
the later actual FinalizeLaunchSettings writer.

The native Host call is75B35F..75B36C, then7791F0 begins at75B371.76B500 first
calls5A46D0 on Session54+14, then5144A0 on the Creator selected by map mode.
Mode2 selects Launch+1C, hence Session+70. It does not copy the appended
Creator88 into the selected launch object. It copies GameSettings parameters
and rules while retaining Creator constructor extents, Biome, weather and seed.

`native_game_settings.py` runs the whole5A46D0 outer procedure:18 cases,
original/v14, PC24/53/64. Scalar and six CString stores, their addresses and
call order are measured. The nested rule/parameter/entry-array assignments
are named successful callback seams; their allocation, exception/reentry and
container algorithms are not certified by this fixture. Separate Creator
author evidence covers5144A0 and constructor/clone arrays.

`native_assignment.py` runs whole76B500 on48 cases across both images/all PCs.
It measures first GameSettings copy, then the actual selected Creator target.
Those two component-copy calls are named seams here and have separate proofs.

The scalar projection includesC/10/14,18/1C, localized20/24 and28/2C,
display30/34, strings38/3C,40 and entries44. Native5A46D0 does not copy derived
MapHeader50..64 or appended Creator metadata. Unobserved source14 and header
localized text companions remain required errors; no empty values are invented.
Existing parameter label companions and raw parameter modified68 bytes are
copied from their actual sealed source owners into independent current storage.

Validation recorded in `rust03.log`: two focused Rust unit tests pass, plus
the actual-source test on tutorial02, Firefight and Berlin. It verifies distinct
arrays and that a later actual Creator name-store cannot mutate Session68 or
source Header90. `windows-source04.log` records a passing actual Windows
QPC/GUID/reset/local-Host/selected-map composition test. This historical run
ends before the later player-refresh work and before the subsequent modified68
companion addition; current final checks are recorded in the later checkpoint.

Preserved unsuccessful builds:rust01 encountered a concurrently written
Creator source getter before its correction;rust02 encountered a foreign
creator-label empty-slice type ambiguity. Neither log certifies the later tree.
No original assets, executable, profiles or releases were modified.
