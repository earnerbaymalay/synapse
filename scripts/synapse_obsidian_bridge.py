#!/usr/bin/env python3
"""
Synapse-to-Obsidian Memory Bridge
Exports distilled high-signal observations from Synapse Brain memory (~/.synapse/memory/observations.jsonl)
into the Obsidian AI Knowledge Base vault (~/Obsidian/AI-Knowledge-Base/) with frontmatter, tags, and wikilinks.
Idempotent and zero-telemetry.
"""

import os
import sys
import json
from datetime import datetime

VAULT_DIR = os.path.expanduser(os.environ.get("OBSIDIAN_AI_VAULT", "~/Obsidian/AI-Knowledge-Base"))
MEMORY_LOG = os.path.expanduser("~/.synapse/memory/observations.jsonl")

KIND_MAP = {
    "decision": "Decisions",
    "insight": "Insights",
    "solution": "Solutions",
    "workflow": "Workflows",
    "milestone": "Daily",
    "note": "Daily",
    "summary": "Daily",
}

def parse_ts(raw):
    if isinstance(raw, (int, float)):
        dt = datetime.fromtimestamp(raw)
        return dt.strftime("%Y-%m-%d"), dt.strftime("%H:%M"), dt.isoformat()
    if isinstance(raw, str):
        return raw[:10], raw[11:16] if len(raw) >= 16 else "00:00", raw
    dt = datetime.now()
    return dt.strftime("%Y-%m-%d"), dt.strftime("%H:%M"), dt.isoformat()

def export_observations():
    if not os.path.isfile(MEMORY_LOG):
        print(f"No Synapse memory log found at {MEMORY_LOG}", file=sys.stderr)
        return 0

    exported = 0
    with open(MEMORY_LOG, "r", encoding="utf-8") as f:
        for line in f:
            line = line.strip()
            if not line:
                continue
            try:
                obs = json.loads(line)
            except Exception:
                continue

            obs_id = obs.get("id")
            title = obs.get("title", f"Observation #{obs_id}")
            narrative = obs.get("narrative", "")
            kind = obs.get("kind", "note").lower()
            topic = obs.get("topic", "general")
            tags = obs.get("tags", [])
            model = obs.get("model", "unknown")
            dt_str, hm_str, iso_str = parse_ts(obs.get("ts"))

            folder_name = KIND_MAP.get(kind, "Daily")
            target_dir = os.path.join(VAULT_DIR, folder_name)
            os.makedirs(target_dir, exist_ok=True)

            if folder_name == "Daily":
                daily_file = os.path.join(target_dir, f"{dt_str}.md")
                header_needed = not os.path.exists(daily_file)
                marker = f"<!-- synapse-obs-{obs_id} -->"
                if os.path.exists(daily_file):
                    with open(daily_file, "r", encoding="utf-8", errors="ignore") as df:
                        if marker in df.read():
                            continue

                with open(daily_file, "a", encoding="utf-8") as df:
                    if header_needed:
                        df.write(f"# Daily AI Log: {dt_str}\n\n- **Tags**: #daily #synapse\n- **Date**: {dt_str}\n\n### Synapse Memory Stream\n\n")
                    tag_str = " ".join(f"#{t}" for t in tags)
                    df.write(f"\n#### [{hm_str}] {title} {marker}\n")
                    df.write(f"- **Topic**: [[{topic}]] | **Kind**: `{kind}` | **Model**: `{model}`\n")
                    if tag_str:
                        df.write(f"- **Tags**: {tag_str}\n")
                    if narrative:
                        df.write(f"\n{narrative}\n")
                exported += 1
            else:
                safe_name = "".join(c if c.isalnum() or c in "-_." else "_" for c in title).strip("_")
                note_file = os.path.join(target_dir, f"{safe_name}.md")
                if os.path.exists(note_file):
                    continue

                tag_list = [f'"{t}"' for t in tags] + [f'"{topic}"', f'"{kind}"']
                with open(note_file, "w", encoding="utf-8") as nf:
                    nf.write(f"---\n")
                    nf.write(f"id: {obs_id}\n")
                    nf.write(f"title: \"{title}\"\n")
                    nf.write(f"kind: \"{kind}\"\n")
                    nf.write(f"topic: \"{topic}\"\n")
                    nf.write(f"model: \"{model}\"\n")
                    nf.write(f"date: \"{iso_str}\"\n")
                    nf.write(f"tags: [{', '.join(tag_list)}]\n")
                    nf.write(f"---\n\n")
                    nf.write(f"# {title}\n\n")
                    nf.write(f"> **Recorded by**: `{model}` · **Topic**: [[{topic}]] · **Timestamp**: {iso_str}\n\n")
                    if narrative:
                        nf.write(f"## Detail\n\n{narrative}\n\n")
                exported += 1

    print(f"✔ Exported {exported} new Synapse observations into Obsidian vault at {VAULT_DIR}")
    return exported

if __name__ == "__main__":
    export_observations()
