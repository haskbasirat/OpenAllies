from pathlib import Path
import sys,json,hashlib,struct
HERE=Path(__file__).resolve().parent;GAME=HERE.parents[3]
sys.path.insert(0,str(GAME/'reconstruction/tools'))
from convert_models import NifFormat,ORIGINAL,string
from convert_models_animations import read_nif
for relative in ['Buildings/GBR/GBRCorps/GBRCorps.nif','Buildings/GBR/GBRCorps/GBRCorpsPack.nif','Buildings/GBR/GBRDepotOil/GBRDepotOil.nif']:
 data=read_nif(ORIGINAL/relative);print(relative)
 for root in data.roots:
  print('root',root.__class__.__name__,string(getattr(root,'name','')),'version',hex(data.version))
  pending=list(getattr(root,'extra_data_list',[]));extra=getattr(root,'extra_data',None)
  if extra is not None:pending.append(extra)
  seen=set()
  while pending:
   extra=pending.pop(0)
   if extra is None or id(extra)in seen:continue
   seen.add(id(extra));record=dict(kind=extra.__class__.__name__,name=string(getattr(extra,'name','')))
   for field in ['string_data','integer_data','binary_data','value','num_bytes','bytes_remaining']:
    if hasattr(extra,field):
     value=getattr(extra,field)
     try:value=bytes(value) if not isinstance(value,(int,str,bytes)) else value
     except: value=repr(value)
     if isinstance(value,bytes):value=dict(length=len(value),hex=value.hex())
     record[field]=value
   print(json.dumps(record));extra=getattr(extra,'next_extra_data',None)
   if extra is not None:pending.append(extra)
 def walk(node,depth=0):
  if node is None:return
  geometry=isinstance(node,NifFormat.NiGeometry)
  if geometry or str(getattr(node,'name','')).lower().find('selection')>=0:
   vertices=getattr(getattr(node,'data',None),'vertices',[])
   print('node',depth,node.__class__.__name__,string(node.name),'flags',int(node.flags),'vertices',len(vertices),'skin',getattr(node,'skin_instance',None)is not None)
  for child in getattr(node,'children',[]):walk(child,depth+1)
 for root in data.roots:walk(root)
