"""Phase8 oracle corpus: actual 546AF0 conditions, events and actions.

Run:  PYTHONPATH=<reconstruction/tools/python-packages> python native_elements.py
Writes native-elements.json.  See harness.py for the seam policy.
"""
from pathlib import Path
import copy, itertools, sys
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import harness as h  # noqa: E402

P = h.param
bits = h.core.bits
A1, A2, K1, K2 = 0x10003, 0x10004, 0x10002, 0x10005


def base():
    return {
        'time_bits': bits(10), 'precision': 'Double53',
        'variables': {'counter': bits(2.0), 'level': bits(1.75)},
        'triggers': [{'id': 10, 'enabled': 1, 'activation_bits': bits(1)}],
        'objects': {
            'actor_a': {'kind': 'actor', 'slot': 3, 'gen': 1, 'words': {'0x8': 0x1800020}},
            'actor_b': {'kind': 'actor', 'slot': 4, 'gen': 1, 'words': {'0x8': 0x1}},
            'kingdom_a': {'kind': 'kingdom', 'slot': 2, 'gen': 1, 'size': 0x400},
            'kingdom_b': {'kind': 'kingdom', 'slot': 5, 'gen': 1, 'size': 0x400},
            'area_a': {'kind': 'area', 'slot': 6, 'gen': 1},
            'camera_a': {'kind': 'camera', 'slot': 7, 'gen': 1},
            'speech_a': {'kind': 'audio'},
        },
        'objective_defs': [
            {'name': 'obj_primary', 'text': 'B text', 'flag': 1},
            {'name': 'obj_secondary', 'text': 'A text', 'flag': 0},
            {'name': 'obj_third', 'text': 'C text', 'flag': 0},
        ],
        'kingdoms': {
            'kingdom_a': {'objectives': [['obj_primary', 0]], 'result': 0, 'team_members': None},
            'kingdom_b': {'objectives': [], 'result': 0, 'team_members': None},
        },
        'manager': {'context_word': 0, 'subjects': [], 'condition_kingdom': 0, 'event_kingdom': 0, 'events': []},
        'returns': {
            'NamedActor': {'by_arg': {'actor_a': '@actor_a', 'actor_b': '@actor_b'}},
            'NamedKingdom': {'by_arg': {'kingdom_a': '@kingdom_a', 'kingdom_b': '@kingdom_b'}},
            'Owner': {'actor_a': '@kingdom_a', 'actor_b': '@kingdom_b'},
            'IsArea': {'area_a': 1},
            'IsCamera': {'camera_a': 1},
            'DataByIds:audio': {'by_arg': {'speech_a': '@speech_a'}},
            'SortObjectivesVar': 1,
            'GameVar': {'by_arg': {'TeamVictory': 0}},
            'LocalKingdom': '@kingdom_a',
            'KingdomDefaultLose': 1,
        },
    }


ROWS = []


def add(name, opcode, params, precision=None, **changes):
    c = base()
    for key, value in changes.items():
        if isinstance(value, dict) and isinstance(c.get(key), dict):
            merged = copy.deepcopy(c[key])
            for k, v in value.items():
                merged[k] = v
            c[key] = merged
        else:
            c[key] = value
    c['name'] = name
    c['element'] = h.element(opcode, params)
    for pc in (precision or ['Double53']):
        row = copy.deepcopy(c)
        row['precision'] = pc
        row['name'] = f'{name}_{pc}'
        ROWS.append(row)


# Selector variants -------------------------------------------------------------
ACTORS = [('named_a', 'mi_actor_named', 'actor_a'), ('named_missing', 'mi_actor_named', 'nobody'),
          ('id_a', 'mi_actor_id', A1), ('id_stale', 'mi_actor_id', 0x20003), ('any', 'mi_actor_any', 0),
          ('condition', 'mi_actor_condition', 0), ('event', 'mi_actor_event', 0)]
KINGDOMS = [('named_a', 'mi_kingdom_named', 'kingdom_a'), ('named_missing', 'mi_kingdom_named', 'nokingdom'),
            ('id_a', 'mi_kingdom_id', K1), ('id_actor', 'mi_kingdom_id', A1), ('any', 'mi_kingdom_any', 0),
            ('condition', 'mi_kingdom_condition', 0), ('event', 'mi_kingdom_event', 0)]
CTX = {'context_word': A2, 'subjects': [A1], 'condition_kingdom': '@kingdom_b', 'event_kingdom': '@kingdom_a', 'events': []}


def actor_param(v):
    return P('ma_actor', v[1], v[2])


def kingdom_param(v):
    return P('ma_kingdom', v[1], v[2])


# 15 actor_owned_by_kingdom (condition) ----------------------------------------
for a, k in itertools.product(ACTORS, KINGDOMS):
    add(f'owned_{a[0]}_{k[0]}', 15, [actor_param(a), kingdom_param(k)], manager=CTX)
add('owned_owner_mismatch', 15, [actor_param(ACTORS[0]), kingdom_param(KINGDOMS[0])],
    returns={'Owner': {'actor_a': '@kingdom_b'}})

# 28 actor_is_dead / 23 kingdom_default_lose / 107 object_removed / 79 object_destroyed
for a in ACTORS:
    add(f'dead_{a[0]}', 28, [actor_param(a)], manager=CTX)
    add(f'removed_{a[0]}', 107, [actor_param(a)], manager=CTX)
    add(f'destroyed_{a[0]}', 79, [actor_param(a)], manager=CTX)
for k in KINGDOMS:
    add(f'default_lose_{k[0]}', 23, [kingdom_param(k)], manager=CTX)
add('default_lose_false', 23, [kingdom_param(KINGDOMS[0])], returns={'KingdomDefaultLose': 0})

# 40 actor_transfer_control_to_kingdom
for a, k in itertools.product(ACTORS[:3], KINGDOMS[:3]):
    add(f'transfer_{a[0]}_{k[0]}', 40, [actor_param(a), kingdom_param(k)], manager=CTX)

