# Original authored material census

The read-only inventory parsed all **1,847** models named by `research/phase6-material/model-materials.json`. Every original source SHA-256, historical exported mesh ordinal, hierarchy, name and vertex/triangle count matched that audited sidecar. The population contains 11,035 exported ordinary mesh rows, of which the sidecar retains 10,699 and marks 7,113 retained rows as non-team-color meshes. Counts below are mesh rows, not unique property objects or models.

The script is [inventory_original_materials.py](inventory_original_materials.py), and complete counts, first examples, source hashes and limits are in [original-material-census.json](original-material-census.json). The census reuses the existing PyFFI reader and historical static export traversal, including its nearest-LOD selection. It does not execute native rendering or establish parser/native visual equivalence. The population includes effects, cutscenes and marker meshes that happen to be in the model export; “retained” does not mean visible in a particular map or drawn through the ordinary runtime path.

| Authored effective state | All retained meshes | Retained non-team meshes | Meaning for current review |
| --- | ---: | ---: | --- |
| Texture apply=MODULATE (2) | 10,503 | 6,919 | Current ordinary shader assumes modulation |
| Texture apply=REPLACE (0) | 58 | 58 | Apply-mode data is actually exposed and discarded |
| No texturing property | 138 | 136 | No invented map/default included in the count |
| Base map only | 10,559 | 6,975 | Dominant authored map combination |
| Base plus glow map | 2 | 2 | Non-base map input is exposed and discarded |
| Base clamp both axes (0) | 941 | 285 | Current fixed repeat addressing differs from retained authored metadata |
| Base wrap both axes (3) | 9,620 | 6,692 | Dominant addressing mode |
| Base NEAREST, no mipmaps (0) | 1 | 1 | Current fixed linear/trilinear sampler cannot represent this authored choice |
| Base BILERP, no mipmaps (1) | 821 | 165 | Current fixed mip-enabled sampler discards this distinction |
| Base TRILERP (2) | 9,739 | 6,811 | Dominant filter choice; exact mip/filter parity still unproven |
| Base map with a texture-transform descriptor | 7 | 5 | Descriptor presence alone does not prove a visible static transform |
| Transform with identity static components | 6 | 4 | No static transform effect established |
| Transform with nonidentity static components | 1 | 1 | Raw V translation 1, with wrap addressing; visual impact is not established |
| Authored vertex-color array present | 5,920 | 2,754 | Array is dropped by ordinary export, but source-routing controls whether it is used |
| Effective source routing=IGNORE (0) | 10,310 | 6,930 | Every **present** effective routing property in this population selects IGNORE |
| Effective source-routing property absent | 389 | 183 | Native default still needs its own evidence |
| Lighting=E/A/D (1) | 10,237 | 6,857 | Current fixed illumination is not complete native E/A/D proof |
| Lighting=emissive-only (0) | 73 | 73 | A concrete mode combination that the ordinary material schema drops |
| Effective stencil draw=BOTH (3) | 5,660 | 3,393 | Current Boolean can represent this observed authored value |
| Effective stencil property absent | 5,039 | 3,720 | Default-mode native rendering is a separate question |
| Effective stencil enabled | 0 | 0 | No active stencil-buffer use found in this population |
| Effective shade property present | 0 | 0 | No authored flat-shading case found in this population |
| Material ambient is not white | 10,573 | 6,989 | Authored ambient is not exported; nonwhite is not evidence it is visibly used |
| Material specular is nonzero | 3,017 | 2,256 | Authored specular is not exported; native enable/lighting path matters |
| Material emissive is nonzero | 680 | 680 | Emissive RGB is already retained, but lighting/apply order needs proof |

No retained ordinary exported mesh in this census has a dark, detail, gloss, bump, decal or shader map, a second UV stream, an authored base UV index other than zero, an explicit CCW-only/CW-only draw mode, or an effective source-color route other than IGNORE. These remain representation gaps, but the census does **not** establish a currently affected retained asset for those cases. All 10,699 retained rows have an effective material property, so the converter's white missing-material fallback is not exposed by this population either. Full original NIFs, discarded LODs, particles, external effects and runtime material changes are outside these negative results.

## Concrete original examples

Each example below is present in the retained non-team group. Full source SHA-256, hierarchy and source-node ordinal are available in the JSON; mesh index uses the existing converted-mesh order.

