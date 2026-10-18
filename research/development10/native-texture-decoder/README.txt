Actual Resource textures: CPU decoder and modern GPU allocation

The decoder consumes the same selected TextureInput, preserves alpha and
physical DDS levels, then uses the independently native-proved pixel layout
to select the actual quality-mode mip span. The selected stream remains alive
until the loader's ordered close. Failures retain their decoded prefix on the
same source. Neither decoding nor uploading writes Resource/Task ready flags.

Comparing the original readers caught three standards-decoder differences:
TGA horizontal/alpha descriptor handling, DDS header pitch, and DDS alpha/caps
handling. Current extracted production tests pass all90 defined native pixel
rows plus five basic codec tests. Native grayscale PNG bytes depend on
uninitialized heap contents; modern deterministic grayscale expansion is an
explicit rendering adaptation, not a claim of native byte parity.

Frozen client52 separately passed the actual original-source Resource identity
test and real headless GPU mip/alpha readback. That binary predates the final
native reader corrections. GPU receipt fields retain real texture/view/device
ownership and weak pixel correlation without preventing native CPU discard.
The GPU test covers the upload allocation, not natural battle Source2C use.

See findings.json for exact source/artifact hashes and remaining boundaries.
No playable battle, original D3D9 pixel parity, direct texture reload, or native
PNG resize success is claimed by this checkpoint.
