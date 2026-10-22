#!/usr/bin/env python3
"""A stub agent CLI for the composer's tests (R5a, docs/design/chat-composer.md › Tests).

It takes Claude Code's arguments (`-p PROMPT --mcp-config JSON --output-format stream-json ...`),
calls the Scrim MCP endpoint the config names over HTTP, as the real CLI would, and prints
Claude Code's stream-json events. No model: what it does is read from the prompt.
- "rectangle named NAME": create_node, a 100x100 rectangle named NAME (default).
- "batches N": N batches of 3 rectangles, half a second apart.
- "slow N": start a grandchild (`sleep 300`), print its pid, then wait N seconds.
- "login": print a sign-in failure with a token-like string to stderr and exit 1 at once.
- "exit3": exit 3 after an error line.
- "env": print its environment's names; "cwd": its working folder and what is in it.
- "cost": the result reports usage and cost (otherwise it reports neither).
- "unreachable": Scrim isn't reached (the init event says so), and it exits 1.
It always calls finish_working when it has worked on the canvas."""
import json, os, re, subprocess, sys, time, urllib.request

args = sys.argv[1:]


def arg(flag):
    return args[args.index(flag) + 1] if flag in args and args.index(flag) + 1 < len(args) else None


prompt = arg('-p') or ''
config = json.loads(arg('--mcp-config') or '{}')
server = config.get('mcpServers', {}).get('scrim', {})
url = server.get('url', '')
session = None


def emit(event):
    print(json.dumps(event), flush=True)


def rpc(method, params, rid=1):
    global session
    headers = {'Content-Type': 'application/json', 'Accept': 'application/json, text/event-stream'}
    if session:
        headers['Mcp-Session-Id'] = session
    for k, v in server.get('headers', {}).items():
        headers[k] = re.sub(r'\$\{(\w+)\}', lambda m: os.environ.get(m.group(1), ''), v)
    req = urllib.request.Request(url, data=json.dumps({'jsonrpc': '2.0', 'id': rid, 'method': method, 'params': params}).encode(), headers=headers)
    with urllib.request.urlopen(req, timeout=30) as r:
        session = r.headers.get('Mcp-Session-Id') or session
        body = r.read().decode()
    return json.loads(body[body.index('{'):])


def tool(name, arguments):
    emit({'type': 'assistant', 'message': {'content': [{'type': 'tool_use', 'name': f'mcp__scrim__{name}', 'input': arguments}]}})
    return rpc('tools/call', {'name': name, 'arguments': arguments})


if 'login' in prompt:
    print('Error: Not logged in · Please run /login (key sk-ant-api03-abcdefghijklmnop)', file=sys.stderr, flush=True)
    sys.exit(1)
if 'cwd' in prompt.split():
    emit({'type': 'stub', 'cwd': os.getcwd(), 'files': sorted(os.listdir('.'))})
if 'env' in prompt.split():
    emit({'type': 'stub', 'env': sorted(os.environ.keys())})
if 'unreachable' in prompt:
    emit({'type': 'system', 'subtype': 'init', 'mcp_servers': [{'name': 'scrim', 'status': 'failed'}]})
    emit({'type': 'result', 'subtype': 'error_during_execution', 'is_error': True, 'result': 'Scrim was not reachable.'})
    sys.exit(1)
try:
    rpc('initialize', {'protocolVersion': '2025-06-18', 'capabilities': {}, 'clientInfo': {'name': 'scrim-stub-agent', 'version': '1.0'}})
    emit({'type': 'system', 'subtype': 'init', 'mcp_servers': [{'name': 'scrim', 'status': 'connected'}]})
except OSError as e:
    emit({'type': 'system', 'subtype': 'init', 'mcp_servers': [{'name': 'scrim', 'status': 'failed'}]})
    print(f'MCP server "scrim" failed: {e}', file=sys.stderr, flush=True)
    sys.exit(1)
made = []
m = re.search(r'batches (\d+)', prompt)
if m:
    for b in range(int(m.group(1))):
        cmds = [{'command': 'shape.rectangle', 'params': {'x': 120 * k, 'y': 120 * b, 'width': 100, 'height': 100}} for k in range(3)]
        tool('batch', {'commands': cmds})
        made.append(f'batch {b + 1}')
        time.sleep(0.5)
else:
    name = (re.search(r'named (\w+)', prompt) or [None, 'Test'])[1]
    r = tool('create_node', {'type': 'rectangle', 'name': name, 'x': 0, 'y': 0, 'width': 100, 'height': 100})
    made.append(name)
m = re.search(r'slow (\d+)', prompt)
if m:
    g = subprocess.Popen(['sleep', '300'])
    emit({'type': 'stub', 'grandchild': g.pid})
    time.sleep(int(m.group(1)))
tool('finish_working', {})
if 'exit3' in prompt:
    print('boom: something went wrong', file=sys.stderr, flush=True)
    sys.exit(3)
result = {'type': 'result', 'subtype': 'success', 'is_error': False, 'result': f'Made {", ".join(made)}.'}
if 'cost' in prompt:
    result.update({'usage': {'input_tokens': 1200, 'output_tokens': 80}, 'total_cost_usd': 0.0123})
emit(result)