# 17 difficulty_compare_difficulty
DIFF = [('int2', 'mi_int', 2), ('fixed', 'mi_fixed', bits(1.9)), ('fixed_neg', 'mi_fixed', bits(-2.5)),
        ('fixed_big', 'mi_fixed', bits(3e9)), ('fixed_nan', 'mi_fixed', 0x7fc00000), ('var', 'mi_variable', 'level'),
        ('map', 'mi_difficulty_level_map', 0), ('named', 'mi_difficulty_level_specified', 'hard'),
        ('named_missing', 'mi_difficulty_level_specified', 'nightmare'), ('named_empty', 'mi_difficulty_level_specified', '')]
for (ln, li, lv), kind, (rn, ri, rv) in itertools.product(DIFF, [0, 2, 3, 5], DIFF[:1] + DIFF[6:8]):
    add(f'difficulty_{ln}_{kind}_{rn}', 17, [P('ma_difficulty_level', li, lv), P('ma_compare', 'mi_compare', kind),
                                            P('ma_difficulty_level', ri, rv)],
        precision=['Single24', 'Double53', 'Extended64'], map_difficulty=2,
        returns={'DifficultyByName': {'by_arg': {'hard': 2}}})

# 97/98 objective_succeeded/failed ----------------------------------------------
for op in [97, 98]:
    for state in [0, 1, 2]:
        for obj in ['obj_primary', 'obj_secondary', 'missing']:
            for k in KINGDOMS[:3] + KINGDOMS[5:6]:
                add(f'objective{op}_{state}_{obj}_{k[0]}', op,
                    [P('ma_objective', 'mi_objective', obj), kingdom_param(k)], manager=CTX,
                    kingdoms={'kingdom_a': {'objectives': [['obj_secondary', 1], ['obj_primary', state]], 'result': 0, 'team_members': None},
                              'kingdom_b': {'objectives': [['obj_primary', 2]], 'result': 0, 'team_members': None}})

# 50/51 gain/lose objective, 58/59/60 set objective state ------------------------
GAIN_K = {'kingdom_a': {'objectives': [['obj_primary', 0]], 'result': 0, 'team_members': None},
          'kingdom_b': {'objectives': [], 'result': 0, 'team_members': None}}
for name in ['obj_primary', 'obj_secondary', 'obj_third', 'missing', '']:
    for k in KINGDOMS[:2] + KINGDOMS[4:5]:
        for sort in [0, 1]:
            add(f'gain_{name or "empty"}_{k[0]}_sort{sort}', 50, [kingdom_param(k), P('ma_objective', 'mi_objective', name)],
                returns={'SortObjectivesVar': sort})
    add(f'gain_{name or "empty"}_into_empty', 50, [P('ma_kingdom', 'mi_kingdom_named', 'kingdom_b'), P('ma_objective', 'mi_objective', name)])
    for k in KINGDOMS[:2]:
        add(f'lose_{name or "empty"}_{k[0]}', 51, [kingdom_param(k), P('ma_objective', 'mi_objective', name)],
            kingdoms={'kingdom_a': {'objectives': [['obj_secondary', 1], ['obj_primary', 0], ['obj_third', 2]], 'result': 0, 'team_members': None},
                      'kingdom_b': {'objectives': [], 'result': 0, 'team_members': None}})
SORTED = {'kingdom_a': {'objectives': [['obj_secondary', 0], ['obj_primary', 0], ['obj_third', 0]], 'result': 0, 'team_members': None},
          'kingdom_b': {'objectives': [], 'result': 0, 'team_members': None}}
add('gain_sorted_middle', 50, [kingdom_param(KINGDOMS[0]), P('ma_objective', 'mi_objective', 'obj_primary')], kingdoms=SORTED)
for op in [58, 59, 60]:
    for name in ['obj_primary', 'obj_third', 'missing']:
        for k in KINGDOMS[:2] + KINGDOMS[4:5]:
            add(f'state{op}_{name}_{k[0]}', op, [P('ma_objective', 'mi_objective', name), kingdom_param(k)],
                kingdoms={'kingdom_a': {'objectives': [['obj_secondary', 1], ['obj_primary', 2]], 'result': 0, 'team_members': None},
                          'kingdom_b': {'objectives': [], 'result': 0, 'team_members': None}})

# 56/57 mission won/lost ---------------------------------------------------------
for op in [56, 57]:
    for k in KINGDOMS[:2] + KINGDOMS[4:5]:
        add(f'mission{op}_{k[0]}', op, [kingdom_param(k)])
    for result in [0, 1, 2]:
        for team_var, members in [(0, None), (1, None), (1, 1), (1, 2), (1, 0)]:
            for local in ['@kingdom_a', '@kingdom_b']:
                for world48, flag239 in [(0, 0), (2, 0), (0, 1)]:
                    add(f'mission{op}_r{result}_tv{team_var}_m{members}_{local[1:]}_w{world48}_f{flag239}', op,
                        [kingdom_param(KINGDOMS[0])], world_48=world48,
                        kingdoms={'kingdom_a': {'objectives': [], 'result': result, 'team_members': members, 'flag_239': flag239},
                                  'kingdom_b': {'objectives': [], 'result': 0, 'team_members': None}},
                        returns={'GameVar': {'by_arg': {'TeamVictory': team_var}}, 'LocalKingdom': local})

# 87 play_speech ----------------------------------------------------------------
for name in ['speech_a', 'unknown_speech', '']:
    add(f'speech_{name or "empty"}', 87, [P('ma_speech', 'mi_speech', name)])

# Event predicates ----------------------------------------------------------------
G = lambda n: 'gtid:' + n  # noqa: E731
EV_OBJECTS = {'org_a': {'kind': 'misc', 'members': ['actor_b']},
              'data_a': {'kind': 'data'}, 'data_b': {'kind': 'data'}, 'data_x': {'kind': 'data'}}


def world(**extra):
    objects = copy.deepcopy(base()['objects'])
    objects.update(copy.deepcopy(EV_OBJECTS))
    for k, v in extra.items():
        objects[k] = v
    return objects


