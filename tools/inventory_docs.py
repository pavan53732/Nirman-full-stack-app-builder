#!/usr/bin/env python3
"""Transient documentation-topology inventory generator.

Produces a machine-generated inventory of every canonical Markdown document:
heading path, line range, block kind, schema names, contract IDs, milestone
IDs, ADR IDs, citations, authority declarations, projection declarations,
and supersession relationships.

This is generated evidence for the documentation topology migration. It is
NOT a canonical document and MUST NOT be committed as permanent audit
material. Run it, read it, act on it, discard it.
"""

from __future__ import annotations

import json
import os
import re
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))

DOCS = [
    "README.md",
    "AGENTS.md",
    "nirman-build-spec.md",
    "nirman-technical-architecture.md",
    "nirman-milestones.md",
    "nirman-decisions.md",
    "nirman-adrs.md",
]

RE_CONTRACT = re.compile(r"\bCONTRACT\.[A-Z][A-Z0-9_]*(?:\.[A-Z0-9_]+)*")
RE_CAP = re.compile(r"\bCAP\.[A-Z][A-Z0-9_]*(?:\.[A-Z0-9_]+)*")
RE_CLAUSE = re.compile(r"\bCLAUSE\.[A-Z][A-Z0-9_]*(?:\.[A-Z0-9_]+)*")
RE_ADR = re.compile(r"\bADR-\d{3}[A-Z]?\b")
RE_MILESTONE = re.compile(r"\bM(\d{1,3})\b")
RE_SCHEMA = re.compile(r"\b([A-Z][A-Za-z0-9]*(?:Contract|Registry|Record|Manifest|"
                       r"Profile|Plan|Session|Decision|Result|Entry|Event|Report|"
                       r"State|Graph|Package|Snapshot|Envelope|Payload|Spec|Specs|"
                       r"Set|Block|Lock|Index|Trace|Log|Draft|Mutation|Revision|"
                       r"Checkpoint|Evidence|Command|Capability|Config|Status))\b")
RE_CITATION = re.compile(r"\b(?:BS|TA|DP|DEC|ADRS|SCHEMAS|MILESTONES)\s*§\s*[0-9]+(?:\.[0-9A-Za-z]+)*")
RE_AUTHORITY = re.compile(r"(canonical\s+owner|authoritative\s+section|authority\s+section|"
                          r"single\s+authority|canonical\s+authority|sole\s+.*?index|"
                          r"canonical\s+schema\s+owner)", re.IGNORECASE)
RE_PROJECTION = re.compile(r"(projection|restated|implementation\s+view|explanatory\s+only|"
                           r"derived\s+projection|non-authoritative|non-normative)", re.IGNORECASE)
RE_SUPERSEDE = re.compile(r"(supersede[sd]?|supersession|revers(?:e|al)\s+trigger|"
                          r"deprecat(?:e|ed)|obsolete)", re.IGNORECASE)
RE_HEADING = re.compile(r"^(#{1,6})\s+(.*)$")

# Heuristic camelCase schema-name detector for fenced blocks.
RE_FENCE_TOP = re.compile(r"^\s*([A-Z][A-Za-z0-9]*)\s*$")
RE_FIELD = re.compile(r"^\s*([a-z][A-Za-z0-9_]*)\s*[:(\s]")


def classify_fence(lines):
    """Classify a fenced block as schema / vocabulary / other."""
    if not lines:
        return "other", []
    names = []
    fieldish = 0
    for ln in lines:
        s = ln.rstrip()
        if not s:
            continue
        if RE_FENCE_TOP.match(s):
            names.append(s.strip())
            continue
        if RE_FIELD.match(s):
            fieldish += 1
    if names and fieldish >= 1:
        return "schema", names
    if names and fieldish == 0 and len(names) > 1:
        return "vocabulary", names
    return "other", names


