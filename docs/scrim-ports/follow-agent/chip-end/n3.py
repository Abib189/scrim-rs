"""n3.py CONTROL_PORT MCP_URL OUT — Fix N's live check through the real mcp-remote@0.14.3 bridge (an MCP
client on stdin/stdout, as Claude Desktop runs it): a poster in three batches; after the second, the
person scrolls the canvas (the control channel's `ui.input`: a wheel turn over the canvas), so
following pauses and the chip shows; the third lands while paused; then finish_working; then a second
build far away, with no other input, which the camera follows. Window captures every 100 ms with
`ui.inspect` (follow, pausedBy, chip, view), and the diagnostics at the end. Writes OUT/log.json and
OUT/fNNNN.png."""
import json, os, subprocess, sys, threading, time
sys.path.insert(0, os.path.dirname(os.path.abspath(sys.argv[0])))
from d3 import App

port, url, out = int(sys.argv[1]), sys.argv[2], sys.argv[3]
os.makedirs(out, exist_ok=True)
a = App(port)
# Fix Q3: never against the person's own settings. The app must run with SCRIM_SETTINGS_DIR set to
# a scratch folder (and a scratch HOME).
if not a.call('app.settings-dir', {}).get('result', {}).get('override'):
    raise SystemExit('start the app with SCRIM_SETTINGS_DIR set to a scratch settings folder')
T0 = time.monotonic()
log = {'url': url, 'events': [], 'captures': [], 'stderr': []}


def now():
    return round(time.monotonic() - T0, 4)


def event(what, **kw):
    log['events'].append({'t': now(), 'what': what, **kw})


class Bridge:
    def __init__(self):
        self.p = subprocess.Popen(['npx', '-y', 'mcp-remote@0.14.3', url], stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
        self.n = 0
        self.lines = {}
        self.cv = threading.Condition()
        threading.Thread(target=self.reader, daemon=True).start()
        threading.Thread(target=self.errs, daemon=True).start()

    def reader(self):
        for line in self.p.stdout:
            try:
                m = json.loads(line)
            except ValueError:
                continue
            with self.cv:
                if 'id' in m:
                    self.lines[m['id']] = m
                self.cv.notify_all()

    def errs(self):
        for line in self.p.stderr:
            log['stderr'].append({'t': now(), 'line': line.decode('utf-8', 'replace').rstrip()[:300]})

    def send(self, method, params=None, notify=False):
        msg = {'jsonrpc': '2.0', 'method': method}
        if params is not None:
            msg['params'] = params
        if not notify:
            self.n += 1
            msg['id'] = self.n
        self.p.stdin.write((json.dumps(msg) + '\n').encode())
        self.p.stdin.flush()
        if notify:
            return None
        with self.cv:
            ok = self.cv.wait_for(lambda: msg['id'] in self.lines, timeout=120)
        if not ok:
            raise SystemExit(f'no answer to {method}')
        return self.lines.pop(msg['id'])

    def tool(self, name, args):
        event('call', tool=name)
        r = self.send('tools/call', {'name': name, 'arguments': args})
        event('answer', tool=name, error=bool(r.get('error') or r.get('result', {}).get('isError')))
        res = r.get('result', {})
        text = res.get('content', [{}])[0].get('text', '') if res.get('content') else ''
        try:
            return json.loads(text)
        except ValueError:
            return text



def state(c):
    r = c.call('ui.inspect').get('result', {})
    ag = r.get('agent', {})
    v = c.call('engine.execute', {'command': 'view.get'}).get('result', {})
    return {'follow': ag.get('follow'), 'pausedBy': ag.get('pausedBy'), 'chip': ag.get('chip'), 'working': ag.get('working'), 'finished': ag.get('finished'),
            'alpha': ag.get('workingAlpha'), 'view': [v.get('cx'), v.get('cy'), v.get('zoom')]}


def capture_loop(stop):
    c, k = App(port), 0
    while not stop.is_set():
        t = time.monotonic()
        ok = c.call('ui.screenshot', {'path': f'{out}/f{k:04d}.png'}).get('ok')
        log['captures'].append({'k': k, 't': round(t - T0, 4), 'ok': ok, **state(c)})
        k += 1
        time.sleep(max(0.0, 0.1 - (time.monotonic() - t)))


def batch(b, poster, n, row):
    cmds = [{'command': 'shape.rectangle', 'params': {'x': 40 + (k % 8) * 125, 'y': 40 + row * 130 + (k // 8) * 130, 'width': 100, 'height': 100, 'parent': poster}} for k in range(n)]
    b.tool('batch', {'commands': cmds})


a.call('ui.set', {'windowSize': [1440, 900], 'settings': False, 'home': False})
time.sleep(0.8)
a.call('window.focus', {'id': 0})
a.run('file.new')
a.call('ui.set', {'home': False})
time.sleep(0.8)
a.run('panel.tab.ai')
a.run('pref.agentGrace', {'seconds': 15})
a.call('ui.input', {'events': [{'move': [700, 20]}]})
b = Bridge()
b.send('initialize', {'protocolVersion': '2025-06-18', 'capabilities': {}, 'clientInfo': {'name': 'local-agent-mode-scrim', 'version': '1.0.0'}})
b.send('notifications/initialized', notify=True)
b.send('tools/list', {})
time.sleep(1.0)
stop = threading.Event()
th = threading.Thread(target=capture_loop, args=(stop,))
th.start()
time.sleep(0.5)
event('batch', i=1)
poster = b.tool('run_command', {'command': 'frame.create', 'params': {'x': 0, 'y': 0, 'width': 1080, 'height': 1350, 'name': 'Poster'}})['id']
batch(b, poster, 8, 0)
time.sleep(3.0)
event('batch', i=2)
batch(b, poster, 8, 1)
time.sleep(1.5)
event('scroll')
a.call('ui.input', {'events': [{'move': [720, 460]}]})
a.call('ui.input', {'events': [{'scroll': [0, -200]}]})
time.sleep(2.0)
event('batch', i=3)
batch(b, poster, 8, 2)
time.sleep(2.5)
event('finish')
b.tool('finish_working', {})
event('finish-done')
time.sleep(3.0)
event('second-build')
second = b.tool('run_command', {'command': 'frame.create', 'params': {'x': 4000, 'y': 0, 'width': 1080, 'height': 1350, 'name': 'Poster 2'}})['id']
batch(b, second, 8, 0)
event('second-build-done')
time.sleep(3.0)
b.tool('finish_working', {})
time.sleep(1.5)
stop.set()
th.join()
log['diagnostics'] = [e for e in a.run('diagnostics.events').get('events', []) if e.get('kind') in ('follow', 'working')]
b.p.stdin.close()
try:
    b.p.wait(timeout=10)
except subprocess.TimeoutExpired:
    b.p.terminate()
json.dump(log, open(f'{out}/log.json', 'w'), indent=1)
print('captures', len(log['captures']))
