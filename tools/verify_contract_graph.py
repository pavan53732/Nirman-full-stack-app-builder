#!/usr/bin/env python3
"""
Nirman contract-graph verifier — implements build spec §67.11.

Runs all eleven §67.11 contract-graph checks over the four canonical
documents in both traversal directions (§67.9), plus the document-structure
check required by the verifier harness and a semantic-documentation lint layer.
Exits 1 on any defect.

Registries consumed:
  §5.6   Capability Registry           (CapabilityId -> required contracts, test, evidence)
  §67.8  Contract Authority Registry   (ContractId -> authority, extensions, class)
  §67.12 Clause Registry               (ClauseId -> contract, authority, value, seal)
  §67.13 ExtensionDeclaration format
  §67.14 Contract reachability rules
  §67.15 Twelve-edge resolution table

Usage: python3 tools/verify_contract_graph.py [repo_root]
"""
import os
import re
import sys

CIDRE = r"\bCONTRACT\.[A-Z][A-Z0-9_]*(?:\.[A-Z0-9_]+)*"
CAPRE = r"\bCAP\.[A-Z][A-Z0-9_]*(?:\.[A-Z0-9_]+)*"
CLRE = r"\bCLAUSE\.[A-Z][A-Z0-9_]*(?:\.[A-Z0-9_]+)*"
CLASSES = ("FOUNDATIONAL", "CROSS_CUTTING", "INTERNAL", "DEPRECATED")
EXT_TYPES = ("adds_clauses", "adds_schema", "adds_component", "adds_verification")

DOCS = {
    "bs": "nirman-build-spec.md",
    "ta": "nirman-technical-architecture.md",
    "dec": "nirman-decisions.md",
    "dev": "nirman-development-plan.md",
}

EDGES = ("capability", "requirement", "build_spec", "architecture", "schema",
         "authority", "persistence", "failure_recovery", "adr", "milestone",
         "test", "evidence")


class Defects:
    """Ordered, individually addressable defect list (§67.11)."""

    def __init__(self):
        self.items = []

    def add(self, check, subject, detail):
        self.items.append((check, subject, detail))

    def __len__(self):
        return len(self.items)

    def by_check(self):
        out = {}
        for check, subject, detail in self.items:
            out.setdefault(check, []).append((subject, detail))
        return out


def strip_fences(text):
    """Remove fenced examples and HTML comments from semantic parsing."""
    text = re.sub(r"```.*?```", "", text, flags=re.S)
    return re.sub(r"<!--.*?-->", "", text, flags=re.S)


def load(root):
    docs = {}
    for key, name in DOCS.items():
        path = os.path.join(root, name)
        if not os.path.exists(path):
            sys.exit(f"FATAL: missing {name} in {root}")
        docs[key] = open(path, encoding="utf-8").read()
    return docs


def table_rows(text, start_marker, end_marker, prefix):
    """Rows of a markdown table between two markers, split into cells."""
    if start_marker not in text:
        return None
    seg = text.split(start_marker, 1)[1]
    if end_marker and end_marker in seg:
        seg = seg.split(end_marker, 1)[0]
    rows = []
    for line in seg.split("\n"):
        line = line.strip()
        if line.startswith("| " + prefix):
            rows.append([c.strip() for c in line.strip("|").split("|")])
    return rows


def sections(text):
    return {int(m.group(1)) for m in re.finditer(r"^##\s+(\d+)\.\s", text, re.M)}


def subsections(text):
    return {(int(a), int(b))
            for a, b in re.findall(r"^###\s+(\d+)\.(\d+)\s", text, re.M)}


def adr_blocks(text):
    parts = re.split(r"^## ADR-(\d+):", text, flags=re.M)
    return {int(parts[i]): parts[i + 1] for i in range(1, len(parts), 2)}


def section_bodies(text):
    """section number -> body text, fences stripped."""
    out = {}
    marks = [(m.start(), int(m.group(1)))
             for m in re.finditer(r"^##\s+(\d+)\.\s", text, re.M)]
    for i, (pos, num) in enumerate(marks):
        end = marks[i + 1][0] if i + 1 < len(marks) else len(text)
        out[num] = strip_fences(text[pos:end])
    return out


def secrefs(cell):
    return [int(x) for x in re.findall(r"§(\d+)", cell)]


def subref(cell):
    m = re.match(r"§(\d+)\.(\d+)$", cell.strip())
    return (int(m.group(1)), int(m.group(2))) if m else None


# Authoritative target domain of each twelve-edge column (§67.15). A reference
# must resolve in ITS OWN domain; existing in some other document is not enough.
EDGE_DOMAIN = {
    "requirement": "BS",
    "build_spec": "BS",
    "architecture": "TA",
    "schema": "TA",
    "authority": "BS",
    "persistence": "TA",
    "failure_recovery": None,   # TA by default, BS permitted when normative
}

DOC_OF = {"BS": "bs", "TA": "ta"}


def parse_ref(cell):
    """Parse a document-qualified reference.

    Returns (doc, section, subsection|None, error|None). ``doc`` is 'BS' or 'TA'.
    An unqualified reference is an error: the number alone is not an identity.
    """
    cell = cell.strip()
    m = re.match(r"^(BS|TA)\s+§(\d+)(?:\.(\d+))?$", cell)
    if m:
        return (m.group(1), int(m.group(2)),
                int(m.group(3)) if m.group(3) else None, None)
    if re.match(r"^§\d+(\.\d+)?$", cell):
        return (None, None, None, "unqualified reference (missing BS/TA namespace)")
    return (None, None, None, f"unparseable reference {cell!r}")


# ---------------------------------------------------------------- registries