def scan(path):
    with open(path, "r", encoding="utf-8", errors="replace") as fh:
        lines = fh.read().split("\n")

    rel = os.path.relpath(path, ROOT).replace("\\", "/")
    n = len(lines)
    blocks = []
    in_fence = False
    fence_lang = ""
    fence_start = 0
    fence_buf = []
    stack = {1: "", 2: "", 3: "", 4: "", 5: "", 6: ""}
    cur_head = []
    i = 0
    while i < n:
        line = lines[i]
        stripped = line.strip()
        if stripped.startswith("```"):
            if not in_fence:
                in_fence = True
                fence_lang = stripped[3:].strip()
                fence_start = i + 1
                fence_buf = []
            else:
                in_fence = False
                kind, names = classify_fence(fence_buf)
                blocks.append({
                    "file": rel,
                    "kind": ("schema-fence" if kind == "schema"
                             else "vocab-fence" if kind == "vocabulary"
                             else "other-fence"),
                    "heading_path": " > ".join(cur_head),
                    "start": fence_start,
                    "end": i + 1,
                    "lines": i + 1 - fence_start,
                    "lang": fence_lang,
                    "schema_names": names,
                    "text": "\n".join(fence_buf),
                })
            i += 1
            continue
        if in_fence:
            fence_buf.append(line)
            i += 1
            continue

        m = RE_HEADING.match(line)
        if m:
            lvl = len(m.group(1))
            title = m.group(2).strip()
            for deeper in range(lvl, 7):
                stack[deeper] = ""
            stack[lvl] = title
            cur_head = [stack[k] for k in range(1, 7) if stack[k]]
            blocks.append({
                "file": rel,
                "kind": "heading",
                "heading_path": " > ".join(cur_head),
                "start": i + 1,
                "end": i + 1,
                "lines": 1,
                "level": lvl,
                "title": title,
            })
            i += 1
            continue
        if stripped.startswith("|") and "|" in stripped[1:]:
            blocks.append({
                "file": rel,
                "kind": "table-row",
                "heading_path": " > ".join(cur_head),
                "start": i + 1,
                "end": i + 1,
                "lines": 1,
                "text": stripped,
            })
        i += 1

    return lines, blocks


def annotate(blocks, lines):
    """Attach ID/authority metadata to every block."""
    for b in blocks:
        text = b.get("text", "")
        # For headings, use the section body (up to next heading) for scanning.
        if b["kind"] == "heading":
            end = len(lines)
            for other in blocks:
                if (other["kind"] == "heading" and other["start"] > b["start"]
                        and other["level"] <= b["level"]):
                    end = min(end, other["start"] - 1)
            text = "\n".join(lines[b["start"]:end])
            b["section_lines"] = end - b["start"]
        b["contract_ids"] = sorted(set(RE_CONTRACT.findall(text)))
        b["cap_ids"] = sorted(set(RE_CAP.findall(text)))
        b["clause_ids"] = sorted(set(RE_CLAUSE.findall(text)))
        b["adr_ids"] = sorted(set(RE_ADR.findall(text)))
        b["milestone_ids"] = sorted(
            {"M" + m for m in set(RE_MILESTONE.findall(text))})
        b["citations"] = sorted(set(RE_CITATION.findall(text)))
        b["authority_decls"] = sorted(set(m.group(0).lower()
                                          for m in RE_AUTHORITY.finditer(text)))
        b["projection_decls"] = sorted(set(m.group(0).lower()
                                           for m in RE_PROJECTION.finditer(text)))
        b["supersession"] = sorted(set(m.group(0).lower()
                                       for m in RE_SUPERSEDE.finditer(text)))
        if b["kind"] == "heading":
            b.pop("text", None)
    return blocks


def main():
    all_blocks = []
    all_lines = {}
    for d in DOCS:
        p = os.path.join(ROOT, d)
        if not os.path.exists(p):
            print(f"MISSING: {d}", file=sys.stderr)
            continue
        lines, blocks = scan(p)
        all_lines[d] = len(lines)
        all_blocks.extend(annotate(blocks, lines))

    out = {
        "root": ROOT,
        "documents": all_lines,
        "blocks": all_blocks,
    }
    dest = sys.argv[1] if len(sys.argv) > 1 else None
    payload = json.dumps(out, indent=1)
    if dest:
        with open(dest, "w", encoding="utf-8") as fh:
            fh.write(payload)
        print(f"wrote {dest}: {len(all_blocks)} blocks, "
              f"{sum(all_lines.values())} lines")
    else:
        print(payload)


if __name__ == "__main__":
    main()
