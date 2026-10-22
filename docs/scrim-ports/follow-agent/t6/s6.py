"""s6.py BINARY SCRATCH OUT — Fix T6, part A: the 60 s grace, centring and the reveal, in the real app.
The app runs on a scratch HOME and a scratch settings folder (SCRIM_SETTINGS_DIR, Scrim's rules 9;
MCP on 7690, never 7680). The agent is an MCP client on the app's HTTP endpoint, named as Claude
Desktop's bridge names itself. It builds four batches of 18 layers (an artboard and 17 layers in it),
each 3,000 units from the last, 20, 36 and 50 s apart, then calls finish_working. A sampler reads
`ui.inspect` and `view.get` every 100 ms. Settings ▸ Diagnostics comes from the scratch settings file
after the app quits (never the clipboard). Writes OUT/s6.json."""
import http.client, json, os, subprocess, sys, threading, time
HERE = os.path.dirname(os.path.abspath(sys.argv[0]))
sys.path.insert(0, HERE)
from d3 import App

exe, scratch, out = sys.argv[1], sys.argv[2], sys.argv[3]
home, settings = os.path.join(scratch, 'home'), os.path.join(scratch, 'settings')
for d in (home, settings, out):
    os.makedirs(d, exist_ok=True)
json.dump({'prefs': {'mcp': {'port': 7690, 'port_chosen': True}, 'settings_version': 8}}, open(os.path.join(settings, 'state.json'), 'w'))
env = dict(os.environ, HOME=home, SCRIM_SETTINGS_DIR=settings)
app = subprocess.Popen([exe, '--control', '7979'], env=env, stdout=open(os.path.join(out, 'app.log'), 'w'), stderr=subprocess.STDOUT, cwd=home)
for _ in range(200):
    try:
        a = App(7979)
        a.run('mcp.status')
        break
    except Exception:
        time.sleep(0.25)
if not a.call('app.settings-dir', {}).get('result', {}).get('override'):
    raise SystemExit('the app is not on a scratch settings folder (SCRIM_SETTINGS_DIR)')
port = a.run('mcp.status').get('port')
if port == 7680:
    raise SystemExit("the MCP endpoint is on 7680, the person's own Scrim")
a.call('window.focus', {'id': 0})
a.call('ui.set', {'windowSize': [1200, 800]})
a.run('file.new')
a.call('ui.set', {'home': False, 'rightTab': 'design'})
time.sleep(1.0)


class Mcp:
    def __init__(self, port, name):
        self.n, self.session = 0, None
        self.c = http.client.HTTPConnection('127.0.0.1', port, timeout=60)
        self.rpc('initialize', {'protocolVersion': '2025-06-18', 'capabilities': {}, 'clientInfo': {'name': name, 'version': '0.14.3'}})
        self.rpc('notifications/initialized', notify=True)

    def rpc(self, method, params=None, notify=False):
        body = {'jsonrpc': '2.0', 'method': method}
        if not notify:
            self.n += 1
            body['id'] = self.n
        if params is not None:
            body['params'] = params
        h = {'Content-Type': 'application/json', 'Accept': 'application/json, text/event-stream'}
        if self.session:
            h['Mcp-Session-Id'] = self.session
        self.c.request('POST', '/mcp', json.dumps(body).encode(), h)
        r = self.c.getresponse()
        self.session = r.getheader('Mcp-Session-Id') or self.session
        txt = r.read()
        return json.loads(txt) if txt else None

    def tool(self, name, args):
        res = self.rpc('tools/call', {'name': name, 'arguments': args})['result']
        text = res['content'][0].get('text', '') if res.get('content') else ''
        if res.get('isError'):
            raise SystemExit(f'{name}: {text[:300]}')
        try:
            return json.loads(text)
        except ValueError:
            return text


t0 = time.monotonic()
samples, done = [], threading.Event()


def sample():
    s = App(7979)
    while not done.is_set():
        r = s.call('ui.inspect').get('result', {})
        ag = r.get('agent', {})
        v = s.call('engine.execute', {'command': 'view.get', 'params': {}}).get('result', {})
        samples.append({
            't': round(time.monotonic() - t0, 2),
            'working': ag.get('working'),
            'alpha': ag.get('workingAlpha'),
            'dots': ag.get('indicator') is not None,
            'follow': ag.get('follow'),
            'held': ag.get('revealHeld'),
            'view': [v.get('cx'), v.get('cy'), v.get('zoom'), v.get('vw'), v.get('vh')],
            'canvas': r.get('canvas', {}).get('rect'),
        })
        time.sleep(0.1)


threading.Thread(target=sample, daemon=True).start()
m = Mcp(port, 'local-agent-mode-scrim (via mcp-remote 0.14.3)')
steps = []
gaps = [20, 36, 50]
for i in range(4):
    x = 3000 * (i + 1)
    f = m.tool('run_command', {'command': 'frame.create', 'params': {'x': x, 'y': 0, 'width': 640, 'height': 480, 'name': f'Batch {i}'}})
    # 17 layers in a 6 × 3 grid: their union is (x + 20, 20) to (x + 580, 280).
    kids = [{'command': 'shape.rectangle', 'params': {'x': 20 + (k % 6) * 100, 'y': 20 + (k // 6) * 100, 'width': 60, 'height': 60, 'parent': f['id']}} for k in range(17)]
    m.tool('batch', {'commands': kids})
    steps.append({'t': round(time.monotonic() - t0, 2), 'batch': i, 'x': x, 'centre': [x + 300, 150]})
    a.call('window.focus', {'id': 0})
    if i < 3:
        time.sleep(gaps[i])
time.sleep(3)
m.tool('finish_working', {})
steps.append({'t': round(time.monotonic() - t0, 2), 'finish': True})
time.sleep(2)
done.set()
time.sleep(0.3)
final = a.call('ui.inspect').get('result', {}).get('agent', {})
a.call('window.focus', {'id': 0})
a.call('app.quit', {})
try:
    app.wait(timeout=20)
except subprocess.TimeoutExpired:
    print('the app did not quit', flush=True)
state = json.load(open(os.path.join(settings, 'state.json')))


def find(v):
    if isinstance(v, dict):
        if isinstance(v.get('diagnostics'), list):
            return v['diagnostics']
        return next((r for r in map(find, v.values()) if r is not None), None)
    return None


diag = find(state) or []
json.dump({'steps': steps, 'samples': samples, 'reveals': final.get('reveals'), 'diagnostics': diag}, open(os.path.join(out, 's6.json'), 'w'), indent=1)
print(json.dumps({'steps': steps, 'reveals': final.get('reveals', {}).get('recent', [])[-8:]}, indent=1)[:3000])
print('diagnostics', [(e.get('kind'), e.get('detail')) for e in diag if e.get('kind') in ('working', 'reveal', 'follow')])
