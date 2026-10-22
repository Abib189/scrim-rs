"""p3.py BINARY HOME OUT [--block PORT] — Fix P3: Claude Desktop's entry, started with Scrim closed.
1. The app is started once on a scratch HOME whose Settings ▸ MCP port is 7690 (never the real
   7680), and asked for Claude Desktop's entry (`mcp.claude-desktop-status`); then it quits.
2. The entry is started exactly as Claude Desktop starts it (its command and args, stdio), with the
   app closed, and spoken to as Desktop does: initialize ("claude-ai"), notifications/initialized,
   tools/list, then a tool call -> "Scrim is not running".
3. The app is started again (with --block PORT, something already holds that port, so it falls
   back to the next). The same bridge process, never restarted, receives tools/list_changed; the
   next tool calls go through: inspect_document, and a frame made in the window.
Window captures (`ui.screenshot`) at each stage; everything on stdout/stderr logged with times."""
import json, os, socket, subprocess, sys, threading, time
sys.path.insert(0, os.path.dirname(os.path.abspath(sys.argv[0])))
from d3 import App

exe, home, out = sys.argv[1], sys.argv[2], sys.argv[3]
block = int(sys.argv[sys.argv.index('--block') + 1]) if '--block' in sys.argv else None
os.makedirs(out, exist_ok=True)
T0 = time.monotonic()
log = {'events': [], 'stdout': [], 'stderr': [], 'block': block}


def now():
    return round(time.monotonic() - T0, 3)


def event(what, **kw):
    log['events'].append({'t': now(), 'what': what, **kw})
    print(f'{now():7.2f} {what} {kw}', flush=True)


def launch():
    # Fix Q3: a scratch HOME and a scratch settings folder (SCRIM_SETTINGS_DIR), never the person's.
    env = dict(os.environ, HOME=home, SCRIM_SETTINGS_DIR=d)
    env.pop('ELECTRON_RUN_AS_NODE', None)
    p = subprocess.Popen([exe, '--control', '7979'], env=env, stdout=open(f'{out}/app.log', 'a'), stderr=subprocess.STDOUT, cwd=home)
    for _ in range(80):
        try:
            a = App(7979)
            a.run('mcp.status')
            return p, a
        except Exception:
            time.sleep(0.25)
    raise SystemExit('the app did not come up')


def quit_app(p, a):
    # The app's own Quit (File ▸ Quit), so it shuts down as a person's quit does.
    try:
        a.call('app.quit', {})
    except (OSError, ValueError):
        pass
    try:
        p.wait(timeout=15)
    except subprocess.TimeoutExpired:
        p.terminate()
        p.wait(timeout=10)


def shot(a, name):
    a.call('ui.set', {'windowSize': [1440, 900], 'home': False})
    time.sleep(0.6)
    r = a.call('ui.screenshot', {'path': f'{out}/{name}.png'})
    event('capture', name=name, ok=r.get('ok'))


# 1. Settings ▸ MCP on 7690, the app once, its Claude Desktop entry.
d = os.path.join(home, 'scrim-settings')
os.makedirs(d, exist_ok=True)
state = os.path.join(d, 'state.json')
if not os.path.exists(state):
    json.dump({'prefs': {'mcp': {'port': 7690, 'port_chosen': True}, 'settings_version': 6}}, open(state, 'w'))
p, a = launch()
status = a.run('mcp.status')
entry = a.run('mcp.claude-desktop-status')
log['first_port'] = status['port']
log['entry'] = entry['entry']['scrim']
log['entry_state'] = entry['state']
event('entry', entry=entry['entry']['scrim'], state=entry['state'], port=status['port'])
quit_app(p, a)
event('app quit', note_left=os.path.exists(os.path.join(d, 'mcp-endpoint.json')), tools_noted=os.path.exists(os.path.join(d, 'mcp-tools.json')))
held = None
if block:
    held = socket.socket()
    held.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
    held.bind(('127.0.0.1', block))
    held.listen(1)
    event('port held by something else', port=block)