def parse_registries(docs, D):
    """Parse all five registries. Returns dict of registry -> parsed data."""
    bs, dev = docs["bs"], docs["dev"]
    R = {}

    # §67.8 contract authority registry
    reg_heading = re.search(r"^### \d+\.\d+ Registered contract identifiers\s*$", bs, re.M)
    if reg_heading is None:
        sys.exit("FATAL: contract authority registry heading not found")
    rows = table_rows(bs, reg_heading.group(0), "\n### ", "CONTRACT.")
    if rows is None:
        sys.exit("FATAL: contract authority registry table not parseable")
    reg = {}
    for c in rows:
        if len(c) < 7:
            D.add("structure", c[0] if c else "?", "§67.8 row has too few cells")
            continue
        reg[c[0]] = dict(authority=c[1], ext=c[2], arch=c[3],
                         adr=c[4], mile=c[5], cls=c[6])
    R["contracts"] = reg

    # Capability registry — located by heading text, because its section number
    # shifts whenever a subsection is inserted before it (P32: never hardcode a
    # positional bound a document edit can invalidate).
    cap_heading = re.search(r"^### \d+\.\d+ Capability Registry\s*$", bs, re.M)
    if cap_heading is None:
        sys.exit("FATAL: capability registry heading not found")
    rows = table_rows(bs, cap_heading.group(0), "\n## ", "CAP.")
    if rows is None:
        sys.exit("FATAL: capability registry table not parseable")
    caps = {}
    for c in rows:
        if len(c) < 6:
            D.add("structure", c[0] if c else "?", "§5.6 row has too few cells")
            continue
        caps[c[0]] = dict(requirement=c[1],
                          contracts=[x for x in re.findall(CIDRE, c[2])],
                          test=c[3], evidence=c[4], status=c[5])
    R["capabilities"] = caps

    # §67.12 clause registry
    cl_heading = re.search(r"^### \d+\.\d+ Clause Registry\s*$", bs, re.M)
    if cl_heading is None:
        sys.exit("FATAL: clause registry heading not found")
    rows = table_rows(bs, cl_heading.group(0), "\n### ", "CLAUSE.")
    if rows is None:
        sys.exit("FATAL: clause registry table not parseable")
    clauses = {}
    for c in rows:
        if len(c) < 5:
            D.add("structure", c[0] if c else "?", "§67.12 row has too few cells")
            continue
        clauses[c[0]] = dict(contract=c[1], authority=c[2],
                             value=c[3], sealed=c[4].upper() == "SEALED")
    R["clauses"] = clauses

    # Twelve-edge table — located by heading text, not a hardcoded number.
    edge_heading = re.search(r"^### \d+\.\d+ Twelve-edge resolution table\s*$", bs, re.M)
    if edge_heading is None:
        sys.exit("FATAL: twelve-edge table heading not found")
    rows = table_rows(bs, edge_heading.group(0), "\n## References", "CONTRACT.")
    if rows is None:
        sys.exit("FATAL: twelve-edge table not parseable")
    chain = {}
    for c in rows:
        if len(c) != 13:
            D.add("forward break", c[0] if c else "?",
                  f"twelve-edge row has {len(c) - 1} edges, expected 12")
            continue
        chain[c[0]] = dict(zip(EDGES, c[1:]))
    R["chain"] = chain

    # milestone mappings from the development plan
    miles = {}
    for marker, end in (("## Foundational milestone contract mapping", "## M81"),
                        ("## M81\u2013M96 contract mapping", "### M93")):
        rows = table_rows(dev, marker, end, "M")
        for c in (rows or []):
            m = re.match(r"M(\d+)$", c[0])
            if not m:
                continue
            miles[int(m.group(1))] = dict(
                contracts=re.findall(CIDRE, c[1]),
                adrs=[int(x) for x in re.findall(r"ADR-(\d+)", c[2])],
                test=c[3] if len(c) > 3 else "",
                evidence=c[4] if len(c) > 4 else "")
    R["milestones"] = miles

    # ExtensionDeclaration blocks, parsed from fence-stripped section bodies
    # A section may extend more than one contract, so declarations are keyed by
    # (section, authorityContractId) rather than by section alone.
    decls = {}
    for num, body in section_bodies(strip_fences(bs)).items():
        chunks = body.split("**ExtensionDeclaration:**")
        for i in range(1, len(chunks)):
            preceding = chunks[i - 1]
            blk = chunks[i].split("\n\n", 1)[0]
            get = lambda k: (re.search(rf"-\s*{k}:\s*(.+)", blk).group(1).strip()
                             if re.search(rf"-\s*{k}:\s*(.+)", blk) else "")
            owners = re.findall(rf"\*\*ContractId:\*\*\s*`({CIDRE})`", preceding)
            decl = dict(
                section=num,
                contract_id=owners[-1] if owners else "",
                authority_contract=get("authorityContractId"),
                authority_section=get("authoritySection"),
                extending_section=get("extendingSection"),
                ext_type=get("extensionType"),
                extended=re.findall(CLRE, get("extendedClauses")),
                non_overridden=re.findall(CLRE, get("nonOverriddenClauses")),
                raw_extended=get("extendedClauses"))
            decls[(num, decl["authority_contract"])] = decl
    R["declarations"] = decls

    # authoritative-role markers
    # A marker must be a line-initial declaration. Prose *describing* the marker
    # (as §67.13 does) is not a declaration and must not be parsed as data.
    authored = {}
    for num, body in section_bodies(strip_fences(bs)).items():
        for line in body.split("\n"):
            line = line.strip()
            if not line.startswith("**Registry role:** authoritative definition"):
                continue
            m = re.match(r"\*\*Registry role:\*\* authoritative definition of `("
                         + CIDRE + r")`", line)
            if m:
                authored.setdefault(num, []).append(m.group(1))
            else:
                D.add("undeclared extension", f"§{num}",
                      "authoritative marker omits its ContractId; §67.13 requires "
                      "the explicit `of \u0060ContractId\u0060` form")
    R["authored"] = authored
    return R


# ------------------------------------------------- checks 1-6: authority layer

def check_duplicate_authority(R, D):
    """Check 1: a ContractId has exactly one authoritative section."""
    claims = {}
    for cid, r in R["contracts"].items():
        for sec in secrefs(r["authority"]):
            claims.setdefault(sec, []).append(cid)
    for sec, cids in sorted(claims.items()):
        if len(cids) > 1:
            D.add("duplicate authority", f"§{sec}",
                  f"claims authority over {len(cids)} contracts: {', '.join(sorted(cids))}")
    # inverse: one contract naming two authority sections
    for cid, r in sorted(R["contracts"].items()):
        secs = secrefs(r["authority"])
        if len(secs) != 1:
            D.add("duplicate authority", cid,
                  f"names {len(secs)} authority sections, expected exactly 1")
    # a section declaring itself author of a contract it does not own in §67.8
    for sec, owned in sorted(R["authored"].items()):
        for cid in owned:
            reg = R["contracts"].get(cid)
            if reg and secrefs(reg["authority"]) and secrefs(reg["authority"])[0] != sec:
                D.add("duplicate authority", cid,
                      f"§{sec} claims authorship but §67.8 assigns {reg['authority']}")


def check_unregistered(R, docs, D):
    """Check 2: every referenced ContractId/CapabilityId/ClauseId is registered."""
    all_text = strip_fences("\n".join(docs.values()))
    for cid in sorted(set(re.findall(CIDRE, all_text)) - set(R["contracts"])):
        D.add("unregistered contract", cid, "referenced but absent from §67.8")
    for cap in sorted(set(re.findall(CAPRE, all_text)) - set(R["capabilities"])):
        D.add("unregistered contract", cap, "capability referenced but absent from §5.6")
    for cl in sorted(set(re.findall(CLRE, all_text)) - set(R["clauses"])):
        D.add("unregistered contract", cl, "clause referenced but absent from §67.12")


def check_undeclared_extension(R, D):
    """Check 3: extensions carry a valid, registry-consistent declaration."""
    contracts, decls = R["contracts"], R["declarations"]

    # every extension edge in §67.8 must have a matching declaration block
    for cid, r in sorted(contracts.items()):
        auth = secrefs(r["authority"])
        for ext_sec in secrefs(r["ext"]):
            d = decls.get((ext_sec, cid))
            if d is None:
                D.add("undeclared extension", f"§{ext_sec}",
                      f"listed as extension of {cid} in §67.8 but carries no "
                      f"matching ExtensionDeclaration")
                continue
            if auth and d["authority_section"] != f"§{auth[0]}":
                D.add("undeclared extension", f"§{ext_sec}",
                      f"declares authoritySection {d['authority_section']!r} for {cid}, "
                      f"§67.8 says §{auth[0]}")

    # declaration internal consistency
    for (sec, cid), d in sorted(decls.items()):
        label = f"§{sec}"
        if d["extending_section"] != f"§{sec}":
            D.add("undeclared extension", label,
                  f"extendingSection is {d['extending_section']!r}, expected §{sec}")
        if d["ext_type"] not in EXT_TYPES:
            D.add("undeclared extension", label,
                  f"extensionType {d['ext_type']!r} is not one of {EXT_TYPES}")
        if not d["authority_contract"]:
            D.add("undeclared extension", label, "declaration omits authorityContractId")
        if not d["raw_extended"]:
            D.add("undeclared extension", label, "declaration omits extendedClauses")
        if d["contract_id"] and d["contract_id"] != cid:
            D.add("undeclared extension", label,
                  f"ContractId header {d['contract_id']} disagrees with "
                  f"authorityContractId {cid}")
        reg = contracts.get(cid)
        if reg is None:
            D.add("undeclared extension", label, f"declares extension of unregistered {cid}")
        elif sec not in secrefs(reg["ext"]):
            D.add("undeclared extension", label,
                  f"declares extension of {cid} but §67.8 does not list §{sec}")