DESTROYED = [{'kind': 0, 'words': [G('actor_a'), G('actor_b'), 7]},
             {'kind': 2, 'words': [G('actor_b'), '@area_a']},
             {'kind': 0, 'words': [G('kingdom_b'), G('actor_a'), 9]}]
ACTOR_IDS = ACTORS + [('id_zero', 'mi_actor_id', 0), ('id_kingdom', 'mi_actor_id', K2)]
for dead_a in [0, 1]:
    objs = world()
    objs['actor_a']['words'] = {'0x8': dead_a}
    objs['actor_b']['words'] = {'0x8': 0, '0x7c': '@org_a'}
    for evs_name, evs in [('events', DESTROYED), ('none', []), ('other', DESTROYED[1:2])]:
        for a, b in itertools.product(ACTOR_IDS, ACTOR_IDS):
            add(f'destroys_d{dead_a}_{evs_name}_{a[0]}_{b[0]}', 3, [actor_param(a), actor_param(b)], objects=objs,
                manager=dict(CTX, events=evs))
        for a in ACTOR_IDS:
            add(f'destroys_d{dead_a}_{evs_name}_{a[0]}_any2', 3, [actor_param(a), P('ma_actor', 'mi_actor_any', 0)],
                objects=objs, manager=dict(CTX, events=evs))
            add(f'destroys_d{dead_a}_{evs_name}_any1_{a[0]}', 3, [P('ma_actor', 'mi_actor_any', 0), actor_param(a)],
                objects=objs, manager=dict(CTX, events=evs))
        add(f'destroys_d{dead_a}_{evs_name}_any_any', 3, [P('ma_actor', 'mi_actor_any', 0), P('ma_actor', 'mi_actor_any', 0)],
            objects=objs, manager=dict(CTX, events=evs))
objs = world()
objs['actor_a']['words'] = {'0x8': 1}
objs['actor_b']['words'] = {'0x8': 0, '0x7c': '@org_a'}
objs['org_a']['members'] = ['kingdom_b']
add('destroys_org_member', 3, [P('ma_actor', 'mi_actor_id', A2), P('ma_actor', 'mi_actor_id', A1)], objects=objs,
    manager=dict(CTX, events=[{'kind': 0, 'words': [G('actor_a'), G('kingdom_b'), 1]}]))

CAPTURES = [{'kind': 1, 'words': [G('actor_a'), G('actor_b'), G('kingdom_a')]},
            {'kind': 1, 'words': [G('actor_b'), G('actor_a'), G('kingdom_b')]},
            {'kind': 1, 'words': [G('actor_a'), G('actor_b'), G('kingdom_b')]}]
for a, k in itertools.product(ACTORS, KINGDOMS):
    add(f'captured_{a[0]}_{k[0]}', 2, [actor_param(a), kingdom_param(k)], manager=dict(CTX, events=CAPTURES))
for vt158, vt15c in [(1, '@actor_b'), (1, 0), (0, '@actor_b')]:
    for evs_name, evs in [('first', CAPTURES), ('reordered', CAPTURES[1:])]:
        tag = str(vt15c).replace('@', '')
        add(f'captured_vt{vt158}_{tag}_{evs_name}', 2, [actor_param(ACTORS[0]), kingdom_param(KINGDOMS[0])],
            manager=dict(CTX, events=evs), returns={'Vt158': {'actor_a': vt158}, 'Vt15C': {'actor_a': vt15c}})

AREAS = [{'kind': 2, 'words': [G('actor_a'), '@area_a']}, {'kind': 2, 'words': [G('actor_b'), '@area_a']},
         {'kind': 2, 'words': [G('actor_a'), '@camera_a']}, {'kind': 0, 'words': [G('actor_a'), G('actor_b'), 0]}]
AREA_REFS = [('area', 'mi_area', 0x10006), ('camera', 'mi_area', 0x10007), ('stale', 'mi_area', 0x20006), ('zero', 'mi_area', 0)]
for a, (an, ai, av) in itertools.product(ACTOR_IDS + [('any', 'mi_actor_any', 0)], AREA_REFS):
    add(f'enters_{a[0]}_{an}', 5, [actor_param(a), P('ma_area', ai, av)], manager=dict(CTX, events=AREAS))
add('enters_none', 5, [P('ma_actor', 'mi_actor_any', 0), P('ma_area', 'mi_area', 0x10006)], manager=dict(CTX, events=[]))
add('enters_dupe_subject', 5, [P('ma_actor', 'mi_actor_any', 0), P('ma_area', 'mi_area', 0x10006)],
    manager=dict(CTX, events=AREAS + AREAS[:1]))

BUILD = [{'kind': 3, 'words': [0, G('kingdom_a'), '@data_a', 1]}, {'kind': 3, 'words': [0, G('kingdom_a'), '@data_b', 0]},
         {'kind': 4, 'words': [0, G('kingdom_b'), '@data_a', 0]}, {'kind': 4, 'words': [0, G('kingdom_a'), '@data_b', 1]},
         {'kind': 3, 'words': [0, G('kingdom_b'), '@data_b', 1]}]
DATA = [('any', 'mi_actor_data_any', 0), ('a', 'mi_actor_data_named', 'data_a'), ('b', 'mi_actor_data_named', 'data_b'),
        ('missing', 'mi_actor_data_named', 'nodata'), ('notdata', 'mi_actor_data_named', 'data_x')]
DATA_RET = {'DataByIds:actor_data': {'by_arg': {'data_a': '@data_a', 'data_b': '@data_b', 'data_x': '@data_x'}},
            'IsActorData': {'data_a': 1, 'data_b': 1}}
for op in [6, 7, 8]:
    for k, d in itertools.product(KINGDOMS, DATA):
        for evs_name, evs in [('all', BUILD), ('none', [])]:
            add(f'build{op}_{k[0]}_{d[0]}_{evs_name}', op, [kingdom_param(k), P('ma_actor_data', d[1], d[2])], objects=world(),
                manager=dict(CTX, events=evs), returns=DATA_RET)

