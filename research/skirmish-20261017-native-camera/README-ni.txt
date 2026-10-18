NiCamera child arithmetic: bounded completed port

native_camera::ni::update_6fe050 mutates the same camera State. The concrete
null-parent/null-attachment path produces the world transform, direction basis,
six frustum planes, view/projection matrix, position and four frustum corners.
Positive parent45CAB0 and attached virtual38 calls remain required services,
with their actual mutation prefixes tested. RequiredChildren is suitable only
when the actual constructor-owned fields establish those null branches.

Proof: native-camera-ni.json has 150 paired rows / 300 separate original-v14
executions across all three x87 precisions. It covers arbitrary finite bases,
perspective/orthographic frusta and required-service stops. The fresh default
native_camera_ni.py replay is read-only and passed. Three owned Rust tests pass,
including the full Ni output slices for the existing 180 native pose rows.
All six combined parent/pose/NI tests pass. The scoped production library check
also passed. ni-findings.json records hashes, logs and exact proof limits;
ni-source-snapshot and frozen-ni-tests.exe retain the accepted source and binary.

This is not a constructor, device, VWorld readiness or playable battle proof.
Arithmetic equivalence is certified for the finite nondegenerate input domain;
exceptional nonfinite/overflow/status-register behavior and arbitrary private
numeric-operation failure injection are not claimed. The new direct fixture
does not use the existing pose oracle's declared hardware trig transport.
