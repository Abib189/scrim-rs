"""ie.py MODE CONTROL_PORT OUT [ARG] — Stage I.1's live checks. The agent is an MCP client on the app's
real HTTP endpoint ("codex-mcp-client", its own session); the control channel only captures, inspects
and sets the starting state.
MODE: steps        six batches (6, 12, 18, 24, 30, 36 layers) 2.5 s apart, then finish_working;
                   a capture thread takes a window screenshot and ui.inspect every 250 ms.
      grace        the same with the grace period at 5 s and no finish_working.
      sync ON|OFF  the six steps with a capture 1.0 s after each, dots and reveal on (ON) or off (OFF),
                   for the pixel diff outside the canvas.
      speed SPEED  one 24-layer and one 36-layer batch in view at SPEED; reveal durations measured.
      perf SPEED   (the Fixture open) a 16-layer batch into an artboard; frame and render times."""
import http.client, json, sys, threading, time
sys.path.insert(0, sys.argv[0].rsplit('/', 1)[0])
from d3 import App

mode, port, out = sys.argv[1], int(sys.argv[2]), sys.argv[3]
arg = sys.argv[4] if len(sys.argv) > 4 else None
a = App(port)
log = {'mode': mode, 'arg': arg, 'events': [], 'captures': []}
T0 = time.monotonic()


def now():
    return round(time.monotonic() - T0, 3)


def event(what, **kw):
    log['events'].append({'t': now(), 'what': what, **kw})


class Mcp:
    def __init__(self, port, name):
        self.port, self.n, self.session = port, 0, None
        self.c = http.client.HTTPConnection('127.0.0.1', port, timeout=60)
        init = self.rpc('initialize', {'protocolVersion': '2025-06-18', 'capabilities': {}, 'clientInfo': {'name': name, 'version': '0.162.1'}})
        self.rpc('notifications/initialized', notify=True)
        self.info = init['result']['serverInfo']

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
        r = self.rpc('tools/call', {'name': name, 'arguments': args})
        res = r['result']
        text = res['content'][0].get('text', '') if res.get('content') else ''
        if res.get('isError'):
            raise SystemExit(f'{name} {args}: {text}')
        try:
            return json.loads(text)
        except ValueError:
            return text


