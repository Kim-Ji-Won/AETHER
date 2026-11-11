#!/usr/bin/env python3
"""Prepare the pinned E4c runtime. Network is opt-in; never use a global Node binary."""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess
import tarfile
import urllib.request

ROOT = Path(__file__).resolve().parents[1]
DESKTOP = ROOT / "apps/desktop"
PIN = json.loads((DESKTOP / "sidecar/node-provenance.json").read_text())
OUT = DESKTOP / "src-tauri/sidecar/generated"
CACHE = ROOT / "target/e4c-download"


def verify(data, expected):
    if hashlib.sha256(data).hexdigest() != expected:
        raise SystemExit("BLOCKED: pinned SHA-256 mismatch")


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--download", action="store_true")
    parser.add_argument("--verify-only", action="store_true")
    args = parser.parse_args()
    if args.verify_only:
        for name, key in [("node", "node_sha256"), ("LICENSE", "license_sha256")]:
            verify((OUT / name).read_bytes(), PIN[key])
        print("PASS: pinned Node and complete LICENSE")
        return
    CACHE.mkdir(parents=True, exist_ok=True)
    archive = CACHE / PIN["archive"]
    if not archive.exists():
        if not args.download:
            raise SystemExit("BLOCKED: run python3 scripts/prepare_node_sidecar.py --download first")
        with urllib.request.urlopen(PIN["url"], timeout=60) as response:
            if not response.url.startswith("https://nodejs.org/"):
                raise SystemExit("BLOCKED: unexpected download origin")
            data = response.read(80 * 1024 * 1024)
        verify(data, PIN["archive_sha256"])
        archive.write_bytes(data)
    verify(archive.read_bytes(), PIN["archive_sha256"])
    OUT.mkdir(parents=True, exist_ok=True)
    # Read only named regular members; never extract arbitrary archive paths/symlinks.
    with tarfile.open(archive) as tar:
        prefix = PIN["archive"].removesuffix(".tar.gz")
        for member, output, key in [("bin/node", "node", "node_sha256"), ("LICENSE", "LICENSE", "license_sha256")]:
            info = tar.getmember(f"{prefix}/{member}")
            if not info.isfile():
                raise SystemExit("BLOCKED: expected regular archive member")
            data = tar.extractfile(info).read()
            verify(data, PIN[key])
            (OUT / output).write_bytes(data)
    (OUT / "node").chmod(0o755)
    version = subprocess.check_output([str(OUT / "node"), "--version"], env={}, text=True).strip()
    if version != "v" + PIN["version"]:
        raise SystemExit("BLOCKED: Node version mismatch")
    (OUT / "node-provenance.json").write_text(json.dumps(PIN, indent=2) + "\n")
    print(f"PASS: verified bundled Node {version} and LICENSE; no global installation")


if __name__ == "__main__":
    main()
