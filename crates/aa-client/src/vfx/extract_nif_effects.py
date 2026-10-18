"""Extract the original Gamebryo effect NIFs (Data/Effects) into the compact
animation/particle table used by the Rust VFX player (nif_effects.json).

The converted mesh JSON only keeps an authored static pose, and most effect
NIFs are particle systems that convert to nothing. This tool keeps what the
player needs: node hierarchy with keyframe tracks, billboard nodes, sprite
geometry (quads in node space) with alpha/colour/visibility/flip controllers,
and NiPSys particle emitters with their modifiers. Coordinates stay in the
NIF's Z-up space; the runtime converts with (x, z, -y).

Usage (PyFFI from the reconstruction tools vendor folder):
    python -I extract_nif_effects.py <reconstruction root>
"""
from __future__ import annotations

import json
import math
import re
import sys
from pathlib import Path

ROOT = Path(sys.argv[1] if len(sys.argv) > 1 else r"E:/Games/Axis and Alies/reconstruction")
sys.path.insert(0, str(ROOT / 'tools'))
import convert_models as cm  # noqa: E402  (patches PyFFI for legacy NIFs)

N = cm.NifFormat
DATA = cm.ORIGINAL
HERE = Path(__file__).resolve().parent
OUT = HERE / 'nif_effects.json'
TEX_OUT = HERE / 'textures'


def r(v, n=5):
    v = float(v)
    if not math.isfinite(v):
        v = 3.0e38 if v > 0 else -3.0e38
    return round(v, n)


def s(v):
    return v.decode('latin1') if isinstance(v, bytes) else str(v)


def vec3(v):
    return [r(v.x), r(v.y), r(v.z)]


def rows(m):
    """pyffi Matrix33 (row-vector convention) -> column-vector matrix rows."""
    a = [[m.m_11, m.m_12, m.m_13], [m.m_21, m.m_22, m.m_23], [m.m_31, m.m_32, m.m_33]]
    # p' = p * A (row vector) == A^T p (column vector)
    return [[a[j][i] for j in range(3)] for i in range(3)]


def quat_to_cols(w, x, y, z):
    # standard column-vector rotation matrix of a unit quaternion
    return [[1 - 2 * (y * y + z * z), 2 * (x * y - w * z), 2 * (x * z + w * y)],
            [2 * (x * y + w * z), 1 - 2 * (x * x + z * z), 2 * (y * z - w * x)],
            [2 * (x * z - w * y), 2 * (y * z + w * x), 1 - 2 * (x * x + y * y)]]


def mat_to_quat(m):
    tr = m[0][0] + m[1][1] + m[2][2]
    if tr > 0:
        S = math.sqrt(tr + 1.0) * 2
        w = 0.25 * S
        x = (m[2][1] - m[1][2]) / S
        y = (m[0][2] - m[2][0]) / S
        z = (m[1][0] - m[0][1]) / S
    elif m[0][0] > m[1][1] and m[0][0] > m[2][2]:
        S = math.sqrt(1.0 + m[0][0] - m[1][1] - m[2][2]) * 2
        w = (m[2][1] - m[1][2]) / S
        x = 0.25 * S
        y = (m[0][1] + m[1][0]) / S
        z = (m[0][2] + m[2][0]) / S
    elif m[1][1] > m[2][2]:
        S = math.sqrt(1.0 + m[1][1] - m[0][0] - m[2][2]) * 2
        w = (m[0][2] - m[2][0]) / S
        x = (m[0][1] + m[1][0]) / S
        y = 0.25 * S
        z = (m[1][2] + m[2][1]) / S
    else:
        S = math.sqrt(1.0 + m[2][2] - m[0][0] - m[1][1]) * 2
        w = (m[1][0] - m[0][1]) / S
        x = (m[0][2] + m[2][0]) / S
        y = (m[1][2] + m[2][1]) / S
        z = 0.25 * S
    n = math.sqrt(w * w + x * x + y * y + z * z) or 1.0
    return [x / n, y / n, z / n, w / n]


def mmul(a, b):
    return [[sum(a[i][k] * b[k][j] for k in range(3)) for j in range(3)] for i in range(3)]


