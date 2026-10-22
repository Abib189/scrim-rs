"""l2.py CONTROL_PORT OUT_JSON SPEED CAMERA [SEQUENCE] — Fix L2's reproduction, one run on a fresh app.
The agent is an MCP client on the app's real HTTP endpoint ("codex-mcp-client", its own session); the
control channel only sets the starting state and reads `ui.inspect` (its reads don't count as work).
SPEED: fast | normal | slow.
CAMERA: far     the camera far from where the work lands (view.set to (-6000, -4000), zoom 0.6);
        target  the camera already on the work (the artboard's centre, zoom 0.6);
        paused  a first step in view, then a wheel turn over the canvas (the person's input:
                follow pauses), then the 36-layer step far off screen.
SEQUENCE: brief (default)  one step: an artboard, then 35 rectangles in it as one batch (36 layers);
          ie               Stage I.1's speed run as it was: a 24-layer step at x 0, `view.set` back
                           to (320, 240) zoom 0.6, then the 36-layer step at x 760.
Records every `ui.inspect` › agent sample (follow, pausedBy, revealing, reveals) for 8 s after each step."""
import http.client, json, sys, time
sys.path.insert(0, sys.argv[0].rsplit('/', 1)[0])
from d3 import App

port, out, speed, camera = int(sys.argv[1]), sys.argv[2], sys.argv[3], sys.argv[4]
seq = sys.argv[5] if len(sys.argv) > 5 else 'brief'
a = App(port)
# Fix Q3: never against the person's own settings. The app must run with SCRIM_SETTINGS_DIR set to
# a scratch folder (and a scratch HOME).
if not a.call('app.settings-dir', {}).get('result', {}).get('override'):
    raise SystemExit('start the app with SCRIM_SETTINGS_DIR set to a scratch settings folder')


class Mcp:
    def __init__(self, port, name):
        self.n, self.session = 0, None
        self.c = http.client.HTTPConnection('127.0.0.1', port, timeout=60)
        self.rpc('initialize', {'protocolVersion': '2025-06-18', 'capabilities': {}, 'clientInfo': {'name': name, 'version': '0.162.1'}})
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
            raise SystemExit(f'{name} {args}: {text}')
        try:
            return json.loads(text)
        except ValueError:
            return text


def step(m, i, n, x):
    f = m.tool('run_command', {'command': 'frame.create', 'params': {'x': x, 'y': 0, 'width': 640, 'height': 480, 'name': f'Step {i + 1}'}})
    kids = [{'command': 'shape.rectangle', 'params': {'x': 20 + (k % 8) * 76, 'y': 20 + (k // 8) * 76, 'width': 60, 'height': 60, 'parent': f['id']}} for k in range(n - 1)]
    m.tool('batch', {'commands': kids})
    return f['id']


def agent():
    r = a.call('ui.inspect').get('result', {})
    return r.get('agent', {})


def watch(seconds=8.0):
    t0, samples = time.monotonic(), []
    while time.monotonic() - t0 < seconds:
        ag = agent()
        samples.append({'t': round(time.monotonic() - t0, 3), 'follow': ag.get('follow'), 'pausedBy': ag.get('pausedBy'), 'revealing': ag.get('revealing')})
        time.sleep(0.03)
    return samples


log = {'speed': speed, 'camera': camera, 'sequence': seq, 'steps': []}
a.call('ui.set', {'windowSize': [1440, 900], 'settings': False, 'home': False})
time.sleep(0.8)
a.call('window.focus', {'id': 0})
a.run('file.new')
a.call('ui.set', {'home': False})
time.sleep(0.8)
a.run('panel.tab.ai')
a.run('pref.revealSpeed', {'speed': speed})
a.run('camera.follow-agents', {'on': True})
mport = a.run('mcp.status')['port']
m = Mcp(mport, 'codex-mcp-client')
time.sleep(1.0)

if seq == 'ie':
    plan = [(0, 24, 0, (320, 240)), (1, 36, 760, (320, 240))]
elif camera == 'paused':
    plan = [(0, 6, 0, (320, 240)), (1, 36, 20000, None)]
else:
    plan = [(0, 36, 760, (-6000, -4000) if camera == 'far' else (760 + 320, 240))]
for i, n, x, v in plan:
    if v:
        a.run('view.set', {'cx': v[0], 'cy': v[1], 'zoom': 0.6})
    else:
        # The person: the pointer over the canvas, one wheel turn.
        a.call('ui.input', {'events': [{'move': [750, 500]}]})
        a.call('ui.input', {'events': [{'scroll': [0, -40]}]})
    time.sleep(0.6)
    before = agent()
    t0 = time.monotonic()
    fid = step(m, i, n, x)
    landed = round(time.monotonic() - t0, 3)
    samples = watch()
    after = agent()
    log['steps'].append({'layers': n, 'x': x, 'view': list(v) + [0.6] if v else 'the person scrolled', 'frame': fid, 'reply_s': landed, 'follow_before': before.get('follow'),
                         'reveals': after.get('reveals'), 'samples': samples, 'view_after': a.run('view.get')})
m.tool('finish_working', {})
log['diagnostics'] = [e for e in a.run('diagnostics.events').get('events', []) if e.get('kind') in ('follow', 'reveal')]
json.dump(log, open(out, 'w'), indent=1)
for s in log['steps']:
    tallies = [t for t in (s['reveals'] or {}).get('recent', []) if t.get('created')]
    print(speed, camera, seq, s['layers'], 'follow before:', s['follow_before'], 'paused by:', sorted({x['pausedBy'] for x in s['samples']}, key=str),
          'max revealing:', max((x['revealing'] or 0) for x in s['samples']), 'totals:', json.dumps({k: (s['reveals'] or {}).get(k) for k in ('entered', 'skipped')}))
print('  diagnostics events:', json.dumps(log['diagnostics']))
