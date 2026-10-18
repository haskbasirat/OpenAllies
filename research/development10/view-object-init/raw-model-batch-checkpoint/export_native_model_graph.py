"""Project exact selected NIF source fields for the static modern model backend.

This parses one immutable byte allocation with the existing PyFFI reader. It
exports binary32 words, not transformed/rounded converted mesh coordinates.
Input SHA, converted SHA, canonical material-audit SHA and every audited mesh's
source-node ordinal bind this projection to the existing paired render artifact.
It is not a native NiStream ABI or unsupported dynamic-node implementation.
"""
from pathlib import Path
import sys,io,json,struct,hashlib,argparse,time,uuid,collections
from concurrent.futures import ProcessPoolExecutor
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[2];GAME=ROOT.parent
sys.path.insert(0,str(GAME/'reconstruction/tools'))
sys.path.insert(0,str(ROOT/'research/texture-phase2'))
from convert_models import NifFormat,ORIGINAL,string
from export_model_materials import extras,tag_evidence,next_context,TARGETS
from index_native_graphs import build_index,write_atomic

def sha(b):return hashlib.sha256(b).hexdigest()
def w(f):return struct.unpack('<I',struct.pack('<f',f))[0]
def xyz(v):return [w(v.x),w(v.y),w(v.z)]
def text(v):
 s=string(v)
 if not s.isascii() or '\0'in s:raise ValueError('Non-ASCII/embedded NUL NIF string conversion requires actual native loader')
 return s
def source_extras(node):
 rows=extras(node);items=list(getattr(node,'extra_data_list',[]));linked=getattr(node,'extra_data',None);seen=set()
 while linked is not None and id(linked)not in seen:
  seen.add(id(linked));items.append(linked);linked=getattr(linked,'next_extra_data',None)
 seen.clear();unique=[]
 for item in items:
  if item is not None and id(item)not in seen:seen.add(id(item));unique.append(item)
 assert len(rows)==len(unique)
 for row,item in zip(rows,unique):
  row['kind']=type(item).__name__
  if isinstance(item,NifFormat.NiIntegerExtraData):row['integer']=int(item.integer_data)
  if isinstance(item,NifFormat.NiBooleanExtraData):row['boolean']=bool(item.boolean_data)
  if isinstance(item,NifFormat.NiBinaryExtraData):row['binary']=bytes(item.binary_data).hex()
  if isinstance(item,NifFormat.NiStringsExtraData):row['strings']=[text(x)for x in item.data]
 return rows

def project(relative,audit,audit_sha):
 path=ORIGINAL/relative;source=path.read_bytes();data=NifFormat.Data();data.read(io.BytesIO(source))
 converted_path=GAME/'reconstruction/assets/converted/models'/Path(relative).with_suffix('.json')
 converted=converted_path.read_bytes()
 record=audit['models'][relative]
 assert sha(source)==record['source_sha256'] and sha(converted)==record['converted_sha256']
 nodes=[];seen={};active=set();blocks={id(b):i for i,b in enumerate(data.blocks)}
 def walk(node,parent=None,hierarchy=(),context=None):
  if node is None:return None
  if not isinstance(node,NifFormat.NiAVObject):raise ValueError('NonAV root/child '+type(node).__name__)
  if id(node) in seen:raise ValueError('Repeated/aliased AV object requires graph alias semantics')
  index=len(nodes);seen[id(node)]=index;name=text(node.name);hierarchy=hierarchy+(name,)
  rotation=[w(getattr(node.rotation,key))for key in ['m_11','m_21','m_31','m_12','m_22','m_32','m_13','m_23','m_33']]
  rawextras=source_extras(node);own=tag_evidence(rawextras,list(hierarchy));state=next_context(isinstance(node,NifFormat.NiGeometry),context or {},own)
  tags=[]
  for item in rawextras:
   if item['extra_name']in ['Mod','Eq','Ambient_RepColor','EmRC','RepColor','FastPath','tred','Sg','Sg2','mkf']:tags.append(item['extra_name'])
   if item['is_string']:
    for line in item['value'].split('\r\n'):
     candidate=line.rsplit('=',1)[0]
     for known in ['Mod','Eq','Ambient_RepColor','EmRC','RepColor','FastPath','tred','Sg','Sg2','mkf']:
      if candidate.isascii() and candidate.lower()==known.lower():tags.append(known)
  row=dict(kind=type(node).__name__,name=name,parent=parent,hierarchy=list(hierarchy),
    local=rotation+xyz(node.translation)+[w(node.scale)],flags=int(node.flags),children=[],
    vertices=[],normals=[],sphere=None,skin=getattr(node,'skin_instance',None)is not None,
    shadow='shadow'in state,team='team'in state,tags=tags,extras=rawextras,
    properties=[],controllers=[],data_block=None)
  nodes.append(row)
  ctrl=getattr(node,'controller',None);controllers=set()
  while ctrl is not None:
   if id(ctrl)in controllers:raise ValueError('cyclic Controller list')
   controllers.add(id(ctrl));row['controllers'].append(dict(kind=type(ctrl).__name__,flags=int(ctrl.flags),stop=w(ctrl.stop_time)))
   ctrl=getattr(ctrl,'next_controller',None)
  for prop in node.properties:
   if prop is None:continue
   p=dict(kind=type(prop).__name__,block=blocks[id(prop)],extras=source_extras(prop))
   if isinstance(prop,NifFormat.NiAlphaProperty):p['flags']=int(prop.flags);p['threshold']=int(prop.threshold)
   if isinstance(prop,NifFormat.NiTexturingProperty):
    #These are the seven actual serialized descriptor slots. Presence is
    #different from descriptor.filter;5A9840 reads slot3 pointer presence.
    slots=['base','dark','detail','gloss','glow','bump_map','decal_0']
    p['slots']=[]
    for slot in slots:
     descriptor=getattr(prop,slot+'_texture',None) if getattr(prop,'has_'+slot+'_texture',False) else None
     if descriptor is None:p['slots'].append(None);continue
     texture=getattr(descriptor,'source',None)
     p['slots'].append(dict(texture=None if texture is None else text(texture.file_name),source=None if texture is None else blocks[id(texture)],filter=int(descriptor.filter_mode)))
    p['shader_textures']=int(getattr(prop,'num_shader_textures',0))
   row['properties'].append(p)
  if isinstance(node,NifFormat.NiGeometry) and node.data is not None:
   row['data_block']=blocks[id(node.data)]
   row['vertices']=[xyz(v)for v in node.data.vertices]
   row['normals']=[xyz(v)for v in node.data.normals]if node.data.has_normals else []
   row['sphere']=xyz(node.data.center)+[w(node.data.radius)]
   row['has_normals']=bool(node.data.has_normals)
  row['children']=[walk(child,index,hierarchy,state)for child in getattr(node,'children',[])]
  return index
 roots=[walk(root)for root in data.roots]
 textures=[dict(block=i,filename=None if not b.use_external else text(b.file_name),extras=source_extras(b))for i,b in enumerate(data.blocks)if isinstance(b,NifFormat.NiSourceTexture)]
 return dict(schema=2,source=relative,source_sha256=sha(source),converted_sha256=sha(converted),
  material_audit_sha256=audit_sha,version=data.version,roots=roots,nodes=nodes,
  textures=textures,mesh_mapping=[{k:r[k]for k in ['mesh_index','source_node_ordinal','name','hierarchy','vertex_count','triangle_count']}for r in record['corrections']])

