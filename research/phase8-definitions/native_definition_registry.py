"""Original AA.exe definition registry/index oracles (phase 8 definitions).

Every case executes ORIGINAL instructions under Unicorn. Seams are listed per
section in the fixture. Output: native-definition-registry.json.

Sections
  wildcard     859830 whole-path wildcard matcher
  find         5AF720/5AF7F0 resource FindFirst/FindNext (5AEF60 segment rule)
  sort         85BD30 qsort (859A10) + 85BE40 over real index matches
  lookup       427BD0 Thing DataManager Find with translation map
  translate    48D2D0 Translate handler (insert, duplicate, missing target)
  dispatch     48D660 root dispatch, exception abandons the rest of the file
  templates    5BD080 parse-time template registry; 5BCDD0 unload unregisters
  thing        4BCA00 Thing registration, duplicate IDS throws
  kingdom      4CDC70 Kingdom resource vector, unknown IDS skipped
"""
from pathlib import Path
import sys, json, struct, itertools, random
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
from native_machine import Machine, native_windows, P, V, H, lines_of
from unicorn.x86_const import *

DATA = HERE.parents[2] / 'reconstruction/assets/original/Data'
VISITED = {}


def keep(m):
    VISITED.update(m.visited)


def units(s):
    return [ord(c) for c in s]


# ---------------------------------------------------------------- wildcard
def wildcard_cases():
    pats = ['*.tgi', 'Units/USA/*.tgi', 'Game/SVars*.tgi', 'index*.LST', 'font_*.tgi',
            'a*b*c', '*', 'abc', 'ABC', 'a*', '*x', 'Fonts/special_glyphs*.tgi', '', 'x?y',
            'Localization/*.tgi', 'a*.*', '*.', 'templates.lst']
    paths = ['a.tgi', 'A.TGI', 'Units/USA/usa_battleship.tgi', 'Units/usa_battleship.tgi',
             'Game/SVars.tgi', 'Game/SVars_AA.tgi', 'Game/SVars.backup.tgi', 'index_aa.lst',
             'INDEX_AA.LST', 'font_large', 'font_large.tgi', 'font_large.tgx', 'abc', 'aXbYc',
             'acb', 'x?y', 'xzy', '', 'Localization/Hotkeys/Actions/a.tgi', 'a.b.c', 'b',
             'templates.lst', 'Templates.LST', 'Fonts/special_glyphs01.tgi', 'ab']
    return [(p, s) for p in pats for s in paths]


def section_wildcard():
    m = Machine()
    rows = []
    for p, s in wildcard_cases():
        pp = m.cstring(p)
        sp = m.cstring(s)
        r = m.cdecl(0x859830, [pp, sp]) & 0xff
        rows.append({'pattern': p, 'path': s, 'match': bool(r)})
    keep(m)
    return {'function': '859830', 'cases': rows}


# -------------------------------------------------------------------- find
TYPE_NODE = None


def make_resource_manager(m, paths):
    """Fixture resource manager: map at +60 {buckets, nbuckets}, entries
    {resource, next}; resource +38 = path CString, vtable+4 = type chain."""
    global TYPE_NODE
    mgr = m.alloc(0x100)
    buckets = m.alloc(4 * max(1, len(paths)))
    m.w(mgr + 0x60, buckets)
    m.w(mgr + 0x64, max(1, len(paths)))
    typenode = m.alloc(16)  # not the directory type B3D9F4; next = 0
    vt = m.alloc(0x40)

    def gettype(mm, u):
        mm.ret(typenode)
    stub = m.stub(gettype)
    m.w(vt + 4, stub)
    for i, path in enumerate(paths):
        res = m.alloc(0x40)
        m.w(res, vt)
        m.w(res + 0x38, m.cstring(path))
        e = m.alloc(8)
        m.w(e, res)
        m.w(e + 4, 0)
        m.w(buckets + 4 * i, e)
    return mgr


def native_find_all(m, mgr, pattern):
    fd = m.alloc(0x300)
    pat = m.cstring(pattern)
    r = m.thiscall(0x5af720, mgr, [pat, fd])
    h = r['eax']
    found = []
    if h != 0xffffffff:  # 5AF7DF: INVALID_HANDLE_VALUE when nothing matched
        while True:
            found.append(m.raw_wstr(fd + 0x2c))
            r = m.thiscall(0x5af7f0, mgr, [h, fd])
            if not (r['eax'] & 0xff):
                break
    return found