def step(m, i, n, x):
    """One step: an artboard, then n - 1 layers in it as one batch."""
    f = m.tool('run_command', {'command': 'frame.create', 'params': {'x': x, 'y': 0, 'width': 640, 'height': 480, 'name': f'Step {i + 1}'}})
    kids = [{'command': 'shape.rectangle', 'params': {'x': 20 + (k % 8) * 76, 'y': 20 + (k // 8) * 76, 'width': 60, 'height': 60, 'parent': f['id']}} for k in range(n - 1)]
    m.tool('batch', {'commands': kids})
    return f['id']


def agent_state(c):
    r = c.call('ui.inspect').get('result', {})
    ag = r.get('agent', {})
    return r, {'indicator': ag.get('indicator'), 'working': ag.get('working'), 'finished': ag.get('finished'),
               'revealing': ag.get('revealing'), 'clients': ag.get('workingClients'), 'follow': ag.get('follow'), 'degraded': ag.get('degraded')}


def capture_loop(stop, period=0.25):
    c = App(port)
    k = 0
    while not stop.is_set():
        t = time.monotonic()
        path = f'{out}/f{k:03d}.png'
        ok = c.call('ui.screenshot', {'path': path}).get('ok')
        _, st = agent_state(c)
        log['captures'].append({'k': k, 't': round(t - T0, 3), 'ok': ok, **st})
        k += 1
        time.sleep(max(0.0, period - (time.monotonic() - t)))


a.call('ui.set', {'windowSize': [1440, 900], 'settings': False, 'home': False})
time.sleep(0.8)
a.call('window.focus', {'id': 0})
if mode != 'perf':
    a.run('file.new')
    a.call('ui.set', {'home': False})
time.sleep(0.8)
a.run('panel.tab.ai')
a.run('pref.agentGrace', {'seconds': 5 if mode == 'grace' else 15})
a.run('pref.revealSpeed', {'speed': arg if mode in ('speed', 'perf') else 'normal'})
if mode == 'sync':
    a.run('pref.workingIndicator', {'on': arg == 'ON'})
    a.run('pref.revealAgentWork', {'on': arg == 'ON'})
mport = a.run('mcp.status')['port']
log['mcp_port'] = mport
a.call('ui.input', {'events': [{'move': [700, 20]}]})
m = Mcp(mport, 'codex-mcp-client')
log['session'] = bool(m.session)
time.sleep(1.0)

if mode in ('steps', 'grace'):
    stop = threading.Event()
    th = threading.Thread(target=capture_loop, args=(stop,))
    th.start()
    time.sleep(0.6)
    for i, n in enumerate([6, 12, 18, 24, 30, 36]):
        event('step', i=i, layers=n)
        step(m, i, n, 760 * i)
        event('step-done', i=i)
        if i < 5:
            time.sleep(2.5)
    time.sleep(2.5)
    if mode == 'steps':
        event('finish')
        r = m.tool('finish_working', {})
        event('finish-done', reply=r)
        time.sleep(2.5)
    else:
        event('last-request')
        time.sleep(7.5)
    stop.set()
    th.join()
elif mode == 'sync':
    for i, n in enumerate([6, 12, 18, 24, 30, 36]):
        step(m, i, n, 760 * i)
        time.sleep(1.0)
        a.shot(f'{out}/s{i}.png', 0.0)
        event('shot', i=i)
        time.sleep(1.5)
    m.tool('finish_working', {})
    time.sleep(1.0)
    a.shot(f'{out}/s9-finished.png', 0.0)
elif mode == 'speed':
    for n in (24, 36):
        a.run('view.set', {'cx': 320, 'cy': 240, 'zoom': 0.6})
        time.sleep(0.6)
        t0 = time.monotonic()
        step(m, 0 if n == 24 else 1, n, 0 if n == 24 else 760)
        landed = time.monotonic() - t0
        samples = []
        while time.monotonic() - t0 < 8.0:
            _, st = agent_state(a)
            samples.append({'t': round(time.monotonic() - t0, 3), 'revealing': st['revealing'], 'follow': st['follow']})
            time.sleep(0.03)
        on = [s['t'] for s in samples if (s['revealing'] or 0) > 0]
        log.setdefault('reveals', []).append({'layers': n, 'batch_reply_s': round(landed, 3), 'first': on[0] if on else None, 'last': on[-1] if on else None,
                                              'duration_s': round(on[-1] - on[0], 3) if on else None, 'samples': samples})
        a.shot(f'{out}/{arg}-{n}-final.png', 0.5)
    m.tool('finish_working', {})
elif mode == 'perf':
    target = a.run('node.query', {'query': '//FRAME', 'limit': 1})['results'][0]
    a.run('camera.focus-layer', {'id': target['id']})
    log['target'] = target
    time.sleep(2.0)
    kids = [{'command': 'shape.rectangle', 'params': {'x': 20 + k * 30, 'y': 20, 'width': 24, 'height': 24, 'parent': target['id']}} for k in range(16)]
    t0 = time.monotonic()
    m.tool('batch', {'commands': kids})
    log['batch_s'] = round(time.monotonic() - t0, 3)
    samples = []
    while time.monotonic() - t0 < 7.5:
        r, st = agent_state(a)
        samples.append({'t': round(time.monotonic() - t0, 3), 'frameMs': r.get('frameMs'), 'renderMs': r.get('renderMs'), **st})
        time.sleep(0.03)
    log['perf'] = samples
    m.tool('finish_working', {})

log['history'] = [(c['author'], c['label']) for c in a.run('history.changes')['changes']][-4:]
json.dump(log, open(f'{out}/log.json', 'w'), indent=1)
print('done', mode, arg, len(log['captures']), 'captures')
