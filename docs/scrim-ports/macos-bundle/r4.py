"""r4.py BUNDLE SCRATCH OUT — R4: the double-clickable Scrim.app, checked on a scratch HOME and a
scratch settings folder (SCRIM_SETTINGS_DIR, Scrim's rules 9). Never the person's own settings or
Claude Desktop's real configuration: both are only read (timestamp, size, checksum) before and after.
1. `open -n BUNDLE` (Launch Services, no terminal): the window comes up (Launch Services lists it
   as a foreground app) and its MCP endpoint answers `initialize` as "scrim".
2. `Contents/MacOS/figcraft mcp-bridge`, started the way Claude Desktop starts its entry (stdio):
   initialize, tools/list, a tool call through to the window. Meanwhile Launch Services doesn't list
   the bridge's process (no Dock icon, not a foreground app).
3. Connect Claude Desktop from inside the bundle: its entry is this bundle's figcraft with
   `mcp-bridge`. Written into the scratch configuration it reads Configured.
4. The app moved (a copy elsewhere, opened instead): Out of date. Re-register, pressed by keyboard
   in the AI tab as a person would, makes it Configured with the new path.
Steps 1's capture and 4's keyboard press need the window to draw: with the screen locked, macOS draws no
window, so they are skipped and the log says so. The moved copy is unregistered from Launch Services and
deleted at the end. Paths in the log are
written as <bundle>, <moved>, <scratch>."""
import hashlib, json, os, shutil, subprocess, sys, threading, time, urllib.request
sys.path.insert(0, os.path.dirname(os.path.abspath(sys.argv[0])))
from d3 import App

bundle, scratch, out = os.path.realpath(sys.argv[1]), os.path.realpath(sys.argv[2]), sys.argv[3]
home, settings = os.path.join(scratch, 'home'), os.path.join(scratch, 'settings')
moved = os.path.join(scratch, 'Moved', 'Scrim.app')
for d in (home, settings, out):
    os.makedirs(d, exist_ok=True)
json.dump({'prefs': {'mcp': {'port': 7690, 'port_chosen': True}, 'settings_version': 7}}, open(os.path.join(settings, 'state.json'), 'w'))
LSREGISTER = '/System/Library/Frameworks/CoreServices.framework/Frameworks/LaunchServices.framework/Support/lsregister'
REAL = [os.path.expanduser('~/Library/Application Support/Claude/claude_desktop_config.json'),
        os.path.expanduser('~/Library/Application Support/design.scrim.app/state.json')]
T0 = time.monotonic()
log = {'events': []}


def scrub(v):
    s = json.dumps(v)
    for path, name in ((moved, '<moved>'), (bundle, '<bundle>'), (scratch, '<scratch>'), (os.path.expanduser('~'), '/Users/<user>')):
        s = s.replace(path, name)
    return json.loads(s)


def event(what, **kw):
    e = scrub({'t': round(time.monotonic() - T0, 2), 'what': what, **kw})
    log['events'].append(e)
    print(e, flush=True)


def fingerprint(path):
    if not os.path.exists(path):
        return None
    st = os.stat(path)
    return [int(st.st_mtime), st.st_size, hashlib.sha256(open(path, 'rb').read()).hexdigest()[:16]]


def screen_locked():
    out_ = subprocess.run(['ioreg', '-n', 'Root', '-d1', '-a'], capture_output=True, text=True).stdout
    i = out_.find('CGSSessionScreenIsLocked')
    return i >= 0 and '<true/>' in out_[i:i + 80]


def lsappinfo_pids():
    """The pids Launch Services lists as applications."""
    out_ = subprocess.run(['lsappinfo', 'list'], capture_output=True, text=True).stdout
    return {int(l.split('pid = ')[1].split()[0]) for l in out_.splitlines() if 'pid = ' in l}


def launch(app_path):
    subprocess.run(['open', '-n', app_path, '--env', f'HOME={home}', '--env', f'SCRIM_SETTINGS_DIR={settings}', '--args', '--control', '7979'], check=True)
    for _ in range(120):
        try:
            a = App(7979)
            a.run('mcp.status')
            return a
        except Exception:
            time.sleep(0.25)
    raise SystemExit('the app did not come up')


def quit_app(a):
    try:
        a.call('app.quit', {})
    except (OSError, ValueError):
        pass
    for _ in range(60):
        if not subprocess.run(['pgrep', '-f', 'Contents/MacOS/figcraft --control 7979'], capture_output=True).stdout:
            return
        time.sleep(0.25)


def pid_of(app_path):
    r = subprocess.run(['pgrep', '-f', os.path.join(app_path, 'Contents/MacOS/figcraft')], capture_output=True, text=True).stdout.split()
    return int(r[0]) if r else None


log['real_before'] = [fingerprint(p) for p in REAL]

# 1. Opened by Launch Services, no terminal.
a = launch(bundle)
sd = a.call('app.settings-dir', {}).get('result', {})
if not sd.get('override'):
    raise SystemExit('the app is not on a scratch settings folder (SCRIM_SETTINGS_DIR)')
pid = pid_of(bundle)
info = subprocess.run(['lsappinfo', 'info', '-only', 'name,bundleID,applicationtype', str(pid)], capture_output=True, text=True).stdout.strip()
event('opened with `open`', pid_listed_by_launch_services=pid in lsappinfo_pids(), info=info)
LOCKED = screen_locked()
log['screen_locked'] = LOCKED
if LOCKED:
    event('screen locked: macOS draws no window, so no capture and no keyboard input')