def section_find():
    rows = []
    cases = [
        ('Units/USA/*.tgi', 'Units/USA/usa_battleship.tgi'),
        ('Units/*.tgi', 'Units/USA/usa_battleship.tgi'),
        ('Buildings/Villages/EasternEurope/*.tgi',
         'Buildings/Villages/EasternEurope/EasternEuropeCityCenter02/eastern_europe_city_center02.tgi'),
        ('Buildings/Villages/EasternEurope/*.tgi', 'Buildings/Villages/EasternEurope/eeurope_citycenter_02.tgi'),
        ('index*.LST', 'index_aa.lst'),
        ('index*.LST', 'ui/index_x.lst'),
        ('Localization/*.tgi', 'Localization/Hotkeys/Actions/hotkeys.tgi'),
        ('Localization/*.tgi', 'Localization/strings_AA.tgi'),
        ('Game/SVars*.tgi', 'Game/SVars_AA.tgi'),
        ('Game/SVars*.tgi', 'game/svars.tgi'),
        ('Terrain/*.tgi', 'Terrain/Transitions/transition01.tgi'),
        ('Terrain/*/*.tgi', 'Terrain/Desert/desert.tgi'),
        ('*/x.tgi', 'a/x.tgi'),
        ('*/x.tgi', 'a/b/x.tgi'),
        ('a/', 'a/b'),
        ('a/b', 'a/'),
        ('Fonts/font_*.tgi', 'Fonts/font_large.tgi'),
        ('Fonts/font_*.tgi', 'Fonts/font_large'),
    ]
    for pattern, path in cases:
        m = Machine()
        mgr = make_resource_manager(m, [path])
        found = native_find_all(m, mgr, pattern)
        rows.append({'pattern': pattern, 'path': path, 'match': bool(found), 'name': found[0] if found else None})
        keep(m)
    return {'functions': ['5AF720', '5AF7F0', '5AEF60', '859830', '859B70'], 'cases': rows,
            'seams': ['Fixture resource manager map (+60) with one {resource,next} entry per case; '
                      'resource vtable+4 returns a non-directory type chain.']}


# -------------------------------------------------------------------- sort
def resource_paths():
    out = []
    for p in sorted(DATA.rglob('*')):
        if p.is_file():
            out.append(str(p.relative_to(DATA)).replace('\\', '/'))
    return out


def native_sorted(m, names):
    """85BD30's qsort(859A10) over {CString, flag} entries, read back with
    85BE40 (same harness as phase7 native_settings_order.py)."""
    array = m.alloc(max(len(names), 1) * 8)
    for i, name in enumerate(names):
        m.w(array + i * 8, m.cstring(name))
    m.cdecl(0x96ce20, [array, len(names), 8, 0x859a10])
    enum = m.alloc(0x288)
    m.w(enum + 0x278, array)
    m.w(enum + 0x27c, len(names))
    out = m.alloc(4)
    m.w(out, m.cstring(''))
    values = []
    while m.thiscall(0x85be40, enum, [out])['eax'] & 0xff:
        values.append(m.wstr(m.r(out)))
    return values


def section_sort():
    index = (DATA / 'index_aa.lst').read_bytes()
    lines = [l.rstrip(b'\r\n').decode('latin-1') for l in index.split(b'\n')]
    patterns = [l for l in lines if l and not l.startswith(';') and ('*' in l or '?' in l)]
    paths = resource_paths()
    rows = []
    for pattern in patterns:
        m = Machine()
        # Candidate set: same directory as the pattern (the matcher is proven
        # separately); the order of the full paths is what is checked here.
        d = pattern.rsplit('/', 1)[0] + '/' if '/' in pattern else ''
        names = [p for p in paths if p.lower().startswith(d.lower()) and '/' not in p[len(d):]]
        random.Random(len(names)).shuffle(names)
        rows.append({'pattern': pattern, 'input': names, 'sorted': native_sorted(m, names)})
        keep(m)
    return {'functions': ['96CE20 qsort', '859A10', '85BE40'], 'cases': rows}


# ------------------------------------------------------------------ lookup
NBUCKETS = 31


def native_hash(m, text_ptr):
    cell = m.alloc(4)
    m.w(cell, text_ptr)
    return m.cdecl(0x405030, [cell])


def make_thing_manager(m):
    """Thing DataManager built by its original constructor 4BD9C0 (argument
    0x200 as at 48A95C): vtable A21260, translation map +C (count +14),
    registry +30, per-category lists +5C."""
    mgr = m.alloc(0x200)
    m.thiscall(0x4bd9c0, mgr, [0x200])
    assert m.r(mgr) == 0xa21260
    return mgr


def register_def(m, mgr, ids):
    d = m.alloc(0x40)
    s = m.cstring(ids)
    m.w(d + 8, s)
    e = m.alloc(8)
    bucket = native_hash(m, s) % m.r(mgr + 0x34)
    head = mgr + 0x30
    m.w(e, d)
    m.w(e + 4, m.r(m.r(head) + 4 * bucket))
    m.w(m.r(head) + 4 * bucket, e)
    return d


def insert_translation(m, mgr, old, new):
    """Original 478E10 insert on DataManager+C, bucket from 5043F0."""
    mp = mgr + 0xc
    o = m.alloc(4)
    n = m.alloc(4)
    m.w(o, m.cstring(old))
    m.w(n, m.cstring(new))
    bucket = m.thiscall(0x5043f0, mp, [m.r(o)])['eax']
    m.thiscall(0x478e10, mp, [o, n, bucket])