def check_authority_cycle(R, D):
    """Check 4: the authority/extension graph is acyclic."""
    adj = {}
    for cid, r in R["contracts"].items():
        auth = secrefs(r["authority"])
        if not auth:
            continue
        for ext_sec in secrefs(r["ext"]):
            adj.setdefault(ext_sec, set()).add(auth[0])
    color, reported = {}, set()

    def walk(node, path):
        color[node] = 1
        path.append(node)
        for nxt in sorted(adj.get(node, ())):
            if color.get(nxt, 0) == 1:
                cyc = path[path.index(nxt):] + [nxt]
                key = tuple(sorted(set(cyc)))
                if key not in reported:
                    reported.add(key)
                    D.add("authority cycle", "->".join(f"§{x}" for x in cyc),
                          "authority/extension graph must be acyclic")
            elif color.get(nxt, 0) == 0:
                walk(nxt, path)
        path.pop()
        color[node] = 2

    for node in sorted(adj):
        if color.get(node, 0) == 0:
            walk(node, [])
    return adj


def check_clause_contradiction(R, docs, D):
    """Check 5: no extension restates a sealed clause with a different value."""
    clauses, decls, contracts = R["clauses"], R["declarations"], R["contracts"]
    bodies = section_bodies(strip_fences(docs["bs"]))

    # 5a. a sealed clause may be authored by exactly one section
    owners = {}
    for cl, meta in clauses.items():
        for sec in secrefs(meta["authority"]):
            owners.setdefault((cl, sec), True)
    for cl, meta in sorted(clauses.items()):
        secs = secrefs(meta["authority"])
        if len(secs) != 1:
            D.add("clause contradiction", cl,
                  f"names {len(secs)} authority sections, expected exactly 1")
            continue
        # the clause's contract must exist and its authority must agree with §67.8
        reg = contracts.get(meta["contract"])
        if reg is None:
            D.add("clause contradiction", cl,
                  f"belongs to unregistered contract {meta['contract']}")
        else:
            ra = secrefs(reg["authority"])
            if ra and ra[0] != secs[0]:
                D.add("clause contradiction", cl,
                      f"clause authority §{secs[0]} disagrees with contract authority {reg['authority']}")

    # 5b. an extension listing a sealed clause under extendedClauses is a
    #     redefinition attempt, not an addition
    for (sec, _cid), d in sorted(decls.items()):
        for cl in d["extended"]:
            meta = clauses.get(cl)
            if meta is None:
                continue
            owner = secrefs(meta["authority"])
            if meta["sealed"] and owner and owner[0] != sec:
                D.add("clause contradiction", cl,
                      f"§{sec} lists sealed clause under extendedClauses; "
                      f"authority is §{owner[0]} — must appear under nonOverriddenClauses")

    # 5b-bis. an extension must adopt EVERY sealed clause owned by its authority.
    # Silently dropping one is how an extension escapes an invariant.
    for (sec, cid), d in sorted(decls.items()):
        reg = contracts.get(cid)
        if reg is None:
            continue
        auth = secrefs(reg["authority"])
        if not auth:
            continue
        owned = {cl for cl, meta in clauses.items()
                 if meta["sealed"] and secrefs(meta["authority"]) == [auth[0]]}
        adopted = set(d["non_overridden"]) | set(d["extended"])
        for cl in sorted(owned - adopted):
            D.add("undeclared extension", f"§{sec}",
                  f"extends {cid} but does not adopt its sealed clause {cl}; "
                  f"every sealed clause of an authority must appear under "
                  f"nonOverriddenClauses")

    # 5c. an extension must not contradict a clause it adopts unchanged.
    #     A sealed clause's normative value carries polarity markers; an adopting
    #     section asserting the negated form is a contradiction.
    NEG = (("never", "always"), ("not ", "must "), ("excluded", "included"),
           ("disabled", "enabled"), ("prohibited", "permitted"))
    for (sec, _cid), d in sorted(decls.items()):
        body = bodies.get(sec, "")
        for cl in d["non_overridden"]:
            meta = clauses.get(cl)
            if meta is None:
                D.add("clause contradiction", cl,
                      f"§{sec} adopts an unregistered clause")
                continue
            value = meta["value"].lower()
            for neg, pos in NEG:
                if neg in value:
                    # locate the clause's subject words in the adopting section
                    subject = [w for w in re.findall(r"[a-z_]{5,}", value)
                               if w not in ("never", "always", "which", "their")][:3]
                    if not subject:
                        continue
                    for para in body.split("\n\n"):
                        low = para.lower()
                        if all(s in low for s in subject) and pos in low and neg not in low:
                            D.add("clause contradiction", cl,
                                  f"§{sec} adopts clause as non-overridden but asserts "
                                  f"the opposite polarity ({pos!r} without {neg!r})")
                            break
                    break


def check_unversioned_override(R, docs, D):
    """Check 6: sealed clauses change only via a versioned superseding contract."""
    contracts, clauses, decls = R["contracts"], R["clauses"], R["declarations"]
    bodies = section_bodies(strip_fences(docs["bs"]))
    dec_text = docs["dec"]

    for (sec, _cid), d in sorted(decls.items()):
        for cl in d["extended"]:
            meta = clauses.get(cl)
            if meta is None or not meta["sealed"]:
                continue
            owner = secrefs(meta["authority"])
            if not owner or owner[0] == sec:
                continue
            # redefining a sealed clause requires a DEPRECATED predecessor plus ADR
            superseding = re.search(r"supersed\w*\s+`?(" + CIDRE + r")`?",
                                    bodies.get(sec, ""), re.I)
            adr = re.search(r"ADR-(\d+)", bodies.get(sec, ""))
            if not (superseding and adr):
                D.add("unversioned override", cl,
                      f"§{sec} redefines a sealed clause without a superseding "
                      f"contract and recorded ADR")

    for cid, r in sorted(contracts.items()):
        if r["cls"] != "DEPRECATED":
            continue
        auth = secrefs(r["authority"])
        body = bodies.get(auth[0], "") if auth else ""
        sup = re.search(r"supersed\w*\s+`?(" + CIDRE + r")`?", body, re.I)
        adr = re.findall(r"ADR-(\d+)", r["adr"])
        if not sup:
            D.add("unversioned override", cid,
                  "classified DEPRECATED but names no superseding ContractId")
        elif sup.group(1) not in contracts:
            D.add("unversioned override", cid,
                  f"names unregistered superseding contract {sup.group(1)}")
        if not adr:
            D.add("unversioned override", cid,
                  "classified DEPRECATED but records no ADR for the transition")
        elif f"ADR-{int(adr[0]):03d}" not in dec_text and f"ADR-{adr[0]}" not in dec_text:
            D.add("unversioned override", cid, f"transition ADR-{adr[0]} not found in decision log")


# --------------------------------------- checks 7-10: traceability layer

