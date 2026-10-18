"""Project exact selected NIF source fields for the static modern model backend.

This parses one immutable byte allocation with the existing PyFFI reader. It
exports binary32 words, not transformed/rounded converted mesh coordinates.
Input SHA, converted SHA, canonical material-audit SHA and every audited mesh's
source-node ordinal bind this projection to the existing paired render artifact.
It is not a native NiStream ABI or unsupported dynamic-node implementation.
"""
from pathlib import Path
import sys,io,json,struct,hashlib,argparse
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[2];GAME=ROOT.parent
sys.path.insert(0,str(GAME/'reconstruction/tools'))
sys.path.insert(0,str(ROOT/'research/texture-phase2'))
from convert_models import NifFormat,ORIGINAL,string
from export_model_materials import extras,tag_evidence,next_context,TARGETS

def sha(b):return hashlib.sha256(b).hexdigest()
def w(f):return struct.unpack('<I',struct.pack('<f',f))[0]
def xyz(v):return [w(v.x),w(v.y),w(v.z)]
def text(v):
 s=string(v)
 if not s.isascii() or '\0'in s:raise ValueError('Non-ASCII/embedded NUL NIF string conversion requires actual native loader')
 return s

def project(relative,audit,audit_sha):
 path=ORIGINAL/relative;source=path.read_bytes();data=NifFormat.Data();data.read(io.BytesIO(source))
 converted_path=GAME/'reconstruction/assets/converted/models'/Path(relative).with_suffix('.json')
 converted=converted_path.read_bytes()
 record=audit['models'][relative]
 assert sha(source)==record['source_sha256'] and sha(converted)==record['converted_sha256']
 nodes=[];seen={};active=set()
 def walk(node,parent=None,hierarchy=(),context=None):
  if node is None:return None
  if not isinstance(node,NifFormat.NiAVObject):raise ValueError('NonAV root/child '+type(node).__name__)
  if id(node) in seen:raise ValueError('Repeated/aliased AV object requires graph alias semantics')
  index=len(nodes);seen[id(node)]=index;name=text(node.name);hierarchy=hierarchy+(name,)
  rotation=[w(getattr(node.rotation,key))for key in ['m_11','m_21','m_31','m_12','m_22','m_32','m_13','m_23','m_33']]
  rawextras=extras(node);own=tag_evidence(rawextras,list(hierarchy));state=next_context(isinstance(node,NifFormat.NiGeometry),context or {},own)
  tags=[]
  for item in rawextras:
   if item['extra_name']in ['Mod','Eq','RepColor','Sg','Sg2']:tags.append(item['extra_name'])
   if item['is_string']:
    for line in item['value'].split('\r\n'):
     candidate=line.rsplit('=',1)[0]
     for known in ['Mod','Eq','RepColor','Sg','Sg2']:
      if candidate.isascii() and candidate.lower()==known.lower():tags.append(known)
  row=dict(kind=type(node).__name__,name=name,parent=parent,hierarchy=list(hierarchy),
    local=rotation+xyz(node.translation)+[w(node.scale)],flags=int(node.flags),children=[],
    vertices=[],sphere=None,skin=getattr(node,'skin_instance',None)is not None,
    shadow='shadow'in state,team='team'in state,tags=tags,extras=rawextras,
    properties=[],controllers=[])
  nodes.append(row)
  ctrl=getattr(node,'controller',None);controllers=set()
  while ctrl is not None:
   if id(ctrl)in controllers:raise ValueError('cyclic Controller list')
   controllers.add(id(ctrl));row['controllers'].append(dict(kind=type(ctrl).__name__,flags=int(ctrl.flags),stop=w(ctrl.stop_time)))
   ctrl=getattr(ctrl,'next_controller',None)
  for prop in node.properties:
   if prop is None:continue
   p=dict(kind=type(prop).__name__)
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
     p['slots'].append(dict(texture=None if texture is None else text(texture.file_name),filter=int(descriptor.filter_mode)))
    p['shader_textures']=int(getattr(prop,'num_shader_textures',0))
   row['properties'].append(p)
  if isinstance(node,NifFormat.NiGeometry) and node.data is not None:
   row['vertices']=[xyz(v)for v in node.data.vertices]
   row['sphere']=xyz(node.data.center)+[w(node.data.radius)]
   row['has_normals']=bool(node.data.has_normals)
  row['children']=[walk(child,index,hierarchy,state)for child in getattr(node,'children',[])]
  return index
 roots=[walk(root)for root in data.roots]
 return dict(schema=1,source=relative,source_sha256=sha(source),converted_sha256=sha(converted),
  material_audit_sha256=audit_sha,version=data.version,roots=roots,nodes=nodes,
  mesh_mapping=[{k:r[k]for k in ['mesh_index','source_node_ordinal','name','hierarchy','vertex_count','triangle_count']}for r in record['corrections']])

def main():
 ap=argparse.ArgumentParser();ap.add_argument('--source',action='append');args=ap.parse_args()
 audit_bytes=(ROOT/'assets-cache/model-materials.json').read_bytes();audit=json.loads(audit_bytes)
 output=ROOT/'assets-cache/model-native-graphs';output.mkdir(exist_ok=True)
 for source in args.source or TARGETS:
  record=project(source,audit,sha(audit_bytes));raw=(json.dumps(record,separators=(',',':'),ensure_ascii=False)+'\n').encode()
  (output/(record['source_sha256']+'.json')).write_bytes(raw)
  print(json.dumps(dict(source=source,nodes=len(record['nodes']),bytes=len(raw),sha256=sha(raw))))
if __name__=='__main__':main()