def rot_x(a):
    c, s_ = math.cos(a), math.sin(a)
    return [[1, 0, 0], [0, c, -s_], [0, s_, c]]


def rot_y(a):
    c, s_ = math.cos(a), math.sin(a)
    return [[c, 0, s_], [0, 1, 0], [-s_, 0, c]]


def rot_z(a):
    c, s_ = math.cos(a), math.sin(a)
    return [[c, -s_, 0], [s_, c, 0], [0, 0, 1]]


STATS = {'quat_direct': 0, 'quat_conj': 0, 'euler': 0, 'euler_order': {}}


def timing(ctrl):
    return {
        'start': r(ctrl.start_time), 'stop': r(ctrl.stop_time),
        'freq': r(ctrl.frequency), 'phase': r(ctrl.phase),
        'cycle': (int(ctrl.flags) >> 1) & 3,
    }


def key_group(group, conv):
    """KeyGroup -> {'i': interpolation, 'k': [[t, value..., (fwd..., bwd...)]]}."""
    interp = int(group.interpolation) if group.num_keys else 1
    keys = []
    for k in group.keys:
        row = [r(k.time)] + conv(k.value)
        if interp == 2:
            row += conv(k.forward) + conv(k.backward)
        keys.append(row)
    return {'i': interp, 'k': keys}


def fl(v):
    return [r(v)]


def v3(v):
    return [r(v.x), r(v.y), r(v.z)]


def col4(v):
    return [r(v.r), r(v.g), r(v.b), r(v.a)]


def keyframe_track(data, node):
    out = {}
    if data.translations.num_keys:
        out['t'] = key_group(data.translations, v3)
    if data.scales.num_keys:
        out['s'] = key_group(data.scales, fl)
    rt = int(getattr(data, 'rotation_type', 1)) if data.num_rotation_keys else 0
    if data.num_rotation_keys and rt != 4:
        bind = rows(node.rotation)
        keys = []
        for k in data.quaternion_keys:
            q = k.value
            keys.append((float(k.time), q.w, q.x, q.y, q.z))
        # Determine the stored convention against the bind pose at t=first.
        _, w, x, y, z = keys[0]
        direct = quat_to_cols(w, x, y, z)
        conj = quat_to_cols(w, -x, -y, -z)
        err_d = sum(abs(direct[i][j] - bind[i][j]) for i in range(3) for j in range(3))
        err_c = sum(abs(conj[i][j] - bind[i][j]) for i in range(3) for j in range(3))
        sign = 1
        if err_c + 1e-4 < err_d:
            sign = -1
            STATS['quat_conj'] += 1
        else:
            STATS['quat_direct'] += 1
        out['q'] = {'i': 1, 'k': [[r(t), r(sign * x), r(sign * y), r(sign * z), r(w)] for t, w, x, y, z in keys]}
    elif data.num_rotation_keys and rt == 4:
        # Euler XYZ float tracks (radians), evaluated at runtime.
        STATS['euler'] += 1
        groups = list(data.xyz_rotations)
        bind = rows(node.rotation)

        def first(g):
            ks = list(g.keys)
            return float(ks[0].value) if ks else 0.0
        a0 = [first(g) for g in groups]
        orders = {
            'xyz': mmul(mmul(rot_x(a0[0]), rot_y(a0[1])), rot_z(a0[2])),
            'zyx': mmul(mmul(rot_z(a0[2]), rot_y(a0[1])), rot_x(a0[0])),
        }
        errs = {o: sum(abs(m[i][j] - bind[i][j]) for i in range(3) for j in range(3)) for o, m in orders.items()}
        best = min(errs, key=errs.get)
        if abs(errs['xyz'] - errs['zyx']) > 1e-3:
            STATS['euler_order'][best] = STATS['euler_order'].get(best, 0) + 1
        else:
            STATS['euler_order']['tie'] = STATS['euler_order'].get('tie', 0) + 1
        if min(errs.values()) > 1e-2:
            STATS['euler_order']['mismatch'] = STATS['euler_order'].get('mismatch', 0) + 1
        out['e'] = [key_group(g, fl) for g in groups]
    return out or None