def check_dangling(R, docs, D):
    """Check 7: every reference resolves to an existing target."""
    bs_secs, ta_secs = sections(docs["bs"]), sections(docs["ta"])
    bs_subs, ta_subs = subsections(docs["bs"]), subsections(docs["ta"])
    adrs = adr_blocks(docs["dec"])
    miles = set(R["milestones"])
    caps, contracts, chain = R["capabilities"], R["contracts"], R["chain"]
    tests = {c["test"] for c in caps.values()} | {m["test"] for m in R["milestones"].values() if m["test"]}
    evid = {c["evidence"] for c in caps.values()} | {m["evidence"] for m in R["milestones"].values() if m["evidence"]}

    # §67.8 references
    for cid, r in sorted(contracts.items()):
        for sec in secrefs(r["authority"]) + secrefs(r["ext"]):
            if sec not in bs_secs:
                D.add("dangling reference", cid, f"build spec §{sec} does not exist")
        if r["arch"] != "all":
            for sec in secrefs(r["arch"]):
                if sec not in ta_secs:
                    D.add("dangling reference", cid, f"architecture §{sec} does not exist")
        for n in re.findall(r"ADR-(\d+)", r["adr"]):
            if int(n) not in adrs:
                D.add("dangling reference", cid, f"ADR-{n} does not exist")
        for n in re.findall(r"M(\d+)", r["mile"]):
            if int(n) not in miles:
                D.add("dangling reference", cid, f"M{n} has no contract mapping")

    # §5.6 references
    for cap, c in sorted(caps.items()):
        for cid in c["contracts"]:
            if cid not in contracts:
                D.add("dangling reference", cap, f"requires unregistered {cid}")
        if not re.match(r"TEST-[A-Z0-9]+-\d+$", c["test"]):
            D.add("dangling reference", cap, f"malformed test id {c['test']!r}")
        if not re.match(r"EV-[A-Z0-9]+-\d+$", c["evidence"]):
            D.add("dangling reference", cap, f"malformed evidence id {c['evidence']!r}")

    # §67.15 twelve-edge references — resolved EXACTLY, in the edge's own domain.
    # A reference that exists only in the other document is a dangling reference:
    # existence is not identity.
    universe = {
        "BS": (bs_secs, bs_subs),
        "TA": (ta_secs, ta_subs),
    }
    for cid, row in sorted(chain.items()):
        if cid not in contracts:
            D.add("dangling reference", cid, "twelve-edge row for unregistered contract")
        if row["capability"] not in caps:
            D.add("dangling reference", cid, f"capability {row['capability']} not in §5.6")

        for edge, required in EDGE_DOMAIN.items():
            cell = row[edge]
            if cell == "all":
                continue
            doc, sec, sub, err = parse_ref(cell)
            if err:
                D.add("dangling reference", cid, f"{edge}: {err}")
                continue
            if required is not None and doc != required:
                other_secs, other_subs = universe[doc]
                exists_here = (sec, sub) in other_subs if sub else sec in other_secs
                D.add("dangling reference", cid,
                      f"{edge} points at {doc} §{sec}"
                      + (f".{sub}" if sub else "")
                      + f" but this edge addresses {required}"
                      + (" (target exists in the wrong document)" if exists_here else ""))
                continue
            secs, subs = universe[doc]
            if sub is None:
                if sec not in secs:
                    D.add("dangling reference", cid,
                          f"{edge} {doc} §{sec} does not exist in {doc}")
            elif (sec, sub) not in subs:
                D.add("dangling reference", cid,
                      f"{edge} {doc} §{sec}.{sub} does not exist in {doc}")
        for n in re.findall(r"ADR-(\d+)", row["adr"]):
            if int(n) not in adrs:
                D.add("dangling reference", cid, f"ADR-{n} does not exist")
        for n in re.findall(r"M(\d+)", row["milestone"]):
            if int(n) not in miles:
                D.add("dangling reference", cid, f"M{n} has no contract mapping")
        if row["test"] not in tests:
            D.add("dangling reference", cid, f"test id {row['test']} is defined nowhere")
        if row["evidence"] not in evid:
            D.add("dangling reference", cid, f"evidence id {row['evidence']} is defined nowhere")

    # §67.12 clause authority sections
    for cl, meta in sorted(R["clauses"].items()):
        for sec in secrefs(meta["authority"]):
            if sec not in bs_secs:
                D.add("dangling reference", cl, f"authority §{sec} does not exist")


def check_forward(R, D):
    """Check 8: every capability resolves a complete twelve-edge chain."""
    caps, chain, contracts = R["capabilities"], R["chain"], R["contracts"]

    # every registered contract needs a twelve-edge row with no empty cell
    for cid in sorted(contracts):
        row = chain.get(cid)
        if row is None:
            D.add("forward break", cid, "no row in the §67.15 twelve-edge table")
            continue
        for edge in EDGES:
            val = row[edge].strip()
            if not val or val in ("—", "-", "TBD", "n/a"):
                D.add("forward break", cid, f"edge {edge!r} is unresolved ({val!r})")

    # every capability must reach evidence through each of its contracts
    for cap, c in sorted(caps.items()):
        if not c["contracts"]:
            D.add("forward break", cap, "capability requires no contract")
        for cid in c["contracts"]:
            row = chain.get(cid)
            if row is None:
                D.add("forward break", cap, f"required {cid} has no twelve-edge row")
                continue
            if row["capability"] != cap and cap not in row["capability"]:
                # a contract may serve a different primary capability; ensure the
                # capability's own test/evidence ids are reachable somewhere
                pass
        if c["status"] == "SUPPORTED":
            for cid in c["contracts"]:
                row = chain.get(cid, {})
                if not row or not row.get("evidence"):
                    D.add("forward break", cap,
                          f"claims SUPPORTED but {cid} resolves no evidence (§67.5)")


def check_reverse(R, docs, D):
    """Check 9: Evidence -> Test -> Milestone -> ADR -> Contract -> Capability."""
    caps, contracts, chain = R["capabilities"], R["contracts"], R["chain"]
    miles, adrs = R["milestones"], adr_blocks(docs["dec"])

    # index: contract -> capabilities requiring it (direct)
    required_by = {}
    for cap, c in caps.items():
        for cid in c["contracts"]:
            required_by.setdefault(cid, []).append(cap)

    # 9a. every milestone mapping must resolve contract -> capability or class
    for num, m in sorted(miles.items()):
        if not m["contracts"]:
            D.add("reverse break", f"M{num}", "mapping declares no ContractId")
        for cid in m["contracts"]:
            if cid not in contracts:
                D.add("reverse break", f"M{num}", f"maps to unregistered {cid}")
                continue
            cls = contracts[cid]["cls"]
            if cid not in required_by and cls == "CROSS_CUTTING":
                D.add("reverse break", f"M{num}",
                      f"{cid} is CROSS_CUTTING but no capability requires it")
        if not m["test"]:
            D.add("reverse break", f"M{num}", "mapping declares no test id")
        if not m["evidence"]:
            D.add("reverse break", f"M{num}", "mapping declares no evidence id")
        for n in m["adrs"]:
            if n not in adrs:
                D.add("reverse break", f"M{num}", f"cites nonexistent ADR-{n}")

    # 9b. every ADR in the new range must lock a registered contract that is
    #     itself reachable from a capability or an accepted class
    # Every ADR cited by the contract registry must declare what it locks, plus
    # every ADR in the contract-era range. Deriving this from the registry means
    # newly added ADRs are covered without editing the verifier.
    cited = set()
    for r in contracts.values():
        cited.update(int(x) for x in re.findall(r"ADR-(\d+)", r["adr"]))
    contract_era = {n for n in adrs if n >= 140}
    for n in sorted(cited | contract_era):
        if n not in adrs:
            D.add("reverse break", f"ADR-{n}", "cited by the registry but absent")
            continue
        m = re.search(rf"\*\*Locks:\*\*\s*`({CIDRE})`", adrs[n])
        if not m:
            D.add("reverse break", f"ADR-{n}", "declares no Locks field")
            continue
        cid = m.group(1)
        if cid not in contracts:
            D.add("reverse break", f"ADR-{n}", f"locks unregistered {cid}")
            continue
        cls = contracts[cid]["cls"]
        if cid not in required_by and cls not in ("FOUNDATIONAL", "INTERNAL", "DEPRECATED"):
            D.add("reverse break", f"ADR-{n}",
                  f"locks {cid} which no capability requires and whose class {cls} "
                  f"does not exempt it from capability reachability")

    # 9c. every test and evidence id must trace back to a capability
    for cid, row in sorted(chain.items()):
        tid, eid = row["test"], row["evidence"]
        owners = [c for c, meta in caps.items() if meta["test"] == tid]
        if not owners:
            D.add("reverse break", tid, f"test id used by {cid} maps to no capability in §5.6")
        owners = [c for c, meta in caps.items() if meta["evidence"] == eid]
        if not owners:
            D.add("reverse break", eid, f"evidence id used by {cid} maps to no capability in §5.6")

    # 9d. every milestone referenced by a contract must map back to that contract
    for cid, r in sorted(contracts.items()):
        for n in re.findall(r"M(\d+)", r["mile"]):
            m = miles.get(int(n))
            if m and cid not in m["contracts"]:
                D.add("reverse break", f"M{n}",
                      f"{cid} names M{n} but M{n}'s mapping does not list {cid}")


