#!/usr/bin/env python3
"""List OpenCode models from models.dev/api.json for our providers."""

import json
import urllib.request

PROVIDERS = ["minimax-cn", "kimi-for-coding"]
URL = "https://models.dev/api.json"


def fetch():
    req = urllib.request.Request(
        URL,
        headers={"User-Agent": "Mozilla/5.0 (cc-switch-tui script)"},
    )
    with urllib.request.urlopen(req, timeout=30) as resp:
        return json.loads(resp.read().decode())


def main():
    try:
        data = fetch()
    except Exception as e:
        print(f"fetch failed: {e}")
        return

    for pid in PROVIDERS:
        provider = data.get(pid)
        if not provider:
            print(f"\n=== {pid} NOT FOUND ===")
            continue
        name = provider.get("name", "???")
        print(f"\n=== {pid} ({name}) ===")
        models = provider.get("models", {})
        if not models:
            print("  (no models)")
            continue
        for mid in sorted(models.keys()):
            m = models[mid]
            display = m.get("name") or mid
            print(f"  {mid:30s}  ({display})")


if __name__ == "__main__":
    main()
