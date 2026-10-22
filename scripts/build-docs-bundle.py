#!/usr/bin/env python3
"""Regenerate the Markdown bundle embedded in docs/docs.html.

docs.html renders every docs/**/*.md client-side from `window.SYNAPSE_DOCS`.
Run this after editing any doc so the hub never serves stale copies:

    python3 scripts/build-docs-bundle.py
"""
import json
import pathlib
import re
import sys

ROOT = pathlib.Path(__file__).resolve().parent.parent
DOCS = ROOT / "docs"
HUB = DOCS / "docs.html"


def main() -> int:
    bundle = {
        str(p.relative_to(DOCS)).replace("\\", "/"): p.read_text(encoding="utf-8")
        for p in sorted(DOCS.rglob("*.md"))
    }
    html = HUB.read_text(encoding="utf-8")
    payload = "window.SYNAPSE_DOCS = " + json.dumps(bundle, ensure_ascii=True) + ";"
    updated, n = re.subn(
        r"window\.SYNAPSE_DOCS = \{.*?\};(?=\n)", lambda _: payload, html, count=1, flags=re.S
    )
    if n != 1:
        print("docs.html: SYNAPSE_DOCS assignment not found", file=sys.stderr)
        return 1
    HUB.write_text(updated, encoding="utf-8")
    print(f"docs.html: bundled {len(bundle)} documents")
    return 0


if __name__ == "__main__":
    sys.exit(main())