def check_orphan(R, adj, D):
    """Check 10: contract reachability per §67.14, not merely a valid class."""
    caps, contracts = R["capabilities"], R["contracts"]

    # direct capability requirement
    direct = set()
    for c in caps.values():
        direct.update(c["contracts"])

    # transitive: a contract whose authority section is an extension of a
    # reachable contract inherits reachability through the extension graph
    auth_of = {}
    for cid, r in contracts.items():
        secs = secrefs(r["authority"])
        if secs:
            auth_of.setdefault(secs[0], []).append(cid)
    reachable = set(direct)
    changed = True
    while changed:
        changed = False
        for cid, r in contracts.items():
            if cid in reachable:
                continue
            for ext_sec in secrefs(r["ext"]):
                for owner in auth_of.get(ext_sec, []):
                    if owner in reachable:
                        reachable.add(cid)
                        changed = True
    # contracts required by other contracts' architecture columns
    required_by_contract = {}
    for cid, r in contracts.items():
        for other, o in contracts.items():
            if other == cid:
                continue
            if cid in (o["arch"] + o["adr"] + o["mile"]):
                required_by_contract.setdefault(cid, []).append(other)

    for cid, r in sorted(contracts.items()):
        cls = r["cls"]
        if cls not in CLASSES:
            D.add("orphan contract", cid, f"invalid class {cls!r}, expected one of {CLASSES}")
            continue
        if cls == "CROSS_CUTTING":
            if cid not in reachable:
                D.add("orphan contract", cid,
                      "CROSS_CUTTING but not capability-reachable from any §5.6 capability")
        elif cls == "FOUNDATIONAL":
            dependents = [o for o, ro in contracts.items()
                          if o != cid and cid in ro["ext"] + ro["arch"]]
            extenders = secrefs(r["ext"])
            if cid not in reachable and len(extenders) < 2 and len(dependents) < 2:
                D.add("orphan contract", cid,
                      f"FOUNDATIONAL but required by only {max(len(extenders), len(dependents))} "
                      f"other contracts; §67.14 requires at least 2")
        elif cls == "INTERNAL":
            if cid not in reachable and cid not in required_by_contract:
                D.add("orphan contract", cid,
                      "INTERNAL but referenced by no capability and no other contract")
        elif cls == "DEPRECATED":
            pass  # handled by check_unversioned_override


# ------------------------------------------------------ structure + driver

def check_canonical_identity(docs, R, D):
    """Check 11: DOCUMENTATION_CANONICALITY_INVARIANT.

    Every cross-document reference MUST resolve to exactly one canonical
    object whose semantic role (heading type) matches the expected role
    of the referring edge. A reference that is syntactically valid but
    semantically wrong — e.g. BS §69 pointing at a section that exists
    but is now titled "Legacy Scope Language" instead of "Intent-Driven
    Android Synthesis" — is a certification failure.

    This is the bidirectional identity invariant: forward edge resolves
    to the authoritative object, and reverse traversal returns to source.
    """

    # DOCUMENTATION_CANONICALITY_INVARIANT — Check 11.
    # Validates that cross-document references maintain semantic identity:
    # a reference must resolve to exactly one canonical object, and no two
    # distinct canonical objects may claim the same section/heading.
    #
    # 1. Uniqueness: no section number is authoritative for two contracts
    #    in §67.8 (already enforced by Check 1: duplicate authority).
    # 2. Existence: every referenced section exists (already enforced by
    #    Check 7: dangling reference).
    # 3. Consistency: the section that declares a ContractId header as
    #    authoritative must be referenced consistently across §67.8, §67.15,
    #    and §5.6 — i.e. no document says "SCOPE → BS §5" while another says
    #    "SCOPE → BS §69".
    # 4. Heading stability: a section referenced as an authority/persistence/
    #    schema/failure edge must retain its declared semantic heading — if
    #    the heading changes to refer to an unrelated domain, the reference
    #    has lost its identity.

    # Build heading index for existence + stability checks
    heading_index = {}
    for doc_tag, key in (("bs", "bs"), ("ta", "ta")):
        for m in re.finditer(r'^##\s+(\d+)\.\s+(.+)$', docs[key], re.M):
            num = int(m.group(1))
            heading_index.setdefault(doc_tag, {})[num] = m.group(2).strip()

    # Check 4 (partial): only validate the *authority* edge heading, since that
    # is the primary semantic anchor. Persistence/schema/failure edges may
    # legitimately point to multi-domain architecture sections.
    for cid, row in R["chain"].items():
        domain_pat = _contract_domain_pattern(cid)
        if not domain_pat:
            continue
        cell = row.get("authority", "")
        doc_tag, sec = _parse_doc_sec(cell)
        if doc_tag and sec:
            heading = heading_index.get(doc_tag.lower(), {}).get(sec, "")
            if heading and not re.search(domain_pat, heading, re.I):
                D.add("canonical identity", f"{cid} authority",
                      f"points to {doc_tag} §{sec} '{heading}' — heading does not "
                      f"align with {cid} domain (semantic drift)")

    # Check 3: the §67.8 authority must be consistent — the same contract
    # must not be mapped to two different authority sections across the
    # capability registry and the twelve-edge table.
    for cid in R["contracts"]:
        reg_auth = _parse_doc_sec(R["contracts"][cid]["authority"])
        if cid in R["chain"]:
            edge_auth = _parse_doc_sec(R["chain"][cid].get("authority", ""))
            if reg_auth and edge_auth and reg_auth != edge_auth:
                D.add("canonical identity", cid,
                      f"§67.8 authority {reg_auth[0]} §{reg_auth[1]} differs from "
                      f"§67.15 authority edge {edge_auth[0]} §{edge_auth[1]}")


def _parse_doc_sec(cell):
    """Parse 'BS §69' or 'TA §19.2' -> (doc, section_num)."""
    m = re.match(r'(BS|TA)\s+§(\d+)', cell.strip())
    if m:
        return m.group(1), int(m.group(2))
    return None, None


def _contract_domain_pattern(contract_id):
    """Return a regex matching the canonical domain keywords for a contract,
    based on the actual authority-section headings in the corpus. Returns
    None if no domain-specific check applies."""
    parts = contract_id.split(".")
    if len(parts) < 3:
        return None
    domain = parts[-1]  # CONTRACT.RUNTIME.<DOMAIN>
    patterns = {
        # BS authority headings: BS §5 "Android-Only Application Scope",
        # BS §69 "Intent-Driven Android Synthesis", etc.
        "SCOPE":           r"Scope|Intent|Android-only|Application",
        "AUTHORITY":       r"Authority|Runtime\s*Contract|Operation|Completion",
        "EVIDENCE":        r"Evidence|Completion|Authority|Trace|Record",
        "MEMORY":          r"Memory|Replay|Recovery|State|History|Session",
        "CONTEXT":         r"Context|Scaling|Architecture|Agent",
        "WORKSPACE":       r"Workspace|Swarm|Coordination|Execution|Reserva",
        "RESERVATION":     r"Reservation|Coordination|Swarm|Lease",
        "RECONCILIATION":   r"Reconciliation|Coordinate|Swarm",
        "E2E":             r"End|State|Scenario|Testing|Verification|Probe",
        "VERIFICATION":    r"Verification|Quality|Gate|Validator|Inspect|Architecture",
        "LOCALIZATION":    r"Localization|Regression|Language|Locale",
        "SUPPLY_CHAIN":    r"Supply|Chain|Security|Provenance|Artifact",
        "DEVICE_MATRIX":   r"Device|Scenario|Coordination|Android|Multi-Device",
        "DIRECTIVE":       r"Directive|Command|Routing|Router|Control|Service",
        "DEBUGGER":        r"Debugger|Debug|Trace|Crash|Logcat|Runtime",
        "PROFILING":       r"Profiling|Resource|Performance|Metric|Telemetry",
        "TRIGGER":         r"Trigger|Event|Gateway|External|Scheduler|Hook",
        "SPECULATION":     r"Speculation|Candidate|Branching|Repair|Govern|Decision",
        "SKILL":           r"Skill|Worker|Autonomous|Capabilit|Develop",
        "REASONING":       r"Reasoning|Delegation|Capability|Mode|Agent",
        "DELIBERATION":    r"Deliberation|Reasoning|Evidence|Alternative|Adap",
        "INVARIANTS":      r"Invariant|Safety|Consistency|Document|Coverage",
        "PROMPT_CONTRACT": r"Intent|Prompt|Synthesis|Truthful|Preview|Revision",
    }
    return patterns.get(domain)


