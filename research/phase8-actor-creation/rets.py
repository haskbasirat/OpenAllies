import sys
from disfunc import func
for a in sys.argv[1:]:
    ins = func(int(a, 16))
    rets = sorted({i.op_str or '0' for i in ins if i.mnemonic.startswith('ret')})
    print(a, rets, len(ins))
