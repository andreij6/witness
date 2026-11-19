import os, sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path[:0] = [HERE, os.path.join(HERE, "screens")]

import wlib
for module in ("start", "capture", "case", "item_check", "settings", "web_mobile", "web_desktop", "foundations"):
    __import__(module)
wlib.finish(sys.argv[1])