| Lead | Model / mesh | What the census establishes |
| --- | --- | --- |
| Nearest sampling | `Buildings/Campaign/IwoJimaBeachBunkerMG01/IwoJimaBeachBunkerMG01.nif`, mesh 0 `Scene Root/Plane04` | Filter 0; source SHA-256 `1bd9f840c83a65f3259d12a47b575db16540bc1a777e758b81e0e12991f30008` |
| Bilinear without mipmaps | `Effects/HealthBar/HealthBar.NIF`, mesh 0 `Scene Root/Healthbar/Editable Mesh` | Filter 1; source SHA-256 `240a7ebdd4cda38e69d4ab2fdd65e2a3b56690e9e073040bfb5c6fb7f70432f1` |
| Clamp both axes | `Effects/SpecialOps/AssemblyLine/AssemblyLine.nif`, mesh 1 `Scene Root/FXDummy/FX3/Plane` | Clamp 0; source SHA-256 `8b1d27d3c91816bbf33d765bd680c6ef68090750721d7795bb07a638af24aa78` |
| Replace apply mode | `Cutscene/Demo1/Demo1.nif`, mesh 10 `Scene Root/beam/Editable Mesh` | Apply 0; source SHA-256 `96d6656fa6c7979195b54ce4be90b6ef9469a0b22f3db307f00ccd202e1031e0` |
| Glow input | `Special/Things/ThingStartLocation/ThingStartLocation.nif`, mesh 2 `Scene Root/Object01` | Base+glow slots; source SHA-256 `8cb0ec533d813e90caa8fb8f3bd14587f67a7de01e36d1b68a0a7ee0c107ac1a` |
| Transform descriptor | `Buildings/Campaign/ChaparralAirfield/ChaparralAirfield.nif`, mesh 4 `Scene Root/lights` | Descriptor present; source SHA-256 `3bf31b965e6bd4d04c39537c964389a2d17c6432943bd2006d5bc8d988f2412b`; this example has identity static components, so it is not proof of a static UV mismatch |
| Emissive-only mode | `Effects/Explosions/Temperate/Medium/TemperateMed_01.NIF`, mesh 5 `Scene Root/Editable Mesh` | Lighting 0 with source routing IGNORE; source SHA-256 `8b618ab553f4816728268593107e20dedbaaa63180e1515df21759aadb1bbe95` |
| Authored colors | `Buildings/Campaign/AmmoBunker/AmmoBunker.nif`, mesh 0 `Scene Root/ResouceAmmo` | Authored color array present; source SHA-256 `85c248d97e678d9b95a0c7f753e8b881f64f7d9b6836e3c45c2bcd1b984ec72b`; color-array presence alone is not proof the original used it |

Material alpha is also not confined to [0,1] in authored input: examples include `Effects/MortarShot/MortarShot.nif` at -4 and `Effects/TransportUnits/RUSTransportTruckSolid.nif` at 2. The SDK material documentation describes the expected range but says it is not error-checked. Preserve these raw observations and validate native packing/clamping before adding a blanket import clamp. They do not supersede the existing rule that material opacity alone must not enable blending when authoritative alpha flags are present.

Six of the seven retained texture-transform descriptors have translation (0,0), scale (1,1), rotation 0, method 1 and center (0.5,0.5). The seventh is `Effects/Waypoints/WaypointUnpack.nif`, mesh 0 `Scene Root/Arrow01`, source SHA-256 `480197c1e7d4439a7ca373609169b37cfccd4d2e4c3077b8e7c5d1f61adafb07`: it differs only by translation (0,1), with wrap/wrap and trilinear sampling. Its nonidentity components alone do not establish a visible static mismatch because whole-period translation can be equivalent under wrapping. Animated texture-transform controllers and actual native matrix/UV behavior remain separate validation targets.

## Priority implication

The strongest concrete follow-up is authored **sampler state**, followed by the exposed replace-apply and emissive-only modes through their actual AA runtime paths. The single nearest-filter bunker mesh supplies a building example that does not depend on a cutscene/particle route. Optional glow has two known marker inputs. Merely retaining the thousands of dropped vertex colors is not a justified visual fix while every observed explicit route ignores them. No observed CW-only, flat-shade or active stencil case supports prioritizing a broad culling/stencil rewrite this turn.