def section_lookup():
    rows = []
    scenarios = [
        {'registered': ['alpha', 'beta'], 'translations': [], 'queries': ['alpha', 'ALPHA', 'beta', 'gamma', '']},
        {'registered': ['alpha', 'beta'],
         'translations': [['old1', 'alpha'], ['old2', 'missing'], ['alpha', 'beta']],
         'queries': ['old1', 'OLD1', 'old2', 'alpha', 'beta', 'missing', 'gamma', '']},
        {'registered': ['x', 'y'], 'translations': [['x', 'z']], 'queries': ['x', 'y', 'z']},
        {'registered': ['a1'], 'translations': [['b1', 'c1'], ['c1', 'a1']], 'queries': ['b1', 'c1', 'a1']},
    ]
    for sc in scenarios:
        m = Machine()
        mgr = make_thing_manager(m)
        defs = {ids: register_def(m, mgr, ids) for ids in sc['registered']}
        for old, new in sc['translations']:
            insert_translation(m, mgr, old, new)
        names = {v: k for k, v in defs.items()}
        out = []
        for q in sc['queries']:
            r = m.thiscall(0x427bd0, mgr, [m.cstring(q)])['eax']
            g = m.thiscall(0x48d500, mgr, [m.cstring(q)])['eax']  # generic DataManager Find
            out.append({'query': q, 'result': names.get(r) if r else None,
                        'result_48d500': names.get(g) if g else None})
        rows.append({'registered': sc['registered'], 'translations': sc['translations'], 'results': out,
                     'translation_count_word': m.r(mgr + 0x14)})
        keep(m)
    return {'function': '427BD0 and 48D500 (generic Find, same manager object)', 'cases': rows,
            'seams': ['Thing manager built by the original constructor 4BD9C0; registry entries {def,next} with '
                      'def+8 IDS inserted at bucket 405030(IDS) % [+34]; translations inserted by the original '
                      '478E10 with 5043F0 buckets.']}


# --------------------------------------------------------------- translate
def make_gamedata(m, managers):
    """GameData stub at [B36530]: +834 handler map {buckets,nbuckets} of
    {name, manager, next} (4154D0 layout)."""
    gd = m.alloc(0x1000)
    m.w(0xb36530, gd)
    m.w(gd + 4, m.cstring(''))  # GameData+4 current data-file name CString (48D6A9)
    mp = gd + 0x834
    b = m.alloc(4 * NBUCKETS)
    m.w(mp, b)
    m.w(mp + 4, NBUCKETS)
    for name, mgr in managers.items():
        s = m.cstring(name)
        bucket = m.thiscall(0x5043f0, mp, [s])['eax']
        e = m.alloc(12)
        m.w(e, s)
        m.w(e + 4, mgr)
        m.w(e + 8, m.r(b + 4 * bucket))
        m.w(b + 4 * bucket, e)
    return gd


def translation_entries(m, mgr):
    mp = mgr + 0xc
    out = []
    for i in range(m.r(mp + 4)):
        e = m.r(m.r(mp) + 4 * i)
        while e:
            out.append([m.wstr(m.r(e)), m.wstr(m.r(e + 4))])
            e = m.r(e + 8)
    return sorted(out)


TRANSLATE_TEMPLATE = (DATA / 'Templates/template_translate.tgi').read_bytes()


def translate_node(node, old, new):
    return ('[Translate template=Translate]\r\n{\r\n\tnode = %s\r\n\tids_old = %s\r\n'
            '\tids_new = %s\r\n}\r\n' % (node, old, new)).encode()


def section_translate():
    rows = []
    scenarios = [
        [('Thing', 'o1', 'alpha')],
        [('Thing', 'o1', 'alpha'), ('Thing', 'o1', 'beta')],
        [('Thing', 'o2', 'missing')],
        [('Bogus', 'o3', 'alpha')],
        [('thing', 'o4', 'BETA')],
        [('Thing', 'o5', 'o6'), ('Thing', 'o6', 'alpha')],
    ]
    for sc in scenarios:
        m = Machine()
        assert m.reparse(TRANSLATE_TEMPLATE, 'Templates/template_translate.tgi')['error'] is None
        mgr = make_thing_manager(m)
        for ids in ['alpha', 'beta']:
            register_def(m, mgr, ids)
        make_gamedata(m, {'Thing': mgr})
        parsed = m.reparse(b''.join(translate_node(*t) for t in sc), 'translate.tgi')
        assert parsed['error'] is None, parsed
        root = parsed['root']
        steps = []
        for i in range(m.r(root + 0x38)):
            node = m.r(m.r(root + 0x34) + 4 * i)
            r = m.thiscall(0x48d2d0, 0, [node])
            steps.append({'node': i, 'thrown': r.get('thrown'),
                          'translations_after': translation_entries(m, mgr)})
            if 'thrown' in r:
                break
        rows.append({'input': [list(t) for t in sc], 'registered': ['alpha', 'beta'], 'steps': steps})
        keep(m)
    return {'function': '48D2D0', 'cases': rows,
            'seams': ['Translate nodes come from the original 5BD080 parser with Templates/template_translate.tgi '
                      'parsed first (kept registered).',
                      'GameData+834 handler map stub holds only "Thing" -> a Thing manager built by the original '
                      'constructor 4BD9C0; registry entries are fixture-inserted.',
                      'A throw (410C40 -> 96BEF5) stops the case; the message is the last CString built.']}


