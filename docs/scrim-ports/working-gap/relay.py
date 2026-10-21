"""relay.py LISTEN_PORT UPSTREAM_PORT LOG_JSONL — a transparent HTTP/1.1 relay that logs what an MCP
client does on the wire: each connection opening and closing, and each request's method, path,
Mcp-Session-Id, JSON-RPC method (and tool name), with the response's status and Mcp-Session-Id.
One upstream connection per client connection, closed when the client's closes, so keep-alive and
connection reuse are exactly the client's. The Host header is rewritten to the app's port (the endpoint admits only its own Host); nothing
else is changed. Bodies are not logged (only the JSON-RPC method, the
tool name and the batch size)."""
import json, socket, sys, threading, time

listen, upstream, path = int(sys.argv[1]), int(sys.argv[2]), sys.argv[3]
T0 = time.monotonic()
lock = threading.Lock()
out = open(path, 'a')
conns = [0]


def log(**kw):
    with lock:
        out.write(json.dumps({'t': round(time.monotonic() - T0, 4), 'wall': time.time(), **kw}) + '\n')
        out.flush()


def read_message(f):
    """One HTTP message (headers and a Content-Length or chunked body) from file `f`, raw, plus
    its start line and headers; None at EOF."""
    head = b''
    while not head.endswith(b'\r\n\r\n'):
        line = f.readline()
        if not line:
            return None
        head += line
    lines = head.decode('latin-1').split('\r\n')
    headers = {}
    for h in lines[1:]:
        if ':' in h:
            k, v = h.split(':', 1)
            headers[k.strip().lower()] = v.strip()
    body = b''
    if headers.get('transfer-encoding', '').lower() == 'chunked':
        raw = b''
        while True:
            size_line = f.readline()
            raw += size_line
            n = int(size_line.strip().split(b';')[0], 16)
            chunk = f.read(n + 2)
            raw += chunk
            body += chunk[:-2]
            if n == 0:
                break
        return lines[0], headers, head + raw, body
    n = int(headers.get('content-length', '0') or 0)
    if n:
        body = f.read(n)
    return lines[0], headers, head + body, body


def describe(body):
    try:
        m = json.loads(body)
    except ValueError:
        return {}
    ms = m if isinstance(m, list) else [m]
    d = []
    for x in ms:
        e = {'rpc': x.get('method'), 'id': x.get('id')}
        p = x.get('params') or {}
        if x.get('method') == 'tools/call':
            e['tool'] = p.get('name')
            a = p.get('arguments') or {}
            if 'commands' in a:
                e['commands'] = len(a['commands'])
            if 'command' in a:
                e['command'] = a['command']
        if x.get('method') == 'initialize':
            e['clientInfo'] = p.get('clientInfo')
        d.append(e)
    return {'messages': d}


def serve(c, n):
    log(event='open', conn=n)
    u = socket.create_connection(('127.0.0.1', upstream))
    cf, uf = c.makefile('rb'), u.makefile('rb')
    try:
        while True:
            req = read_message(cf)
            if req is None:
                break
            start, h, raw, body = req
            log(event='request', conn=n, line=start, session=h.get('mcp-session-id'), connection=h.get('connection'), **describe(body))
            # The endpoint admits only its own Host (DNS rebinding): name it, as the client would.
            raw = raw.replace(f'127.0.0.1:{listen}'.encode(), f'127.0.0.1:{upstream}'.encode(), 1).replace(f'localhost:{listen}'.encode(), f'localhost:{upstream}'.encode(), 1)
            u.sendall(raw)
            resp = read_message(uf)
            if resp is None:
                log(event='upstream-closed', conn=n)
                break
            rstart, rh, rraw, rbody = resp
            log(event='response', conn=n, line=rstart, session=rh.get('mcp-session-id'), connection=rh.get('connection'))
            c.sendall(rraw)
            if (rh.get('connection') or '').lower() == 'close':
                break
    except (OSError, ValueError) as e:
        log(event='error', conn=n, error=str(e))
    finally:
        log(event='close', conn=n)
        for s in (c, u):
            try:
                s.close()
            except OSError:
                pass


srv = socket.socket()
srv.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
srv.bind(('127.0.0.1', listen))
srv.listen(64)
log(event='listening', port=listen, upstream=upstream)
while True:
    c, _ = srv.accept()
    conns[0] += 1
    threading.Thread(target=serve, args=(c, conns[0]), daemon=True).start()
