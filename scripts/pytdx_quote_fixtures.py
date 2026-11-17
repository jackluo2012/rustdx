#!/usr/bin/env python3
"""Generate offline Rust quote fixtures by running the actual pytdx parser.
Usage: python pytdx_quote_fixtures.py /path/to/pytdx /path/to/output.json
Requires six. Reference commit: 2857fdad08534533610bd2aeca387f760c4baa42.
"""
import json,random,struct,subprocess,sys
from pathlib import Path
ref=Path(sys.argv[1]).resolve()
sys.path.insert(0,str(ref))
from pytdx.parser.get_security_quotes import GetSecurityQuotesCmd
from pytdx.parser.get_minute_time_data import GetMinuteTimeData
from pytdx.parser.get_history_minute_time_data import GetHistoryMinuteTimeData

def encode(n):
    v=abs(n); b=(v & 63) | (64 if n<0 else 0); v >>= 6
    out=bytearray([b | (128 if v else 0)])
    while v:
        b=v & 127; v >>= 7; out.append(b | (128 if v else 0))
    return out

def quote(rng,index):
    market=index % 2; code='000001' if market==0 else '600000'
    price=rng.choice([1,1190,60000,2000000000])
    stamp=[11298811,9561810,10600000,10999999,13654321][index % 5]
    data=bytearray(struct.pack('<B6sH',market,code.encode(),1))
    for n in [price,-1,-price if index%3==0 else -2,2,-1,stamp,-price,index*100003,index]: data.extend(encode(n))
    data.extend(struct.pack('<I',[0,1235775464,1326643033,0x7f800001,0x80000000][index%5]))
    for n in [index*50003,index*50000,0,0]: data.extend(encode(n))
    for level in range(5):
        for n in [-level,-price if index%7==0 else level+1,index*(level+1),index*(level+2)]: data.extend(encode(n))
    data.extend(struct.pack('<H',0))
    for n in [0,1,-1,0]: data.extend(encode(n))
    data.extend(struct.pack('<hH',rng.randint(-30000,30000),1))
    return bytes(data),market,code

def normalize(q):
    fields={k:q[k] for k in ['code','price','last_close','open','high','low','vol','cur_vol','amount','s_vol','b_vol']}
    for i in range(1,6):
        for side in ['bid','ask']:
            fields[f'{side}{i}']=q[f'{side}{i}']; fields[f'{side}{i}_vol']=q[f'{side}_vol{i}']
    fields['speed']=q['reversed_bytes9']
    fields['servertime']=q['servertime'].zfill(12)
    return fields

rng=random.Random(20261117); fixtures=[]
for i in range(128):
    entries=[quote(rng,i)]
    if i % 8==0: entries.append(quote(rng,i+1))
    body=struct.pack('<HH',0,len(entries))+b''.join(e[0] for e in entries)
    cmd=GetSecurityQuotesCmd(None)
    cmd.setParams([(e[1],e[2]) for e in entries])
    expected=cmd.parseResponse(body)
    fixtures.append({'request_hex':cmd.send_pkg.hex(),'body_hex':body.hex(),'stocks':[[e[1],e[2]] for e in entries], 'quotes':[normalize(q) for q in expected]})
record={'reference':'rainx/pytdx','commit':subprocess.check_output(['git','-C',str(ref),'rev-parse','HEAD'],text=True).strip(),'fixtures':fixtures}
minutes=[]
for index in range(32):
    points=[]
    for k in range(index+1):
        for n in [1000 if k==0 else rng.randint(-5,5),0,rng.randint(0,100000)]:
            points.extend(encode(n))
    market=index%2; code='000001' if market==0 else '600000'
    case={'market':market,'code':code,'date':20261117}
    for name,cmd,header in [('minute',GetMinuteTimeData(None),struct.pack('<HH',index+1,0)),('history_minute',GetHistoryMinuteTimeData(None),struct.pack('<HI',index+1,0))]:
        body=header+bytes(points)
        cmd.setParams(market,code,*([20261117] if name=='history_minute' else []))
        case[name]={'request_hex':cmd.send_pkg.hex(),'body_hex':body.hex(),'points':cmd.parseResponse(body)}
    minutes.append(case)
record['minutes']=minutes
Path(sys.argv[2]).write_text(json.dumps(record,ensure_ascii=False,indent=2)+'\n')
print(f'generated {len(fixtures)} fixtures from pytdx {record["commit"]}')
