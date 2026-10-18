# Original executable quaternion type 3 source scout

This is a read-only source/instruction audit, not an executable oracle or a Rust parity claim. No native helper was executed by this scout. All addresses below refer to original `AA.exe`. The recorded byte ranges also match the accepted v14 image. No SDK source was read.

## Wire and ownership

The generic rotation sampler `862170` dispatches interpolation type 3 through `B45CF8 + 3*4 = B45D04`. Registration `8656C0` writes evaluator `865050`, loader `8655B0` into `B45DF4`, and derived-control initializer `865300` into `B458CC`.

`8655B0(stream,count)` allocates `count*0x44+4`, stores the allocation count in the preceding DWORD, and returns the first key. The per-key in-memory layout is:

| Offset | Meaning | Serialized? |
| --- | --- | --- |
| `00` | Float32 time | Yes |
| `04` | Byte stride, `44`; other bytes are padding | No |
| `08,0C,10,14` | Quaternion W, X, Y, Z | Yes |
| `18,1C,20` | Tension, continuity, bias | Yes |
| `24..30` | Outgoing control quaternion, WXYZ | Derived |
| `34..40` | Incoming control quaternion, WXYZ | Derived |

Thus each serialized type-3 key is eight Float32s: time, W, X, Y, Z, T, C, B, with no serialized stride or derived controls. `8620E0` reads time and WXYZ using `8B7C80` and `871530`; the loader then performs three individual four-byte reads in T/C/B order. It constructs the value quaternion as zero before loading; it does not construct or load the two derived controls. Do not read uninitialized controls for a singleton.

`865300(keys,count)` does nothing when `count < 2`. Otherwise, in order:

1. Run `862490` on the complete array.
2. Build key 0 with previous key 0 and next key 1.
3. Build each interior key i in increasing order with previous i-1 and next i+1.
4. Build the last key with previous count-2 and next last.

`862490` first visits consecutive pairs in increasing order. If their WXYZ dot product is strictly negative, it negates all four components of the later key in place. The dot is reduced Z, then Y, then X, then W at the selected x87 precision, without a Float32 sum spill. Only after this complete sign pass does it clamp every key's W to [-1,1]. It does not normalize quaternion lengths or clamp X/Y/Z.

## Primitive ABI and arithmetic

The following describe finite ordinary inputs. NaN/exception flags need a separate qualification.

- `870F10`: cdecl `(out,q)`, returns out in EAX. Writes W,-X,-Y,-Z.
- `870E80`: thiscall `ECX=left`, stack `(out,right)`, callee pops 8. Hamilton left*right; returns out. Each component is stored as Float32.
- `870DF0`: thiscall `ECX=left`, stack `(out,right)`, callee pops 8. Component addition, machine instruction order `right + left`.
- `870E20`: same ABI, component `left - right`.
- `8B7DD0`: cdecl `(out,Float32 scale,q)`, returns out. Multiplies every component by scale, stores Float32. Despite the decompiler's CRect type, there is no float-to-integer conversion.
- `870FB0`: cdecl `(out,q)`, returns out. Quaternion log described below.
- `8B7EE0`: cdecl `(out,q)`, returns out. Quaternion exp described below.
- `8B7F80`: cdecl `(out,Float32 t,left,right)`, returns out. Polynomial normalized blend described below.
- `870F40`: cdecl `(out,Float32 t,qLeft,cLeft,cRight,qRight)`, returns out. Three-stage SQUAD composition below.
- `8650A0`: thiscall `ECX=currentKey`, stack `(previousKey,nextKey)`, callee pops 8. Stores current key's two derived controls.
- `865050`: cdecl `(Float32 u,leftKey,rightKey,out)`, invokes `870F40(out,u,left.q,left.outgoing,right.incoming,right.q)`.

For the Hamilton product `a*b`, the actual reduction order is:

```
W = ((bW*aW - bX*aX) - aY*bY) - aZ*bZ
X = ((aX*bW + bZ*aY) + bX*aW) - aZ*bY
Y = ((aZ*bX + aW*bY) + aY*bW) - bZ*aX
Z = ((bZ*aW + aX*bY) + aZ*bW) - aY*bX
```

The machine computes Z, Y, X, W and then stores W, X, Y, Z as Float32. Retain x87 precision during each reduction. Reassociation can change bits.

## Log and exp

`870FB0` sets angle to pi when `W <= -1`, to `acos(W)` when `-1 < W < 1`, and to zero when `W >= 1`. The acos helper is actual CRT `96DBB0`; do not replace it in the native oracle. It evaluates `s = sin(angle)`, then uses `angle/s` when `abs(s) >= Float32(0.001)` and 1 when below that threshold. Output is `(positiveZero, coefficient*X, coefficient*Y, coefficient*Z)`, stored as Float32. It does not length-normalize its input or output.

Pi is the actual mutable global `B43E5C`, constructed by startup initializer `9E7960`: `Float32(4*atan(1.0))` via native FPATAN. That transcendental is subject to the existing Unicorn precision qualification. Ordinary hemisphere-continuous unit input can avoid the W<=-1 log fallback, but the global must not be silently supplied as zero if that branch is included.

`8B7EE0` ignores input W. It computes `length = sqrt((X*X + Y*Y) + Z*Z)` in x87 precision, writes `Float32(cos(length))` for output W, and computes `s = sin(length)`. The vector coefficient is `s/length` when `abs(s) >= Float32(0.001)` and 1 below that threshold. Notice that the comparison is on sin(length), not length. Output X/Y/Z are coefficient times input X/Y/Z, stored as Float32. There is no separate normalization.

