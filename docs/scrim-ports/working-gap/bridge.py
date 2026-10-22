"""bridge.py CONTROL_PORT MCP_URL OUT [--no-finish] [--grace N] — Fix M's reproduction with the real
bridge. Spawns `npx -y mcp-remote@0.14.3 MCP_URL` and speaks MCP to it over stdin/stdout, newline-
delimited JSON-RPC, as Claude Desktop does: initialize (clientInfo "local-agent-mode-scrim"),
notifications/initialized, tools/list, then a poster in six batches (6, 12, 18, 24, 30, 36 layers)
with 3-5 s between them, then finish_working (unless --no-finish).
The control channel sets the starting state, captures the window every 100 ms (`ui.screenshot`
plus `ui.inspect`) and samples `ui.inspect` every 25 ms, and reads the diagnostics at the end.
Writes OUT/log.json and OUT/fNNNN.png."""
import json, os, subprocess, sys, threading, time
sys.path.insert(0, os.path.dirname(os.path.abspath(sys.argv[0])))
from d3 import App

port, url, out = int(sys.argv[1]), sys.argv[2], sys.argv[3]
finish = '--no-finish' not in sys.argv
grace = int(sys.argv[sys.argv.index('--grace') + 1]) if '--grace' in sys.argv else 15
# --agent: after each batch, what the Desktop agent did between its batches (L3's record): a
# screenshot, inspect_document and lint_design. --tall: a 1080 x 1920 poster, each batch further down
# it, so following moves the camera between batches.
agent_calls, tall = '--agent' in sys.argv, '--tall' in sys.argv
# --real: a poster as an agent builds one: text in several sizes and fonts, ellipses and rounded
# rectangles with fills, each batch a band further down a 1080 x 1350 poster.
real = '--real' in sys.argv
# --artboards: each batch on its own new 1080 x 1350 poster, 1200 apart (as an agent duplicating its
# poster and building the next), so the indicator moves to a new artboard at each boundary.
artboards = '--artboards' in sys.argv
os.makedirs(out, exist_ok=True)
a = App(port)
# Fix Q3: never against the person's own settings. The app must run with SCRIM_SETTINGS_DIR set to
# a scratch folder (and a scratch HOME).
if not a.call('app.settings-dir', {}).get('result', {}).get('override'):
    raise SystemExit('start the app with SCRIM_SETTINGS_DIR set to a scratch settings folder')
T0 = time.monotonic()
log = {'artboards': artboards, 'url': url, 'finish': finish, 'grace': grace, 'agent': agent_calls, 'tall': tall, 'real': real, 'events': [], 'captures': [], 'samples': [], 'stderr': []}


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


def agent_state(c):
    r = c.call('ui.inspect').get('result', {})
    ag = r.get('agent', {})
    return {'frameMs': r.get('frameMs'), 'renderMs': r.get('renderMs'), 'alpha': ag.get('workingAlpha'), 'indicator': ag.get('indicator'), 'working': ag.get('working'), 'finished': ag.get('finished'),
            'clients': ag.get('workingClients'), 'follow': ag.get('follow')}


def capture_loop(stop):
    c, k = App(port), 0
    while not stop.is_set():
        t = time.monotonic()
        ok = c.call('ui.screenshot', {'path': f'{out}/f{k:04d}.png'}).get('ok')
        log['captures'].append({'k': k, 't': round(t - T0, 4), 'took': round(time.monotonic() - t, 4), 'ok': ok, **agent_state(c)})
        k += 1
        time.sleep(max(0.0, 0.1 - (time.monotonic() - t)))


def sample_loop(stop):
    c = App(port)
    while not stop.is_set():
        t = now()
        log['samples'].append({'t': t, **agent_state(c)})
        time.sleep(0.025)


a.call('ui.set', {'windowSize': [1440, 900], 'settings': False, 'home': False})
time.sleep(0.8)
a.call('window.focus', {'id': 0})
a.run('file.new')
a.call('ui.set', {'home': False})
time.sleep(0.8)
a.run('panel.tab.ai')
a.run('pref.agentGrace', {'seconds': grace})
a.run('diagnostics.retention', {'days': 7})
a.call('ui.input', {'events': [{'move': [700, 20]}]})

