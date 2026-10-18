"""Actual native VC7.1 wide_vswprintf96C7EC on string/signed32 duplicate
name formats. C locale _getptd is inherited explicit transport, no localizer
or caller-success seam; both images/all precision controls execute the CRT.
"""
from pathlib import Path
import sys,json,hashlib,itertools,struct
HERE=Path(__file__).resolve().parent;sys.path.insert(0,str(HERE.parent))
from native_bootstrap import BootstrapMachine,PE0,PE1,PC,proof_ranges
def units(s):return list(struct.unpack('<%dH'%(len(s.encode('utf-16le',errors='surrogatepass'))//2),s.encode('utf-16le',errors='surrogatepass')))
def main():
 rows=[];visited={}
 for image in('original','v14'):
  m=BootstrapMachine(image)
  for pc,fmt,name,n in itertools.product(PC,('%s (%d)','%s-%d%%','[%s]:%d'),('', 'AI','A\0ignored','\ud800\udfff'),(0,1,2147483647,2147483648,0xffffffff)):
   m.fpu(pc);out=m.alloc(4096);va=m.alloc(8);m.w(va,m.string(name));m.w(va+4,n)
   m.call(0x96c7ec,[out,m.string(fmt),va]);result=m.text(out)
   expected=fmt.replace('%%','\x01').replace('%s',name.split('\0')[0]).replace('%d',str(n if n<0x80000000 else n-0x100000000)).replace('\x01','%')
   assert units(result)==units(expected),(fmt,name,n,result,expected)
   rows.append({'image':image,'precision':pc,'format':units(fmt),'name':units(name),'number':n,'output':units(result)});visited.update(m.visited)
 out={'passed':True,'scope':__doc__,'producer_sha256':hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
  'original_sha256':hashlib.sha256(PE0.b).hexdigest(),'v14_sha256':hashlib.sha256(PE1.b).hexdigest(),
  'cases':rows,'executed_windows':proof_ranges(visited)}
 (HERE/'native-name-format.json').write_text(json.dumps(out,indent=2)+'\n');print('passed',len(rows),'name-format rows')
if __name__=='__main__':main()