# ---------------------------------------------------------------- dispatch
def section_dispatch():
    """48D660 over a parsed file: handler vf0 per root, exception from a
    handler (thrown by the original 410C40) is caught by 48D660's own catch
    funclet and the remaining roots are not dispatched."""
    scenarios = [
        {'name': 'no_exception', 'src': '[A]\r\n{\r\n}\r\n[Template T]\r\n{\r\n}\r\n[B]\r\n{\r\n}\r\n[C]\r\n{\r\n}\r\n[D]\r\n{\r\n}\r\n', 'throw': []},
        {'name': 'handler_exception', 'src': '[A]\r\n{\r\n}\r\n[B]\r\n{\r\n}\r\n[C]\r\n{\r\n}\r\n[D]\r\n{\r\n}\r\n', 'throw': ['C']},
        {'name': 'first_root_exception', 'src': '[C]\r\n{\r\n}\r\n[A]\r\n{\r\n}\r\n', 'throw': ['C']},
        {'name': 'template_only', 'src': '[Template T]\r\n{\r\n}\r\n[Template U]\r\n{\r\n}\r\n', 'throw': []},
        {'name': 'unknown_tag', 'src': '[A]\r\n{\r\n}\r\n[Z]\r\n{\r\n}\r\n[B]\r\n{\r\n}\r\n', 'throw': []},
        {'name': 'duplicate_roots_dispatch_by_label', 'src': '[A]\r\n{\r\n}\r\n[A]\r\n{\r\n}\r\n[B]\r\n{\r\n}\r\n', 'throw': []},
    ]
    rows = []
    for sc in scenarios:
        m = Machine()
        m.watch_frames(0x48d684, 0xae0bc0)
        calls = []

        def make_vf0(tag):
            def vf0(mm, u):
                esp = u.reg_read(UC_X86_REG_ESP)
                node = mm.r(esp + 4)
                calls.append({'tag': tag, 'name': mm.wstr(mm.r(node)), 'label': mm.wstr(mm.r(node + 4))})
                if tag in sc['throw']:
                    # original 410C40(fmt) builds and throws TGC_BadDataException
                    mm.w(esp + 4, mm.cstring('fixture handler exception'))
                    u.reg_write(UC_X86_REG_EIP, 0x410c40)
                else:
                    mm.ret(0, pop=4)
            return m.stub(vf0)

        def first_time(mm, u):
            mm.ret(0)
        ft = m.stub(first_time)
        managers = {}
        for tag in ['A', 'B', 'C', 'D']:
            vt = m.alloc(0x20)
            m.w(vt, make_vf0(tag))
            m.w(vt + 0xc, ft)
            obj = m.alloc(0x20)
            m.w(obj, vt)
            managers[tag] = obj
        gd = make_gamedata(m, managers)
        parsed = m.reparse(sc['src'].encode(), 'fixture.tgi')
        assert parsed['error'] is None, parsed
        print('  scenario', sc['name'], flush=True)
        r = m.thiscall(0x48d660, gd, [parsed['resource']])
        rows.append({'name': sc['name'], 'source': sc['src'], 'throwing_handlers': sc['throw'],
                     'dispatched': calls, 'returned': (r.get('eax', 0) & 0xff) if 'eax' in r else None,
                     'unhandled': r.get('thrown'), 'caught': [e['type'] for e in m.eh_events],
                     'throw_formats': m.throw_formats})
        keep(m)
    return {'function': '48D660', 'cases': rows,
            'seams': ['GameData+834 handler map stub; handler objects are fixture stubs whose vf0 records the '
                      'node and, when configured, transfers to the original 410C40 throw helper; vf C returns 0.',
                      'C++ EH dispatch seam (native_machine.py): the original 48D660 catch funclet named by its '
                      'FuncInfo AE0BC0 runs and execution resumes at the continuation it returns with '
                      'ESP=[EBP-10]. Inner-frame destructors are skipped.']}