def artifact_name(source,source_sha,aliases):
 suffix='-'+sha(source.lower().encode())[:16]if aliases[source_sha]>1 else''
 return source_sha+suffix+'.json'

def export_one(job):
 source,entry,audit_sha,artifact,staged=job
 try:
  record=project(source,dict(models={source:entry}),audit_sha)
  raw=(json.dumps(record,separators=(',',':'),ensure_ascii=False)+'\n').encode()
  kinds=collections.Counter(n['kind']for n in record['nodes'])
  tags=sorted({t for n in record['nodes']for t in n['tags']})
  #This is a conservative inventory, not a runtime success certificate.
  #Every exported unknown class/skin remains explicit for the backend to refuse.
  unsupported_kinds=sorted(set(kinds)-{'NiNode','NiTriShape','NiTriStrips'})
  skins=sum(bool(n['skin'])for n in record['nodes'])
  active_controllers=sum(bool(c['flags']&1)for n in record['nodes']for c in n['controllers'])
  unsupported_tags=sorted(set(tags)-{'RepColor','Sg','Sg2','Eq'})
  candidate=record['version']==0x0a010000 and not unsupported_kinds and not skins and not active_controllers and not unsupported_tags
  path=Path(staged)/artifact
  with path.open('xb')as f:f.write(raw)
  return dict(source=source,source_sha256=record['source_sha256'],artifact=artifact,projection_sha256=sha(raw),bytes=len(raw),nodes=len(record['nodes']),node_kinds=dict(kinds),version=record['version'],skin_nodes=skins,active_controller_count=active_controllers,unsupported_kinds=unsupported_kinds,unsupported_tags=unsupported_tags,static_scope_candidate=candidate)
 except Exception as error:
  return dict(source=source,source_sha256=entry['source_sha256'],error_class=type(error).__name__,error=str(error))