def decode_pixels(pix):
    """First mip level of an embedded NiPixelData as a PIL RGBA image."""
    from PIL import Image
    if not pix.mipmaps:
        return None
    mip = pix.mipmaps[0]
    w, h, off = int(mip.width), int(mip.height), int(mip.offset)
    raw = bytes(bytearray(int(x) & 255 for x in pix.pixel_data[0]))
    fmt = int(pix.pixel_format)
    if fmt == 1:  # RGBA8 (masks R=0xff, G=0xff00, ...)
        return Image.frombytes('RGBA', (w, h), raw[off:off + w * h * 4])
    if fmt == 0:  # RGB8
        return Image.frombytes('RGB', (w, h), raw[off:off + w * h * 3]).convert('RGBA')
    if fmt in (4, 5, 6):  # DXT1 / DXT3 / DXT5
        n = {4: 1, 5: 2, 6: 3}[fmt]
        block = 8 if fmt == 4 else 16
        size = max(1, (w + 3) // 4) * max(1, (h + 3) // 4) * block
        return Image.frombytes('RGBA', (w, h), raw[off:off + size], 'bcn', n)
    print('unsupported embedded pixel format', fmt)
    return None


class Extractor:
    def __init__(self):
        self.conv = cm.Converter()
        self.textures = []
        self.tex_index = {}
        self.embedded = {}
        TEX_OUT.mkdir(exist_ok=True)
        for old in TEX_OUT.glob('*.png'):
            old.unlink()

    def texture(self, nif_path, source, tag):
        if source is None:
            return -1
        if int(getattr(source, 'use_external', 1)) == 1:
            url = self.conv.resolve_texture(nif_path, source.file_name)
            if not url:
                return -1
            if url.startswith('/assets/converted/textures/'):
                key = url[len('/assets/converted/textures/'):]
            else:
                key = 'original:' + url[len('/assets/original/Data/'):]
        else:
            pix = source.pixel_data
            if pix is None:
                return -1
            img = decode_pixels(pix)
            if img is None:
                return -1
            import hashlib
            digest = hashlib.sha1(img.tobytes()).hexdigest()[:12]
            if digest in self.embedded:
                key = self.embedded[digest]
            else:
                name = re.sub(r'[^A-Za-z0-9]+', '_', nif_path.relative_to(DATA / 'Effects').with_suffix('').as_posix()).lower()
                fname = f'{name}_{tag}.png'
                key = 'embedded:' + fname
                self.embedded[digest] = key
                img.save(TEX_OUT / fname)
        if key not in self.tex_index:
            self.tex_index[key] = len(self.textures)
            self.textures.append(key)
        return self.tex_index[key]

    def extract(self, path):
        d = N.Data()
        with open(path, 'rb') as f:
            d.read(f)
        self.data = d
        self.path = path
        self.nodes = []
        self.node_index = {}
        self.sprites = []
        self.emitters = []
        self.duration = 0.0
        self.tex_tag = 0
        self.psys_list = []
        for root in d.roots:
            if isinstance(root, N.NiAVObject):
                self.walk(root, -1, {}, False)
        # particle systems are walked as nodes; resolve them after the node table exists
        for psys, props, nidx in self.psys_list:
            self.particle_system(psys, props, nidx)
        if not self.sprites and not self.emitters:
            return None
        return {
            'duration': r(self.duration),
            'nodes': self.nodes,
            'sprites': self.sprites,
            'emitters': self.emitters,
        }

    def ctrl_duration(self, ctrl):
        self.duration = max(self.duration, float(ctrl.stop_time))

    def add_node(self, node, parent):
        idx = len(self.nodes)
        self.node_index[id(node)] = idx
        entry = {
            'name': s(node.name),
            'p': parent,
            't': vec3(node.translation),
            'q': [r(c) for c in mat_to_quat(rows(node.rotation))],
            's': r(node.scale),
        }
        if isinstance(node, N.NiBillboardNode):
            mode = int(getattr(node, 'billboard_mode', 0)) if self.data.version >= 0x0A010000 else (int(node.flags) >> 5) & 3
            entry['bb'] = mode
        c = node.controller
        while c is not None:
            if isinstance(c, N.NiKeyframeController) and c.data is not None:
                track = keyframe_track(c.data, node)
                if track:
                    track.update(timing(c))
                    entry['kf'] = track
                    self.ctrl_duration(c)
            elif isinstance(c, N.NiVisController) and c.data is not None:
                entry['vis'] = dict(timing(c), k=[[r(k.time), int(k.value)] for k in c.data.keys])
                self.ctrl_duration(c)
            c = c.next_controller
        if int(node.flags) & 1:
            entry['hidden'] = True
        self.nodes.append(entry)
        return idx

    def walk(self, node, parent, props, hidden):
        if not hasattr(self, 'psys_list'):
            self.psys_list = []
        if id(node) in self.node_index:
            return
        props = dict(props)
        for p in node.properties:
            if p:
                props[p.__class__.__name__] = p
        idx = self.add_node(node, parent)
        hidden = hidden or bool(int(node.flags) & 1)
        name = s(node.name).lower()
        if isinstance(node, N.NiParticleSystem):
            self.psys_list.append((node, props, idx))
            return
        if isinstance(node, N.NiParticles):
            self.psys_list.append((node, props, idx))
            return
        if isinstance(node, N.NiTriBasedGeom) and node.data is not None:
            if not hidden and not name.startswith(('selectionbox', 'selection box', 'collision')):
                self.sprite(node, props, idx)
            return
        for child in getattr(node, 'children', []):
            if child is not None and isinstance(child, N.NiAVObject):
                self.walk(child, idx, props, hidden)

    def material(self, props):
        out = {'color': [1, 1, 1], 'emissive': [0, 0, 0], 'alpha': 1.0, 'blend': 0, 'blended': False,
               'tex': -1, 'vc_emissive': False}
        mat = props.get('NiMaterialProperty')
        if mat is not None:
            out['color'] = [r(mat.diffuse_color.r), r(mat.diffuse_color.g), r(mat.diffuse_color.b)]
            out['emissive'] = [r(mat.emissive_color.r), r(mat.emissive_color.g), r(mat.emissive_color.b)]
            out['alpha'] = r(mat.alpha)
            c = mat.controller
            while c is not None:
                if isinstance(c, N.NiAlphaController) and c.data is not None:
                    out['akeys'] = dict(timing(c), **key_group(c.data.data, fl))
                    self.ctrl_duration(c)
                elif isinstance(c, N.NiMaterialColorController) and c.data is not None:
                    target = int(getattr(c, 'target_color', 0))
                    out['ckeys'] = dict(timing(c), target=target, **key_group(c.data.data, v3))
                    self.ctrl_duration(c)
                c = c.next_controller
        alpha = props.get('NiAlphaProperty')
        if alpha is not None:
            flags = int(alpha.flags)
            src = (flags >> 1) & 15
            dst = (flags >> 5) & 15
            if flags & 1:
                out['blended'] = True
                if dst == 0 and src == 0:
                    out['blend'] = 2          # ONE, ONE
                elif dst == 0:
                    out['blend'] = 1          # SRCALPHA, ONE
                else:
                    out['blend'] = 0          # SRCALPHA, INVSRCALPHA
            if flags & 512:
                out['atest'] = r(alpha.threshold / 255.0)
        vc = props.get('NiVertexColorProperty')
        if vc is not None:
            out['vc_emissive'] = int(vc.vertex_mode) == 1
        tex = props.get('NiTexturingProperty')
        if tex is not None and tex.has_base_texture and tex.base_texture.source is not None:
            self.tex_tag += 1
            out['tex'] = self.texture(self.path, tex.base_texture.source, self.tex_tag)
            c = tex.controller
            while c is not None:
                if isinstance(c, N.NiFlipController):
                    frames = []
                    for src_tex in c.sources:
                        self.tex_tag += 1
                        frames.append(self.texture(self.path, src_tex, self.tex_tag))
                    out['flip'] = dict(timing(c), delta=r(c.delta), frames=frames)
                    self.ctrl_duration(c)
                elif isinstance(c, N.NiTextureTransformController) and c.data is not None:
                    out['uvanim'] = dict(timing(c), op=int(c.operation), **key_group(c.data.data, fl))
                    self.ctrl_duration(c)
                c = c.next_controller
        return out

    def sprite(self, geom, props, idx):
        data = geom.data
        verts = list(data.vertices)
        if not verts:
            return
        mat = self.material(props)
        if not mat['blended']:
            return  # opaque helper geometry left in effect files (preview props)
        if mat['alpha'] <= 0.0 and 'akeys' not in mat and 'ckeys' not in mat:
            return  # never visible
        uvs = list(data.uv_sets[0]) if data.uv_sets else []
        colors = list(data.vertex_colors) if getattr(data, 'has_vertex_colors', False) else []
        tris = [tuple(int(i) for i in t) for t in data.get_triangles()]
        tris = [t for t in tris if len(set(t)) == 3]
        if not tris:
            return

        # pair triangles that share an edge into quads (o1, e1, e2, o2)
        used = [False] * len(tris)
        quads = []
        edge_map = {}
        for ti, t in enumerate(tris):
            for a, b in ((t[0], t[1]), (t[1], t[2]), (t[2], t[0])):
                edge_map.setdefault(frozenset((a, b)), []).append(ti)
        for ti, t in enumerate(tris):
            if used[ti]:
                continue
            paired = False
            for a, b in ((t[0], t[1]), (t[1], t[2]), (t[2], t[0])):
                for tj in edge_map[frozenset((a, b))]:
                    if tj == ti or used[tj]:
                        continue
                    u = tris[tj]
                    o1 = [v for v in t if v not in (a, b)][0]
                    o2 = [v for v in u if v not in (a, b)][0]
                    quads.append([o1, a, b, o2])
                    used[ti] = used[tj] = True
                    paired = True
                    break
                if paired:
                    break
            if not paired:
                used[ti] = True
                quads.append([t[0], t[1], t[2], t[2]])

        def uv(i):
            c = uvs[i] if i < len(uvs) else None
            return [r(c.u if c else 0), r(c.v if c else 0)]

        groups = {}  # node index -> list of quads (20 floats)
        if geom.is_skin() and geom.skin_instance is not None and geom.skin_instance.data is not None:
            # Rigid skinning: each quad follows the bone with the largest weight.
            skin = geom.skin_instance
            sdata = skin.data
            so = sdata.get_transform()
            root = self.data.roots[0]
            gw = geom.get_transform(root) if geom is not root else geom.get_transform()
            post = so * gw
            post_rows = post.as_list()
            # column-major (glam) array of the column-vector matrix == row-vector rows
            post_cols = [r(x) for row in post_rows for x in row]
            STATS['skinned_post'] = STATS.get('skinned_post', 0) + 1
            weights = {}
            offsets = []
            for bi, bone in enumerate(skin.bones):
                bd = sdata.bone_list[bi]
                offsets.append(bd.get_transform())
                for w in bd.vertex_weights:
                    weights.setdefault(int(w.index), []).append((bi, float(w.weight)))
            for q in quads:
                score = {}
                for vi in q:
                    for bi, w in weights.get(vi, []):
                        score[bi] = score.get(bi, 0.0) + w
                if not score:
                    continue
                bi = max(score, key=score.get)
                bone = skin.bones[bi]
                node = self.node_ref(bone)
                if node < 0:
                    continue
                off = offsets[bi]
                row = []
                for vi in q:
                    v = verts[vi] * off
                    row += [r(v.x), r(v.y), r(v.z)] + uv(vi)
                groups.setdefault(node, []).append(row)
            STATS['skinned'] = STATS.get('skinned', 0) + 1
        else:
            for q in quads:
                row = []
                for vi in q:
                    v = verts[vi]
                    row += [r(v.x), r(v.y), r(v.z)] + uv(vi)
                groups.setdefault(idx, []).append(row)
        for node, qs in groups.items():
            entry = {'node': node, 'quads': qs}
            if geom.is_skin() and geom.skin_instance is not None and geom.skin_instance.data is not None:
                entry['post'] = post_cols
            entry.update({k: v for k, v in mat.items()})
            if colors and len(colors) == len(verts):
                avg = [sum(getattr(c, ch) for c in colors) / len(colors) for ch in 'rgba']
                entry['vcolor'] = [r(x) for x in avg]
            self.sprites.append(entry)

    def node_ref(self, node):
        if node is None:
            return -1
        return self.node_index.get(id(node), -1)

    def world(self, node):
        """Bind-pose (rotation cols, translation, scale) of a node in root space."""
        chain = []
        i = self.node_ref(node)
        while i >= 0:
            chain.append(self.nodes[i])
            i = self.nodes[i]['p']
        rot = [[1, 0, 0], [0, 1, 0], [0, 0, 1]]
        t = [0.0, 0.0, 0.0]
        sc = 1.0
        for n in reversed(chain):
            q = n['q']
            m = quat_to_cols(q[3], q[0], q[1], q[2])
            lt = n['t']
            t = [t[i] + sc * sum(rot[i][k] * lt[k] for k in range(3)) for i in range(3)]
            rot = mmul(rot, m)
            sc *= n['s']
        return rot, t, sc

    def particle_system(self, psys, props, idx):
        mat = self.material(props)
        if isinstance(psys, N.NiParticleSystem):
            self.npsys(psys, mat, idx)
        else:
            self.legacy(psys, mat, idx)

    def npsys(self, psys, mat, idx):
        mods = [m for m in psys.modifiers if m is not None]
        em = None
        out = {'node': idx, 'world': bool(psys.world_space)}
        out.update(mat)
        if psys.data is not None:
            out['max'] = int(psys.data.num_vertices)
        active = {}
        c = psys.controller
        ctl = {}
        while c is not None:
            self.ctrl_duration(c)
            if isinstance(c, N.NiPSysEmitterCtlr):
                ctl['emit'] = dict(timing(c))
                if c.data is not None:
                    ctl['emit']['rate'] = key_group(c.data.float_keys, fl)
                    ctl['emit']['vis'] = [[r(k.time), int(k.value)] for k in c.data.visibility_keys]
            elif isinstance(c, N.NiPSysModifierActiveCtlr) and c.data is not None:
                active[s(c.modifier_name)] = dict(timing(c), k=[[r(k.time), int(k.value)] for k in c.data.keys])
            elif isinstance(c, N.NiPSysEmitterInitialRadiusCtlr) and c.data is not None:
                ctl['radius'] = dict(timing(c), **key_group(c.data.data, fl))
            elif isinstance(c, N.NiPSysEmitterSpeedCtlr) and c.data is not None:
                ctl['speed'] = dict(timing(c), **key_group(c.data.data, fl))
            c = c.next_controller
        out['ctl'] = ctl
        gravity = []
        drag = []
        for m in mods:
            name = s(m.name)
            act = active.get(name)
            if isinstance(m, N.NiPSysEmitter):
                em = m
                e = {
                    'speed': r(m.speed), 'speed_var': r(m.speed_variation),
                    'decl': r(m.declination), 'decl_var': r(m.declination_variation),
                    'planar': r(m.planar_angle), 'planar_var': r(m.planar_angle_variation),
                    'color': col4(m.initial_color), 'radius': r(m.initial_radius),
                    'life': r(m.life_span), 'life_var': r(m.life_span_variation),
                }
                if isinstance(m, N.NiPSysBoxEmitter):
                    e.update(shape='box', obj=self.node_ref(m.emitter_object), size=[r(m.width), r(m.height), r(m.depth)])
                elif isinstance(m, N.NiPSysSphereEmitter):
                    e.update(shape='sphere', obj=self.node_ref(m.emitter_object), size=[r(m.radius), 0, 0])
                elif isinstance(m, N.NiPSysCylinderEmitter):
                    e.update(shape='cylinder', obj=self.node_ref(m.emitter_object), size=[r(m.radius), r(m.height), 0])
                elif isinstance(m, N.NiPSysMeshEmitter):
                    pts = []
                    for mesh in m.emitter_meshes:
                        if mesh is None or mesh.data is None:
                            continue
                        rot, t, sc = self.world(mesh)
                        vs = list(mesh.data.vertices)
                        ns = list(mesh.data.normals) if mesh.data.has_normals else []
                        step = max(1, len(vs) // 48)
                        for i in range(0, len(vs), step):
                            v = vs[i]
                            p = [t[k] + sc * (rot[k][0] * v.x + rot[k][1] * v.y + rot[k][2] * v.z) for k in range(3)]
                            if ns:
                                nn = ns[i]
                                n = [rot[k][0] * nn.x + rot[k][1] * nn.y + rot[k][2] * nn.z for k in range(3)]
                            else:
                                n = [0, 0, 1]
                            pts.append([r(x) for x in p + n])
                    axis = m.emission_axis
                    e.update(shape='mesh', obj=-1, points=pts, vel_type=int(m.initial_velocity_type),
                             axis=[r(axis.x), r(axis.y), r(axis.z)])
                else:
                    continue
                if act:
                    e['active'] = act
                out['emitter'] = e
            elif isinstance(m, N.NiPSysGrowFadeModifier):
                out['grow'] = r(m.grow_time)
                out['fade'] = r(m.fade_time)
            elif isinstance(m, N.NiPSysColorModifier) and m.data is not None:
                out['colors'] = key_group(m.data.data, col4)
            elif isinstance(m, N.NiPSysRotationModifier):
                out['rot_speed'] = r(m.initial_rotation_speed)
            elif isinstance(m, N.NiPSysGravityModifier):
                rot, _, _ = self.world(m.gravity_object) if m.gravity_object is not None else ([[1, 0, 0], [0, 1, 0], [0, 0, 1]], 0, 1)
                a = m.gravity_axis
                axis = [rot[k][0] * a.x + rot[k][1] * a.y + rot[k][2] * a.z for k in range(3)]
                _, gpos, _ = self.world(m.gravity_object) if m.gravity_object is not None else (None, [0, 0, 0], 1)
                g = {'axis': [r(x) for x in axis], 'strength': r(m.strength), 'decay': r(m.decay),
                     'type': int(m.force_type), 'turb': r(m.turbulence), 'pos': [r(x) for x in gpos]}
                if act:
                    g['active'] = act
                gravity.append(g)
            elif isinstance(m, N.NiPSysDragModifier):
                rot, _, _ = self.world(m.parent) if m.parent is not None else ([[1, 0, 0], [0, 1, 0], [0, 0, 1]], 0, 1)
                a = m.drag_axis
                axis = [rot[k][0] * a.x + rot[k][1] * a.y + rot[k][2] * a.z for k in range(3)]
                dd = {'axis': [r(x) for x in axis], 'pct': r(m.percentage)}
                if act:
                    dd['active'] = act
                drag.append(dd)
            elif isinstance(m, N.NiPSysColliderManager):
                col = m.collider
                if col is not None and isinstance(col, N.NiPSysPlanarCollider):
                    rot, t, sc = self.world(col.collider_object) if col.collider_object is not None else ([[1, 0, 0], [0, 1, 0], [0, 0, 1]], [0, 0, 0], 1)
                    out['collider'] = {'bounce': r(col.bounce), 'die': bool(col.die_on_collide), 'z': r(t[2])}
        if gravity:
            out['gravity'] = gravity
        if drag:
            out['drag'] = drag
        if 'emitter' not in out:
            return
        self.emitters.append(out)

    def legacy(self, parts, mat, idx):
        """NetImmerse 10.0 NiParticles + NiParticleSystemController."""
        c = parts.controller
        out = {'node': idx, 'world': True}
        out.update(mat)
        while c is not None:
            if isinstance(c, N.NiParticleSystemController):
                self.ctrl_duration(c)
                e = {
                    'speed': r(c.speed), 'speed_var': r(c.speed_random),
                    'decl': r(c.vertical_direction), 'decl_var': r(c.vertical_angle),
                    'planar': r(c.horizontal_direction), 'planar_var': r(c.horizontal_angle),
                    'color': [1, 1, 1, 1], 'radius': r(c.size),
                    'life': r(c.lifetime), 'life_var': r(c.lifetime_random),
                    'shape': 'box', 'obj': self.node_ref(c.emitter), 'size': [0, 0, 0],
                }
                out['emitter'] = e
                out['ctl'] = {'emit': dict(timing(c), rate={'i': 1, 'k': [[0, r(c.emit_rate)]]},
                                           vis=[[r(c.emit_start_time), 1], [r(c.emit_stop_time), 0]])}
                mod = c.particle_extra
                grav = []
                while mod is not None:
                    if isinstance(mod, N.NiParticleGrowFade):
                        out['grow'] = r(mod.grow)
                        out['fade'] = r(mod.fade)
                    elif isinstance(mod, N.NiParticleColorModifier) and mod.color_data is not None:
                        out['colors'] = key_group(mod.color_data.data, col4)
                        ks = out['colors']['k']
                        if ks:
                            # legacy colour keys use absolute seconds over the lifetime
                            life = max(float(c.lifetime), 1e-3)
                            for k in ks:
                                k[0] = r(min(k[0] / life, 1.0)) if ks[-1][0] > 1.01 else k[0]
                    elif isinstance(mod, N.NiParticleRotation):
                        out['rot_speed'] = r(mod.rotation_speed)
                    elif isinstance(mod, N.NiGravity):
                        d = mod.direction
                        grav.append({'axis': [r(d.x), r(d.y), r(d.z)], 'strength': r(mod.force), 'decay': 0,
                                     'type': int(mod.type), 'turb': 0, 'pos': vec3(mod.position)})
                    mod = mod.next_modifier
                if grav:
                    out['gravity'] = grav
            c = c.next_controller
        if 'emitter' in out:
            self.emitters.append(out)


def referenced_models():
    models = set()
    for f in list((DATA / 'Effects').glob('*.tgi')) + list((DATA / 'Projectiles').rglob('*.tgi')):
        t = f.read_text(encoding='latin1')
        for m in re.finditer(r'^\s*model\s*=\s*(\S+)', t, re.I | re.M):
            models.add(m.group(1).replace('\\', '/'))
    return models


# Combat/ambient effect folders played by the particle player. UI markers,
# flags, transports, waypoints and the large special-operation set pieces are
# real animated models and stay with the model pipeline.
INCLUDE = ('gunshot/', 'tankshot/', 'artilleryshot/', 'mortarshot/', 'rocketattacks/', 'explosions/',
           'burningeffects/', 'flakshot/', 'moveeffect/', 'crusheddoodad/', 'construction/pack',
           'specialops/smokebarrage/', 'specialops/atomicbomb/atomicsparks', 'specialops/vweapons/v1explosion',
           'specialops/vweapons/v1groundfire', 'specialops/vweapons/v1shock', 'specialops/courage/couragecloud',
           'specialops/fear/fearcloud', 'specialops/economicaid/grounddust', 'specialops/resupply/')


def main():
    files = {p.relative_to(DATA).as_posix().lower(): p for p in DATA.rglob('*') if p.is_file()}
    ex = Extractor()
    effects = {}
    failures = {}
    for m in sorted(referenced_models(), key=str.lower):
        key = m.lower().lstrip('/')
        if not key.startswith(INCLUDE):
            continue
        path = files.get('effects/' + key) or files.get(key)
        if not path or path.suffix.lower() != '.nif':
            failures[key] = 'missing'
            continue
        rel = path.relative_to(DATA / 'Effects').as_posix().lower() if 'effects/' + key in files else key
        if rel in effects:
            continue
        try:
            fx = ex.extract(path)
        except Exception as e:  # keep going; report
            failures[rel] = f'{type(e).__name__}: {e}'
            continue
        if fx is None:
            failures[rel] = 'empty'
            continue
        effects[rel] = fx
    out = {'format': 'aa-nif-effects-v1', 'textures': ex.textures, 'effects': effects, 'failures': failures}
    OUT.write_text(json.dumps(out, separators=(',', ':')), encoding='utf-8')
    embedded = sorted(k[len('embedded:'):] for k in ex.textures if k.startswith('embedded:'))
    lines = ['//! Generated by extract_nif_effects.py: textures stored inside effect NIFs.',
             'pub const EMBEDDED: &[(&str, &[u8])] = &[']
    lines += [f'    ("{n}", include_bytes!("textures/{n}")),' for n in embedded]
    lines += ['];', '']
    (HERE / 'embedded_textures.rs').write_text(chr(10).join(lines), encoding='utf-8')
    print('effects', len(effects), 'textures', len(ex.textures), 'failures', failures)
    print(STATS)


if __name__ == '__main__':
    main()