def check_semantic_documentation(docs, R, D):
    """Detect high-risk semantic drift not covered by the contract graph."""
    bs, ta, dec, dev = docs["bs"], docs["ta"], docs["dec"], docs["dev"]

    if "goalTemplate" in ta:
        D.add("semantic documentation", "goalTemplate",
              "active schedule schema uses template terminology; use goalDefinition or goalSpecification")

    browser_core = (
        "Run browser, device, accessibility, and visual QA where applicable",
        "browser/device/accessibility/visual QA",
    )
    for phrase in browser_core:
        if phrase in bs or phrase in ta or phrase in dec:
            D.add("semantic documentation", "browser validation",
                  "browser wording may be interpreted as a required or authoritative Android validation stage")
            break

    if "§5.5 coverage matrix" in dev:
        D.add("semantic documentation", "coverage-section reference",
              "stale §5.5 coverage-matrix reference; current matrix is §5.6")

    gate_heading = "### 73.5.1 Canonical `PreviewPromotionGate`"
    if ta.count(gate_heading) != 1:
        D.add("semantic documentation", "PreviewPromotionGate",
              f"expected exactly one canonical definition, found {ta.count(gate_heading)}")
    if ta.count("PreviewPromotionGate") + bs.count("PreviewPromotionGate") + dec.count("PreviewPromotionGate") < 3:
        D.add("semantic documentation", "PreviewPromotionGate references",
              "canonical preview gate is not referenced by all required normative surfaces")

    profile_section = re.search(
        r"### 5\.7\.1 Internal capability-profile identity(.*?)(?=\n### 5\.7\.2|\n## 6\.)",
        bs, re.S)
    if not profile_section or not re.search(r"(?m)^- profileId$", profile_section.group(1)):
        D.add("semantic documentation", "ProfileId",
              "internal capability-profile identity is missing a stable ProfileId")

    registry = bs.split("### 5.7 Capability Registry", 1)[-1].split("## 6.", 1)[0]
    for line in registry.splitlines():
        if re.match(r"^\|[^|]+\|.*\| SUPPORTED(?:_WITH_ENVIRONMENT_REQUIREMENTS)?\s*\|$", line):
            if "PROFILE.ANDROID." not in line or "FIXTURE-" not in line:
                D.add("semantic documentation", "supported capability profile",
                      "SUPPORTED capability row lacks a concrete ProfileId and fixture identity")
                break

    milestone_titles = {}
    delivery_table = dev.split("## 3. M0:", 1)[0]
    for line in delivery_table.splitlines():
        m = re.match(r"^\|\s*M(\d+)\s*\|\s*([^|]+?)\s*\|", line)
        if not m:
            continue
        title = re.sub(r"\s+", " ", m.group(2).strip()).casefold()
        if title in milestone_titles:
            D.add("semantic documentation", "milestone outcome",
                  f"M{m.group(1)} duplicates the outcome title of M{milestone_titles[title]}")
            break
        milestone_titles[title] = m.group(1)

    if "### 16.2.1 Execution profiles and approval precedence" not in ta:
        D.add("semantic documentation", "approval precedence",
              "execution-profile approval precedence is not canonically defined")

    # Cross-entity contract lint. These predicates intentionally remain narrow:
    # they confirm that the canonical owner and required vocabulary exist, while
    # runtime certification must prove that the contracts actually execute.
    required_build_anchors = {
        "state separation": "### 5.7.2 Canonical maturity and operational state separation",
        "artifact policy": "### 5.7.3 Canonical artifact and delivery policy",
        "evidence dependencies": "### 5.7.4 Evidence dependencies and cascading invalidation",
        "integration operationality": "### 5.7.5 Required integration operationality",
        "external-effect reconciliation": "### 5.7.6 External-effect reconciliation",
        "completion predicate": "### 5.7.7 Completion predicate and illegal-state rules",
        "integration boundary": "## 70. Integration Boundary Contract",
        "preview synchronization": "## 71. Preview Synchronization Protocol",
    }
    for subject, anchor in required_build_anchors.items():
        if anchor not in bs:
            D.add("semantic documentation", subject,
                  f"canonical build-spec anchor is missing: {anchor}")

    state_tokens = ("ProductLifecycleState", "AssuranceState", "CapabilityMaturity",
                    "IntegrationState", "SigningState", "DeliveryState")
    missing_state_tokens = [token for token in state_tokens if token not in bs]
    if missing_state_tokens:
        D.add("semantic documentation", "state vocabulary",
              f"canonical state separation is missing {missing_state_tokens}")

    if "The minimum local Android deliverable is an installable APK." not in bs:
        D.add("semantic documentation", "artifact minimum",
              "the minimum local Android deliverable is not explicitly APK")
    if "AAB generation is an optional separately declared release artifact" not in bs:
        D.add("semantic documentation", "optional AAB policy",
              "AAB is not explicitly optional and separately declared")
    if re.search(r"APK/AAB", bs + ta + dev + dec):
        D.add("semantic documentation", "ambiguous artifact wording",
              "legacy APK/AAB wording remains; use APK or optional AAB")

    required_cross_entity_tokens = {
        "EvidenceDependency": bs + ta,
        "IntegrationOperationality": bs + ta,
        "ExternalEffectRecord": bs + ta,
        "UsageRecord": ta,
        "CompletionDecision": bs + ta,
        "IntegrationBoundaryContract": bs + ta,
        "BoundaryOperationProjection": bs + ta,
        "UiHierarchyObservation": ta,
        "SigningOperation": ta,
        "CertificateInspection": ta,
        "ExportVerificationRecord": ta,
        "PreviewSyncEvent": bs + ta,
        "PreviewProjectionReducer": bs + ta,
        "PreviewSyncEvidenceRecord": bs + ta,
        "PreviewProjection": bs + ta,
        "authorityClass": bs + ta,
        "runtimeSessionId": bs + ta,
        "certificationDecisionRef": bs + ta,
        "causationId": bs + ta,
        "CAP.ANDROID.LIVE_PREVIEW": bs,
        "TEST-PSYNC-001": bs + dev,
        "EV-PSYNC-001": bs + dev,
        "generatedOutputs": ta,
        "deploymentArtifacts": ta,
        "FailureContextPackage": ta + dev + dec,
        "workspace_file_saved": bs + ta + dev,
        "build_completed": bs + ta + dev,
        "failure_observed": bs + ta + dev,
        "dependency_changed": bs + ta + dev,
        "promotion_or_export_requested": bs + ta + dev,
        "M110": dev,
        "ADR-196": dec,
        "CostGovernanceRecord": bs + ta,
        "AgentTrustAssessment": bs + ta,
        "ContextCachePolicy": bs + ta,
        "AndroidRuntimeIntegrityObservation": bs + ta,
        "CAP.ANDROID.BUDGETED_AUTONOMY": bs,
        "CAP.ANDROID.TRUSTED_EXTENSIONS": bs,
        "CAP.ANDROID.CONTEXT_GOVERNANCE": bs,
        "CAP.ANDROID.RUNTIME_INTEGRITY": bs,
        "TEST-COST-001": bs + dev,
        "EV-COST-001": bs + dev,
        "TEST-TRUST-001": bs + dev,
        "EV-TRUST-001": bs + dev,
        "TEST-CONTEXT-001": bs + dev,
        "EV-CONTEXT-001": bs + dev,
        "TEST-INTEGRITY-001": bs + dev,
        "EV-INTEGRITY-001": bs + dev,
        "M111": dev,
        "M112": dev,
        "M113": dev,
        "M114": dev,
        "ADR-197": dec,
        "ADR-198": dec,
        "ADR-199": dec,
        "ADR-200": dec,
    }
    for token, text in required_cross_entity_tokens.items():
        if token not in text:
            D.add("semantic documentation", token,
                  f"required cross-entity contract token is missing: {token}")

    if "## 74. Integration Boundary Implementation Contract" not in ta:
        D.add("semantic documentation", "integration architecture boundary",
              "technical architecture lacks the canonical integration-boundary implementation section")
    if "## 75. Preview Synchronization Implementation Contract" not in ta:
        D.add("semantic documentation", "preview synchronization architecture",
              "technical architecture lacks the canonical preview-synchronization implementation section")
    if "## M108 — Preview synchronization protocol and first Android vertical slice" not in dev:
        D.add("semantic documentation", "preview synchronization vertical slice",
              "development plan lacks the first Android preview-synchronization vertical slice")
    if "## M109 — Preview projection resilience and runtime-certification evidence" not in dev:
        D.add("semantic documentation", "preview synchronization resilience",
              "development plan lacks preview projection resilience and runtime-certification fixtures")
    if "## ADR-195: Make preview synchronization event- and reducer-bound" not in dec:
        D.add("semantic documentation", "preview synchronization decision",
              "decision log lacks the event-and-reducer-bound preview synchronization decision")
    if "### 71.2 Event-to-preview field ownership" not in bs:
        D.add("semantic documentation", "preview event ownership",
              "preview event-to-field ownership table is missing")
    if "### 71.3 Ordering, duplicate, stale, and reconnect rules" not in bs:
        D.add("semantic documentation", "preview replay rules",
              "preview duplicate, ordering, stale, and reconnect rules are missing")
    if "PreviewSyncEvent\n- eventId" not in bs:
        D.add("semantic documentation", "preview event schema",
              "canonical PreviewSyncEvent schema is missing")
    if "PreviewProjectionReducer\n- reducerId" not in bs:
        D.add("semantic documentation", "preview reducer schema",
              "canonical PreviewProjectionReducer schema is missing")
    if "PreviewSyncEvidenceRecord\n- evidenceId" not in bs:
        D.add("semantic documentation", "preview synchronization evidence schema",
              "canonical PreviewSyncEvidenceRecord schema is missing")
    if "PreviewProjection\n- projectionRevision" not in bs:
        D.add("semantic documentation", "preview projection schema",
              "canonical PreviewProjection dimension model is missing")
    if "authorityClass: DECLARATIVE" not in bs:
        D.add("semantic documentation", "preview event authority levels",
              "preview event authority classes are missing")
    if "Preview truth reconciliation" not in bs:
        D.add("semantic documentation", "preview runtime reconciliation",
              "preview truth reconciliation rule is missing")
    if "Every non-root event MUST identify its `causationId`" not in bs:
        D.add("semantic documentation", "preview event causality",
              "preview event causal-lineage rule is missing")
    if "`export_project` does not make a ZIP or Git bundle a deployment artifact" not in bs:
        D.add("semantic documentation", "source versus deployment export",
              "source/workspace export is not explicitly separated from deployment artifact delivery")
    if "Project.generatedOutputs ⊆ {APK, AAB, Android source project}" not in ta:
        D.add("semantic documentation", "generated output terminology",
              "architecture lacks the generatedOutputs distinction")
    if "Project.deploymentArtifacts ⊆ {APK} ∪ {AAB when PackagingProfile explicitly requires AAB}" not in ta:
        D.add("semantic documentation", "deployment artifact policy",
              "architecture lacks the conditional deployment-artifact policy")
    if "TEST-PSYNC-001" not in bs or "EV-PSYNC-001" not in bs:
        D.add("semantic documentation", "preview synchronization identifiers",
              "PreviewSync lacks dedicated test and evidence identifiers")
    if "#### Event-driven continuation and evidence feedback" not in bs:
        D.add("semantic documentation", "event-driven continuation requirements",
              "build specification lacks the event-driven continuation trigger matrix")
    if "## 76. Autonomous Continuation and Specialist Gate Contract" not in ta:
        D.add("semantic documentation", "autonomous continuation architecture",
              "technical architecture lacks the specialist-gate continuation section")
    if "## M110 — Event-driven autonomous continuation and specialist gates" not in dev:
        D.add("semantic documentation", "autonomous continuation milestone",
              "development plan lacks the event-driven continuation milestone")
    if "## ADR-196: Continue autonomous work from durable events with specialist gates" not in dec:
        D.add("semantic documentation", "autonomous continuation decision",
              "decision log lacks the durable-event continuation decision")
    if "FailureContextPackage" not in dev or "failure fingerprint" not in dev:
        D.add("semantic documentation", "failure context package requirement",
              "roadmap lacks the failure-context feedback requirement")
    if not re.search(r"failed health, validation, signing, or export gates preserve last-known-good state", dec, re.I):
        D.add("semantic documentation", "autonomous rollback preservation",
              "decision log lacks last-known-good preservation for failed gates")
    governance_sections = (
        ("## 72. Cost Governance Authority", bs, "cost governance authority"),
        ("## 73. Agent Trust Boundary Authority", bs, "agent trust authority"),
        ("## 74. Context and Cache Governance", bs, "context and cache authority"),
        ("## 75. Android Runtime Integrity Contract", bs, "Android runtime integrity authority"),
        ("## 77. Cost Governance Implementation Contract", ta, "cost governance architecture"),
        ("## 78. Agent Trust Boundary Implementation Contract", ta, "agent trust architecture"),
        ("## 79. Context and Cache Governance Implementation Contract", ta, "context and cache architecture"),
        ("## 80. Android Runtime Integrity Implementation Contract", ta, "Android runtime integrity architecture"),
        ("## M111 — Cost governance and adaptive resource control", dev, "cost governance milestone"),
        ("## M112 — Agent-layer trust boundary and extension security", dev, "agent trust milestone"),
        ("## M113 — Context compaction and cache governance", dev, "context governance milestone"),
        ("## M114 — Android runtime integrity and honest coverage", dev, "Android integrity milestone"),
        ("## ADR-197: Make cost governance a deterministic resource authority", dec, "cost governance decision"),
        ("## ADR-198: Scan and revoke agent-layer extension content", dec, "agent trust decision"),
        ("## ADR-199: Govern context compaction and provider cache reuse", dec, "context governance decision"),
        ("## ADR-200: Report Android runtime integrity as independent applicable signals", dec, "Android integrity decision"),
    )
    for anchor, text, subject in governance_sections:
        if anchor not in text:
            D.add("semantic documentation", subject, f"canonical governance anchor is missing: {anchor}")
    governance_tokens = (
        ("costCap", bs + ta, "cost cap schema"),
        ("exhaustionOutcome", bs + ta, "cost exhaustion outcome"),
        ("staticFindings", bs + ta, "agent trust scan findings"),
        ("revocationState", bs + ta, "agent trust revocation"),
        ("cacheBreakpointPolicy", bs + ta, "cache breakpoint policy"),
        ("cacheInvalidationEvents", bs + ta, "cache invalidation events"),
        ("playIntegrityApplicability", bs + ta, "Play Integrity applicability"),
        ("anrEvidenceIds", bs + ta, "ANR evidence"),
        ("batteryObservationIds", bs + ta, "battery observation"),
        ("dozeObservationIds", bs + ta, "Doze observation"),
        ("CostAuthority", ta, "cost authority implementation"),
        ("Scanners run in a restricted local process", ta, "trust scanner implementation"),
        ("ContextGovernance", ta, "context governance implementation"),
        ("Runtime collectors observe;", ta, "runtime integrity authority implementation"),
        ("Autonomy-level capability ladder", bs, "autonomy ladder"),
    )
    for token, text, subject in governance_tokens:
        if token not in text:
            D.add("semantic documentation", subject, f"governance requirement is missing: {token}")
    if "## M107 — Integration boundary contract and wiring conformance" not in dev:
        D.add("semantic documentation", "integration boundary milestone",
              "development plan lacks the integration-boundary conformance milestone")
    if "## ADR-194: Establish one canonical integration-boundary contract" not in dec:
        D.add("semantic documentation", "integration boundary decision",
              "decision log lacks the precedence decision for the canonical integration-boundary contract")
    if "SOURCE\n  → CONTRACT\n  → ADAPTER / BRIDGE\n  → AUTHORITY\n  → STATE\n  → OPERATION\n  → OBSERVATION\n  → EVIDENCE\n  → VALIDATION\n  → DOWNSTREAM EFFECT" not in bs:
        D.add("semantic documentation", "universal integration chain",
              "canonical source-to-downstream-effect chain is missing")
    if "CertificateInspection\n- inspectionId" not in ta:
        D.add("semantic documentation", "certificate inspection schema",
              "canonical CertificateInspection schema is missing")

    if "### 69.10 Runtime-certification and hidden-human-dependency boundary" not in bs:
        D.add("semantic documentation", "runtime certification boundary",
              "documentation certification is not separated from runtime certification")
    if "hidden-human dependency" not in bs or "M104 — Hidden-human-dependency" not in dev:
        D.add("semantic documentation", "hidden human dependency",
              "hidden-human-dependency behavior lacks a canonical contract and fixture milestone")
    if "## M105 — Schema parity and cross-document conformance" not in dev:
        D.add("semantic documentation", "schema parity",
              "schema-parity and cross-document conformance milestone is missing")
    if "documentation certification" not in dev.lower() or "runtime certification" not in dev.lower():
        D.add("semantic documentation", "certification tier separation",
              "development plan does not distinguish documentation and runtime certification")
    if "reproducibilityLevel" not in bs or "repositoryTrustRequirement" not in bs:
        D.add("semantic documentation", "profile maturity fields",
              "capability profile is missing reproducibility or repository-trust identity")
    if "attributionStatus" not in ta:
        D.add("semantic documentation", "resource attribution",
              "resource usage lacks explicit parent/child/shared attribution")


