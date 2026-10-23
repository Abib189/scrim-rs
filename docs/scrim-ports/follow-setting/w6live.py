"""w6live.py BINARY SCRATCH OUT — Fix W6: Follow on by default, and agents can't change a setting.

The app (BINARY) starts twice on one scratch HOME and one scratch settings folder (SCRIM_SETTINGS_DIR,
Scrim's rules 9; MCP on 7690, never 7680; control channel on 7979). The folder starts with the real
settings file a Fix T build wrote (`apps/figcraft/tests/fixtures/settings/state-v8-fix-t.json`, from
before Fix V) with following off and no marker, as an agent left it before Fix V2.

First launch: following should be on (the repair), with its line. A stub MCP client named as Codex
names itself then asks to change the reveal, its speed, the dots, the theme, the accent, the motion
and following; calls finish_working; turns a tool off, then asks to turn it on; then builds 40
rectangles in one batch inside an artboard it makes 3,000 units from the view. A sampler reads `ui.inspect` and `view.get`
every 100 ms. The app quits; Settings ▸ Diagnostics and the settings are read from the saved file.

Second launch: nothing more changes (no second repair, following still on). Writes OUT/w6.json."""
import http.client, json, math, os, subprocess, sys, threading, time

HERE = os.path.dirname(os.path.abspath(sys.argv[0]))
REPO = os.path.abspath(os.path.join(HERE, '..', '..', '..'))
sys.path.insert(0, os.path.join(REPO, 'docs', 'scrim-ports', 'follow-agent', 't6'))
from d3 import App

exe, scratch, out = sys.argv[1], sys.argv[2], sys.argv[3]
home, settings = os.path.join(scratch, 'home'), os.path.join(scratch, 'settings')
for d in (home, settings, out):
    os.makedirs(d, exist_ok=True)
state = json.load(open(os.path.join(REPO, 'apps', 'figcraft', 'tests', 'fixtures', 'settings', 'state-v8-fix-t.json')))
state['prefs']['follow_agents'] = False
state['prefs'].pop('follow_chosen', None)
state['prefs']['mcp'] = {'port': 7690, 'port_chosen': True, 'require_auth': False}
json.dump(state, open(os.path.join(settings, 'state.json'), 'w'))
env = dict(os.environ, HOME=home, SCRIM_SETTINGS_DIR=settings)
REPAIR = 'follow setting: OFF → ON (made by a settings migration: saved off was never chosen)'
KEYS = ['follow_agents', 'follow_chosen', 'reveal_agent_work', 'reveal_speed', 'working_indicator', 'theme', 'accent', 'motion', 'reduce_motion', 'settings_version']


def launch():
    app = subprocess.Popen([exe, '--control', '7979'], env=env, stdout=open(os.path.join(out, 'app.log'), 'a'), stderr=subprocess.STDOUT, cwd=home)
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
    return app, a, port


def quit(app, a):
    a.call('window.focus', {'id': 0})
    a.call('app.quit', {})
    try:
        app.wait(timeout=20)
    except subprocess.TimeoutExpired:
        app.kill()
    return json.load(open(os.path.join(settings, 'state.json')))['prefs']


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


# First launch.
app, a, port = launch()
a.call('window.focus', {'id': 0})
a.call('ui.set', {'windowSize': [1200, 800]})
a.run('file.new')
a.call('ui.set', {'home': False, 'rightTab': 'ai'})
time.sleep(1.0)
ins = a.call('ui.inspect').get('result', {}).get('agent', {})
at_start = {'follow': ins.get('follow'), 'followSetting': ins.get('aiPanel', {}).get('header', {}).get('followSetting')}
m = Mcp(port, 'codex-mcp-client')
asks = [
    ('pref.revealAgentWork', {'on': False}), ('pref.revealSpeed', {'speed': 'slow'}), ('pref.workingIndicator', {'on': False}),
    ('pref.theme', {'theme': 'dark'}), ('pref.accent', {'preset': 'violet'}), ('pref.motion', {'motion': 'off'}),
    ('camera.follow-agents', {}), ('camera.follow-agents', {'on': False}),
]
refusals = []
for cmd, p in asks:
    text, err = m.tool('run_command', {'command': cmd, 'params': p})
    refusals.append({'command': cmd, 'params': p, 'refused': err, 'said': ' '.join(text.split())[:160]})
allowed = {}
text, err = m.tool('finish_working', {})
allowed['finish_working'] = {'ok': not err, 'said': ' '.join(text.split())[:80]}
text, err = m.tool('run_command', {'command': 'mcp.set-tool', 'params': {'tool': 'export', 'on': False}})
allowed['mcp.set-tool export off'] = {'ok': not err, 'said': ' '.join(text.split())[:80]}
text, err = m.tool('run_command', {'command': 'mcp.set-tool', 'params': {'tool': 'export', 'on': True}})
allowed['mcp.set-tool export on'] = {'ok': not err, 'said': ' '.join(text.split())[:160]}
after_asks = {k: v for k, v in a.call('ui.inspect').get('result', {}).get('agent', {}).items() if k == 'follow'}

