"""s5.py BINARY SCRATCH OUT [BRIDGE] — Fix S5: the real bridge (BRIDGE mcp-bridge: the `scrim` CLI, or
the app's own binary by default) with a stub client that stays
connected while the app quits and starts again: on the same port, then on another, then on the same
port with no call while it was down.
The app runs on a scratch HOME and a scratch settings folder (SCRIM_SETTINGS_DIR, Scrim's rules 9;
MCP on 7690 then 7691, never 7680); the bridge reads the same scratch folder. The client is this
script on the bridge's stdin and stdout. Around each quit, calls go one after another (the bridge
answers in order), so one is at the app when it goes; every answer and how long it took is kept,
and every `notifications/tools/list_changed`.
Writes OUT/s5.json (OUT/s5-scrim.json when BRIDGE is the `scrim` CLI)."""
import json, os, subprocess, sys, threading, time
HERE = os.path.dirname(os.path.abspath(sys.argv[0]))
sys.path.insert(0, HERE)
from d3 import App

exe, scratch, out = sys.argv[1], sys.argv[2], sys.argv[3]
bridge = sys.argv[4] if len(sys.argv) > 4 else exe
name = 's5-scrim' if os.path.basename(bridge) == 'scrim' else 's5'
home, settings = os.path.join(scratch, 'home'), os.path.join(scratch, 'settings')
for d in (home, settings, out):
    os.makedirs(d, exist_ok=True)
env = dict(os.environ, HOME=home, SCRIM_SETTINGS_DIR=settings)
env.pop('SCRIM_MCP_AUTH', None)
t0 = time.monotonic()
events = []


def now():
    return round(time.monotonic() - t0, 3)


def note(kind, **kw):
    events.append(dict(t=now(), kind=kind, **kw))
    print(now(), kind, json.dumps(kw)[:200], flush=True)


def set_port(port):
    path = os.path.join(settings, 'state.json')
    state = json.load(open(path)) if os.path.exists(path) else {}
    state.setdefault('prefs', {}).setdefault('mcp', {}).update({'port': port, 'port_chosen': True})
    state['prefs']['settings_version'] = 7
    json.dump(state, open(path, 'w'))


def start_app(port, n):
    set_port(port)
    app = subprocess.Popen([exe, '--control', '7979'], env=env, stdout=open(os.path.join(out, f'app-{n}.log'), 'w'), stderr=subprocess.STDOUT, cwd=home)
    for _ in range(200):
        try:
            a = App(7979)
            st = a.run('mcp.status')
            if st.get('port'):
                break
        except Exception:
            time.sleep(0.25)
    if not a.call('app.settings-dir', {}).get('result', {}).get('override'):
        raise SystemExit('the app is not on a scratch settings folder (SCRIM_SETTINGS_DIR)')
    st = a.run('mcp.status')
    if st.get('port') == 7680:
        raise SystemExit("the MCP endpoint is on 7680, the person's own Scrim")
    note('app started', pid=app.pid, port=st.get('port'), state=st.get('state'))
    return app, a


def quit_app(app, a):
    a.call('window.focus', {'id': 0})
    a.call('app.quit', {})
    t = time.monotonic()
    try:
        app.wait(timeout=30)
        note('app quit', after=round(time.monotonic() - t, 3))
    except subprocess.TimeoutExpired:
        note('app did not quit in 30 s')


class Client:
    def __init__(self):
        self.p = subprocess.Popen([bridge, 'mcp-bridge'], env=env, stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=open(os.path.join(out, f'{name}-bridge.err'), 'w'), text=True, bufsize=1)
        self.n, self.answers, self.cv = 0, {}, threading.Condition()
        threading.Thread(target=self.read, daemon=True).start()

    def read(self):
        for line in self.p.stdout:
            msg = json.loads(line)
            if 'id' in msg and msg.get('id') is not None and 'method' not in msg:
                with self.cv:
                    self.answers[msg['id']] = (now(), msg)
                    self.cv.notify_all()
            else:
                note('from the bridge', method=msg.get('method'))

    def send(self, msg):
        self.p.stdin.write(json.dumps(msg) + '\n')
        self.p.stdin.flush()

    def rpc(self, method, params=None, timeout=330):
        self.n += 1
        i = self.n
        sent = now()
        self.send({'jsonrpc': '2.0', 'id': i, 'method': method, 'params': params or {}})
        with self.cv:
            self.cv.wait_for(lambda: i in self.answers, timeout=timeout)
        got = self.answers.get(i)
        return sent, got

    def call(self, label):
        sent, got = self.rpc('tools/call', {'name': 'list_commands', 'arguments': {'filter': 'zoom'}})
        if not got:
            note('call', label=label, sent=sent, answer=None)
            return None
        at, msg = got
        res = msg.get('result') or {}
        text = (res.get('content') or [{}])[0].get('text', '') if res else json.dumps(msg.get('error'))
        ok = bool(res) and not res.get('isError')
        note('call', label=label, sent=sent, took=round(at - sent, 3), ok=ok, text=text[:90])
        return ok


app, a = start_app(7690, 1)
c = Client()
_, init = c.rpc('initialize', {'protocolVersion': '2025-06-18', 'capabilities': {}, 'clientInfo': {'name': 's5-stub-client', 'version': '1'}})
note('initialize', server=init[1]['result']['serverInfo'], listChanged=init[1]['result']['capabilities'].get('tools', {}).get('listChanged'))
c.send({'jsonrpc': '2.0', 'method': 'notifications/initialized'})
_, tl = c.rpc('tools/list')
note('tools/list', tools=len(tl[1]['result']['tools']))
c.call('before any restart')


def calls_across_quit(label):
    """Calls one after another while the app quits; returns when the quit is done."""
    stop = threading.Event()

    def caller():
        k = 0
        while not stop.is_set():
            c.call(f'{label}: call {k} across the quit')
            k += 1
            time.sleep(0.05)

    th = threading.Thread(target=caller)
    th.start()
    time.sleep(0.6)
    quit_app(app, a)
    time.sleep(1.0)
    stop.set()
    th.join(timeout=340)


# 1. The same port.
calls_across_quit('restart 1 (same port)')
c.call('restart 1: while the app is closed')
app, a = start_app(7690, 2)
time.sleep(5)
c.call('restart 1: the first call after it')

# 2. Another port.
calls_across_quit('restart 2 (port 7691)')
app, a = start_app(7691, 3)
time.sleep(5)
c.call('restart 2: the first call after it')

# 3. The same port, no call while it was down.
quit_app(app, a)
app, a = start_app(7691, 4)
time.sleep(5)
c.call('restart 3 (no call while down): the first call after it')

quit_app(app, a)
c.p.stdin.close()
try:
    c.p.wait(timeout=10)
    note('bridge ended', code=c.p.returncode)
except subprocess.TimeoutExpired:
    note('bridge did not end')
json.dump(events, open(os.path.join(out, f'{name}.json'), 'w'), indent=1)
