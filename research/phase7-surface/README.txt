Native target-hit geometry46E150/4EEE60/4EEEE0

Implemented in aa-combat::surface. Source uses native six-word gameplay Shape
from aa-spatial::collision, not a rendered model, inferred bounding box, or mesh
scale. Pure shape_projection preserves native point/default globals, circle
normalization, X f32 store/Y extended box clamp and transform. ground_position
negates input direction, multiplies actor radius with the original X store,
captures facing before target-shape getter, then reads updated actor position.
hit_position captures Z/height ratio before virtual height getter.

1104 native cases:552 shape projections and552 complete actor wrappers;
60 of the wrapper cases mutate facing/position/ratio within getters. Original
virtual radius/height/shape getters are explicit fixture boundaries, not claims
of concrete subclass getters. Scalar radius/height values return x87 double
values; computation remains explicit PC24/53/64 with f32 stores. Square and rect
share the native box branch. Unknown kinds follow native startup defaults.

Native entry functions and math are executed directly, unchanged from original
and accepted v14. ABI assertions apply to the original oracle only. No Rust ABI,
nonfinite/unmasked x87, invalid native pointer, renderer, or live-world claim.
Ordinary target_shape binding is def+68 when override byte+80 is set, otherwise
soft+50; this module requires that actual getter result from the owning registry.

Two Rust tests compare every native output and a failure test confirms getter
mutations remain visible after an error. No mutation rollback is promised.
Commands: python research/phase7-surface/native_surface.py
cargo test -p aa-combat --test native_surface
cargo clippy -p aa-combat --all-targets -- -D warnings
