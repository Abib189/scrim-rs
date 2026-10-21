"""d3.py — control-channel helper for the live checks: one JSON line per request on 127.0.0.1:PORT."""
import json, socket, time


class App:
    def __init__(self, port):
        self.s = socket.create_connection(("127.0.0.1", port), timeout=120)
        self.f = self.s.makefile("rwb")
        self.n = 0

    def call(self, m, p={}):
        self.n += 1
        self.f.write((json.dumps({"id": self.n, "method": m, "params": p}) + "\n").encode())
        self.f.flush()
        while True:
            r = json.loads(self.f.readline())
            if r.get("id") == self.n:
                return r

    def run(self, cmd, params={}):
        r = self.call("engine.execute", {"command": cmd, "params": params})
        if not r.get("ok"):
            raise SystemExit(f"{cmd}: {r}")
        return r["result"]

    def click(self, x, y):
        for ev in ([{"move": [x, y]}], [{"down": [x, y]}], [{"up": [x, y]}]):
            self.call("ui.input", {"events": ev})
            time.sleep(0.12)
