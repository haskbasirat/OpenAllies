from pathlib import Path
P=Path(__file__).resolve().parent
basis=P.parent/'skirmish-20261017-fx-object-animation/native_fx_object_animation.py'
(P/'basis-native-type8.py').write_bytes(basis.read_bytes())
s=basis.read_text(encoding='utf-8')
a=s.index('def cases():');b=s.index('class NativeThrow',a)
s=s[:a]+'''def subtype(kind,**replace):
 d=node(type=FXC[kind],action='UNHIDE',location='head',equipment_type_ids='',equipment_IDS='',audio_ids='select',aspect_ids='summer',skin='',radar_icon='radar.tga',src_color=[0x3f800000,0,0],dst_color=[0,0x3f800000,0],blink_count=3,pivot='head',axis=[0,0,0x3f800000],max_angle=0x3f800000,max_impulse=0x40000000,max_damage=0x40400000,vel_decay=0x3f000000,vel_falloff=0x3e800000,stiffness=0x40800000)
 d.update(replace);return d
def cases():
 out=[]
 fields={2:['action','location','equipment_type_ids','equipment_IDS'],4:['audio_ids'],5:['aspect_ids'],6:['skin'],7:['radar_icon','src_color','dst_color','blink_count','at'],9:['pivot','axis','max_angle','max_impulse','max_damage','vel_decay','vel_falloff','stiffness']}
 for kind,names in fields.items():
  out.append(dict(name=f'type{kind}_ordinary',nodes=[subtype(kind)],roots=[0]))
  for field in names+['health']:
   out.append(dict(name=f'type{kind}_required_{field}',nodes=[subtype(kind)],roots=[0],fail=field))
 for action in ['HIDE','UNHIDE','FLIP','REMOVE','ADD','REPLACE','missing']:
  for equipment in ['', 'hat', 'missing']:
   out.append(dict(name='equipment_'+action+'_'+equipment,nodes=[subtype(2,action=action,equipment_IDS=equipment)],roots=[0]))
 for field in ['location','equipment_type_ids']:
  for text in ['','head','missing']:
   out.append(dict(name=field+'_'+text,nodes=[subtype(2,**{field:text})],roots=[0]))
 for kind,field in [(4,'audio_ids'),(5,'aspect_ids'),(6,'skin'),(7,'radar_icon'),(9,'pivot')]:
  for text in ['', 'missing', 'valid.tga']:
   out.append(dict(name=f'type{kind}_{text}',nodes=[subtype(kind,**{field:text})],roots=[0]))
 for field in ['src_color','dst_color']:
  for channel in [0,0x3f800000,0x3f000000,0x3b808081,0xbf800000,0x3f800001]:
   out.append(dict(name=field+'_'+str(channel),nodes=[subtype(7,**{field:[channel,0x3f000000,0x3f800000]})],roots=[0]))
 for stop in ['texture_path','texture','texture_push','texture_read','texture_render']:
  out.append(dict(name='rfx_'+stop,nodes=[subtype(7)],roots=[0],fail=stop))
 out += [dict(name='mixed_children',nodes=[subtype(2,ChainImmediate=[1,2],Chain=[3]),subtype(4),subtype(5),subtype(9)],roots=[0,1]),dict(name='partial_nested',nodes=[subtype(2,Chain=[1,2]),subtype(5),subtype(4,audio_ids='missing')],roots=[0])]
 return out
''' +s[b:]
s=s.replace("source_indices[game+0x2c0]='ActorState'","source_indices[game+0x2c0]='ActorState'\n source_indices[game+0x1d8]='EquipmentType';source_indices[game+0x224]='Equipment'")
s=s.replace("m.seam(0x5b4ae0,lambda mm:mm.ret_st0_f32(input_value(mm,text(mm.arg(1))),4))","m.seam(0x5b4ae0,lambda mm:mm.ret_st0_f32(input_value(mm,text(mm.arg(1))),4))\n m.seam(0x5b5130,lambda mm:mm.ret_st0_f32(input_value(mm,text(mm.arg(1))),4))")
s=s.replace("0xb39d78:('HealthTrigger',HEALTH)","0xb39d78:('HealthTrigger',HEALTH),0xb3b0f0:('EquipmentAction',['HIDE','UNHIDE','FLIP','REMOVE','ADD','REPLACE'])")
s=s.replace("references={};reference_values={}",'''references={};reference_values={};texture_values={}
 def texture_path(mm):
  value=text(m.r(mm.arg(1)));events.append(['texture_path',value])
  if c.get('fail')=='texture_path':raise NativeThrow('required texture_path')
  mm.ret(pop=4)
 m.seam(0x4763c0,texture_path)
 def texture(mm):
  value=text(m.r(mm.arg(1)));events.append(['texture',value,mm.arg(2)])
  if c.get('fail')=='texture':raise NativeThrow('required texture')
  p=0 if value=='missing' else m.alloc(0x80)
  if p:texture_values[p]=value
  mm.ret(p)
 m.seam(0x5a77e0,texture)
 for address,label in [(0x5a6d10,'texture_push'),(0x5a9330,'texture_read'),(0x5a6340,'texture_render')]:
  def policy(mm,label=label):
   pointer=mm.ecx()
   if pointer==0:raise NativeThrow('required null Texture at5A6D10')
   assert pointer in texture_values
   events.append([label,texture_values[pointer]])
   if c.get('fail')==label:raise NativeThrow('required '+label)
   mm.ret(pop=4)
  m.seam(address,policy)
''')
a=s.index(' roots=linked(lists);out=[]');b=s.index(' return dict(error=error',a)
s=s[:a]+''' roots=linked(lists);out=[]
 vtables={0x9fc24c:0,0x9fc280:1,0x9fc2b4:2,0x9fc1e4:3,0x9fc218:4,0x9fc384:5,0x9fc3b8:6,0x9fc2e8:7,0x9fc31c:8,0x9fc350:9}
 for p in allocated:
  kind=vtables[m.r(p)];immediate=linked(p+8);chain=linked(p+0x10)
  offsets={0:[0x40,0x44,0x48,0x4c,0x50,0x54,0x58,0x5c,0x60,0x68],1:[],2:[0x3c,0x40],3:[0x3c],4:[],5:[],6:[],7:[0x40,0x44,0x48,0x4c],8:[],9:list(range(0x40,0x64,4))}[kind]
  refs={0:[0x3c],1:[],2:[0x44,0x48],3:[0x44],4:[0x3c],5:[],6:[],7:[],8:[],9:[0x3c]}[kind]
  byte_offsets=[4,0x38]+([0x64,0x65] if kind==0 else [0x40] if kind==3 else [])
  words={str(off):m.r(p+off) for off in [0x18,0x1c,0x24,0x28,0x2c,0x30,0x34]+offsets if m.r(p+off)!=0xcdcdcdcd}
  byte={str(off):m.rb(p+off) for off in byte_offsets if m.rb(p+off)!=0xcd}
  refs={str(off):reference_values.get(m.r(p+off)) for off in [0x20]+refs if m.r(p+off)!=0xcdcdcdcd}
  item=dict(kind=kind,words=words,bytes=byte,references=refs,immediate=immediate,chain=chain)
  if kind in [5,8]:item['texts']={'60':text(m.r(p+0x3c))}
  if kind in [6,7]:item['textures']={'60':texture_values.get(m.r(p+0x3c))} if m.r(p+0x3c)!=0xcdcdcdcd else {}
  item['active']=m.call(m.r(m.r(p)+0x20),p,[])&255
  out.append(item)
 for p,item in zip(allocated,out):item['complete']=p in complete
''' +s[b:]
s=s.replace("scope=__doc__,basis_sha256=hashlib.sha256((HERE/'basis-native-fx-sound.py').read_bytes()).hexdigest()","scope=__doc__,basis_sha256=hashlib.sha256((HERE/'basis-native-type8.py').read_bytes()).hexdigest()")
s=s.replace("path=HERE/'native-fx-object-animation.json'","path=HERE/'native-fxc-subtypes.json'")
s=s.replace("path.write_text(json.dumps(doc,indent=2)+'\\n')","path.open('x',encoding='utf-8').write(json.dumps(doc,indent=2)+'\\n')")
s=s.replace('"""Actual type8 factory42B6E0/constructor42B130/base42C1B0/loader42CF10.','"""Actual factory/constructors/base and derived LoadData for types2/4/5/6/7/9.\nTexture allocation/path/current-policy are explicit services. Native490E50\ncolor packing and Fixed stores execute. No GPU/effect execution claim.')
(P/'native_fxc_subtypes.py').write_text(s,encoding='utf-8')