# --------------------------------------------------------------- templates
def section_templates():
    single = {
        'nested_then_root_ref': '[A]\r\n{\r\n\t[Template N]\r\n\t{\r\n\t\tstring a\r\n\t}\r\n}\r\n[X Template=N]\r\n{\r\n\ta = 1\r\n}\r\n',
        'nested_then_sibling_ref': '[A]\r\n{\r\n\t[Template N]\r\n\t{\r\n\t\tstring a\r\n\t}\r\n\t[Y Template=N]\r\n\t{\r\n\t\ta = 1\r\n\t}\r\n}\r\n',
        'forward_ref': '[X Template=N]\r\n{\r\n\ta = 1\r\n}\r\n[Template N]\r\n{\r\n\tstring a\r\n}\r\n',
        'dup_template_same_file': '[Template N]\r\n{\r\n\tstring a\r\n}\r\n[Template N]\r\n{\r\n\tstring b\r\n}\r\n',
        'dup_then_ref': '[Template N]\r\n{\r\n\tstring a\r\n}\r\n[Template N]\r\n{\r\n\tstring b\r\n}\r\n[X Template=N]\r\n{\r\n\ta = 1\r\n}\r\n',
        'ref_between': '[Template N]\r\n{\r\n\tstring a\r\n}\r\n[X Template=N]\r\n{\r\n\ta = 1\r\n}\r\n[Template N]\r\n{\r\n\tstring b\r\n}\r\n',
        'same_name_root_hides_template': '[Template N]\r\n{\r\n\tstring a\r\n}\r\n[N Template=N]\r\n{\r\n\ta = 1\r\n}\r\n[X Template=N]\r\n{\r\n\ta = 2\r\n}\r\n',
        'case_insensitive_ref': '[Template Nn]\r\n{\r\n\tstring a\r\n}\r\n[X Template=nN]\r\n{\r\n\ta = 1\r\n}\r\n',
        'inherit_keyword': '[Template N]\r\n{\r\n\tstring a\r\n}\r\n[Template M inherit=N]\r\n{\r\n\ta = 1\r\n}\r\n[X Template=M]\r\n{\r\n}\r\n',
    }
    rows = []
    for name, src in single.items():
        m = Machine()
        r = m.reparse(src.encode(), 'f1.tgi')
        rows.append({'name': name, 'files': [src], 'released_after': [], 'errors': [
            r['error'][-1] if r['error'] else None]})
        keep(m)
    sequences = [
        ('cross_file_duplicate_while_kept', ['[Template N]\r\n{\r\n\tstring a\r\n}\r\n',
                                             '[Template N]\r\n{\r\n\tstring b\r\n}\r\n'], []),
        ('cross_file_reference_while_kept', ['[Template N]\r\n{\r\n\tstring a\r\n}\r\n',
                                             '[X Template=N]\r\n{\r\n\ta = 1\r\n}\r\n'], []),
        ('reference_after_release', ['[Template N]\r\n{\r\n\tstring a\r\n}\r\n[X Template=N]\r\n{\r\n\ta = 1\r\n}\r\n',
                                     '[Y Template=N]\r\n{\r\n\ta = 1\r\n}\r\n'], [0]),
        ('redeclare_after_release', ['[Template N]\r\n{\r\n\tstring a\r\n}\r\n[X Template=N]\r\n{\r\n\ta = 1\r\n}\r\n',
                                     '[Template N]\r\n{\r\n\tstring b\r\n}\r\n[Y Template=N]\r\n{\r\n\tb = 1\r\n}\r\n'], [0]),
        ('nested_released_with_file', ['[A]\r\n{\r\n\t[Template N]\r\n\t{\r\n\t\tstring a\r\n\t}\r\n}\r\n',
                                       '[X Template=N]\r\n{\r\n\ta = 1\r\n}\r\n'], [0]),
    ]
    for name, files, release in sequences:
        m = Machine()
        errors = []
        for i, src in enumerate(files):
            r = m.reparse(src.encode(), 'f%d.tgi' % i)
            errors.append(r['error'][-1] if r['error'] else None)
            if i in release and not r['error']:
                # 48D970 -> 5AA550 -> ... -> KKC_DataFileResource::Unload 5BCDD0
                m.thiscall(0x5bcdd0, r['resource'], [])
        rows.append({'name': name, 'files': files, 'released_after': release, 'errors': errors})
        keep(m)
    return {'functions': ['5BD080', '5BF0D0', '5BF0A0', '5B5C80', '5BCDD0', '5B3FE0', '5BF080'], 'cases': rows,
            'seams': ['Restoration-agent parser harness seams (line source, heap, pool mutex, empty registry buckets).',
                      'Release is the original KKC_DataFileResource::Unload 5BCDD0 on the parsed resource.']}


# ------------------------------------------------------------------- thing
def section_thing():
    rows = []
    for ids_list in [['alpha', 'beta'], ['alpha', 'ALPHA'], ['alpha', 'beta', 'alpha'], ['']]:
        m = Machine()
        mgr = make_thing_manager(m)
        category = m.alloc(0x40)
        m.w(category + 0x18, 0)  # native kind 0: THING -> 4BBE70 (0x24-byte ThingData)
        steps = []
        for ids in ids_list:
            r = m.thiscall(0x4bca00, mgr, [m.cstring(ids), category])
            found = m.thiscall(0x427bd0, mgr, [m.cstring(ids)])['eax'] if ids else 0
            steps.append({'ids': ids, 'thrown': r.get('thrown'), 'created': r.get('eax'),
                          'lookup_after_is_created': bool(found) and found == r.get('eax'),
                          'lookup_after': bool(found)})
            if 'thrown' in r:
                break
        rows.append({'ids': ids_list, 'steps': steps})
        keep(m)
    return {'function': '4BCA00', 'cases': rows,
            'seams': ['Thing manager built by the original constructor 4BD9C0; category object supplies native '
                      'kind 0 at +18 and list index 0 at +C. A throw stops the case.']}


# ----------------------------------------------------------------- kingdom
def section_kingdom():
    """4CDC70 saved Kingdom resource vector: unknown Resource IDS throws in
    4CDBB0 (41DFB0); 4CDC70's catch funclet consumes the 4-byte value and the
    loop continues; the return flag reports the unknown IDS."""
    rows = []
    scenarios = [
        [('money', 5.0), ('oil', 7.0)],
        [('ipcs', 100.0), ('money', 5.0)],
        [('money', 5.0), ('ipcs', 100.0), ('oil', 7.0)],
        [('', 3.0), ('ammo', 2.0)],
    ]
    for records in scenarios:
        m = Machine()
        m.watch_frames(0x4cdc9d, 0xae4b10)
        mgr = make_thing_manager(m)  # generic DataManager layout read by 48D500
        for i, ids in enumerate(['money', 'ammo', 'oil']):
            d = register_def(m, mgr, ids)
            m.w(d + 0xc, i)
        m.w(0xb3a660, mgr)
        gd = m.alloc(0x100)
        m.w(0xb36530, gd)
        m.w(gd + 0x38, 3)
        payload = struct.pack('<i', len(records))
        for ids, value in records:
            payload += struct.pack('<H', len(ids)) + ids.encode('utf-16-le') + struct.pack('<f', value)
        stream = m.alloc(0x40)
        vt = m.alloc(0x20)
        state = {'at': 0}

        def read(mm, u):
            esp = u.reg_read(UC_X86_REG_ESP)
            dst, n = mm.r(esp + 4), mm.r(esp + 8)
            chunk = payload[state['at']:state['at'] + n]
            state['at'] += n
            mm.u.mem_write(dst, chunk.ljust(n, b'\0'))
            mm.ret(n, pop=8)
        m.w(vt, m.stub(read))
        m.w(stream, vt)
        vector = m.alloc(0x20)
        r = m.cdecl(0x4cdc70, [stream, vector])
        values = [struct.unpack('<f', bytes(m.u.mem_read(m.r(vector) + 4 * i, 4)))[0]
                  for i in range(m.r(vector + 4))]
        rows.append({'records': records, 'returned_flag': r & 0xff, 'vector': values,
                     'consumed_bytes': state['at'], 'payload_bytes': len(payload),
                     'caught': [e['type'] for e in m.eh_events], 'throw_formats': m.throw_formats})
        keep(m)
    return {'function': '4CDC70', 'cases': rows,
            'seams': ['Stream vf0 read is a fixture leaf over the payload bytes (u32 count; per record u16 length, '
                      'UTF-16 IDS, f32 value).',
                      'Resource DataManager at [B3A660]: a DataManager built by 4BD9C0 (same +C/+14/+30 layout read '
                      'by the generic 48D500) with fixture registry entries money/ammo/oil (def+C = index); '
                      'GameData+38 = 3 resources.',
                      'C++ EH dispatch seam to the original 4CDC70 catch funclet (FuncInfo AE4B10).']}