SCRIPTS = [{'kind': 5, 'text': 'Hello'}, {'kind': 5, 'text': 'secondÉ'}, {'kind': 2, 'words': [G('actor_a'), '@area_a']}]
for text in ['hello', 'HELLO', 'Hello', 'secondé', 'secondÉ', 'missing', '']:
    add(f'script_{text.encode().hex() or "empty"}', 9, [P('ma_script_command', 'mi_script_command', text)],
        manager=dict(CTX, events=SCRIPTS))

for op in [12, 13]:
    for a in ACTORS:
        for index in [0, 5, 31, 32, 63, 64, 0x105, 0xffffffff]:
            objs = world()
            objs['actor_a']['words'] = {'0x8': 0, '0x100': 0x80000021, '0x104': 0x80000001}
            objs['actor_b']['words'] = {'0x8': 0, '0x100': 0x1, '0x104': 0}
            add(f'state{op}_{a[0]}_{index}', op, [actor_param(a), P('ma_state', 'mi_state', index)], objects=objs, manager=CTX)

for a, d in itertools.product(ACTORS, DATA[1:]):
    objs = world()
    objs['actor_a']['words'] = {'0x8': 0, '0x4': '@data_a'}
    objs['actor_b']['words'] = {'0x8': 0, '0x4': '@data_b'}
    add(f'type27_{a[0]}_{d[0]}', 27, [actor_param(a), P('ma_actor_data', d[1], d[2])], objects=objs, manager=CTX, returns=DATA_RET)


# Actions: camera / presentation / technology / UI / audio / commands / creation -------------
PRECISIONS = ['Single24', 'Double53', 'Extended64']
CAM_OBJECTS = world()
CAM_OBJECTS['camera_a']['words'] = {'0x20': bits(123.456789), '0x24': bits(-45.5), '0x28': bits(3.0), '0x60': bits(1.75)}
CAM_OBJECTS['camera_b'] = {'kind': 'camera', 'slot': 8, 'gen': 1, 'words': {'0x20': bits(1e-7), '0x24': bits(4096.0625), '0x60': bits(0.333333)}}
CAM_OBJECTS['kingdom_a']['words'] = {'0x18': '$Kingdom A'}
CAM_OBJECTS['kingdom_b']['words'] = {'0x18': '$Second'}
CAM_RET = {'CameraPitch': {'camera_a': bits(-30.25), 'camera_b': bits(12.000001)},
           'CameraYaw': {'camera_a': bits(45.0), 'camera_b': bits(-179.99998)},
           'IsCamera': {'camera_a': 1, 'camera_b': 1}, 'RealTimeMs': 0}
CAMS = [('a', 'mi_camera', 0x10007), ('b', 'mi_camera', 0x10008), ('stale', 'mi_camera', 0x20007),
        ('area', 'mi_camera', 0x10006)]
DURS = [('f2', 'mi_fixed', bits(2.0)), ('fsmall', 'mi_fixed', bits(0.1)), ('int3', 'mi_int', 3),
        ('var', 'mi_variable', 'level'), ('neg', 'mi_fixed', bits(-1.5))]
for op in [71, 72]:
    for k, (cn, ci, cv), (dn, di, dv) in itertools.product(KINGDOMS[:3], CAMS, DURS):
        for ease in ([0, 1] if op == 71 else [0]):
            params = [kingdom_param(k), P('ma_camera', ci, cv), P('ma_value', di, dv)]
            if op == 71:
                params.append(P('ma_boolean', 'mi_boolean', ease))
            add(f'cam{op}_{k[0]}_{cn}_{dn}_{ease}', op, params, precision=PRECISIONS, objects=CAM_OBJECTS,
                returns=CAM_RET, time_bits=bits(1234.5678))
    for ms in [0, 1, 123456789, 0x80000000, 0xffffffff]:
        add(f'cam{op}_clock_{ms}', op, [kingdom_param(KINGDOMS[0]), P('ma_camera', 'mi_camera', 0x10007),
                                        P('ma_value', 'mi_fixed', bits(2.5))] + ([P('ma_boolean', 'mi_boolean', 1)] if op == 71 else []),
            precision=PRECISIONS, objects=CAM_OBJECTS, returns=dict(CAM_RET, RealTimeMs=ms), time_bits=bits(7.0))
add('cam71_bad_camera_warn', 71, [kingdom_param(KINGDOMS[0]), P('ma_camera', 'mi_camera', 0x10006),
                                 P('ma_value', 'mi_fixed', bits(2.5)), P('ma_boolean', 'mi_boolean', 1)],
    objects=CAM_OBJECTS, returns=CAM_RET, warn_invalid_ids=1)
add('cam71_logging', 71, [kingdom_param(KINGDOMS[0]), P('ma_camera', 'mi_camera', 0x10007),
                         P('ma_value', 'mi_fixed', bits(2.5)), P('ma_boolean', 'mi_boolean', 1)],
    objects=CAM_OBJECTS, returns=CAM_RET, camera_log=1)
VARS990 = {'0x2c': bits(0.8), '0x48': bits(-55.5), '0x50': bits(30.125)}
LOCS = [('loc', 'mi_location', [bits(10.5), bits(-20.25), bits(1.0)]), ('loc2', 'mi_location', [bits(0.0000004), bits(99999.5), 0])]
for k, (ln, li, lv), (dn, di, dv) in itertools.product(KINGDOMS[:3], LOCS, DURS[:3]):
    add(f'cam73_{k[0]}_{ln}_{dn}', 73, [kingdom_param(k), P('ma_location', li, lv), P('ma_value', di, dv)],
        precision=PRECISIONS, objects=CAM_OBJECTS, vars990=VARS990, time_bits=bits(88.25))

for enabled in [0, 1, 2]:
    add(f'cutscene42_{enabled}', 42, [P('ma_boolean', 'mi_boolean', enabled)], returns={'Camera700B10': bits(0.75)})
add('cutscene42_wrong_item', 42, [P('ma_boolean', 'mi_int', 1)], returns={'Camera700B10': bits(0.75)})

