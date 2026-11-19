import base64, json, socket, sys
args = [a for a in sys.argv[1:] if not a.startswith("--")]
opts = dict(a[2:].split("=", 1) for a in sys.argv[1:] if a.startswith("--") and "=" in a)
port = int(opts.get("port", 47901))
params = json.loads(args[1]) if len(args) > 1 else {}
s = socket.create_connection(("127.0.0.1", port), timeout=180)
s.sendall((json.dumps({"id": 1, "method": args[0], "params": params}) + "\n").encode())
buf = b""
while not buf.endswith(b"\n"):
    chunk = s.recv(1 << 22)
    if not chunk: break
    buf += chunk
s.close()
r = json.loads(buf.decode())
res = r.get("result")
if isinstance(res, dict) and "data" in res and "out" in opts:
    open(opts["out"], "wb").write(base64.b64decode(res.pop("data")))
    res["saved"] = opts["out"]
print(json.dumps(r)[:int(opts.get("max", 3000))])