# ------------------------------------------------------------ character_view
def section_character_view():
    """Character constructor 563670 asks 4D0720 for the actor's current
    animation set: stance word +15C != -1 -> 45E5E0 (stance -> aspect) else
    Object variant +40; then 45E690 picks ActorData+1BC[set], falling back to
    the last set (+1C0 - 1) for -1 or a null entry."""
    rows = []
    layouts = [
        {'sets': [1, 0, 1, 1]},   # 3 aspects (aspect 1 without its own set) + base
        {'sets': [1]},            # only the base set
        {'sets': []},             # no Character sets (+1C0 = 0)
    ]
    for layout in layouts:
        present = layout['sets']
        for stance, stance_aspect in [(-1, None), (0, 0), (1, 2), (2, 1)]:
            for variant in [-1, 0, 1, 2]:
                if present and variant >= len(present):
                    continue
                m = Machine()
                d = m.alloc(0x400)
                arr = m.alloc(4 * max(1, len(present)))
                set_ptrs = []
                for i, p in enumerate(present):
                    ptr = m.alloc(0x20) if p else 0
                    set_ptrs.append(ptr)
                    m.w(arr + 4 * i, ptr)
                m.w(d + 0x1bc, arr)
                m.w(d + 0x1c0, len(present))
                # +188 stance list: stance[i] -> +8 object -> +8 aspect index
                stances = m.alloc(0x40)
                for i, aspect in enumerate([0, 2, 1]):
                    st = m.alloc(0x20)
                    obj = m.alloc(0x20)
                    m.w(obj + 8, aspect)
                    m.w(st + 8, obj)
                    m.w(stances + 4 * i, st)
                m.w(d + 0x188, stances)
                actor = m.alloc(0x200)
                m.w(actor + 4, d)
                m.w(actor + 0x15c, stance & 0xffffffff)
                m.w(actor + 0x40, variant & 0xffffffff)
                r = m.thiscall(0x4d0720, actor, [])['eax']
                chosen = set_ptrs.index(r) if r and r in set_ptrs else (None if r == 0 else 'other')
                rows.append({'sets_present': present, 'stance': stance, 'variant': variant,
                             'stance_aspects': [0, 2, 1], 'chosen_set': chosen})
                keep(m)
    return {'function': '4D0720 -> 45E5E0 / 45E690', 'cases': rows,
            'seams': ['Actor stub (+4 definition, +15C stance, +40 Object variant); ActorData stub with +1BC set '
                      'pointers (null where an aspect has no animation set), +1C0 count and +188 stance list '
                      '(stance -> +8 -> +8 aspect index).']}


# ----------------------------------------------------------------- denizen
def section_denizen():
    """Denizen constructor 56B720 over an owner definition whose +35C
    Inventory list and +34C Organization list are fixture-built: slot order
    (prepend), per-entry count, and the 568880 flags with an empty +34C."""
    rows = []
    scenarios = [
        {'inventory': [('usa_bomber', 2), ('usa_fighter_corsair', 6)], 'masks': {'usa_bomber': 0x41, 'usa_fighter_corsair': 0x41}},
        {'inventory': [('a', 1), ('b', 0), ('c', -1), ('d', 3)], 'masks': {'a': 0x41, 'b': 0x41, 'c': 0x41, 'd': 0x1}},
        {'inventory': [], 'masks': {}},
        {'inventory': [('a', 2), ('a', 1)], 'masks': {'a': 0}},
    ]
    for sc in scenarios:
        m = Machine()
        defs = {}
        for name, mask in sc['masks'].items():
            d = m.alloc(0x400)
            m.w(d + 8, m.cstring(name))
            m.w(d + 0x174, mask)
            defs[name] = d
        owner_def = m.alloc(0x400)
        head = 0
        entries = []
        for name, count in sc['inventory']:
            e = m.alloc(0x10)
            m.w(e, defs[name])
            m.w(e + 4, m.cstring(name))
            m.w(e + 8, count & 0xffffffff)
            entries.append(e)
        for a, b in zip(entries, entries[1:]):
            m.w(a + 0xc, b)
        m.w(owner_def + 0x35c, entries[0] if entries else 0)
        m.w(owner_def + 0x34c, 0)
        owner = m.alloc(0x200)
        m.w(owner + 4, owner_def)
        comp = m.alloc(0x100)
        m.thiscall(0x56b720, comp, [owner])
        names = {v: k for k, v in defs.items()}
        slots = []
        p = m.r(comp + 0x18)
        while p:
            raw = bytes(m.u.mem_read(p, 0x14))
            slots.append({'target': names.get(m.r(p + 4)), 'flag_c': raw[0xc], 'flag_d': raw[0xd],
                          'word_0': m.r(p), 'word_8': m.r(p + 8)})
            p = m.r(p + 0x10)
        tail = m.r(comp + 0x1c)
        rows.append({'inventory': [list(x) for x in sc['inventory']], 'target_masks': sc['masks'],
                     'slots_head_first': slots,
                     'tail_is_first_created': (tail == 0 and not slots) or (slots and tail != 0),
                     'word_90': hex(m.r(comp + 0x90))})
        keep(m)
    return {'function': '56B720 (+568880 with empty +34C)', 'cases': rows,
            'seams': ['Owner actor/definition stubs: actor+4 -> definition, definition +35C Inventory entries '
                      '{definition, IDS CString, count, next} and +34C = 0; target definitions carry only '
                      'the +174 component mask. Component base ctor 4DB020 and 79E130 vector ctors execute.']}


