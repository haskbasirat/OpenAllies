import sys, json
import native_audio_manager as n
cs = dict(n.cases())
name = sys.argv[1]
m, steps = n.run('Double53', name, cs[name])
for i,(st,s) in enumerate(zip([None]+cs[name]['steps'], steps)):
    print('STEP', i, st, 'result', s['result'])
    for t in s['trace']:
        print('   ', t)
    S=s['state']
    print('   lists', S['lists'], 'queue', S['queue'], 'avail', S['available'], 'flags', S['flags'], 'music/speech', S['music_38'], S['speech_3c'])
    for k,v in S['instances'].items():
        print('   inst', k, {x:v[x] for x in ('data','state_34','current','channel','fade_64','released_39','volume_70','mode_1c')})
