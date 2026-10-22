"""q4.py APP_BINARY SCRATCH OUT --bridge npx|scrim — Fix Q4: a real agent's pauses at the default grace.
Starts the app (release build) with a scratch HOME and a scratch settings folder (SCRIM_SETTINGS_DIR,
Scrim's rules 9) whose MCP port is 7690, and checks that it says so (`app.settings-dir`).
The agent is a real bridge spoken to over stdin/stdout as Claude Desktop does: `npx -y
mcp-remote@0.14.3 <url>` (the client in the report), or Scrim's own `mcp-bridge` (what Connect writes
now). It builds a poster in six batches with 8, 12, 15, 10 and 9 s between them (a real agent's
pauses while it writes the next batch), then calls finish_working.
The control channel captures the window every 100 ms (`ui.screenshot` + `ui.inspect`), samples
`ui.inspect` every 25 ms, and reads the diagnostics at the end. Writes OUT/log.json, OUT/fNNNN.png."""
import json, os, subprocess, sys, threading, time
sys.path.insert(0, os.path.dirname(os.path.abspath(sys.argv[0])))
from d3 import App

exe, scratch, out = sys.argv[1], sys.argv[2], sys.argv[3]
which = sys.argv[sys.argv.index('--bridge') + 1]
settings, home = os.path.join(scratch, 'settings'), os.path.join(scratch, 'home')
os.makedirs(settings, exist_ok=True)
os.makedirs(home, exist_ok=True)
os.makedirs(out, exist_ok=True)
json.dump({'prefs': {'mcp': {'port': 7690, 'port_chosen': True}, 'settings_version': 7}}, open(os.path.join(settings, 'state.json'), 'w'))
env = dict(os.environ, HOME=home, SCRIM_SETTINGS_DIR=settings)
env.pop('ELECTRON_RUN_AS_NODE', None)
app = subprocess.Popen([exe, '--control', '7979'], env=env, stdout=open(f'{out}/app.log', 'w'), stderr=subprocess.STDOUT, cwd=home)
for _ in range(80):
    try:
        a = App(7979)
        a.run('mcp.status')
        break
    except Exception:
        time.sleep(0.25)
sd = a.call('app.settings-dir', {}).get('result', {})
if not sd.get('override'):
    raise SystemExit('the app is not on a scratch settings folder (SCRIM_SETTINGS_DIR)')
port = a.run('mcp.status')['port']
url = f'http://127.0.0.1:{port}/mcp'
T0 = time.monotonic()
log = {'bridge': which, 'url': url, 'settings_override': sd.get('override'), 'events': [], 'captures': [], 'samples': [], 'stderr': []}


def now():
    return round(time.monotonic() - T0, 4)


def event(what, **kw):
    log['events'].append({'t': now(), 'what': what, **kw})


class Bridge:
    def __init__(self, cmd, env):
        self.p = subprocess.Popen(cmd, stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE, env=env)
        self.n, self.lines, self.cv = 0, {}, threading.Condition()
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
            self.cv.wait_for(lambda: msg['id'] in self.lines, timeout=120)
        return self.lines.pop(msg['id'], {})

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
    return {'alpha': ag.get('workingAlpha'), 'indicator': ag.get('indicator'), 'working': ag.get('working'), 'finished': ag.get('finished'), 'clients': ag.get('workingClients')}


def capture_loop(stop):
    c, k = App(7979), 0
    while not stop.is_set():
        t = time.monotonic()
        ok = c.call('ui.screenshot', {'path': f'{out}/f{k:04d}.png'}).get('ok')
        log['captures'].append({'k': k, 't': round(t - T0, 4), 'ok': ok, **state(c)})
        k += 1
        time.sleep(max(0.0, 0.1 - (time.monotonic() - t)))


def sample_loop(stop):
    c = App(7979)
    while not stop.is_set():
        log['samples'].append({'t': now(), **state(c)})
        time.sleep(0.025)


a.call('ui.set', {'windowSize': [1440, 900], 'settings': False, 'home': False})
time.sleep(0.8)
a.call('window.focus', {'id': 0})
a.run('file.new')
a.call('ui.set', {'home': False})
time.sleep(0.8)
a.run('panel.tab.ai')
a.call('ui.input', {'events': [{'move': [700, 20]}]})
cmd = ['npx', '-y', 'mcp-remote@0.14.3', url] if which == 'npx' else [exe, 'mcp-bridge']
b = Bridge(cmd, env)
b.send('initialize', {'protocolVersion': '2025-06-18', 'capabilities': {}, 'clientInfo': {'name': 'local-agent-mode-scrim', 'version': '1.0.0'}})
b.send('notifications/initialized', notify=True)
b.send('tools/list', {})
time.sleep(1.0)
stop = threading.Event()
threads = [threading.Thread(target=capture_loop, args=(stop,)), threading.Thread(target=sample_loop, args=(stop,))]
for th in threads:
    th.start()
time.sleep(0.6)
sizes, gaps = [6, 12, 18, 24, 30, 36], [8, 12, 15, 10, 9]
poster, made = None, 0
for i, n in enumerate(sizes):
    event('batch', i=i, layers=n)
    count = n
    if poster is None:
        poster = b.tool('run_command', {'command': 'frame.create', 'params': {'x': 0, 'y': 0, 'width': 640, 'height': 900, 'name': 'Poster'}})['id']
        count = n - 1
    cmds = [{'command': 'shape.rectangle', 'params': {'x': 20 + (k % 15) * 40, 'y': 20 + (k // 15) * 40, 'width': 30, 'height': 30, 'parent': poster}} for k in range(made, made + count)]
    made += count
    b.tool('batch', {'commands': cmds})
    event('batch-done', i=i)
    if i < 5:
        time.sleep(gaps[i])
time.sleep(2.0)
event('finish')
b.tool('finish_working', {})
event('finish-done')
time.sleep(3.0)
stop.set()
for th in threads:
    th.join()
# A fresh connection: the control server closes one left idle through the batches.
a = App(7979)
log['diagnostics'] = [e for e in a.run('diagnostics.events').get('events', []) if e.get('kind') == 'working']
b.p.stdin.close()
try:
    b.p.wait(timeout=10)
except subprocess.TimeoutExpired:
    b.p.terminate()
a.call('app.quit', {})
try:
    app.wait(timeout=15)
except subprocess.TimeoutExpired:
    app.terminate()
saved = json.load(open(os.path.join(settings, 'state.json')))['prefs']
log['saved_grace'] = (saved.get('agent_grace_secs'), saved.get('agent_grace_chosen'))
json.dump(log, open(f'{out}/log.json', 'w'), indent=1)
print('captures', len(log['captures']), 'samples', len(log['samples']), 'saved grace', log['saved_grace'])