# 2. The entry, as Claude Desktop starts it: its command, its args, stdio.
e = log['entry']
env = dict(os.environ, HOME=home, SCRIM_SETTINGS_DIR=d, **e.get('env', {}))
bridge = subprocess.Popen([e['command'], *e['args']], stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE, env=env)
answers, cv = {}, threading.Condition()


def reader():
    for line in bridge.stdout:
        m = json.loads(line)
        log['stdout'].append({'t': now(), 'msg': m if 'method' in m else {'id': m.get('id'), 'keys': sorted((m.get('result') or {}).keys()) if isinstance(m.get('result'), dict) else None, 'error': m.get('error')}})
        with cv:
            if 'id' in m:
                answers[m['id']] = m
            if m.get('method') == 'notifications/tools/list_changed':
                answers['list_changed'] = now()
            cv.notify_all()


def errs():
    for line in bridge.stderr:
        log['stderr'].append({'t': now(), 'line': line.decode('utf-8', 'replace').rstrip()})


threading.Thread(target=reader, daemon=True).start()
threading.Thread(target=errs, daemon=True).start()


def send(msg):
    bridge.stdin.write((json.dumps(msg) + '\n').encode())
    bridge.stdin.flush()


def ask(i, method, params=None, wait=30):
    send({'jsonrpc': '2.0', 'id': i, 'method': method, **({'params': params} if params is not None else {})})
    with cv:
        cv.wait_for(lambda: i in answers, timeout=wait)
    return answers.get(i)


r = ask(1, 'initialize', {'protocolVersion': '2025-06-18', 'capabilities': {}, 'clientInfo': {'name': 'claude-ai', 'version': '0.1.0'}})
event('initialize answered with Scrim closed', server=r['result']['serverInfo'], listChanged=r['result']['capabilities']['tools']['listChanged'])
send({'jsonrpc': '2.0', 'method': 'notifications/initialized'})
r = ask(2, 'tools/list')
event('tools/list with Scrim closed', tools=len(r['result']['tools']))
r = ask(3, 'tools/call', {'name': 'inspect_document', 'arguments': {}})
event('tool call with Scrim closed', isError=r['result']['isError'], text=r['result']['content'][0]['text'])
time.sleep(5)
event('still running after 5 s', alive=bridge.poll() is None)

# 3. Scrim opens; the same bridge, never restarted.
p, a = launch()
status = a.run('mcp.status')
event('app up', port=status['port'])
a.run('file.new')
a.call('ui.set', {'home': False})
a.run('panel.tab.ai')
with cv:
    cv.wait_for(lambda: 'list_changed' in answers, timeout=10)
event('tools/list_changed received', at=answers.get('list_changed'))
r = ask(4, 'tools/list')
event('tools/list now', tools=len(r['result']['tools']))
r = ask(5, 'tools/call', {'name': 'inspect_document', 'arguments': {}})
event('inspect_document through the bridge', isError=r['result']['isError'])
r = ask(6, 'tools/call', {'name': 'run_command', 'arguments': {'command': 'frame.create', 'params': {'x': 0, 'y': 0, 'width': 640, 'height': 400, 'name': 'Made through scrim mcp-bridge'}}})
event('frame made through the bridge', isError=r['result']['isError'])
time.sleep(1.5)
shot(a, 'window-after')
log['clients'] = a.run('mcp.status').get('clients')
log['bridge_pid_same'] = bridge.poll() is None
bridge.stdin.close()
bridge.wait(timeout=10)
event('stdin closed: bridge exited', code=bridge.returncode)
time.sleep(1.0)
shot(a, 'window-after-bridge-closed')
quit_app(p, a)
if held:
    held.close()
for s in log['stderr']:
    assert 'Bearer' not in s['line']
json.dump(log, open(f'{out}/log.json', 'w'), indent=1)
