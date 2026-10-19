import sys, json, traceback
sys.argv=['x']
import native_audio_manager as n
cs = dict(n.cases())
name = sys.argv[1] if len(sys.argv)>1 else '2d_life_cycle'
for name in list(cs)[:30]:
    try:
        m, steps = n.run('Double53', name, cs[name])
        print(name, 'ok', len(steps), 'instructions', len(m.visited))
    except Exception as e:
        print(name, 'FAIL', repr(e)[:300])
        traceback.print_exc(limit=3)
        break