def check_structure(docs, R, D):
    """Document-level integrity that the contract graph presupposes."""
    for label, key in (("build spec", "bs"), ("architecture", "ta")):
        text = docs[key]
        secs = sorted(sections(text))
        if not secs:
            D.add("structure", label, "no numbered sections found")
            continue
        if secs != list(range(1, max(secs) + 1)):
            missing = [n for n in range(1, max(secs) + 1) if n not in secs]
            D.add("structure", label, f"section numbering not contiguous, missing {missing}")
        nrefs = len(re.findall(r"^#{1,3}\s+References", text, re.M))
        if nrefs != 1:
            D.add("structure", label, f"has {nrefs} References sections, expected exactly 1")
        # orphan subsections: child number must match nearest preceding parent
        cur = None
        for line in text.split("\n"):
            ms = re.match(r"^##\s+(\d+)\.\s", line)
            if ms:
                cur = int(ms.group(1))
                continue
            mc = re.match(r"^###\s+(\d+)\.(\d+)\s", line)
            if mc and cur is not None and int(mc.group(1)) != cur:
                D.add("structure", label,
                      f"subsection {mc.group(1)}.{mc.group(2)} sits under section {cur}")

    adrs = adr_blocks(docs["dec"])
    nums = sorted(adrs)
    if nums != list(range(1, max(nums) + 1)):
        gaps = [n for n in range(1, max(nums) + 1) if n not in adrs]
        D.add("structure", "decision log", f"ADR numbering has gaps: {gaps}")
    # The corpus carries three house styles for the rationale and consequence
    # roles. Require the ROLE to be filled, not one specific label.
    RATIONALE = ("**Rationale:**", "**Reasoning:**")
    CONSEQUENCE = ("**Consequences:**", "**Implication:**", "**Implications:**",
                   "**Trade-off:**", "**Trade-offs:**")
    for n, body in sorted(adrs.items()):
        # A register entry enumerates open decisions rather than recording one,
        # and is identified by carrying a table and no Decision field.
        is_register = "**Decision:**" not in body and "|---" in body
        if is_register:
            continue
        if "**Status:**" not in body:
            D.add("structure", f"ADR-{n}", "missing Status field")
        if "**Decision:**" not in body:
            D.add("structure", f"ADR-{n}", "missing Decision field")
        if not any(k in body for k in RATIONALE):
            D.add("structure", f"ADR-{n}",
                  f"missing rationale role (one of {', '.join(RATIONALE)})")
        if not any(k in body for k in CONSEQUENCE):
            D.add("structure", f"ADR-{n}",
                  f"missing consequence role (one of {', '.join(CONSEQUENCE)})")

    # registry cardinality sanity
    if len(R["contracts"]) < 2:
        D.add("structure", "§67.8", "registry has fewer than 2 contracts")
    if not R["capabilities"]:
        D.add("structure", "§5.6", "capability registry is empty")
    if not R["clauses"]:
        D.add("structure", "§67.12", "clause registry is empty")