TECH_OBJ = world(tech_a={'kind': 'data'})
TECH_RET = {'DataByIds:property': {'by_arg': {'tech_a': '@tech_a'}}}
for k in KINGDOMS + [('all', 'mi_kingdom_all', 0)]:
    for tech in ['tech_a', 'missing']:
        for wk in [['kingdom_a', None, 'kingdom_b'], []]:
            add(f'tech52_{k[0]}_{tech}_{len(wk)}', 52, [kingdom_param(k), P('ma_technology', 'mi_technology', tech)],
                objects=TECH_OBJ, returns=TECH_RET, world_kingdoms=wk, manager=CTX)
        add(f'tech53_{k[0]}_{tech}', 53, [kingdom_param(k), P('ma_technology', 'mi_technology', tech)],
            objects=TECH_OBJ, returns=TECH_RET, manager=CTX)
for a in ACTORS:
    for tech in ['tech_a', 'missing']:
        add(f'tech55_{a[0]}_{tech}', 55, [actor_param(a), P('ma_technology', 'mi_technology', tech)],
            objects=TECH_OBJ, returns=TECH_RET, manager=CTX)

for text in ['Banner text', '']:
    for dn, di, dv in DURS:
        add(f'banner36_{len(text)}_{dn}', 36, [P('ma_localized_string', 'mi_localized_string', {'key': 'IDS_X', 'inline': text}),
                                               P('ma_value', di, dv)], precision=PRECISIONS)

SUN_OBJ = world(sun_a={'kind': 'data', 'words': {hex(0x18 + 4 * i): bits(0.1 * i) for i in range(9)}})
for name in ['sun_a', 'missing']:
    for dn, di, dv in DURS[:3]:
        add(f'sun46_{name}_{dn}', 46, [P('ma_sun', 'mi_sun', name), P('ma_value', di, dv)], objects=SUN_OBJ,
            returns={'DataByIds:sun': {'by_arg': {'sun_a': '@sun_a'}}}, precision=PRECISIONS)

for k in KINGDOMS[:3] + KINGDOMS[4:5]:
    for res in [[0, 1, 0], []]:
        add(f'res112_{k[0]}_{len(res)}', 112, [kingdom_param(k), P('ma_value', 'mi_fixed', bits(250.0))], resources=res)
for name, v in [('pos', bits(30.0)), ('zero', 0), ('negzero', 0x80000000), ('neg', bits(-1.0)), ('nan', 0x7fc00000),
                ('tiny', 1), ('int', None)]:
    params = [P('ma_value', 'mi_fixed', v)] if v is not None else [P('ma_value', 'mi_int', 45)]
    extra = {'rust_unsupported': 'non-finite value register (native passes NaN through 54D9E0)'} if name == 'nan' else {}
    add(f'counter69_{name}', 69, params, time_bits=bits(12.5), **extra)
add('counter70', 70, [])
for name in ['rain', 'missing']:
    add(f'weather108_{name}', 108, [P('ma_weather', 'mi_weather', name)], objects=world(rain={'kind': 'data'}),
        returns={'DataByIds:weather': {'by_arg': {'rain': '@rain'}}})
SOUND_RET = {'DataByIds:audio': {'by_arg': {'boom': '@speech_a'}}}
for name in ['boom', 'missing']:
    add(f'sound85_{name}', 85, [P('ma_sound', 'mi_sound', name)], returns=SOUND_RET)
    for ln, li, lv in LOCS + [('noloc', 'mi_int', 1)]:
        add(f'sound86_{name}_{ln}', 86, [P('ma_sound', 'mi_sound', name), P('ma_location', li, lv)], returns=SOUND_RET)
for a in ACTORS:
    for d8 in [0, 6]:
        add(f'flash109_{a[0]}_{d8}', 109, [actor_param(a)], manager=CTX, world_d8=d8)
    add(f'exp34_{a[0]}', 34, [actor_param(a), P('ma_value', 'mi_fixed', bits(15.5))], manager=CTX)
UI_OBJ = world(ui_root={'kind': 'misc'}, widget_a={'kind': 'misc', 'words': {'0x4': '$ButtonBuild'}})
for name in ['build', 'missing', '']:
    add(f'flashui110_{name or "empty"}', 110, [P('ma_script_command', 'mi_script_command', name)], objects=UI_OBJ,
        returns={'UiFindWidget': {'by_arg': {'build': '@widget_a'}}})
add('flashui110_noroot', 110, [P('ma_script_command', 'mi_script_command', 'build')])
for name in ['listen_me', '']:
    for listen in [0, 1]:
        for ret in [0, 1]:
            add(f'listen95_{name or "empty"}_{listen}_{ret}', 95, [P('ma_script_command', 'mi_script_command', name),
                                                                  P('ma_boolean', 'mi_boolean', listen)],
                returns={'WorldScriptUnlisten': ret})
VFX_OBJ = world(vfx_a={'kind': 'data'}, vfx_bad={'kind': 'data'})
VFX_RET = {'DataByIds:actor_data': {'by_arg': {'vfx_a': '@vfx_a', 'vfx_bad': '@vfx_bad'}}, 'IsVfx': {'vfx_a': 1}}
for name in ['vfx_a', 'vfx_bad', 'missing']:
    for ln, li, lv in LOCS + [('noloc', 'mi_int', 1)]:
        add(f'vfx82_{name}_{ln}', 82, [P('ma_vfx', 'mi_vfx', name), P('ma_location', li, lv)], objects=VFX_OBJ,
            returns=VFX_RET, vars990={'0x64': [bits(0.0), bits(1.0), bits(0.0)]})
for name in ['vfx_a', 'missing']:
    for ln, li, lv in LOCS[:1] + [('noloc', 'mi_int', 1)]:
        for rn, rv in [('zero', 0), ('negzero', 0x80000000), ('r90', bits(90.0)), ('rneg', bits(-12.5)), ('nan', 0x7fc00000)]:
            add(f'vfx83_{name}_{ln}_{rn}', 83, [P('ma_vfx', 'mi_vfx', name), P('ma_location', li, lv), P('ma_value', 'mi_fixed', rv)],
                objects=VFX_OBJ, returns=dict(VFX_RET, Rotate2D=[bits(0.7071), bits(-0.25)]),
                vars990={'0x64': [bits(0.0), bits(1.0), bits(0.0)]}, precision=PRECISIONS,
                **({'rust_unsupported': 'non-finite value register'} if rn == 'nan' else {}))
