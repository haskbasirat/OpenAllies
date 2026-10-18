"""Synthetic memory translation smoke. Not captured actual owner proof."""
import json,random
import native_actual_performer as mod
rows=[]
for pc in mod.h.PRECISIONS:
 for i in range(6):
  before=mod.old.performer_case(mod.old.Oracle(pc),random.Random(0x574300+i),i)
  if before['log_enabled']:continue
  settings=before['settings'];svars=before['svars']
  c=dict(property_values=before['property_values'],modifier=None if before['modifier']is None else[before['modifier'].get(hex(offset),0)for offset in range(4,0x68,4)],body=before['body'],speed=before['speed'],bonus=before['bonus_210'],before=0x7F7FFFFF,
   abilities=[dict(enabled=r['enabled'],source=dict(definition=r['definition'],local={k:v for k,v in r['local'].items()if k!='form'}))for r in before['abilities']],
   secondary=None if before['secondary']is None else[dict(enabled=r['enabled'],source=dict(definition=r['definition'],local={k:v for k,v in r['local'].items()if k!='form'}))for r in before['secondary']],
   current_words=[settings[n]for n in ['damage_minimum','range_power','area_power','offense_weight','defense_weight','movement_weight','movement_default']]+[svars['CombatValueRangedMinimum'],svars['CombatValueDefenseWeightRanged'],settings['clamp']],
   cached_svar_guards=[False,False],environment_controls=dict(sse2=True,mxcsr=0x1F80,x87_control=0x27F),means=dict(maximum_74=0,attack_rate=0,health=0,**before['means']))
  a,pa=mod.run('original',pc,c);b,pb=mod.run('v14',pc,c)
  assert a==b and pa==pb and a['value']==before['value'],(pc,i,a['value'],before['value'])
  rows.append(dict(precision=pc,index=i,value=a['value']))
print(json.dumps(dict(scope=__doc__,rows=len(rows))))