b = Bridge()
init = b.send('initialize', {'protocolVersion': '2025-06-18', 'capabilities': {}, 'clientInfo': {'name': 'local-agent-mode-scrim', 'version': '1.0.0'}})
log['serverInfo'] = init.get('result', {}).get('serverInfo')
b.send('notifications/initialized', notify=True)
tools = b.send('tools/list', {})
log['tools'] = len(tools.get('result', {}).get('tools', []))
time.sleep(1.0)

stop = threading.Event()
threads = [threading.Thread(target=capture_loop, args=(stop,)), threading.Thread(target=sample_loop, args=(stop,))]
for th in threads:
    th.start()
time.sleep(0.6)
sizes, gaps = [6, 12, 18, 24, 30, 36], [3, 5, 4, 3, 5]
poster, made = None, 0
for i, n in enumerate(sizes):
    event('batch', i=i, layers=n)
    count = n
    if artboards:
        poster = b.tool('run_command', {'command': 'frame.create', 'params': {'x': 1200 * i, 'y': 0, 'width': 1080, 'height': 1350, 'name': f'Poster {i + 1}'}})['id']
        count, made = n - 1, 0
        cmds = [{'command': 'shape.rectangle', 'params': {'x': 40 + (k % 8) * 120, 'y': 40 + (k // 8) * 120, 'width': 100, 'height': 100, 'parent': poster}} for k in range(count)]
        b.tool('batch', {'commands': cmds})
        event('batch-done', i=i)
        if i < 5:
            time.sleep(gaps[i])
        continue
    if poster is None:
        w, h = (1080, 1920) if tall else (1080, 1350) if real else (640, 900)
        poster = b.tool('run_command', {'command': 'frame.create', 'params': {'x': 0, 'y': 0, 'width': w, 'height': h, 'name': 'Poster'}})['id']
        count = n - 1
    cmds = []
    for k in range(made, made + count):
        if real:
            j, fonts = k - made, ['Inter', 'Source Serif 4', 'Geist']
            y = 40 + i * 215 + (j // 6) * 34
            if j % 3 == 0:
                cmds.append({'command': 'text.create', 'params': {'x': 40 + (j % 6) * 170, 'y': y, 'text': f'Line {k} — the moon is late tonight', 'fontFamily': fonts[k % 3], 'fontSize': 12 + (k % 5) * 6, 'parent': poster}})
            elif j % 3 == 1:
                cmds.append({'command': 'shape.ellipse', 'params': {'x': 40 + (j % 6) * 170, 'y': y, 'width': 28, 'height': 28, 'parent': poster, 'fill': '#E0603A'}})
            else:
                cmds.append({'command': 'shape.rectangle', 'params': {'x': 40 + (j % 6) * 170, 'y': y, 'width': 150, 'height': 30, 'cornerRadius': 8, 'parent': poster, 'fill': '#1E2A44'}})
        elif tall:
            j = k - made
            cmds.append({'command': 'shape.rectangle', 'params': {'x': 20 + (j % 12) * 85, 'y': 20 + i * 310 + (j // 12) * 85, 'width': 70, 'height': 70, 'parent': poster}})
        else:
            cmds.append({'command': 'shape.rectangle', 'params': {'x': 20 + (k % 15) * 40, 'y': 20 + (k // 15) * 40, 'width': 30, 'height': 30, 'parent': poster}})
    made += count
    b.tool('batch', {'commands': cmds})
    event('batch-done', i=i)
    if agent_calls:
        b.tool('screenshot', {'ids': [poster]})
        b.tool('inspect_document', {'depth': 2})
        b.tool('lint_design', {'id': poster})
        event('agent-calls-done', i=i)
    if i < 5:
        time.sleep(gaps[i])
time.sleep(2.0)
if finish:
    event('finish')
    b.tool('finish_working', {})
    event('finish-done')
    time.sleep(3.0)
else:
    event('last-request')
    time.sleep(grace + 3.0)
stop.set()
for th in threads:
    th.join()
log['diagnostics'] = [e for e in a.run('diagnostics.events').get('events', []) if e.get('kind') in ('working', 'mcpClient')]
b.p.stdin.close()
try:
    b.p.wait(timeout=10)
except subprocess.TimeoutExpired:
    b.p.terminate()
event('bridge-exit', code=b.p.returncode)
json.dump(log, open(f'{out}/log.json', 'w'), indent=1)
print('captures', len(log['captures']), 'samples', len(log['samples']), 'working events', sum(e['kind'] == 'working' for e in log['diagnostics']))