# ----------------------------------------------------------------- generic
FLAG_NAMES = ['ethereal', 'revealed', 'tributable', 'blocking', 'corpse_blocking', 'invulnerable',
              'flying', 'water_based', 'flatten_terrain', 'targetable', 'random_orientation',
              'tile_locked', 'axis_locked', 'invisible', 'radar', 'ownable', 'conform_to_terrain',
              'allow_alpha_blend']
GENERIC_TEMPLATE = ('[Template TD]\r\n{\r\n\tstring IDS\r\n\tlocalized name = null\r\n\t[Flags]\r\n\t{\r\n'
                    + ''.join('\t\tbool %s = false\r\n' % f for f in FLAG_NAMES) + '\t}\r\n}\r\n').encode()


def section_generic():
    """Generic DataManager<T>::Load (vf0 483A40 -> vf1C 4BD1B0, the shape of
    47D030/479080/...): a node whose IDS is already registered reuses the
    definition and runs its LoadData again; KKC_Data base LoadData 47D960
    throws 'loading duplicate node with ids' only when the earlier load set
    a nonempty name. Exercised on DataManager<ThingData> (base vtable A2123C,
    ThingData LoadData 4BB310 -> 47D960)."""
    rows = []
    # Literal (non-'#') names: 5B5620 returns them without a string-table
    # lookup, so the stored name is exactly the authored text.
    scenarios = [
        [('alpha', '"First"'), ('beta', '"Second"')],
        [('alpha', '"First"'), ('ALPHA', '"Again"')],
        [('alpha', 'null'), ('alpha', '"Second"')],
        [('alpha', 'null'), ('alpha', '"Second"'), ('alpha', '"Third"')],
        [('alpha', '""'), ('alpha', 'null')],
    ]
    for mode, sc in [(mode, sc) for mode in (0, 1, 2) for sc in scenarios]:
        m = Machine()
        assert m.reparse(GENERIC_TEMPLATE, 'template_td.tgi')['error'] is None
        mgr = make_thing_manager(m)
        gd = make_gamedata(m, {})
        categories = m.alloc(0x40)  # ThingCategory manager (GameData+3B4) +24 -> [THING]
        vector = m.alloc(0x10)
        m.w(vector, m.alloc(0x40))
        m.w(categories + 0x24, vector)
        m.w(gd + 0x3b4, categories)
        glyphs = m.alloc(0x40)      # GameData+3BC Glyph manager: +28 count 0
        m.w(gd + 0x3bc, glyphs)
        m.w(mgr, 0xa2123c)         # base DataManager<ThingData> vtable
        m.w(mgr + 0x1c, 0xa21238)
        m.w(mgr + 8, mode)         # DataManager mode (47D880 arg; GameData ctor 48A8E0)
        text = b''.join(('[Thing Template=TD]\r\n{\r\n\tIDS = %s\r\n\tname = %s\r\n}\r\n' % (i, n)).encode()
                        for i, n in sc)
        parsed = m.reparse(text, 'generic.tgi')
        assert parsed['error'] is None, parsed
        root = parsed['root']
        steps = []
        for i in range(m.r(root + 0x38)):
            node = m.r(m.r(root + 0x34) + 4 * i)
            r = m.thiscall(0x483a40, mgr, [node])
            steps.append({'ids': sc[i][0], 'name': sc[i][1], 'thrown': r.get('thrown'), 'def': r.get('eax')})
            if 'thrown' in r:
                break
        defs = [s['def'] for s in steps if s['def'] is not None]
        rows.append({'mode': mode, 'input': [list(x) for x in sc], 'steps': [
            {k: v for k, v in s.items() if k != 'def'} for s in steps],
            'same_definition_reused': len(set(defs)) < len(defs)})
        keep(m)
    return {'functions': ['483A40', '4BD1B0', '4BD270', '4BB310', '47D960'], 'cases': rows,
            'seams': ['Manager built by the original 4BD9C0 with the base DataManager<ThingData> vtables '
                      'A2123C/A21238 installed; nodes from the original parser with a fixture template '
                      'declaring IDS, localized name and the 18 Flags booleans. GameData+3B4 ThingCategory '
                      'manager stub supplies the default category read by the ThingData ctor 4BBDB0; '
                      'GameData+3BC Glyph manager stub is empty (localized-name glyph pass 494000 skips). '
                      'A throw stops the case.']}