# The build: 40 rectangles in one batch, 3,000 units right of the view's centre.
v0 = a.call('engine.execute', {'command': 'view.get', 'params': {}}).get('result', {})
t0 = time.monotonic()
samples, done = [], threading.Event()


def sample():
    s = App(7979)
    while not done.is_set():
        r = s.call('ui.inspect').get('result', {}).get('agent', {})
        v = s.call('engine.execute', {'command': 'view.get', 'params': {}}).get('result', {})
        samples.append({'t': round(time.monotonic() - t0, 2), 'follow': r.get('follow'), 'dots': r.get('indicator') is not None,
                        'held': r.get('revealHeld'), 'view': [v.get('cx'), v.get('cy'), v.get('zoom')]})
        time.sleep(0.1)


threading.Thread(target=sample, daemon=True).start()
x0, y0 = round(v0.get('cx', 0) + 3000), round(v0.get('cy', 0))
# The artboard, in its own request as an agent makes one; then the batch: 40 rectangles in it (a
# 10 × 4 grid, their union (x0 + 20, y0 + 20) to (x0 + 500, y0 + 200)).
f, _ = m.tool('run_command', {'command': 'frame.create', 'params': {'x': x0, 'y': y0, 'width': 640, 'height': 480, 'name': 'Far'}})
fid = json.loads(f)['id']
rects = [{'command': 'shape.rectangle', 'params': {'x': 20 + (k % 10) * 50, 'y': 20 + (k // 10) * 50, 'width': 30, 'height': 30, 'parent': fid}} for k in range(40)]
m.tool('batch', {'commands': rects})
landed = round(time.monotonic() - t0, 2)
time.sleep(4.0)
settled = a.call('engine.execute', {'command': 'view.get', 'params': {}}).get('result', {})
final = a.call('ui.inspect').get('result', {}).get('agent', {})
m.tool('finish_working', {})
finished = round(time.monotonic() - t0, 2)
time.sleep(1.0)
done.set()
time.sleep(0.3)
first = quit(app, a)
# The 40 rectangles' union.
ux, uy = x0 + 260, y0 + 110
cx, cy, z = settled.get('cx'), settled.get('cy'), settled.get('zoom')
working = [s for s in samples if landed <= s['t'] <= finished]
moved = [s for s in samples if s['view'][0] is not None and abs(s['view'][0] - v0.get('cx', 0)) > 1 and abs(s['view'][0] - cx) > 1]

# Second launch: nothing more.
app, a, port = launch()
time.sleep(1.0)
again = a.call('ui.inspect').get('result', {}).get('agent', {}).get('follow')
second = quit(app, a)

locked = subprocess.run(['ioreg', '-n', 'Root', '-d1'], capture_output=True, text=True).stdout.find('"CGSSessionScreenIsLocked"=Yes') >= 0
report = {
    'screen_locked': locked,
    'start_file': {'follow_agents': False, 'follow_chosen': '(absent)', 'settings_version': 8},
    'first_launch_follow': at_start,
    'refusals': refusals,
    'follow_after_asks': after_asks,
    'allowed': allowed,
    'saved_after_first': {k: first.get(k) for k in KEYS + ['mcp']},
    'centre_px_from_canvas_centre': round(math.hypot(cx - ux, cy - uy) * z, 2),
    'zoom': [v0.get('zoom'), z],
    'captures_mid_flight': len(moved),
    'reveal_since_open (the artboard and the 40)': {'entered': (final.get('reveals') or {}).get('entered'), 'drawn_final': (final.get('reveals') or {}).get('skipped')},
    'reveal_held_most': max((s['held'] or 0) for s in samples) if samples else None,
    'dots_while_working': f"{sum(1 for s in working if s['dots'])} of {len(working)} captures",
    'diagnostics_first': [(e.get('kind'), e.get('detail')) for e in first.get('diagnostics', [])],
    'second_launch_follow': again,
    'repair_lines_after_second': sum(1 for e in second.get('diagnostics', []) if e.get('detail') == REPAIR),
    'new_lines_on_second_launch': [(e.get('kind'), e.get('detail')) for e in second.get('diagnostics', [])[len(first.get('diagnostics', [])):]],
    'saved_after_second': {k: second.get(k) for k in KEYS},
}
json.dump({'report': report, 'samples': samples}, open(os.path.join(out, 'w6.json'), 'w'), indent=1, ensure_ascii=False)
print(json.dumps(report, indent=1, ensure_ascii=False))
