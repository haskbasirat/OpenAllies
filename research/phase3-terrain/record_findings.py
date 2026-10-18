"""Record narrowly proven terrain changes and unresolved rendering boundaries."""
from pathlib import Path
import importlib.util
import json

HERE = Path(__file__).parent
spec = importlib.util.spec_from_file_location('terrain_uv_oracle', HERE / 'terrain_uv_oracle.py')
uv = importlib.util.module_from_spec(spec)
spec.loader.exec_module(uv)
original = uv.Pe(uv.GAME / 'AA.exe', uv.ORIGINAL_SHA)
accepted = uv.Pe(uv.GAME / 'reconstruction/native-game/runtime-expanded-v14/AA.exe', uv.ACCEPTED_SHA)


def pin(path):
    path = Path(path)
    return dict(path=str(path.relative_to(uv.GAME)).replace('\\', '/'), size=path.stat().st_size,
                sha256=uv.sha(path.read_bytes()))


windows = {}
for name, start, end in [
    ('packed_cell_load', 0x557ce5, 0x557f04),
    ('cell_type_passability', 0x54fdc0, 0x54fec0),
    ('blend_neighborhood_choice', 0x729470, 0x729667),
    ('blend_layer_masks', 0x729760, 0x7299d0),
    ('blend_priority_compare', 0x729230, 0x729258),
    ('blend_texture_matrix_table', 0x729180, 0x729224),
    ('blend_matrix_constants', 0xb28720, 0xb28a20),
    ('terrain_loader', 0x4b87a0, 0x4b8e77),
    ('water_depth_info_loader', 0x4b8530, 0x4b86cf),
]:
    data = original.at(start, end - start)
    assert data == accepted.at(start, end - start), name
    windows[name] = dict(start=hex(start), end_exclusive=hex(end), sha256=uv.sha(data), bytes=data.hex())