The threshold is address `AB760C`, bits `3A83126F`. Equality takes the division branch in both helpers. FSIN/FCOS are executed by the original helper and have the existing Unicorn transcendental limitation.

## Derived TCB controls

Let `q`, `p`, `n` be the current, previous, and next quaternions, and `t`, `tp`, `tn` their times. The constructor computes these full quaternion temporaries through the native primitives:

```
M = log(conjugate(p) * q)
P = log(conjugate(q) * n)
```

It independently stores the following scalar temporaries as Float32:

```
inv = 1 / (tn - tp)
A = 1 - T
Cm = 1 - C
Cp = C + 1
Bm = 1 - B
Bp = B + 1
rL = (t - tp) * inv
```

Each following coefficient is multiplied in the displayed order, retained at the chosen x87 precision, then stored as Float32 for the scale call. Each quaternion scale, add, subtract, exp, and product produces Float32 component stores.

```
SM = scale(M, ((rL * Bp) * Cp) * A)
SP = scale(P, ((rL * Bm) * Cm) * A)
sum = SP + SM
correction = scale(sum - P, 0.5)
outgoing = q * exp(correction)             // key+24

rR = Float32((tn - t) * inv)
SM = scale(M, ((rR * Bp) * Cm) * A)
SP = scale(P, ((rR * Bm) * Cp) * A)
sum = SP + SM
correction = scale(M - sum, 0.5)
incoming = q * exp(correction)             // key+34
```

The first control is fully built and stored before the second coefficient sequence begins. Endpoint duplication is deliberate: for a zero-T/C/B first key, outgoing is q*exp(-0.5*P); it produces a flat endpoint tangent in the later SQUAD composition. Do not substitute a copied endpoint control. Nonuniform times enter through rL/rR, not through a generic component Hermite formula.

## Polynomial normalized blend and SQUAD

`8B7F80` does not perform per-call shortest-arc sign correction. The sign fix belongs to the earlier key-array precomputation. It stores the dot product as Float32 with reduction order:

```
d = Float32(((leftY*rightY + leftX*rightX) + leftW*rightW)
            + rightZ*leftZ)
k = 1 - d * Float32Bits(3F52A2D1)
h(v) = (1 + (k*k * Float32Bits(3F15E2D1))
              * (((v+v) - 3)*v + 1)) * v
weight = h(t)                 when t <= 0.5
weight = 1 - h(1-t)           when t > 0.5
```

The polynomial weight is retained in x87 precision. It constructs each quaternion component as `Float32((right-left)*weight + left)`, then runs `8B7E20` in place. Do not use a standard math-library SLERP or normalization.

`8B7E20` stores `s = Float32(((W*W + X*X) + Y*Y) + Z*Z)`. With runtime globals `c0=B45E38` and `c1=B45E3C`:

```
r = (s - Float32Bits(3F758559)) * c1 + c0
if s <= Float32Bits(3F6A4B55):
    r = ((r*r*s - Float32Bits(3F758559))*c1 + c0) * r
    if s <= Float32Bits(3F26F151):
        r = ((r*r*s - Float32Bits(3F758559))*c1 + c0) * r
```

It multiplies X, Y, Z, then W by the retained r, storing each as Float32. No sqrt normalization, zero-length fallback, or input rejection is present in this helper. Equality takes the refinement branch.

These globals are BSS and require actual startup initializers in the oracle:

```
9E8380:
  B45E38 = Float32(Float32Bits(3F800A31) / sqrt(Float64Bits(3FEEB0AB20000000)))
9E83A0:
  B45E3C = Float32((Float32Bits(BF000000)
                   / (sqrt(Float64Bits(3FEEB0AB20000000)) * Float32Bits(3F758559)))
                   * Float32Bits(3F800A31))
```

The constants are actual PE data; no guessed value is required. Parent independently reports actual initializer outputs `3F82BE63` and `BF0852F1` at all three precisions; this scout has not executed that check.

`870F40` first computes the controls blend, then the key blend, then the final blend, in that order:

```
controls = blend(u, cLeft, cRight)
keys = blend(u, qLeft, qRight)
v = Float32(((1-u) * u) + ((1-u) * u))
result = blend(v, keys, controls)
```

In the final v computation the product remains in x87, is doubled using FADD ST(0),ST(0), then spills once as Float32. The notation above conveys the value, not two independently rounded products.

## Smallest useful independent oracle domain

Run actual original and v14 `9E8380`, `9E83A0`, `8656C0`; supply only heap and byte-stream transport seams. Load two-, three-, and four-key type-3 arrays with `8655B0`, run actual `865300`, and evaluate through `862170` so the registry table, normalized interval, and cursor mutation are included. Record loaded WXYZ/T/C/B, both derived controls, output WXYZ, stream-read sizes, and cursor. Every stream key should perform eight four-byte reads.

Start with finite unit quaternions, increasing finite times, modest rotations, finite T/C/B in [-0.5,0.5], valid initial cursor 0, and query times inside the closed first-to-last key interval. Include unequal time gaps, noncommuting X/Y/Z rotations, each nonzero T/C/B individually, negative-hemisphere representations, two-key duplicated endpoints, singleton direct-copy behavior, and u below/equal/above 0.5. Identity and small-angle rows reach the sin-threshold fallback. Threshold-adjacent log/exp and normalizer-refinement rows can be separate primitive cases. Do not certify empty arrays, invalid cursors, repeated times, out-of-range times, nonfinite values, exception flags, or full controller/NIF lifecycle from this bounded domain.

This scout has only inspected files and PE bytes. Native oracle execution, exact fixture generation, actual asset coverage, and any Rust implementation remain with the parent and implementation owner.
