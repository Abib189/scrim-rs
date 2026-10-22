"""v5live.py BINARY SCRATCH OUT — Fix V5: the Follow agents setting in the real app, after the fix.

The app (BINARY) starts on a scratch HOME and a scratch settings folder (SCRIM_SETTINGS_DIR, Scrim's
rules 9; MCP on 7690, never 7680; control channel on 7979) holding the settings file the build before
Fix T wrote (`apps/figcraft/tests/fixtures/settings/state-v7-before-fix-t.json`: version 7, Follow
on, an unmarked 30 s grace). A stub MCP client on the app's HTTP endpoint, named as Codex's names
itself, first asks `camera.follow-agents {}` (what turned following off before Fix V2), then builds
40 layers (an artboard and 39 rectangles in it, one `batch`) 3,000 units from the view, then calls
finish_working. A sampler reads `ui.inspect` and `view.get` every 100 ms. Settings ▸ Diagnostics is
read from the scratch settings file after the app quits. Writes OUT/v5.json and prints the figures."""
import http.client, json, math, os, subprocess, sys, threading, time

HERE = os.path.dirname(os.path.abspath(sys.argv[0]))
REPO = os.path.abspath(os.path.join(HERE, '..', '..', '..'))
sys.path.insert(0, os.path.join(REPO, 'docs', 'scrim-ports', 'follow-agent', 't6'))
from d3 import App

exe, scratch, out = sys.argv[1], sys.argv[2], sys.argv[3]
home, settings = os.path.join(scratch, 'home'), os.path.join(scratch, 'settings')
for d in (home, settings, out):
    os.makedirs(d, exist_ok=True)
state = json.load(open(os.path.join(REPO, 'apps', 'figcraft', 'tests', 'fixtures', 'settings', 'state-v7-before-fix-t.json')))
state['prefs']['mcp'] = {'port': 7690, 'port_chosen': True, 'require_auth': False}
json.dump(state, open(os.path.join(settings, 'state.json'), 'w'))
env = dict(os.environ, HOME=home, SCRIM_SETTINGS_DIR=settings)
app = subprocess.Popen([exe, '--control', '7979'], env=env, stdout=open(os.path.join(out, 'app.log'), 'w'), stderr=subprocess.STDOUT, cwd=home)
a = None
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
a.call('ui.set', {'home': False, 'rightTab': 'ai'})
time.sleep(1.0)
start = a.call('ui.inspect').get('result', {}).get('agent', {})
at_start = {'follow': start.get('follow'), 'header': {k: start.get('aiPanel', {}).get('header', {}).get(k) for k in ('follow', 'followSetting', 'followDimmed', 'followTooltip')}}
v0 = a.call('engine.execute', {'command': 'view.get', 'params': {}}).get('result', {})


class Mcp:
    def __init__(self, port, name):
        self.n, self.session = 0, None
        self.c = http.client.HTTPConnection('127.0.0.1', port, timeout=60)
        self.rpc('initialize', {'protocolVersion': '2025-06-18', 'capabilities': {}, 'clientInfo': {'name': name, 'version': '0.163.0-alpha.11'}})
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
        return text, bool(res.get('isError'))


t0 = time.monotonic()
samples, done = [], threading.Event()


def sample():
    s = App(7979)
    while not done.is_set():
        r = s.call('ui.inspect').get('result', {})
        ag = r.get('agent', {})
        v = s.call('engine.execute', {'command': 'view.get', 'params': {}}).get('result', {})
        samples.append({'t': round(time.monotonic() - t0, 2), 'follow': ag.get('follow'), 'dots': ag.get('indicator') is not None,
                        'held': ag.get('revealHeld'), 'view': [v.get('cx'), v.get('cy'), v.get('zoom'), v.get('vw'), v.get('vh')]})
        time.sleep(0.1)