for k in [('all', 'mi_kingdom_all', 0)] + KINGDOMS[:3]:
    add(f'message47_{k[0]}', 47, [kingdom_param(k), P('ma_localized_string', 'mi_localized_string', {'key': 'IDS_MSG', 'inline': 'Hello there'})])
ANIM_OBJ = world(animset={'kind': 'misc'}, anim_a={'kind': 'misc'}, animarr={'kind': 'misc', 'words': {'0x0': 0, '0x4': '@animset'}},
                 def_anim={'kind': 'data', 'words': {'0x1c0': 2, '0x1bc': '@animarr'}}, def_none={'kind': 'data', 'words': {'0x1c0': 0}})
ANIM_OBJ['actor_a']['words'] = {'0x8': 0, '0x4': '@def_anim'}
ANIM_OBJ['actor_b']['words'] = {'0x8': 0, '0x4': '@def_none'}
for a in ACTORS[:3] + [('id_b', 'mi_actor_id', A2)]:
    for name in ['wave', 'missing', '']:
        for ret in [0, 1]:
            add(f'anim101_{a[0]}_{name or "empty"}_{ret}', 101, [actor_param(a), P('ma_animation', 'mi_animation', name)],
                objects=ANIM_OBJ, returns={'AnimationByName': {'by_arg': {'wave': '@anim_a'}}, 'ActorPlayAnimation': ret})

CMD_OBJ = world(cmd_a={'kind': 'data'}, cmd_move={'kind': 'data'}, body_a={'kind': 'misc', 'words': {'0x28': bits(1.25)}},
                org_b={'kind': 'misc', 'words': {'0xd8': bits(0.5)}})
CMD_OBJ['actor_a']['words'] = {'0x8': 0x40, '0x6c': '@body_a'}
CMD_OBJ['actor_b']['words'] = {'0x8': 0, '0x7c': '@org_b'}
CMD_RET = {'DataByIds:command': {'by_arg': {'attack': '@cmd_a'}}, 'ProcessCommand': 1,
           'DataByIds:actor_data': {'by_arg': {'data_a': '@data_a'}}, 'IsActorData': {'data_a': 1}}
for a in ACTORS[:3] + [('id_b', 'mi_actor_id', A2)]:
    for cmd in ['attack', 'missing']:
        add(f'cmd102_{a[0]}_{cmd}', 102, [actor_param(a), P('ma_command', 'mi_command', cmd)], objects=CMD_OBJ, returns=CMD_RET, manager=CTX)
        for ln, li, lv in LOCS[:1] + [('noloc', 'mi_int', 1)]:
            add(f'cmd103_{a[0]}_{cmd}_{ln}', 103, [actor_param(a), P('ma_command_location', 'mi_command', cmd), P('ma_location', li, lv)],
                objects=CMD_OBJ, returns=CMD_RET, manager=CTX)
            add(f'cmd104_{a[0]}_{cmd}_{ln}', 104, [actor_param(a), P('ma_command_actor_data_location', 'mi_command', cmd),
                                                   P('ma_actor_data', 'mi_actor_data_named', 'data_a'), P('ma_location', li, lv)],
                objects=CMD_OBJ, returns=CMD_RET, manager=CTX)
        for t in ACTORS[:3]:
            add(f'cmd105_{a[0]}_{cmd}_{t[0]}', 105, [actor_param(a), P('ma_command_actor', 'mi_command', cmd), actor_param(t)],
                objects=CMD_OBJ, returns=CMD_RET, manager=CTX)
    for ln, li, lv in LOCS + [('noloc', 'mi_int', 1)]:
        for dn, di, dv in DURS[:3]:
            add(f'move31_{a[0]}_{ln}_{dn}', 31, [actor_param(a), P('ma_location', li, lv), P('ma_value', di, dv)],
                objects=CMD_OBJ, returns=dict(CMD_RET, ProcessCommand=0), manager=CTX, precision=PRECISIONS,
                command_defs={'5': 'cmd_move'})

CREATE_OBJ = world(unit_a={'kind': 'data', 'words': {'0x174': 0}}, building_a={'kind': 'data', 'words': {'0x174': 0x80}})
CREATE_RET = {'DataByIds:actor_data': {'by_arg': {'unit_a': '@unit_a', 'building_a': '@building_a'}},
              'IsActorData': {'unit_a': 1, 'building_a': 1},
              'FindPlacement': {'ok': 1, 'pos': [bits(11.0), bits(-21.0)], 'facing': [bits(0.5), bits(0.25)]},
              'WorldCreateActor': '@actor_b', 'WorldCreateActor150': '@actor_a'}
DATA_ITEM = {29: ('ma_actor_data', 'mi_actor_data_named'), 74: ('ma_organization_data', 'mi_organization_data'),
             75: ('ma_unit_data', 'mi_unit_data'), 76: ('ma_building_data', 'mi_building_data')}
for op in [29, 74, 75, 76]:
    arg, item = DATA_ITEM[op]
    for data in ['unit_a', 'building_a', 'missing']:
        for k in KINGDOMS[:2] + KINGDOMS[4:5]:
            for ln, li, lv in LOCS[:1] + [('noloc', 'mi_int', 1)]:
                add(f'create{op}_{data}_{k[0]}_{ln}', op, [P(arg, item, data), kingdom_param(k), P('ma_location', li, lv),
                                                         P('ma_string', 'mi_string', 'new_name')],
                    objects=CREATE_OBJ, returns=CREATE_RET)
    add(f'create{op}_noplace', op, [P(arg, item, 'unit_a'), kingdom_param(KINGDOMS[0]), P('ma_location', *LOCS[0][1:]),
                                    P('ma_string', 'mi_string', '')], objects=CREATE_OBJ, returns=dict(CREATE_RET, FindPlacement={'ok': 0}))
    add(f'create{op}_fail', op, [P(arg, item, 'unit_a'), kingdom_param(KINGDOMS[0]), P('ma_location', *LOCS[0][1:]),
                                 P('ma_string', 'mi_string', 'x')], objects=CREATE_OBJ, returns=dict(CREATE_RET, WorldCreateActor=0))


