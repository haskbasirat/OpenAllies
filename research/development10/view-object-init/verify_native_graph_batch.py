"""Independently check every published raw graph against its exact audit pair.

This inventory is not a replacement for Model::process_metadata. It reports
known structural exclusions without turning a source projection into a load,
render, reached-map, or GameReady certificate.
"""
from pathlib import Path
import collections,hashlib,json
from index_native_graphs import build_index,write_atomic

ROOT=Path(__file__).resolve().parents[3]
def sha(raw):return hashlib.sha256(raw).hexdigest()
def finite(word):return isinstance(word,int)and 0<=word<=0xffffffff and word&0x7f800000!=0x7f800000

def structural_reasons(graph):
 reasons=set();nodes=graph['nodes'];roots=graph['roots']
 if graph['version']!=0x0a010000:reasons.add('nif_version')
 if len(roots)!=1 or roots[0]is None:reasons.add('root_count_or_null')
 else:
  root=roots[0]
  assert 0<=root<len(nodes)and nodes[root]['parent']is None
  seen=set();work=[root]
  while work:
   i=work.pop();assert i not in seen;seen.add(i)
   for child in nodes[i]['children']:
    if child is not None:assert 0<=child<len(nodes)and nodes[child]['parent']==i;work.append(child)
  assert len(seen)==len(nodes)
  if any(e['extra_name']=='ppdv'and e['kind']=='NiIntegerExtraData'and e.get('integer')==7 for e in nodes[root]['extras']):reasons.add('processed_ppdv7_branch')
 data={};properties={};textures={}
 for t in graph['textures']:
  assert t['block']not in textures;textures[t['block']]=t
  if t['filename']is None:reasons.add('internal_texture')
 for n in nodes:
  if n['kind']not in ('NiNode','NiTriShape','NiTriStrips'):reasons.add('node_class')
  if n['skin']:reasons.add('skin')
  if n['controllers']:reasons.add('controller_graph')
  if any(t not in ('RepColor','Sg','Sg2','Eq')for t in n['tags']):reasons.add('processing_tag')
  if not all(finite(w)for w in n['local']):reasons.add('nonfinite_local')
  if n['sphere']is not None and not all(finite(w)for w in n['sphere']):reasons.add('nonfinite_sphere')
  if not all(finite(w)for v in n['vertices']for w in v):reasons.add('nonfinite_vertices')
  if n['data_block']is not None:
   value=dict(vertices=n['vertices'],sphere=n['sphere'],normals=n['normals']if n.get('has_normals')else None)
   if n['data_block']in data:assert data[n['data_block']]==value
   else:data[n['data_block']]=value
   if n.get('has_normals'):assert len(n['normals'])==len(n['vertices'])
   else:assert not n['normals']
  elif n['kind']in ('NiTriShape','NiTriStrips'):reasons.add('null_geometry_data')
  for p in n['properties']:
   if p['block']in properties:assert properties[p['block']]==p
   else:properties[p['block']]=p
   if p['kind']=='NiTexturingProperty':
    assert len(p['slots'])==7
    if p.get('shader_textures',0):reasons.add('shader_texture_descriptors')
    for d in p['slots']:
     if d is not None and d['source']is not None:
      actual=textures[d['source']]['filename']
      if actual is None and d['texture']is not None:reasons.add('internal_texture_descriptor_requires_loader')
      else:assert actual==d['texture'],(graph['source'],d,textures[d['source']])
 for mapping in graph['mesh_mapping']:
  n=nodes[mapping['source_node_ordinal']]
  assert mapping['name']==n['name']and mapping['hierarchy']==n['hierarchy']and mapping['vertex_count']==len(n['vertices'])
 return sorted(reasons)

def main():
 folder=ROOT/'assets-cache/model-native-graphs';index_path=folder/'index.json';index_raw=index_path.read_bytes();index=json.loads(index_raw)
 assert index==build_index(folder)
 receipt_path=Path(__file__).with_name('raw-model-batch.json');receipt_raw=receipt_path.read_bytes();receipt=json.loads(receipt_raw)
 assert receipt['all_audited']and receipt['requested']==1847 and receipt['index_sha256']==sha(index_raw)
 audit_raw=(ROOT/'assets-cache/model-materials.json').read_bytes();audit=json.loads(audit_raw)
 manifest_raw=(ROOT.parent/'reconstruction/assets/converted/models/manifest.json').read_bytes()
 assert receipt['audit_sha256']==sha(audit_raw)and receipt['converted_manifest_sha256']==sha(manifest_raw)==audit['manifest_sha256']
 all_sources=set(audit['models']);failures={r['source']:r for r in receipt['rows']if'error'in r};success={r['source']:r for r in receipt['rows']if'error'not in r}
 assert not(set(failures)&set(success))and(set(failures)|set(success))==all_sources
 assert set(index['by_source'])=={s.lower()for s in success}
 reasons=collections.Counter();combinations=collections.Counter();node_classes=collections.Counter();rows=[]
 for source,row in success.items():
  selected=index['by_source'][source.lower()];assert selected['artifact']==row['artifact']and selected['source']==source
  graph_raw=(folder/selected['artifact']).read_bytes();assert sha(graph_raw)==selected['sha256']==row['projection_sha256']
  graph=json.loads(graph_raw);paired=audit['models'][source]
  assert graph['schema']==2 and graph['source']==source and graph['source_sha256']==selected['source_sha256']==paired['source_sha256']
  assert graph['converted_sha256']==paired['converted_sha256']and graph['material_audit_sha256']==sha(audit_raw)
  blocked=structural_reasons(graph);reasons.update(blocked);combinations.update([' + '.join(blocked)or'none'])
  node_classes.update(n['kind']for n in graph['nodes']);rows.append(dict(source=source,known_structural_exclusions=blocked))
 aliases=collections.defaultdict(list)
 for source,record in audit['models'].items():aliases[record['source_sha256']].append(source)
 alias_rows=[]
 for digest,sources in aliases.items():
  if len(sources)>1:
   artifacts=[]
   for source in sources:
    if source in success:
     actual=index['by_source'][source.lower()]['artifact'];assert actual==digest+'-'+sha(source.lower().encode())[:16]+'.json';artifacts.append(actual)
   assert len(set(artifacts))==len(artifacts);alias_rows.append(dict(source_sha256=digest,sources=sources,published_artifacts=artifacts))
 for source,digest in receipt['golden_three_sha256'].items():assert sha((folder/index['by_source'][source.lower()]['artifact']).read_bytes())==digest
 assert index_path.read_bytes()==index_raw and receipt_path.read_bytes()==receipt_raw
 report=dict(scope='Independent exhaustive published-artifact integrity and structural inventory. No runtime payload processing, load, render, reached-map, or GameReady acceptance.',requested=len(all_sources),raw_graphs_verified=len(success),explicit_failures=len(failures),no_known_structural_exclusions=combinations['none'],exclusion_counts=dict(reasons),exclusion_combinations=dict(combinations),node_classes=dict(node_classes),source_alias_groups=alias_rows,index_sha256=sha(index_raw),batch_receipt_sha256=sha(receipt_raw),verifier_sha256=sha(Path(__file__).read_bytes()),rows=rows)
 path=Path(__file__).with_name('raw-model-coverage.json');digest=write_atomic(path,report)
 print(json.dumps(dict(report=str(path),sha256=digest,graphs=len(success),failures=len(failures),no_known_structural_exclusions=combinations['none'],exclusions=dict(reasons),index_sha256=sha(index_raw))))
if __name__=='__main__':main()
