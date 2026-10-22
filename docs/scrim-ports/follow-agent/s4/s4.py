"""s4.py BINARY SCRATCH OUT [START] — Fix S4: the report's session in the real app, every camera change
logged with where it was made (`ui.inspect` › agent.cameraLog).
Starts the app on a scratch HOME and a scratch settings folder (SCRIM_SETTINGS_DIR, Scrim's rules 9;
MCP on 7690, never 7680). The agent is an MCP client on the app's real HTTP endpoint, named as Claude
Desktop's bridge names itself; the control channel only sets the start and reads `ui.inspect` (its
reads aren't work). The agent opens a saved file (the `open_file` tool), then builds about 3,000 units
away in six batches, a few seconds apart, as an agent thinking between calls does.
START: editor (default)  an empty file in front;
       home              Home in front, as the app starts (the agent's file opens behind it).
Writes OUT/s4-START.json: the samples (follow, pausedBy, the newest camera change) and the final log."""
import http.client, json, os, subprocess, sys, threading, time
HERE = os.path.dirname(os.path.abspath(sys.argv[0]))
sys.path.insert(0, HERE)
from d3 import App

exe, scratch, out = sys.argv[1], sys.argv[2], sys.argv[3]
start = sys.argv[4] if len(sys.argv) > 4 else 'editor'
home, settings = os.path.join(scratch, 'home'), os.path.join(scratch, 'settings')
for d in (home, settings, out):
    os.makedirs(d, exist_ok=True)
json.dump({'prefs': {'mcp': {'port': 7690, 'port_chosen': True}, 'settings_version': 7}}, open(os.path.join(settings, 'state.json'), 'w'))
# The agents' folder (the default root, ~/Documents/Scrim in the scratch HOME), made before the app
# starts: the endpoint takes its roots then.
saved = os.path.join(home, 'Documents', 'Scrim', 'poster.figcraft')
os.makedirs(os.path.dirname(saved), exist_ok=True)
env = dict(os.environ, HOME=home, SCRIM_SETTINGS_DIR=settings)
app = subprocess.Popen([exe, '--control', '7979'], env=env, stdout=open(os.path.join(out, f'app-{start}.log'), 'w'), stderr=subprocess.STDOUT, cwd=home)
for _ in range(160):
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
    raise SystemExit('the MCP endpoint is on 7680, the person\'s own Scrim')


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
            a.call('window.focus', {'id': 0})
            a.call('app.quit', {})
            raise SystemExit(f'{name} {args}: {json.dumps({k: v for k, v in res.items() if k != "content"})} {text[:300]}')
        try:
            return json.loads(text)
        except ValueError:
            return text


# The saved file: a poster at the origin, written by the app, then closed.
a.call('window.focus', {'id': 0})
a.call('ui.set', {'windowSize': [1200, 760]})
a.run('file.new')
a.run('frame.create', {'x': 0, 'y': 0, 'width': 800, 'height': 600, 'name': 'Poster'})
a.run('file.saveAs', {'path': saved})
a.run('file.close')
if start == 'editor':
    a.run('file.new')
    a.call('ui.set', {'home': False})
else:
    a.call('ui.set', {'home': True})
# The window in front, so it draws (a covered window answers requests without frames).
a.call('window.focus', {'id': 0})
time.sleep(1.5)

samples, done, t0 = [], threading.Event(), time.monotonic()


def sample():
    s = App(7979)
    while not done.is_set():
        ag = s.call('ui.inspect').get('result', {}).get('agent', {})
        log = ag.get('cameraLog') or []
        samples.append({'t': round(time.monotonic() - t0, 2), 'follow': ag.get('follow'), 'pausedBy': ag.get('pausedBy'), 'working': ag.get('working'), 'last': log[-1] if log else None})
        time.sleep(0.15)


threading.Thread(target=sample, daemon=True).start()
steps = []
m = Mcp(port, 'local-agent-mode-scrim (via mcp-remote 0.14.3)')
steps.append((round(time.monotonic() - t0, 2), 'open_file'))
m.tool('open_file', {'path': saved})
time.sleep(4)
for i in range(6):
    x = 3000 + 700 * i
    steps.append((round(time.monotonic() - t0, 2), f'batch {i} at x {x}'))
    f = m.tool('run_command', {'command': 'frame.create', 'params': {'x': x, 'y': 0, 'width': 640, 'height': 480, 'name': f'Batch {i}'}})
    kids = [{'command': 'shape.rectangle', 'params': {'x': x + 20 + k * 76, 'y': 20, 'width': 60, 'height': 60, 'parent': f['id']}} for k in range(8)]
    m.tool('batch', {'commands': kids})
    time.sleep(4)
time.sleep(4)
done.set()
time.sleep(0.3)
final = a.call('ui.inspect').get('result', {}).get('agent', {})
pauses = sorted({s['pausedBy'] for s in samples if s['pausedBy']})
print(start, 'follow', final.get('follow'), 'pauses', pauses, flush=True)
for c in final.get('cameraLog') or []:
    print('  ', round(c['at'], 2), c['by'], c['what'], c['count'], c['to'] and (c['to']['document'], round(c['to']['cx']), round(c['to']['cy']), round(c['to']['zoom'], 3)), flush=True)
a.call('window.focus', {'id': 0})
a.call('app.quit', {})
try:
    app.wait(timeout=20)
except subprocess.TimeoutExpired:
    print('the app did not quit', flush=True)
# Settings ▸ Diagnostics, as the scratch settings folder kept them (never the clipboard).
state = json.load(open(os.path.join(settings, 'state.json')))
def find(v):
    if isinstance(v, dict):
        if isinstance(v.get('diagnostics'), list):
            return v['diagnostics']
        return next((r for r in map(find, v.values()) if r is not None), None)
    return None


diag = [e for e in find(state) or [] if e.get('kind') == 'follow']
json.dump({'start': start, 'steps': steps, 'samples': samples, 'cameraLog': final.get('cameraLog'), 'pausedBy': final.get('pausedBy'), 'follow': final.get('follow'), 'diagnostics': diag}, open(os.path.join(out, f's4-{start}.json'), 'w'), indent=1)
print('diagnostics', [e.get('detail') for e in diag], flush=True)