def main():
 ap=argparse.ArgumentParser(description=__doc__);ap.add_argument('--source',action='append');ap.add_argument('--all-audited',action='store_true');ap.add_argument('--workers',type=int,default=4);args=ap.parse_args()
 if args.all_audited and args.source:ap.error('--all-audited and --source are mutually exclusive')
 if args.workers<1:ap.error('--workers must be positive')
 audit_path=ROOT/'assets-cache/model-materials.json';audit_bytes=audit_path.read_bytes();audit=json.loads(audit_bytes);audit_sha=sha(audit_bytes)
 manifest_path=GAME/'reconstruction/assets/converted/models/manifest.json';manifest_bytes=manifest_path.read_bytes()
 if sha(manifest_bytes)!=audit['manifest_sha256']:raise ValueError('Current converted manifest differs from exact material audit')
 manifest=json.loads(manifest_bytes);manifest_sources=[r['source']for r in manifest['models']]
 if len(set(p.lower()for p in manifest_sources))!=len(manifest_sources):raise ValueError('Canonical converted source identities are ambiguous')
 sources=manifest_sources if args.all_audited else(args.source or TARGETS)
 if args.all_audited and(set(sources)!=set(audit['models'])or len(sources)!=1847 or not audit['complete']):raise ValueError('Comprehensive batch requires the exact complete1847 source audit')
 if len(set(s.lower()for s in sources))!=len(sources)or any(s not in audit['models']for s in sources):raise ValueError('Unregistered/duplicate selected source')
 aliases=collections.Counter(v['source_sha256']for v in audit['models'].values())
 output=ROOT/'assets-cache/model-native-graphs';output.mkdir(exist_ok=True)
 if output.is_symlink():raise ValueError('Refusing linked output directory')
 staged=output/('.batch-'+uuid.uuid4().hex);staged.mkdir()
 golden={name:sha((output/(audit['models'][name]['source_sha256']+'.json')).read_bytes())for name in TARGETS if(output/(audit['models'][name]['source_sha256']+'.json')).is_file()}
 producer=sha(Path(__file__).read_bytes());index_producer=sha((HERE/'index_native_graphs.py').read_bytes())
 jobs=[(s,audit['models'][s],audit_sha,artifact_name(s,audit['models'][s]['source_sha256'],aliases),str(staged))for s in sources]
 if len(set(j[3]for j in jobs))!=len(jobs):raise ValueError('Per-source artifact identities collided')
 started=time.perf_counter();rows=[]
 with ProcessPoolExecutor(max_workers=args.workers)as pool:
  for i,row in enumerate(pool.map(export_one,jobs)):
   rows.append(row)
   if(i+1)%25==0 or 'error'in row or i+1==len(jobs):print(json.dumps(dict(done=i+1,total=len(jobs),failures=sum('error'in r for r in rows),seconds=round(time.perf_counter()-started,2),last=row['source'],error=row.get('error'))),flush=True)
 errors=[r for r in rows if'error'in r];successful=[r for r in rows if'error'not in r]
 if sha(audit_path.read_bytes())!=audit_sha or manifest_path.read_bytes()!=manifest_bytes or sha(Path(__file__).read_bytes())!=producer or sha((HERE/'index_native_graphs.py').read_bytes())!=index_producer:raise ValueError('Input manifest/audit/exporter changed during batch; staged owners retained')
 if any(r['source']in golden for r in errors):raise ValueError('Existing golden graph failed projection; staged owners retained without index publication')
 for row in successful:
  path=staged/row['artifact'];raw=path.read_bytes()
  if sha(raw)!=row['projection_sha256']:raise ValueError('Staged graph changed before publication')
  if row['source']in golden and sha(raw)!=golden[row['source']]:raise ValueError('Existing three native golden raw graph values changed; batch retained without index publication')
 if args.all_audited:build_index(staged,[staged/r['artifact']for r in successful])
 for row in successful:
  destination=output/row['artifact']
  if destination.is_symlink():raise ValueError('Refusing linked artifact output')
  if destination.is_file()and sha(destination.read_bytes())==row['projection_sha256']:(staged/row['artifact']).unlink()
  else:(staged/row['artifact']).replace(destination)
 #Only the final atomic index makes new artifacts discoverable. All failures
 #remain absent; there is no invented empty graph for an unparsed source.
 paths=[output/r['artifact']for r in successful]if args.all_audited else None
 index=build_index(output,paths);index_sha=write_atomic(output/'index.json',index)
 error_classes=dict(collections.Counter(r['error_class']for r in errors));kind_counts=collections.Counter()
 for row in successful:kind_counts.update(row['node_kinds'])
 report=dict(scope='Comprehensive immutable-source projection superset, not reached firstMap/GameReady/render acceptance. Unknown classes are preserved; failed sources have no graph.',schema=2,all_audited=args.all_audited,requested=len(sources),raw_graph_success=len(successful),projection_failures=len(errors),failure_classes=error_classes,static_scope_candidates=sum(r['static_scope_candidate']for r in successful),candidate_scope='Conservative kind/skin/controller/tag/version inventory only; not whole native metadata or runtime load success.',node_kinds=dict(kind_counts),source_alias_groups=sum(c>1 for c in aliases.values()),audit_sha256=audit_sha,converted_manifest_sha256=sha(manifest_bytes),producer_sha256=producer,index_producer_sha256=index_producer,index_sha256=index_sha,golden_three_sha256=golden,elapsed_seconds=time.perf_counter()-started,rows=rows)
 receipt=ROOT/'research/development10/view-object-init/raw-model-batch.json';receipt_sha=write_atomic(receipt,report)
 staged.rmdir()
 print(json.dumps(dict(report=str(receipt),report_sha256=receipt_sha,index_sha256=index_sha,success=len(successful),failures=len(errors),static_scope_candidates=report['static_scope_candidates'],seconds=report['elapsed_seconds'])),flush=True)
if __name__=='__main__':main()


