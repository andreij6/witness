import base64, glob, json, os, socket, sys, zipfile

HERE = os.path.dirname(os.path.abspath(__file__))
ICONS = os.path.join(HERE, "..", "icons")

def ex(command, **params):
    with socket.create_connection(("127.0.0.1", 47901), timeout=120) as s:
        f = s.makefile("rwb")
        f.write((json.dumps({"id": 1, "method": "engine.execute", "params": {"command": command, "params": params}}) + "\n").encode()); f.flush()
        r = json.loads(f.readline())
    if not r.get("ok"): raise RuntimeError(f"{command}: {r.get('error')}")
    return r["result"]

ex("file.new", name="icons")
ids = {}
for k, path in enumerate(sorted(glob.glob(os.path.join(ICONS, "*.svg")))):
    name = os.path.basename(path)[:-4]
    ids[name] = ex("file.import", data=base64.b64encode(open(path, "rb").read()).decode(), name=name + ".svg", x=k * 60, y=0)["id"]
out = os.path.join(sys.argv[1], "icons.figcraft")
ex("file.saveAs", path=out)
nodes = json.loads(zipfile.ZipFile(out).read("document.json"))["nodes"]

def vectors(i, dx=0.0, dy=0.0):
    n = nodes[str(i)]
    t = n.get("transform", [1, 0, 0, 1, 0, 0]); x, y = dx + t[4], dy + t[5]
    if n["type"] == "vector": yield n, x, y
    for c in n.get("children", []): yield from vectors(c, x, y)

cache = {}
page = json.loads(zipfile.ZipFile(out).read("document.json"))["pages"][0]
roots = {nodes[str(c)]["name"]: c for c in nodes[str(page)]["children"]}
for name in ids:
    root = nodes[str(roots[name])]
    parts = [{"x": x - root.get("transform", [0] * 6)[4], "y": y - root.get("transform", [0] * 6)[5], "network": v["network"]} for v, x, y in vectors(roots[name])]
    cache[name] = {"size": root["width"], "parts": parts}
json.dump(cache, open(os.path.join(ICONS, "icons.json"), "w"))
print("cached", len(cache), "icons:", ", ".join(sorted(cache)))