# ----------------------------------------------------------------- headers
def section_headers():
    """5BD080 section-header scan: stray '=' tokens are ignored, a bare
    word after the name is 'Malformed section name.'."""
    rows = []
    for hdr in ['[A = Template=T]', '[A =]', '[A = B]', '[A Template=T =]', '[A == Template=T]',
                '[A= Template=T]', '[A Template=T]', '[A]']:
        m = Machine()
        assert m.reparse(b'[Template T]\r\n{\r\n\tstring ids = x\r\n}\r\n')['error'] is None
        r = m.reparse(hdr.encode() + b'\r\n{\r\n\tids = y\r\n}\r\n')
        if r['error']:
            rows.append({'header': hdr, 'error': m.messages[-1] if m.messages else True})
        else:
            root = m.node(r['root'])
            c = root['children'][0]
            rows.append({'header': hdr, 'error': None, 'name': c['name'],
                         'flag_40_template_base': int(c['flags'], 16) & 0x40 != 0,
                         'fields': {f['name']: m.wstr(f['value']) if f['value'] else None for f in c['fields']}})
        keep(m)
    return {'function': '5BD080', 'cases': rows,
            'seams': ['Restoration-agent parser harness seams; template T parsed first (kept).']}


# ---------------------------------------------------------- translate_file
def section_translate_file():
    """The original parser over the shipped translate.tgi (template parsed
    first, as templates.lst loads it): which Translate roots exist, in order.
    The /* opened at line 1292 closes only at line 5310."""
    data = (DATA / 'translate.tgi').read_bytes()
    m = Machine()
    assert m.reparse(TRANSLATE_TEMPLATE, 'Templates/template_translate.tgi')['error'] is None
    r = m.reparse(data, 'translate.tgi')
    assert r['error'] is None, r
    root = r['root']
    rows = []
    for i in range(m.r(root + 0x38)):
        node = m.r(m.r(root + 0x34) + 4 * i)
        tree = m.node(node)
        fields = {f['name']: (m.wstr(f['value']) if f['value'] else None) for f in tree['fields']}
        rows.append({'name': tree['name'], 'label': tree['label'], 'node': fields.get('node'),
                     'ids_old': fields.get('ids_old'), 'ids_new': fields.get('ids_new')})
    keep(m)
    return {'function': '5BD080', 'source': 'Data/translate.tgi', 'source_sha256': H(data),
            'template_sha256': H(TRANSLATE_TEMPLATE), 'roots': rows,
            'seams': ['Restoration-agent parser harness seams; string field values read from field+10.']}


# ------------------------------------------------------------ construction
def section_construction():
    """5657B0 (called from the Construction constructor 565FA0 -> 565F20):
    the cost vector is built (45F4D0) and appended (5655C0) only when the
    ActorData component mask (+174) has bit 9 (EconomyComponent)."""
    rows = []
    for mask in [0, 1 << 13, (1 << 13) | (1 << 15), 0xfffffdff]:
        m = Machine()
        actor = m.alloc(0x600)
        m.w(actor + 0x174, mask)
        before = set(m.visited)
        m.cdecl(0x5657b0, [actor, 0, 1])
        reached = {a for a in (0x45f4d0, 0x5655c0, 0x6f7810) if a in m.visited and a not in before}
        rows.append({'mask': hex(mask), 'economy_bit9': bool(mask & (1 << 9)),
                     'cost_vector_built': 0x45f4d0 in reached, 'cost_appended': 0x5655c0 in reached})
        keep(m)
    return {'function': '5657B0', 'cases': rows,
            'seams': ['ActorData stub with only the +174 component mask; masks without bit 9 only.']}


SECTIONS = {'wildcard': section_wildcard, 'find': section_find, 'sort': section_sort,
            'lookup': section_lookup, 'translate': section_translate, 'dispatch': section_dispatch,
            'templates': section_templates, 'thing': section_thing, 'kingdom': section_kingdom,
            'construction': section_construction, 'translate_file': section_translate_file,
            'headers': section_headers, 'generic': section_generic, 'denizen': section_denizen,
            'character_view': section_character_view}


def main():
    want = sys.argv[1:] or list(SECTIONS)
    out_path = HERE / 'native-definition-registry.json'
    doc = json.loads(out_path.read_text()) if out_path.exists() else {}
    doc.setdefault('sections', {})
    for name in want:
        print('section', name, flush=True)
        VISITED.clear()
        result = SECTIONS[name]()
        result['native_windows'] = native_windows(VISITED)
        doc['sections'][name] = result
    doc.pop('native_windows_by_section', None)
    doc.update({'schema': 1, 'passed': True, 'producer_sha256': H(Path(__file__).read_bytes()),
                'machine_sha256': H((HERE / 'native_machine.py').read_bytes()),
                'original_sha256': H(P.b), 'v14_sha256': H(V.b)})
    out_path.write_text(json.dumps(doc, indent=1))
    print('wrote', out_path, H(out_path.read_bytes()))


if __name__ == '__main__':
    main()
