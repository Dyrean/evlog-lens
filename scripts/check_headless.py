#!/usr/bin/env python3
"""Render real Rune frames and assert the resulting element trees, without a display."""
import json
import os
from pathlib import Path
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / "artifacts"
OUT.mkdir(exist_ok=True)
metadata = json.loads(subprocess.check_output(
    ["cargo", "metadata", "--format-version", "1", "--no-deps"], cwd=ROOT, text=True))
package = next(package for package in metadata["packages"]
               if Path(package["manifest_path"]).resolve() == ROOT / "Cargo.toml")
binary = next(target["name"] for target in package["targets"] if "bin" in target["kind"])
BIN = Path(metadata["target_directory"]) / "debug" / (binary + (".exe" if os.name == "nt" else ""))
subprocess.run(["cargo", "build", "--locked"], cwd=ROOT, check=True)
rendered = []

# Keep verification independent of this user's saved views and sessions.
with tempfile.TemporaryDirectory(prefix="evlog-headless-") as data_dir:
    env = dict(os.environ, XDG_DATA_HOME=data_dir)
    def render(name, *args, size="1480x940"):
        subprocess.run([str(BIN), *args, "--size", size, "--scale", "1", "--snap", str(OUT / f"{name}.png"), "--tree", str(OUT / f"{name}.json")], cwd=ROOT, env=env, check=True)
        tree = json.loads((OUT / f"{name}.json").read_text())
        rendered.append(name)
        assert not tree["warnings"], (name, tree["warnings"])
        assert tree["title"] == "Evlog Lens"
        assert tree["viewport"] == [int(n) for n in size.split("x")]
        keyed = {e["key"]: e for e in tree["elements"] if e.get("key")}
        for key in ("app",):
            assert keyed[key]["rect"][2] > 0 and keyed[key]["rect"][3] > 0
        return tree, keyed

    _, keys = render("welcome")
    assert "sample" in keys and "drop-zone" in keys
    for scene in ("sample", "incident", "json", "session", "errors"):
        tree, keys = render(scene, "--scene", scene)
        assert keys["matching-count"]["text"] == ("3 matching events" if scene == "errors" else "50 matching events")
        assert keys["event-scroll"]["rect"][3] > 100
        if scene in ("incident", "json", "session"):
            assert "inspector" in keys
        if scene == "json":
            assert "json-" in keys and "toggle-json-/error" in keys
    tree, keys = render("investigation", "--scene", "incident", "--script", "scripts/investigate.json", "--at", "4.5")
    assert keys["search"]["text"] == "status:503 duration>1000"
    assert keys["matching-count"]["text"] == "1 matching event"
    assert any(e.get("text") == "Requests in this session · chronological" for e in tree["elements"])
    assert len([e for e in tree["elements"] if e.get("role") == "row"]) == 1
    _, keys = render("compact", "--scene", "incident", size="1180x720")
    assert keys["event-scroll"]["rect"][3] > 100
    inspector = keys["inspector"]["rect"]
    assert inspector[0] + inspector[2] <= 1181

print(f"PASS: {len(rendered)} headless PNG/tree renders, scripted search and session navigation, compact layout; no Rune warnings.")
