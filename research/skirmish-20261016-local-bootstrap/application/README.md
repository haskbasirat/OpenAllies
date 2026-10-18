# Retained GameAppState

`application_mode.rs` ports the allocation actually returned by40D960:
Application+FC, installed by40CB40 from651810. It is distinct from SessionE4,
SessionF8, MapResourceA8 and the saved World mode. The same owner must pass
through the real784220 launch-notification transition before mission start.

The native constructor projection initializes base4=0, changed8=false,
modeC=1 and10/11=false.651840 stores mode first, then lazily tests and posts
both optional manager resets, then sets changed8. A service failure preserves
the prior stores. This does not infer absent managers or provide no-op posts.

`native_application_mode.py` runs270 original/v14/all-three-precision rows.
The two Rust tests replay the native constructor/getter/setter and partial
error ordering; `rust-tests.log` passed. Successful allocation, manager-post
seams and no arbitrary callback reentry are the qualified domain. This proof
does not certify that the current client has executed784220.