threading.Thread(target=sample, daemon=True).start()
m = Mcp(port, 'codex-mcp-client')
said, refused = m.tool('run_command', {'command': 'camera.follow-agents', 'params': {}})
after_ask = a.call('ui.inspect').get('result', {}).get('agent', {}).get('follow')
# 3,000 units right of the view's centre: an artboard and 39 rectangles in a 13 × 3 grid.
x = round(v0.get('cx', 0) + 3000)
f, _ = m.tool('run_command', {'command': 'frame.create', 'params': {'x': x, 'y': 0, 'width': 640, 'height': 480, 'name': 'Far'}})
fid = json.loads(f)['id']
kids = [{'command': 'shape.rectangle', 'params': {'x': 20 + (k % 13) * 46, 'y': 40 + (k // 13) * 130, 'width': 30, 'height': 30, 'parent': fid}} for k in range(39)]
m.tool('batch', {'commands': kids})
landed = round(time.monotonic() - t0, 2)
time.sleep(4.0)
q, _ = m.tool('run_command', {'command': 'node.query', 'params': {'query': f'//*[@id={fid}]//*', 'limit': 100}})
settled = a.call('engine.execute', {'command': 'view.get', 'params': {}}).get('result', {})
final = a.call('ui.inspect').get('result', {}).get('agent', {})
m.tool('finish_working', {})
finished = round(time.monotonic() - t0, 2)
time.sleep(1.0)
done.set()
time.sleep(0.3)
a.call('window.focus', {'id': 0})
a.call('app.quit', {})
try:
    app.wait(timeout=20)
except subprocess.TimeoutExpired:
    app.kill()
saved = json.load(open(os.path.join(settings, 'state.json')))['prefs']
diag = saved.get('diagnostics', [])
cx, cy, z, vw, vh = (settled.get(k) for k in ('cx', 'cy', 'zoom', 'vw', 'vh'))
# The 39 rectangles (the batch's own pass) and the 40 layers (the artboard, which landed a pass
# earlier, with them): following centres the latest pass's changes (Fix T2).
xs, ys = [x + 20 + (k % 13) * 46 for k in range(39)], [40 + (k // 13) * 130 for k in range(39)]
rects = ((min(xs) + max(xs) + 30) / 2, (min(ys) + max(ys) + 30) / 2)
off = {'the 39 rectangles': round(math.hypot(cx - rects[0], cy - rects[1]) * z, 2), 'the 40 layers (artboard)': round(math.hypot(cx - (x + 320), cy - 240) * z, 2)}
locked = subprocess.run(['ioreg', '-n', 'Root', '-d1'], capture_output=True, text=True).stdout.find('"CGSSessionScreenIsLocked"=Yes') >= 0
moved = [s for s in samples if s['view'][0] is not None and abs(s['view'][0] - v0.get('cx', 0)) > 1 and abs(s['view'][0] - cx) > 1]
report = {
    'at_start': at_start,
    'agent_asked_follow_off': {'refused': refused, 'said': ' '.join(said.split())[:200], 'follow_after': after_ask},
    'view_before': [v0.get('cx'), v0.get('cy'), v0.get('zoom')],
    'view_settled': [cx, cy, z, vw, vh],
    'screen_locked': locked,
    'centre_px_from_canvas_centre': off,
    'captures_mid_flight': len(moved),
    'follow_settled': final.get('follow'),
    # Since the window opened (ui.inspect's own counters): every agent layer, entered or drawn final.
    'reveal_since_open': {'entered': (final.get('reveals') or {}).get('entered'), 'drawn_final': (final.get('reveals') or {}).get('skipped')},
    'diagnostics': [(e.get('kind'), e.get('detail')) for e in diag],
    'saved_follow_agents': saved.get('follow_agents'),
    'saved_settings_version': saved.get('settings_version'),
    'landed_at': landed,
    'dots_drawn_while_working': f"{sum(1 for s in samples if s['dots'] and landed <= s['t'] <= finished)} of {sum(1 for s in samples if landed <= s['t'] <= finished)} captures",
    'reveal_held_most': max((s['held'] or 0) for s in samples),
}
json.dump({'report': report, 'samples': samples}, open(os.path.join(out, 'v5.json'), 'w'), indent=1)
print(json.dumps(report, indent=1, ensure_ascii=False))