else:
    a.call('ui.set', {'windowSize': [1440, 900], 'home': True})
    time.sleep(1.0)
    event('window', capture=a.call('ui.screenshot', {'path': os.path.join(out, 'r4-window.png')}).get('ok'))
endpoint = json.load(open(os.path.join(settings, 'mcp-endpoint.json')))
req = urllib.request.Request(f"http://127.0.0.1:{endpoint['port']}/mcp", data=json.dumps({'jsonrpc': '2.0', 'id': 1, 'method': 'initialize', 'params': {'protocolVersion': '2025-06-18', 'capabilities': {}, 'clientInfo': {'name': 'r4-check', 'version': '1'}}}).encode(), headers={'Content-Type': 'application/json', 'Accept': 'application/json, text/event-stream'})
body = urllib.request.urlopen(req, timeout=10).read().decode()
server = json.loads(body[body.index('{'):])['result']['serverInfo']
event('MCP endpoint', port=endpoint['port'], server=server)

# 2. The bridge, as Claude Desktop starts it.
status = a.run('mcp.claude-desktop-status')
entry = status['entry']['scrim']
event('Claude Desktop entry', entry=entry, config=status['path'], state=status['state'])
assert status['path'].startswith(home), 'the configuration Scrim reads is the scratch one'
before = lsappinfo_pids()
bridge = subprocess.Popen([entry['command'], *entry['args']], stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, env=dict(os.environ, HOME=home, SCRIM_SETTINGS_DIR=settings))
answers, cv = {}, threading.Condition()


def reader():
    for line in bridge.stdout:
        m = json.loads(line)
        with cv:
            answers[m.get('id')] = m
            cv.notify_all()


threading.Thread(target=reader, daemon=True).start()


def ask(i, method, params=None):
    bridge.stdin.write((json.dumps({'jsonrpc': '2.0', 'id': i, 'method': method, **({'params': params} if params is not None else {})}) + '\n').encode())
    bridge.stdin.flush()
    with cv:
        cv.wait_for(lambda: i in answers, timeout=30)
    return answers.get(i)


r = ask(1, 'initialize', {'protocolVersion': '2025-06-18', 'capabilities': {}, 'clientInfo': {'name': 'claude-ai', 'version': '0.1.0'}})
bridge.stdin.write(b'{"jsonrpc":"2.0","method":"notifications/initialized"}\n')
bridge.stdin.flush()
tools = ask(2, 'tools/list')
made = ask(3, 'tools/call', {'name': 'create_node', 'arguments': {'type': 'frame', 'name': 'Made through the bridge', 'x': 0, 'y': 0, 'width': 200, 'height': 120}})
time.sleep(1.0)
after = lsappinfo_pids()
event('the bridge, as Claude Desktop starts it', server=r['result']['serverInfo'], tools=len(tools['result']['tools']), tool_call_error=made['result'].get('isError'),
      bridge_listed_by_launch_services=bridge.pid in after, apps_added_while_it_ran=len(after - before))
bridge.stdin.close()
bridge.wait(timeout=15)
event('bridge exited', code=bridge.returncode)

# 3. Connected: the entry written into the scratch configuration.
cfg = status['path']
os.makedirs(os.path.dirname(cfg), exist_ok=True)
json.dump({'mcpServers': {'scrim': entry}}, open(cfg, 'w'), indent=2)
event('entry written into the scratch configuration', state=a.run('mcp.claude-desktop-status')['state'])
quit_app(a)

# 4. Moved: a copy elsewhere, opened instead.
shutil.rmtree(os.path.dirname(moved), ignore_errors=True)
os.makedirs(os.path.dirname(moved))
subprocess.run(['ditto', bundle, moved], check=True)
a = launch(moved)
st = a.run('mcp.claude-desktop-status')
event('opened from the new place', state=st['state'], registered=st.get('registered'), wants=st['entry']['scrim'])
a.call('ui.set', {'windowSize': [1440, 900], 'home': False})
a.run('file.new')
a.call('ui.set', {'home': False})
a.run('panel.tab.ai')
time.sleep(1.0)


def focus():
    return (a.call('ui.inspect', {}).get('result', {}).get('focus') or {}).get('control')


def press(name, limit=120):
    for _ in range(limit):
        if focus() == name:
            a.call('ui.input', {'events': [{'key': 'Space'}]})
            time.sleep(0.6)
            return True
        a.call('ui.input', {'events': [{'key': 'Tab'}]})
        time.sleep(0.05)
    return False


if LOCKED:
    event('Re-register skipped: it needs the window drawn (the screen is locked)')
else:
    pressed = press('Re-register')
    event('Re-register pressed by keyboard', found=pressed)
    confirmed = press('Re-register', limit=40)
    event('the confirmation pressed', found=confirmed)
    time.sleep(1.0)
st = a.run('mcp.claude-desktop-status')
written = json.load(open(cfg))['mcpServers']['scrim']
event('after re-register', state=st['state'], entry_now=written, backups=len([f for f in os.listdir(os.path.dirname(cfg)) if f != os.path.basename(cfg)]))
quit_app(a)
subprocess.run([LSREGISTER, '-u', moved])
shutil.rmtree(os.path.dirname(moved), ignore_errors=True)
event('moved copy unregistered and deleted', gone=not os.path.exists(moved))
log['real_after'] = [fingerprint(p) for p in REAL]
log['real_unchanged'] = log.pop('real_before') == log.pop('real_after')
json.dump(scrub(log), open(os.path.join(out, 'log.json'), 'w'), indent=1)
print('real files unchanged:', log['real_unchanged'])