# Remaining campaign conditions and actions ----------------------------------------------
LIFE_OBJ = world(body_a={'kind': 'misc', 'words': {'0x10': bits(40.0), '0x14': bits(80.0)}})
LIFE_OBJ['actor_a']['words'] = {'0x8': 0, '0x60': '@body_a'}
for a in ACTORS:
    for kind in [0, 1, 2, 3, 4, 5, 7]:
        for dn, di, dv in [('half', 'mi_fixed', bits(50.0)), ('third', 'mi_fixed', bits(33.333)), ('int', 'mi_int', 49),
                           ('neg', 'mi_fixed', bits(-1.0))]:
            add(f'life10_{a[0]}_{kind}_{dn}', 10, [actor_param(a), P('ma_compare', 'mi_compare', kind), P('ma_value', di, dv)],
                objects=LIFE_OBJ, manager=CTX, precision=PRECISIONS)
for a in ACTORS:
    for (an, ai, av) in AREA_REFS:
        for found in ['@actor_b', 0]:
            for inside in [0, 1]:
                add(f'inside14_{a[0]}_{an}_{str(found)[1:] or "none"}_{inside}', 14, [actor_param(a), P('ma_area', ai, av)],
                    manager=CTX, returns={'IsArea': {'area_a': 1}, 'AreaFirstActor': found, 'ActorInsideArea': inside})
SEL_OBJ = world(player_a={'kind': 'misc', 'words': {'0x50': '@sel_a'}}, player_b={'kind': 'misc', 'words': {'0x50': 0}},
                sel_a={'kind': 'misc', 'lists': {'0x4': ['@actor_b', 0, '@actor_a']}}, data_a={'kind': 'data'})
SEL_OBJ['actor_a']['words'] = {'0x8': 0, '0x4': '@data_a'}
SEL_OBJ['actor_b']['words'] = {'0x8': 0, '0x4': '@data_b'}
SEL_RET = {'KingdomPlayer': {'by_arg': {'@kingdom_a': '@player_a', '@kingdom_b': '@player_b'}},
           'DataByIds:actor_data': {'by_arg': {'data_a': '@data_a', 'data_b': '@data_b'}}, 'IsActorData': {'data_a': 1, 'data_b': 1}}
for a, k in itertools.product(ACTORS, KINGDOMS[:3] + KINGDOMS[5:6]):
    add(f'selected18_{a[0]}_{k[0]}', 18, [actor_param(a), kingdom_param(k)], objects=SEL_OBJ, manager=CTX, returns=SEL_RET)
for d, k in itertools.product(DATA[1:], KINGDOMS[:3] + KINGDOMS[5:6]):
    add(f'dataselected19_{d[0]}_{k[0]}', 19, [P('ma_actor_data', d[1], d[2]), kingdom_param(k)], objects=SEL_OBJ, manager=CTX,
        returns=SEL_RET)
LOCK_OBJ = world(cmd_stop={'kind': 'data'}, thing_a={'kind': 'misc', 'words': {'0x10': 1}}, thing_b={'kind': 'misc', 'words': {'0x10': 0}})
LOCK_OBJ['kingdom_a']['lists'] = {'0x284': ['@actor_a', '@actor_b']}
LOCK_OBJ['kingdom_b']['lists'] = {'0x284': ['@actor_b']}
LOCK_OBJ['actor_a']['words'] = {'0x8': 0, '0x80': '@thing_a'}
LOCK_OBJ['actor_b']['words'] = {'0x8': 0, '0x80': '@thing_b'}
for op in [66, 67]:
    for slots in [['kingdom_a', None, 'kingdom_b'], []]:
        add(f'lock{op}_{len(slots)}', op, [], objects=LOCK_OBJ, world_148=slots, command_defs={'6': 'cmd_stop'},
            returns={'ProcessCommand': 1})
for name in ['intro', '']:
    add(f'cutscene94_{name or "empty"}', 94, [P('ma_string', 'mi_string', name)])

# 30 actor_create_transported_named (543BA0) --------------------------------------------
TR_OBJ = world(transport_def={'kind': 'data', 'words': {'0x1c': 0, '0x414': bits(12.5)}},
               air_def={'kind': 'data', 'words': {'0x1c': 0x1000, '0x414': bits(30.0)}},
               shore_def={'kind': 'data', 'words': {'0x1c': 0x200000, '0x414': 0}},
               cargo={'kind': 'data', 'words': {'0x184': '@transport_def', '0x174': 0, '0x2f0': 77}},
               cargo_air={'kind': 'data', 'words': {'0x184': '@air_def', '0x174': 0x80, '0x2f0': 78}},
               cargo_shore={'kind': 'data', 'words': {'0x184': '@shore_def', '0x174': 0, '0x2f0': 79}},
               cargo_none={'kind': 'data', 'words': {'0x184': 0}},
               boat={'kind': 'actor', 'slot': 9, 'gen': 1, 'words': {'0x8': 0x10, '0xc0': '@boat_slot'}},
               boat_slot={'kind': 'misc'}, cmd_move={'kind': 'data'})
TR_RET = {'DataByIds:actor_data': {'by_arg': {n: '@' + n for n in ['cargo', 'cargo_air', 'cargo_shore', 'cargo_none']}},
          'IsActorData': {n: 1 for n in ['cargo', 'cargo_air', 'cargo_shore', 'cargo_none']},
          'FindPlacement': {'ok': 1, 'pos': [bits(101.0), bits(-50.5)], 'facing': [bits(0.6), bits(0.8)]},
          'WorldCreateTransport': '@boat', 'TransportPath': [bits(150.25), bits(-20.0)], 'TransportPathValid': 1,
          'ProcessCommand': 1}