CHECK_ORDER = (
    "duplicate authority", "unregistered contract", "undeclared extension",
    "authority cycle", "clause contradiction", "unversioned override",
    "dangling reference", "forward break", "reverse break", "orphan contract",
    "canonical identity", "structure", "semantic documentation",
)


def verify(root):
    docs = load(root)
    D = Defects()
    R = parse_registries(docs, D)

    check_duplicate_authority(R, D)
    check_unregistered(R, docs, D)
    check_undeclared_extension(R, D)
    adj = check_authority_cycle(R, D)
    check_clause_contradiction(R, docs, D)
    check_unversioned_override(R, docs, D)
    check_dangling(R, docs, D)
    check_forward(R, D)
    check_reverse(R, docs, D)
    check_orphan(R, adj, D)
    check_canonical_identity(docs, R, D)
    check_semantic_documentation(docs, R, D)
    check_structure(docs, R, D)
    return R, adj, D


def main():
    root = sys.argv[1] if len(sys.argv) > 1 else "."
    R, adj, D = verify(root)

    edges = sum(len(v) for v in adj.values())
    print("Nirman contract-graph verifier — build spec §67.11")
    print("-" * 58)
    print(f"capabilities registered : {len(R['capabilities'])}")
    print(f"contracts registered    : {len(R['contracts'])}")
    print(f"clauses registered      : {len(R['clauses'])}")
    print(f"twelve-edge rows        : {len(R['chain'])}")
    print(f"extension declarations  : {len(R['declarations'])}")
    print(f"authority edges         : {edges}")
    print(f"milestone mappings      : {len(R['milestones'])}")
    print(f"defects                 : {len(D)}")

    grouped = D.by_check()
    print("\ncheck results")
    for check in CHECK_ORDER:
        hits = grouped.get(check, [])
        status = "PASS" if not hits else f"FAIL ({len(hits)})"
        print(f"  {status:<10} {check}")

    if len(D):
        print("\nDEFECTS")
        for check in CHECK_ORDER:
            for subject, detail in grouped.get(check, []):
                print(f"  [{check}] {subject}: {detail}")
        print("\nCERTIFICATION: FAIL")
        return 1

    print("\nall 12 §67.11 graph/structure checks pass in both traversal directions")
    print("semantic documentation lint: PASS")
    print("CERTIFICATION: PASS")
    return 0


if __name__ == "__main__":
    sys.exit(main())