report = dict(
    schema=1,
    status='CPU UV correction implemented and tested; full native terrain renderer remains incomplete',
    original_sha256=uv.ORIGINAL_SHA, accepted_sha256=uv.ACCEPTED_SHA,
    changes=[
        dict(path='rust-rebuild/crates/aa-assets/src/terrain.rs',
             behavior='Source UV is now [worldX/texture_width,1-worldY/texture_height]. Native fmod removes whole repeats per block; direct whole-map coordinates have the same repeat sampling and remain continuous across cells.',
             evidence='terrain-uv-native-fixtures.json', tests='34 original-instruction cases /136 corners, including9 actual palette/cell examples from3 original maps. Rust source_uv matches within2e-5 modulo integer repeats. This is not bit-exact x87 parity.'),
        dict(path='rust-rebuild/crates/aa-assets/src/terrain.rs',
             behavior='Textured vertices use white modulation. Untextured inspection fallback keeps the authored minimap color.',
             evidence='Original0x729F30 normal branch. Client loading.rs already forced white before this change, so this API cleanup does not claim a newly visible color fix.'),
    ],
    integration=dict(
        client_api_change=False,
        instructions=['Keep using PreparedTerrain.vertices UVs directly, without model_uv conversion.',
                      'Keep U/V Repeat addressing; current gpu.rs sampler already does this.',
                      'Existing loading.rs white-color safeguard is redundant but compatible.',
                      'No change to mesh dimensions, height policy, indices, placement transforms or material sidecar.'],
        invalid_claims=['No exact native water, blending, terrain lighting, edge/skirt mesh or texture-filtering parity yet.',
                        'No native GPU execution or screenshots performed by this subtask.']),
    packed_terrain_record=dict(
        scope='Version7, stride8 loader0x557AD0. The old extractor called the dropped bytes tile orientation flags without native evidence.',
        fields=[
            dict(offset=0,size=2,meaning='Terrain-variation table index',consumer='0x557D1D movzx word[record+0]'),
            dict(offset=2,size=2,meaning='Opaque serialized cell-state word; loaded conditionally when stream mode allows, not established as render orientation',consumer='0x557E84 ->0x54EA50 writes lowword tile+0x14'),
            dict(offset=4,size=1,meaning='Index into GMap+0x50 objects, copied to cell+0x2C; pre-v5 maps default this to1 for water type',consumer='0x557E98..0x557EC8'),
            dict(offset=6,size=1,meaning='Cell flags copied to runtime+0x20; type bits4..6 are then replaced from terrain definition. Bit0 participates in passability recalculation.',consumer='0x557D12/0x557D1A,0x557E50..64 and0x54FDC0'),
        ],
        conclusion='Do not reinterpret record secondword as rotation/mirroring. No per-cell texture rotation established or implemented.'),
    blending=dict(
        scope='Source-grounded plan only, not a complete executed blend port.',
        evidence=[
            '0x4B87A0 assigns variation+0x18 priority = terrain.base_priority + variation append ordinal.',
            '0x729470 chooses the lower-priority terrain among the center and relevant edge/corner neighbor cells.',
            '0x729760 collects unique3x3 choices, encodes the eight noncenter neighbors as an8-bit mask, sorts layers by ascending priority using0x729230, closes specific corner gaps, and selects mask resources.',
            '0x72B160 loads textures/AlphaTransitions/Layer0..8.tga from AVars;0x729180 provides identity/rotation/reflection matrices for alpha-mask orientation.',
            '0x72A280 applies separate source texture and alpha texture transforms. It uses native material blend-state fields; this is not an unproven average of neighboring colors.'
        ],
        future_shape='A separate ordered per-cell TerrainLayer list would need palette_index, exact native mask identity/transform and source UVs; client would use a second UV/mask sampler and proved blend state, or bake an equivalent target. Current single-palette cell batches cannot represent it.',
        unresolved=['Full mask resource selection/catalog and ownership', 'GPU alpha equation/filtering/edge sampling', 'World-block subdivision and neighboring-block seams']),
    water=dict(
        scope='Metadata/source findings only; no guessed water surface or opacity formula implemented.',
        evidence=['0x4B87A0 type2 loads max_depth+0x70,anim_fps+0x74,color+0x78, animation frames+0x94/+0x98 and water doodad fields.',
                  '0x4B8530 reads WaterInfo.terrain_ids, ordered DepthInfo rows(depth,RGB,alpha), appends20-byte records and sorts by depth.',
                  'Templates/template_waterinfo_aa.tgi contains authored curves; base terrain textures alone are not the complete water material.'],
        unresolved=['Water surface height/geometry generation', 'Depth-curve interpolation and material composition', 'Animation frame selection/time, reflection/specular/doodads']),
    static_instruction_windows=windows,
    evidence=[pin(HERE / 'terrain_uv_oracle.py'), pin(HERE / 'terrain-uv-native-fixtures.json'),
              pin(HERE / 'terrain-uv-native-assembly.txt'),
              pin(uv.GAME / 'reconstruction/assets/original/Data/Templates/template_terrain.tgi'),
              pin(uv.GAME / 'reconstruction/assets/original/Data/Templates/template_waterinfo_aa.tgi'),
              pin(uv.GAME / 'reconstruction/assets/original/Data/AVars.tgi')],
    implementation=[pin(uv.ROOT / p) for p in ['crates/aa-assets/src/terrain.rs', 'crates/aa-assets/src/types.rs', 'crates/aa-assets/tests/assets.rs']],
    checks=[dict(command='python research/phase3-terrain/terrain_uv_oracle.py',passed=True,cases=34),
            dict(command='cargo test -p aa-assets',passed=True,tests=8,ignored=1),
            dict(command='AA_ASSET_ROOT=<reconstruction> cargo test -p aa-assets --test assets recovered_assets_and_path_safety -- --ignored',passed=True,tests=1),
            dict(command='cargo clippy -p aa-assets --all-targets -- -D warnings',passed=True)],
)
(HERE / 'findings.json').write_text(json.dumps(report, indent=2) + '\n')
print(json.dumps({'findings_sha256': uv.sha((HERE / 'findings.json').read_bytes())}))
