"""Select small exact original-instruction cases for the asset-free diagnostic.

Only this research bundle is written. No artwork, runtime or source fixture changes.
"""
from pathlib import Path
import hashlib,json
ROOT=Path(__file__).resolve().parents[2]
SOURCES={
 'rng':('research/phase4-rng/native-rng-vectors.json','47c76f4bb80b98e6ac886b0befbf5ed7ef3d68eaacd10304d9c0da51b023cc28'),
 'orders':('research/phase4-orders/orders-native-oracle.json','3f51d489066ad4491d22814f091a28567ad9530d7e2bbf16607d0b38bdf5bd36'),
 'actors':('research/phase4-actors/trace_registry.json','c6be4d359c99b590c0475bd53a447e1686bc6038c5d6f6e73388b9115fb8297d'),
 'components':('research/phase4-actors/components-native-oracle.json','f813df40e3e81e30218eadcd9e1b118310b1d41f4b049d4111257fb1c080943d'),
 'body':('research/phase4-body/body-native-oracle.json','25e6c8563ee058a09c8579b1fdfe9ebcea09954c28ef802c103c8af7a844eed9'),
 'movement':('research/phase4-movement/native-movement-fixtures.json','59f585421a04b5dfe2ac79d10b442fb7344f17fed90e84016e4ba00153338c79'),
}
def main():
 data={};sources={}
 for name,(path,digest) in SOURCES.items():
  raw=(ROOT/path).read_bytes();assert hashlib.sha256(raw).hexdigest()==digest,path
  data[name]=json.loads(raw);sources[name]={'path':path,'sha256':digest}
 rng=data['rng'];orders=data['orders']
 bundle={'schema':1,'original_sha256':rng['original_sha256'],'accepted_native_sha256':rng['v14_sha256'],'source_oracles':sources,
  'rng':{'constructors':[r for r in rng['constructors']if r['seed'] in [0,0x12345678] and r['address']!='0x85d880'],
         'bounded':[r for r in rng['bounded']if r['state_before']==0x12345678 and r['bound'] in [1,7,0xffffffff]],
         'converted':[r for r in rng['converted']if r['value'] in [0,0x7fffffff,0x80000000,0xffffffff]]},
  'order_registry':{'count':len(orders['registry']),'samples':[r for r in orders['registry']if r['id']in[0,47,66,84,86,298,356]]},
  'orders':[r for r in orders['packets']if r['name']in['two_ordered_moves','missing_actor']],
  'actors':[r for r in data['actors']['cases']if r['name']in['fresh-and-stale','saved-and-swap']],
  'components':[r for r in data['components']['cases']if r['input']=={'mode':'existing','present':[0,3,4,30],'fail_slot':-1}],
  'body':[r for r in data['body']['cases']if r['name']=='heal_callback_and_clock' and r['precision']=='Extended64'],
  'movement':[r for r in data['movement']['fixtures']if r['input']['name']in['straight_Extended64','collision_Extended64']]}
 assert len(bundle['components'])==len(bundle['body'])==1
 assert len(bundle['orders'])==len(bundle['actors'])==len(bundle['movement'])==2
 p=Path(__file__).with_name('mechanics-samples.json');p.write_text(json.dumps(bundle,separators=(',',':'))+'\n',encoding='utf-8',newline='\n')
 print(json.dumps({'bytes':p.stat().st_size,'sha256':hashlib.sha256(p.read_bytes()).hexdigest()}))
if __name__=='__main__':main()
