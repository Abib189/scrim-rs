"""repro.py BINARY SCRATCH — Fix V1: where did the Follow agents setting go?

Starts the app (BINARY) once per case, each time on its own scratch HOME and scratch settings folder
(SCRIM_SETTINGS_DIR, Scrim's rules 9; MCP on 7690, never 7680; control channel on 7979), with a
settings file written as a given build wrote it. It reads following's state from `ui.inspect` (the
engine's state and the AI panel's header button), quits the app, and reads `follow_agents` back from
the settings file the app saved. The last cases are an agent: an MCP client on the app's HTTP
endpoint running `camera.follow-agents` through `run_command` and through `batch`.

The settings files are `apps/figcraft/tests/fixtures/settings/`: `state-v7-before-fix-t.json`, as
the build before Fix T wrote it (settings_version 7, Follow on, an unmarked 30 s grace), and
`state-v8-fix-t.json`, as a Fix T build wrote it. Earlier versions are the first with the keys
added after them removed (follow_agents came with version 3). Prints one line per case."""
import copy, http.client, json, os, subprocess, sys, time

HERE = os.path.dirname(os.path.abspath(sys.argv[0]))
sys.path.insert(0, os.path.join(HERE, '..', 'follow-agent', 't6'))
from d3 import App

exe, scratch = sys.argv[1], sys.argv[2]
FIX = os.path.join(HERE, '..', '..', '..', 'apps', 'figcraft', 'tests', 'fixtures', 'settings')
old = json.load(open(os.path.join(FIX, 'state-v7-before-fix-t.json')))
fix_t = json.load(open(os.path.join(FIX, 'state-v8-fix-t.json')))
# The key each settings_version added (Settings::settings_version's list).
ADDED = {3: ['follow_agents'], 4: ['reveal_agent_work', 'working_indicator'], 5: ['agent_grace_secs'], 6: ['reveal_speed'], 7: ['agent_grace_chosen']}


def at_version(v, follow=True):
    d = copy.deepcopy(old)
    p = d['prefs']
    for since, keys in ADDED.items():
        if v < since:
            for k in keys:
                p.pop(k, None)
    if v == 0:
        p.pop('settings_version')
    else:
        p['settings_version'] = v
    if 'follow_agents' in p:
        p['follow_agents'] = follow
    return d


class Mcp:
    def __init__(self, port):
        self.n, self.session = 0, None
        self.c = http.client.HTTPConnection('127.0.0.1', port, timeout=60)
        self.rpc('initialize', {'protocolVersion': '2025-06-18', 'capabilities': {}, 'clientInfo': {'name': 'repro-agent', 'version': '1.0'}})
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
        return (res.get('content') or [{}])[0].get('text', ''), bool(res.get('isError'))


def case(name, state, agent=None):
    root = os.path.join(scratch, name)
    home, settings = os.path.join(root, 'home'), os.path.join(root, 'settings')
    for d in (home, settings):
        os.makedirs(d, exist_ok=True)
    state = copy.deepcopy(state)
    state.setdefault('prefs', {})['mcp'] = {'port': 7690, 'port_chosen': True, 'require_auth': False}
    json.dump(state, open(os.path.join(settings, 'state.json'), 'w'))
    env = dict(os.environ, HOME=home, SCRIM_SETTINGS_DIR=settings)
    app = subprocess.Popen([exe, '--control', '7979'], env=env, stdout=open(os.path.join(root, 'app.log'), 'w'), stderr=subprocess.STDOUT, cwd=home)
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
    # A document open with the AI tab showing, so the header's Follow button is drawn.
    a.run('file.new')
    a.call('ui.set', {'home': False, 'rightTab': 'ai'})
    time.sleep(1.0)
    said = ''
    if agent:
        m = Mcp(port)
        for tool, args in agent:
            text, err = m.tool(tool, args)
            text = ' '.join(text.split())
            if tool == 'list_commands':
                at = text.find('camera.follow-agents')
                text = text[max(0, at - 10):at + 160]
            said += f' | {tool}{" (refused)" if err else ""}: {text[:170]}'
        time.sleep(0.5)
    r = a.call('ui.inspect').get('result', {})
    engine = r.get('agent', {}).get('follow')
    button = r.get('agent', {}).get('aiPanel', {}).get('header', {}).get('follow')
    a.call('window.focus', {'id': 0})
    a.call('app.quit', {})
    try:
        app.wait(timeout=20)
    except subprocess.TimeoutExpired:
        app.kill()
    saved = json.load(open(os.path.join(settings, 'state.json')))['prefs']
    # What Settings ▸ Diagnostics kept that mentions following.
    traces = [e.get('detail', '') for e in saved.get('diagnostics', []) if 'follow' in (e.get('kind', '') + e.get('detail', '')).lower()]
    print(f"{name:28} read {state['prefs'].get('follow_agents', '(no key)')!s:8} engine {engine!s:10} button {button!s:10} "
          f"saved follow_agents {saved.get('follow_agents')!s:6} settings_version {saved.get('settings_version')} diagnostics on following {len(traces)}{said}", flush=True)


case('fresh', {'prefs': {}})
case('v7-before-fix-t-on', old)
case('v8-fix-t-on', fix_t)
for v in range(0, 8):
    case(f'v{v}-on', at_version(v))
for v in range(3, 8):
    case(f'v{v}-off', at_version(v, follow=False))
case('agent-run-command-no-on', old, [('run_command', {'command': 'camera.follow-agents', 'params': {}})])
case('agent-run-command-on-false', old, [('run_command', {'command': 'camera.follow-agents', 'params': {'on': False}})])
case('agent-list-then-toggle', old, [('list_commands', {'filter': 'agent'}), ('run_command', {'command': 'camera.follow-agents', 'params': {}})])
case('agent-batch', old, [('batch', {'commands': [{'command': 'camera.follow-agents', 'params': {}}]})])