SPAWNS = [('s1', [bits(100.0), bits(-50.0), bits(2.0)]), ('s2', [bits(0.0), bits(0.0), bits(0.0)])]
DESTS = [('d1', [bits(160.0), bits(-10.0), 0]), ('d2', [bits(100.0), bits(-50.0), 0]), ('d3', [bits(3.0), bits(4.0), 0])]
for cargo in ['cargo', 'cargo_air', 'cargo_shore', 'cargo_none', 'missing']:
    for k in KINGDOMS[:2]:
        for (sn, sv), (dn, dv) in itertools.product(SPAWNS, DESTS):
            add(f'transport30_{cargo}_{k[0]}_{sn}_{dn}', 30,
                [P('ma_actor_data', 'mi_actor_data_named', cargo), kingdom_param(k),
                 P('ma_location', 'mi_location', sv), P('ma_location', 'mi_location', dv), P('ma_string', 'mi_string', 'convoy')],
                objects=TR_OBJ, returns=TR_RET, command_defs={'5': 'cmd_move'}, precision=PRECISIONS)
for name, change in [('noplace', {'FindPlacement': {'ok': 0}}), ('nocreate', {'WorldCreateTransport': 0}),
                     ('badpath', {'TransportPathValid': 0})]:
    for cargo in ['cargo', 'cargo_shore']:
        add(f'transport30_{name}_{cargo}', 30, [P('ma_actor_data', 'mi_actor_data_named', cargo),
                                              kingdom_param(KINGDOMS[0]), P('ma_location', 'mi_location', SPAWNS[0][1]),
                                              P('ma_location', 'mi_location', DESTS[0][1]), P('ma_string', 'mi_string', 'x')],
            objects=TR_OBJ, returns=dict(TR_RET, **change), command_defs={'5': 'cmd_move'})
add('transport30_noloc', 30, [P('ma_actor_data', 'mi_actor_data_named', 'cargo'), kingdom_param(KINGDOMS[0]),
                              P('ma_location', 'mi_int', 1), P('ma_location', 'mi_location', DESTS[0][1]), P('ma_string', 'mi_string', 'x')],
    objects=TR_OBJ, returns=TR_RET)

# 77/78 team flares (544A50/544BA0) and 96 ability use (544040) ---------------------------
FL_OBJ = world(flare_cfg={'kind': 'misc'}, flare_a={'kind': 'data', 'words': {'0x57c': '@flare_cfg'}},
               ability_a={'kind': 'data'})
FL_OBJ['actor_a']['words'] = {'0x8': 0, '0x20': bits(5.5), '0x24': bits(-6.25), '0x28': bits(1.0)}
FL_RET = {'DataByIds:actor_data': {'by_arg': {'flare_a': '@flare_a'}}, 'IsActorData': {'flare_a': 1},
          'FlareReadyPosition': 1, 'FlareReadyActor': 1, 'FlareVar': bits(45.0),
          'DataByIds:ability': {'by_arg': {'ability_a': '@ability_a'}}}
for name in ['flare_a', 'missing']:
    for k in KINGDOMS[:2]:
        for ready in [0, 1]:
            for dn, di, dv in DURS[:3]:
                for ln, li, lv in LOCS[:1] + [('noloc', 'mi_int', 1)]:
                    add(f'flare77_{name}_{k[0]}_{ready}_{dn}_{ln}', 77,
                        [P('ma_flare_data_location', 'mi_flare_data_location', name), kingdom_param(k), P('ma_location', li, lv),
                         P('ma_value', di, dv)], objects=FL_OBJ, returns=dict(FL_RET, FlareReadyPosition=ready))
                for a in ACTORS[:3]:
                    add(f'flare78_{name}_{k[0]}_{ready}_{dn}_{a[0]}', 78,
                        [P('ma_flare_data_actor', 'mi_flare_data_actor', name), kingdom_param(k), actor_param(a),
                         P('ma_value', di, dv)], objects=FL_OBJ, returns=dict(FL_RET, FlareReadyActor=ready), manager=CTX)
for name in ['ability_a', 'missing']:
    for k in KINGDOMS[:2] + KINGDOMS[4:5]:
        for (fn, fi, fv), (tn, ti, tv) in itertools.product(LOCS[:1] + [('noloc', 'mi_int', 1)], LOCS):
            add(f'ability96_{name}_{k[0]}_{fn}_{tn}', 96, [P('ma_ability', 'mi_ability', name), kingdom_param(k),
                                                         P('ma_location', fi, fv), P('ma_location', ti, tv)],
                objects=FL_OBJ, returns=FL_RET)

# Trailer/skirmish-only conditions 22/99/100 -----------------------------------------------
for k in KINGDOMS:
    for tech in ['tech_a', 'missing']:
        for has in [0, 1, 0x100]:
            add(f'hastech22_{k[0]}_{tech}_{has}', 22, [kingdom_param(k), P('ma_technology', 'mi_technology', tech)],
                objects=TECH_OBJ, returns=dict(TECH_RET, KingdomHasProperty=has), manager=CTX)
for op, call in [(99, 'IsVisibleTo'), (100, 'IsExploredBy')]:
    for a, k in itertools.product(ACTORS, KINGDOMS[:3] + KINGDOMS[5:6]):
        for v in [0, 1, 0x100]:
            add(f'visible{op}_{a[0]}_{k[0]}_{v}', op, [actor_param(a), kingdom_param(k)], manager=CTX, returns={call: v})

if __name__ == '__main__':
    h.run_cases(ROWS, HERE / 'native-elements.json',
                'Actual 546AF0 element opcodes 15,17,23,28,40,50,51,56,57,58,59,60,79,87,97,98,107 with actual selectors, '
                'TriggerElement leaves, Kingdom objective list methods (4E7050/4E2120/4E21F0/4E2250/4E22B0/4E2020/4F1AD0) '
                'and Kingdom mission result 4E9720.',
                sorted({v[0] for v in h.SEAMS.values()}) + ['virtual ' + v[0] for v in h.VIRTUALS.values()])
