#!/usr/bin/env python3
"""
Nirman contract-graph verifier — implements build spec §67.11.

Runs all twelve §67.11 contract-graph checks (1-12) over the four canonical
documents in both traversal directions (§67.9), plus the three document-structure
checks that BS §67.11 lists as additional to the twelve: Check 13 structure,
Check 14 command payload coverage, Check 15 semantic documentation (which also
carries the skill-body rules of BS §79.7). Exits 1 on any defect.
`--dump-registries` prints the parsed registries.

Registries consumed:
  §5.7   Capability Registry           (CapabilityId -> required contracts, test, evidence)
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

if hasattr(sys.stdout, "reconfigure"):
    sys.stdout.reconfigure(encoding="utf-8", errors="replace")
if hasattr(sys.stderr, "reconfigure"):
    sys.stderr.reconfigure(encoding="utf-8", errors="replace")

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
    """Ordered, individually addressable defect list (§67.11).

    `add` records a genuine defect (a broken invariant). `skip` records a check
    that could not be evaluated because its input — typically Rust source under
    crates/ — is not present in the working tree. Skips are tracked separately
    so certification output can distinguish "nothing to check" from "check failed".
    """

    def __init__(self):
        self.items = []
        self.skips = []

    def add(self, check, subject, detail):
        self.items.append((check, subject, detail))

    def skip(self, check, subject, detail):
        self.skips.append((check, subject, detail))

    def __len__(self):
        return len(self.items)

    def by_check(self):
        out = {}
        for check, subject, detail in self.items:
            out.setdefault(check, []).append((subject, detail))
        return out

    def skips_by_check(self):
        out = {}
        for check, subject, detail in self.skips:
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
        cid = c[0]
        if cid in reg:
            D.add("structure", cid, f"duplicate contract registry identity: {cid}")
            continue
        reg[cid] = dict(authority=c[1], ext=c[2], arch=c[3],
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
            D.add("structure", c[0] if c else "?", "§5.7 row has too few cells")
            continue
        capid = c[0]
        if capid in caps:
            D.add("structure", capid, f"duplicate capability registry identity: {capid}")
            continue
        caps[capid] = dict(requirement=c[1],
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
        clid = c[0]
        if clid in clauses:
            D.add("structure", clid, f"duplicate clause registry identity: {clid}")
            continue
        clauses[clid] = dict(contract=c[1], authority=c[2],
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
        cid = c[0]
        if cid in chain:
            D.add("structure", cid, f"duplicate twelve-edge registry identity: {cid}")
            continue
        chain[cid] = dict(zip(EDGES, c[1:]))
    R["chain"] = chain

    # milestone mappings from the development plan
    # Generic, position-independent parsing of milestone contract mapping tables:
    # A milestone contract mapping table is identified by a header row with
    # 'Milestone' in column 0, 'contract' in column 1, and 'adr' in column 2.
    miles = {}
    in_mapping_table = False
    for line in dev.splitlines():
        line = line.strip()
        if not line.startswith("|") or not line.endswith("|"):
            in_mapping_table = False
            continue
        cells = [c.strip() for c in line.split("|")[1:-1]]
        if not cells:
            in_mapping_table = False
            continue
        lower_cells = [c.lower() for c in cells]
        if len(cells) >= 3 and lower_cells[0] == "milestone" and "contract" in lower_cells[1] and "adr" in lower_cells[2]:
            in_mapping_table = True
            continue
        if in_mapping_table and all(re.match(r"^:?-+:?$", c) for c in cells):
            continue
        if in_mapping_table:
            m = re.match(r"^M(\d+)$", cells[0])
            if not m:
                in_mapping_table = False
                continue
            m_num = int(m.group(1))
            if m_num in miles:
                D.add("structure", f"M{m_num}", f"duplicate milestone registry identity: M{m_num}")
                continue
            owned, extended = [], []
            for part in cells[1].split(","):
                ids = re.findall(CIDRE, part)
                if not ids:
                    continue
                (extended if re.match(r"\s*extends\b", part) else owned).extend(ids)
            miles[m_num] = dict(
                contracts=owned + extended,
                owns=owned,
                extends=extended,
                adrs=[int(x) for x in re.findall(r"ADR-(\d+)", cells[2])] if len(cells) > 2 else [],
                test=cells[3] if len(cells) > 3 else "",
                evidence=cells[4] if len(cells) > 4 else "")
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
            decl_key = (num, decl["authority_contract"])
            if decl_key in decls:
                D.add("structure", f"§{num}/{decl['authority_contract']}",
                      f"duplicate extension declaration identity: §{num} -> {decl['authority_contract']}")
                continue
            decls[decl_key] = decl
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
        D.add("unregistered contract", cap, "capability referenced but absent from the §5.7 capability registry")
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

    # §5.7 capability registry references
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
            D.add("dangling reference", cid, f"capability {row['capability']} not in the §5.7 capability registry")

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
            D.add("reverse break", tid, f"test id used by {cid} maps to no capability in §5.7")
        owners = [c for c, meta in caps.items() if meta["evidence"] == eid]
        if not owners:
            D.add("reverse break", eid, f"evidence id used by {cid} maps to no capability in §5.7")

    # 9d. every milestone referenced by a contract must map back to that contract
    for cid, r in sorted(contracts.items()):
        for n in re.findall(r"M(\d+)", r["mile"]):
            m = miles.get(int(n))
            if m and cid not in m["contracts"]:
                D.add("reverse break", f"M{n}",
                      f"{cid} names M{n} but M{n}'s mapping does not list {cid}")

    # 9e. single canonical owner (DP ownership rule): the §67.8 milestone owns
    #     the contract; every other mapping row that lists it must say
    #     `extends`. Two owners, or an owner that is not the §67.8 milestone,
    #     is a reverse break.
    owners = {}
    for num, m in sorted(miles.items()):
        for cid in m.get("owns", m["contracts"]):
            owners.setdefault(cid, []).append(num)
    for cid, r in sorted(contracts.items()):
        registry_owner = [int(n) for n in re.findall(r"M(\d+)", r["mile"])]
        listed = owners.get(cid, [])
        if not registry_owner:
            continue
        if len(listed) > 1:
            D.add("reverse break", cid,
                  f"two owning milestones in the contract mapping ({', '.join('M%d' % n for n in listed)}); "
                  f"only M{registry_owner[0]} (§67.8) owns it, the others must say 'extends'")
        elif listed and listed[0] not in registry_owner:
            D.add("reverse break", cid,
                  f"mapping owner M{listed[0]} differs from the §67.8 milestone M{registry_owner[0]}")
    for num, m in sorted(miles.items()):
        for cid in m.get("extends", []):
            if cid in contracts and num in [int(n) for n in re.findall(r"M(\d+)", contracts[cid]["mile"])]:
                D.add("reverse break", f"M{num}",
                      f"declares 'extends {cid}' but is that contract's §67.8 owner")

    # 9f. milestone-level test/evidence ids must be declared constituents of the
    #     owning contract's capability-level ids in the milestone's own text.
    cap_tests = {c["test"] for c in caps.values()}
    cap_evidence = {c["evidence"] for c in caps.values()}
    dev = docs["dev"]
    for num, m in sorted(miles.items()):
        if m["test"] in cap_tests and m["evidence"] in cap_evidence:
            continue
        cap_ids = set()
        for cid in m["contracts"]:
            row = chain.get(cid)
            if row:
                cap_ids.update((row["test"], row["evidence"]))
        if not m["test"] or not m["evidence"]:
            continue
        alts = "|".join(re.escape(x) for x in cap_ids) or "NONE"
        ev = re.escape(m["evidence"])
        constituent = rf"`{ev}`[^\n]*constituent[^\n]*`(?:{alts})`"
        incomplete = rf"`(?:{alts})`[^\n]*not complete[^\n]*`{ev}`[^\n]*missing"
        if not cap_ids or not re.search(constituent, dev) or not re.search(incomplete, dev):
            D.add("reverse break", f"M{num}",
                  f"milestone-level ids {m['test']}/{m['evidence']} are not capability-level ids; the plan "
                  f"must state that {m['evidence']} is a constituent of the capability-level evidence "
                  f"({', '.join(sorted(cap_ids)) or 'no §67.15 row for its contracts'}) and that the latter "
                  f"is not complete while {m['evidence']} is missing")


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
                      "CROSS_CUTTING but not capability-reachable from any §5.7 capability")
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
        try:
            domain_pat = _contract_domain_pattern(cid)
        except KeyError as exc:
            D.add("canonical identity", f"{cid} authority", str(exc))
            continue
        cell = row.get("authority", "")
        doc_tag, sec = _parse_doc_sec(cell)
        if doc_tag and sec:
            heading = heading_index.get(doc_tag.lower(), {}).get(sec, "")
            if heading and not re.search(domain_pat, heading, re.I):
                D.add("canonical identity", f"{cid} authority",
                      f"points to {doc_tag} §{sec} '{heading}' — heading does not "
                      f"align with {cid} domain (semantic drift)")

    # The §67.15 architecture edge must resolve to one of the architecture
    # sections §67.8 lists for the contract, and that section must name the
    # contract. An edge that lands on a section belonging to another contract
    # (SPECULATION once pointed at the emulator-scenario coordinator) has lost
    # its identity even though the section exists.
    ta_bodies = {}
    ta_marks = [(m.start(), int(m.group(1))) for m in re.finditer(r"^##\s+(\d+)\.\s", docs["ta"], re.M)]
    for i, (pos, num) in enumerate(ta_marks):
        end = ta_marks[i + 1][0] if i + 1 < len(ta_marks) else len(docs["ta"])
        ta_bodies[num] = docs["ta"][pos:end]
    for cid, row in R["chain"].items():
        cell = row.get("architecture", "").strip()
        reg_arch = R["contracts"].get(cid, {}).get("arch", "").strip()
        if cell == "all" or reg_arch in ("all", "—", "-", ""):
            continue
        doc_tag, sec = _parse_doc_sec(cell)
        if doc_tag != "TA" or not sec:
            continue
        listed = secrefs(reg_arch)
        if sec not in listed:
            D.add("canonical identity", f"{cid} architecture",
                  f"§67.15 architecture edge TA §{sec} is not among the §67.8 architecture "
                  f"sections ({reg_arch}) for this contract")
        elif not re.search(rf"(?<![A-Z_.]){re.escape(cid)}(?![A-Z_])", ta_bodies.get(sec, "")):
            D.add("canonical identity", f"{cid} architecture",
                  f"§67.15 architecture edge TA §{sec} never names `{cid}`")

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
        "LOCALIZATION":    r"Regression Localization|Regression",
        "SUPPLY_CHAIN":    r"Supply|Chain|Security|Provenance|Artifact",
        "DEVICE_MATRIX":   r"Device|Scenario|Coordination|Android|Multi-Device",
        "DIRECTIVE":       r"Directive|Command|Routing|Router|Control|Service",
        "DEBUGGER":        r"Debugger|Debug|Trace|Crash|Logcat|Runtime",
        "PROFILING":       r"Profiling|Resource|Performance|Metric|Telemetry",
        "TRIGGER":         r"Trigger|Event|Gateway|External|Scheduler|Hook",
        "SPECULATION":     r"Speculat|Candidate|Branching",
        "SKILL":           r"Skill|Worker|Autonomous|Capabilit|Develop",
        "REASONING":       r"Reasoning|Delegation|Capability|Mode|Agent",
        "DELIBERATION":    r"Deliberation|Reasoning|Evidence|Alternative|Adap",
        "INVARIANTS":      r"Invariant|Safety|Consistency|Document|Coverage",
        "PROMPT_CONTRACT": r"Intent|Prompt|Synthesis|Truthful|Preview|Revision",
        "RESOURCE_INTEGRITY": r"Resource|Integrity|Runtime",
        "AGENT_TRUST":       r"Trust|Agent|Extension|Boundary",
        "CONTEXT_GOVERNANCE": r"Context|Cache|Governance|Compaction",
        "ANDROID_INTEGRITY": r"Android|Runtime|Integrity",
        # BS §70–§83 authority headings for the newer contracts.
        "INTEGRATION_BOUNDARY":   r"Integration Boundary",
        "PREVIEW_SYNC":           r"Preview Synchronization",
        "FRONTEND_CONTROL_PLANE": r"Frontend.Control-Plane|Frontend",
        "BACKGROUND_CONTINUITY":  r"Background Continuity|Continuity",
        "APK_EXPORT":             r"APK Export|Export Provenance",
        "PLATFORM_CAPABILITY":    r"Platform|Target Environment",
        "AGENT_BUILDABILITY":     r"Buildability",
        "CONTENT_INTELLIGENCE":   r"Content|Writing Intelligence",
        "CONVERSATION_CONTEXT":   r"Conversation Context|Conversation",
        "CHANGE_INTELLIGENCE":    r"Change Intelligence",
    }
    pattern = patterns.get(domain)
    if pattern is None:
        # Every registered contract must have a domain anchor; a new contract
        # added without one would silently escape the drift check.
        raise KeyError(f"no canonical-identity domain pattern for {contract_id}")
    return pattern


def _camel_to_snake(name):
    """Convert camelCase or PascalCase to snake_case."""
    return re.sub(r"(?<!^)(?=[A-Z])", "_", name).lower()


def _parse_field_block(text, schema_name):
    """Extract field names from a fenced text block whose first line is `schema_name`.

    Returns a sorted list of field names (without any type annotation).
    A field is any line matching `^- <field>`, optionally followed by `:` and a
    type expression that may wrap onto subsequent indented lines. Continuation
    lines that don't start with `-` are folded into the previous field.
    """
    pat = re.compile(r"```text\s*\n" + re.escape(schema_name) + r"\s*\n(.+?)\n```",
                     re.S)
    m = pat.search(text)
    if m is None:
        return None
    body = m.group(1)
    fields = []
    for line in body.splitlines():
        s = line.strip()
        if s.startswith("- "):
            fld = s[2:].split(":", 1)[0].strip()
            if fld and re.match(r"^[A-Za-z][A-Za-z0-9]*$", fld):
                fields.append(fld)
    return fields


def _rust_field_present(source, struct_name, snake_field):
    """True iff `pub <snake_field>:` appears inside `struct <struct_name>`."""
    # find the struct block
    pat = re.compile(
        r"(?:pub\s+)?struct\s+" + re.escape(struct_name) + r"\s*\{(.+?)\n\}",
        re.S)
    m = pat.search(source)
    if m is None:
        return False
    body = m.group(1)
    field_pat = re.compile(
        r"^\s*pub\s+" + re.escape(snake_field) + r"\s*:\s", re.M)
    return bool(field_pat.search(body))


def check_command_payload_field_coverage(docs, R, D, root):
    """Check 14 (document-structure, BS §67.11 "Command payload coverage"
    row): every command payload exposes the policy-mandatory fields
    named by the canonical record it materializes.

    The contract is the field list declared in the fenced text block of
    nirman-technical-architecture.md. The implementation is the Rust struct
    in the corresponding crate. A drift between the two (the kind that
    silently weakened the portable M6 policy boundary) is the defect class
    this check exists to catch.
    """
    ta = docs["ta"]
    # The verifier is rooted at the directory the test runner (or user)
    # passed in. For the mutation battery this is a temp dir; for the
    # local-certification entry point it is the repo root. The Rust
    # source lookup must resolve relative to that root, not to the
    # verifier's own location.
    repo_root = root

    # (rust_struct, source_path_relative_to_repo, ta_schema_name,
    #  mandatory_fields_subset_or_None_for_all, name_aliases)
    # name_aliases maps canonical camelCase names to the actual snake_case
    # field in the Rust struct when the names diverged before the check
    # existed (e.g. legacy fields that predate the canonical record).
    #
    # For M11 domain types, `ta_schema_name` is None: there is no canonical
    # fenced text block in TA yet, so the check asserts the struct itself
    # exists with the expected field set declared in the check's
    # `mandatory_fields_subset`. Once a canonical TA block exists, set
    # `ta_schema_name` to the block name and the check will switch to
    # reading the field list from the spec.
    domain_crate = "crates/nirman-domain/src/lib.rs"
    targets = [
        ("ArtifactExportCommandPayload",
         "crates/nirman-ipc/src/lib.rs",
         "ExportVerificationRecord",
         {"packagingProfileId", "artifactKind", "requestFingerprint",
          "idempotencyKey", "deploymentDelivery", "destinationKind",
          "sourceRevision", "destinationPathReference"},
         {"destinationPathReference": "destination_path"}),
        ("PreviewRequest",
         "crates/nirman-preview/src/lib.rs",
         "PreviewRequest",
         None,
         {}),
        # M11 work item 1: Android capability registry
        ("AndroidCapabilityRegistry",
         domain_crate, None,
         {"schemaVersion", "registryId", "compositions", "toolchainLocks",
          "deviceMatrix", "fixtures", "knownExclusions"},
         {}),
        ("TechnologyComposition",
         domain_crate, None,
         {"compositionId", "language", "uiFramework", "runtimeLayer",
          "nativeModules", "buildPlugins", "deviceApis",
          "mixedArchitecture"},
         {}),
        ("ToolchainLock",
         domain_crate, None,
         {"component", "lockedVersion", "compatibleRange"},
         {}),
        ("DeviceMatrixEntry",
         domain_crate, "DeviceMatrixEntry",
         None,
         {}),
        ("FixtureRecord",
         domain_crate, None,
         {"fixtureId", "compositionId", "evidenceStatus",
          "lastVerifiedAtEpochSeconds"},
         {}),
        ("KnownExclusion",
         domain_crate, None,
         {"exclusionId", "description", "rationale"},
         {}),
        # M11 work item 2: diagnostics
        ("AndroidDiagnostic",
         domain_crate, None,
         {"schemaVersion", "diagnosticId", "kind", "status", "message",
          "remediation", "detectedAtEpochSeconds"},
         {}),
        # M11 work item 3: device-manager abstraction
        ("DeviceSession",
         domain_crate, None,
         {"deviceId", "formFactor", "apiLevel", "connectionState",
          "isEmulator"},
         {}),
        # M11 work item 4: logs, install, reload
        ("AndroidLogEntry",
         domain_crate, None,
         {"schemaVersion", "entryId", "tag", "level", "message",
          "recordedAtEpochSeconds"},
         {}),
        ("InstallStatus",
         domain_crate, None,
         {"schemaVersion", "deviceId", "packageName", "state",
          "installedAtEpochSeconds", "errorMessage"},
         {}),
        ("ReloadStatus",
         domain_crate, None,
         {"schemaVersion", "deviceId", "state", "reloadedAtEpochSeconds",
          "errorMessage"},
         {}),
        # M11 work item 5: APK build profiles and delivery. PackagingProfile
        # is the BS §5.7.3 canonical block (TA §36.1 carries the identical
        # block); the delivery record is the single canonical
        # ExportVerificationRecord of TA §74.3 (ADR-203; DP M117 forbids a
        # second export record), surfaced in full by the export response.
        ("PackagingProfile",
         domain_crate, "PackagingProfile",
         None,
         {}),
        ("ExportVerificationRecord",
         domain_crate, "ExportVerificationRecord",
         None,
         {}),
        ("ArtifactExportResponsePayload",
         "crates/nirman-ipc/src/lib.rs",
         "ExportVerificationRecord",
         None,
         {}),
        # M11 work item 6: signing configuration
        ("SigningConfig",
         domain_crate, None,
         {"configId", "keystoreReference", "keyAlias", "signingScheme",
          "keystorePasswordReference", "keyPasswordReference"},
         {}),
    ]

    for struct_name, rel_path, schema_name, mandatory_subset, aliases in targets:
        full_path = os.path.join(repo_root, rel_path)
        if not os.path.exists(full_path):
            D.skip("command payload coverage", struct_name,
                   f"source file {rel_path} not found; cannot verify coverage")
            continue
        with open(full_path, encoding="utf-8") as fh:
            source = fh.read()
        if schema_name is not None:
            canonical = _parse_field_block(ta, schema_name)
            source_label = f"TA §{schema_name}"
            if canonical is None:
                # BS-owned blocks (PackagingProfile §5.7.3, DeviceMatrixEntry §59.2)
                canonical = _parse_field_block(docs["bs"], schema_name)
                source_label = f"BS §{schema_name}"
            if canonical is None:
                D.add("command payload coverage", struct_name,
                      f"neither TA nor BS has a fenced field block for {schema_name}; "
                      f"cannot verify coverage")
                continue
            required = (set(canonical) if mandatory_subset is None
                        else mandatory_subset)
        else:
            # No canonical TA block for this struct yet. The check falls back
            # to the field set declared inline in the `mandatory_subset`,
            # which is the M11 domain-schema source of truth until a fenced
            # block is added to the spec.
            if mandatory_subset is None:
                D.add("command payload coverage", struct_name,
                      "no canonical TA block and no mandatory_subset declared; "
                      "cannot verify coverage")
                continue
            required = mandatory_subset
            source_label = "the M11 domain schema declaration"
        for fld in sorted(required):
            snake = aliases.get(fld, _camel_to_snake(fld))
            if not _rust_field_present(source, struct_name, snake):
                D.add("command payload coverage", struct_name,
                      f"required field {fld!r} (Rust: `{snake}`) is missing "
                      f"or commented out; {source_label} declares it as a "
                      f"policy-mandatory field of the canonical record")


def check_semantic_documentation(docs, R, D, root="."):
    """Check 15 (document-structure, BS §67.11 "Semantic documentation" row):
    detect high-risk semantic drift not covered by the contract graph."""
    bs, ta, dec, dev = docs["bs"], docs["ta"], docs["dec"], docs["dev"]

    if "goalTemplate" in ta:
        D.add("semantic documentation", "goalTemplate",
              "active schedule schema uses template terminology; use goalDefinition or goalSpecification")

    # Every §67.8 architecture section that is the sole implementation of a
    # contract must bind itself to that contract by name (ContractId header or
    # "Implements … `ContractId`" line). A registry row pointing at a section
    # that never mentions the contract is an implementation-less contract —
    # the SPECULATION defect this rule was added for.
    ta_secs = {}
    marks = [(m.start(), int(m.group(1))) for m in re.finditer(r"^##\s+(\d+)\.\s", ta, re.M)]
    for i, (pos, num) in enumerate(marks):
        end = marks[i + 1][0] if i + 1 < len(marks) else len(ta)
        ta_secs[num] = ta[pos:end]
    for cid, r in sorted(R["contracts"].items()):
        if r["arch"].strip() in ("all", "—", "-", ""):
            continue
        secs = secrefs(r["arch"])
        if any(re.search(rf"(?<![A-Z_.]){re.escape(cid)}(?![A-Z_])", ta_secs.get(n, "")) for n in secs):
            continue
        D.add("semantic documentation", f"{cid} architecture binding",
              f"§67.8 maps the contract to TA §{', §'.join(str(n) for n in secs)} but no listed "
              f"section names `{cid}` (ContractId header or Implements line)")
    # Forbidden preview-pipeline paths (TA §73.14). The typed command registry
    # must expose no UI → ADB / Gradle / Metro-Expo / emulator command kind, and
    # the technology adapter must expose only its six resolution operations —
    # any concrete execution operation on it is the forbidden
    # "UI → AndroidTechnologyAdapter.executeBuild | install | …" path.
    reg_rows = re.findall(r"^\| `([a-z_.]+)` \|",
                          bs.split("### 76.1 UICommandRegistry", 1)[-1].split("### 76.2", 1)[0], re.M)
    for kind in reg_rows:
        ns = kind.split(".", 1)[0]
        if ns in ("adb", "gradle", "metro", "expo", "emulator"):
            D.add("semantic documentation", "forbidden preview pipeline path",
                  f"§76.1 registers `{kind}`: a UI → {ns} command kind is a forbidden path (TA §73.14); "
                  "the UI reaches build and device tooling only through PreviewCoordinator")
    ops_block = re.search(r"```text\s*\nAndroidTechnologyAdapter operations\s*\n(.+?)\n```", ta, re.S)
    allowed_ops = {"validatePlan", "initializeProject", "planBuild", "classifyFailure",
                   "resolveBuildAdapter", "resolveDeviceAdapter"}
    if ops_block is None:
        D.add("semantic documentation", "forbidden preview pipeline path",
              "TA §73.10 lacks the `AndroidTechnologyAdapter operations` block that bounds the adapter to resolution operations")
    else:
        declared_ops = set(re.findall(r"^- ([A-Za-z]+)\(", ops_block.group(1), re.M))
        for op in sorted(declared_ops - allowed_ops):
            D.add("semantic documentation", "forbidden preview pipeline path",
                  f"TA §73.10 exposes `AndroidTechnologyAdapter.{op}`; the adapter resolves execution authorities and "
                  "must expose only validatePlan, initializeProject, planBuild, classifyFailure, resolveBuildAdapter, resolveDeviceAdapter (TA §73.14)")
        for op in sorted(allowed_ops - declared_ops):
            D.add("semantic documentation", "forbidden preview pipeline path",
                  f"TA §73.10 no longer declares `AndroidTechnologyAdapter.{op}`, which DP M108 and TA §73.14 require")
    for anchor, label in (("\nUI → ADB\nUI → Gradle\nUI → Metro or Expo\nUI → emulator\n", "TA §73.14 forbidden-path block"),
                          ("No command kind is registered in an `adb.`, `gradle.`, `metro.`, `expo.`, or `emulator.` namespace", "BS §76.1 namespace rule")):
        if anchor not in (ta if label.startswith("TA") else bs):
            D.add("semantic documentation", "forbidden preview pipeline path", f"{label} is missing")
    # The TA §74.5 certification report must carry the §67.11 status vocabulary
    # verbatim; a PASSED/FAILED result would hide the with-skips state.
    report = re.search(r"```text\s*\nDocumentationCertificationReport\s*\n(.+?)\n```", ta, re.S)
    if report is None:
        D.add("semantic documentation", "certification report schema",
              "TA §74.5 lacks the DocumentationCertificationReport field block")
    else:
        body = report.group(1)
        if "- result: FAIL | DOCUMENTATION_CERTIFIED_WITH_RUNTIME_SOURCE_SKIPS | DOCUMENTATION_CERTIFIED" not in body:
            D.add("semantic documentation", "certification report schema",
                  "TA §74.5 `result` must enumerate exactly the §67.11 terminal statuses "
                  "FAIL | DOCUMENTATION_CERTIFIED_WITH_RUNTIME_SOURCE_SKIPS | DOCUMENTATION_CERTIFIED")
        for fld in ("checksUnevaluated", "unevaluatedSubjects"):
            if f"- {fld}" not in body:
                D.add("semantic documentation", "certification report schema",
                      f"TA §74.5 report lacks `{fld}`; the with-skips status is unverifiable without it")
    # In-process hosting of the control plane is a bounded pre-M7 allowance,
    # never an alternative to the two-process architecture (ADR-111, ADR-117).
    for text, label in ((bs, "BS §51.2"), (ta, "TA §57.2")):
        if "in-process inside `Nirman.exe`" in text or "in-process with the WinUI 3 application" in text:
            if "from M7 onward `Nirman.exe` and `NirmanSupervisor.exe` MUST be distinct processes" not in text:
                D.add("semantic documentation", "in-process hosting bound",
                      f"{label} permits in-process control-plane hosting without bounding it to the pre-M7 vertical slice")
            if "An in-process build MUST NOT claim the M7 exit gate, `CAP.ANDROID.BACKGROUND_CONTINUITY`" not in text:
                D.add("semantic documentation", "in-process hosting bound",
                      f"{label} must forbid an in-process build from claiming the M7 exit gate or background continuity")
    # ADR-039's stable launcher/controller must be placed inside the
    # two-process product (ADR-002A): the bootstrap stage of the supervisor,
    # never a third executable.
    if "bootstrap stage of `NirmanSupervisor.exe`" not in ta or "not a third executable" not in ta:
        D.add("semantic documentation", "update controller placement",
              "TA §25.2 must place the stable launcher/controller as the bootstrap stage of NirmanSupervisor.exe (ADR-039 within ADR-002A)")
    if "is the update-controller bootstrap stage of `NirmanSupervisor.exe`, not a third executable" not in bs:
        D.add("semantic documentation", "update controller placement",
              "BS §6.1.1 host process contract must state that the update controller is NirmanSupervisor.exe's bootstrap stage, not a third executable")
    # Orphan component names (audit M20): every named component must exist
    # somewhere as a defined record, table row, or field.
    tolerated = (
        "(there is no separate `ProviderContextDecision` record; the decision is the envelope field)",
        "No `ContextStore`, `RequirementStore`, or `DecisionStore` component exists; those words in earlier drafts named these three.",
        "No `ContextStore`, `RequirementStore`, or `DecisionStore` component exists.",
        "(no separate `ProviderCapabilityProfile` record)",
    )
    for name in ("ProviderContextDecision", "ContextStore", "RequirementStore", "DecisionStore", "ProviderCapabilityProfile"):
        for text, label in ((bs, "BS"), (ta, "TA"), (docs["dev"], "DP")):
            stripped = text
            for t in tolerated:
                stripped = stripped.replace(t, "")
            if name in stripped:
                D.add("semantic documentation", "orphan component",
                      f"{label} names {name}, which is defined nowhere; use the owning record (ProviderContextEnvelope.transmissionDecision, MemoryStore/ContextOrchestrator/ConstraintRegistry, ProviderProfile capabilities)")
    if "| ExactRetriever |" not in ta:
        D.add("semantic documentation", "orphan component", "TA §59.6 uses ExactRetriever but the §59.1 component table does not define it")
    # Component naming identities (audit M19): aliases are allowed only
    # where the identity is stated at the definition site.
    for text, label, needle, why in (
            (bs, "BS §45.3", "`ResourceGovernor` is the process-topology name of the `ResourceIntegrityAuthority`", "ResourceGovernor must be declared the same service as ResourceIntegrityAuthority"),
            (ta, "TA §51.3", "`ResourceGovernor` is the §57.2 process-topology name of `ResourceIntegrityAuthority`", "ResourceGovernor must be declared the same service as ResourceIntegrityAuthority"),
            (ta, "TA §53.1", "It is the `IntegratedAndroidWorkflowCoordinator` of build spec §47.1 and ADR-082, listed as `AndroidWorkflowCoordinator`", "WorkflowCoordinator must be identified with IntegratedAndroidWorkflowCoordinator and AndroidWorkflowCoordinator"),
            (ta, "TA §86.5", "(the `ConversationResolver` of build spec §82.1; one component)", "ConversationContinuationResolver must be identified with BS ConversationResolver"),
            (bs, "BS §80.5.3", "#### 80.5.3 AndroidConstructionContract", "the §42.1 contract schema must carry the canonical ADR-158 name")):
        if needle not in text:
            D.add("semantic documentation", "component naming", f"{label}: {why}")
    retirement = "The earlier name `AndroidApplicationContract` is retired; it was never a separate record."
    for text, label in ((bs, "BS"), (ta, "TA"), (docs["dev"], "DP")):
        if text.replace(retirement, "").count("AndroidApplicationContract"):
            D.add("semantic documentation", "component naming",
                  f"{label} still uses the retired name AndroidApplicationContract; the canonical record is AndroidConstructionContract (ADR-158)")
    # DP ownership map: each row's parenthetical ADR must be the decision
    # that actually governs the row's mechanism (audit found ADR-049, the
    # worker registry, cited for the toolchain manifest, and ADR-068 cited
    # for lease semantics decided by ADR-067).
    dp = docs["dev"]
    own = re.search(r"### Coarse-to-refined milestone ownership map.*?(?=\nRefinement rule)", dp, re.S)
    if own:
        for label, adr, why in (("Toolchain / clean build", "ADR-163", "the Android toolchain manifest and project lock decision"),
                                ("Reservation/lease/capability", "ADR-067", "the renewable-lease and scoped-capability decision")):
            row = re.search(r"^\| " + re.escape(label) + r" \|.*$", own.group(0), re.M)
            if row is None:
                D.add("semantic documentation", "ownership map", f"row {label!r} missing from the DP ownership map")
            elif adr not in row.group(0):
                D.add("semantic documentation", "ownership map", f"row {label!r} must cite {adr} ({why})")
            if row is not None and label == "Toolchain / clean build" and "ADR-049" in row.group(0):
                D.add("semantic documentation", "ownership map", "toolchain row cites ADR-049 (worker registry), which does not govern toolchain authority")
    else:
        D.add("semantic documentation", "ownership map", "DP coarse-to-refined ownership map not found")
    # TA §76.3 specialist gates must map onto the §6.5 canonical roles.
    m763 = re.search(r"### 76\.3 .*?(?=\n### )", ta, re.S)
    m65 = re.search(r"### 6\.5 .*?(?=\n## )", ta, re.S)
    if m763 and m65:
        roles = set(re.findall(r"^\| ([A-Z][A-Za-z ]+?) \| ", m65.group(0), re.M)) - {"Worker role"}
        for gate, role_cell in re.findall(r"^\| ([^|`]+?) \| ([^|]+?) \| [^|]+ \| [^|]+ \|$", m763.group(0), re.M):
            if gate in ("Specialist gate", "---"):
                continue
            named = [r for r in roles if r in role_cell]
            if not named:
                D.add("semantic documentation", "specialist gate roles",
                      f"TA §76.3 gate {gate!r} names no §6.5 canonical worker role ({role_cell.strip()!r})")
    else:
        D.add("semantic documentation", "specialist gate roles", "TA §76.3 or §6.5 not found")
    if "| MUST separate from M7 onward |" not in bs:
        D.add("semantic documentation", "in-process hosting bound",
              "§80.2 row for BS §26.1 must resolve to 'MUST separate from M7 onward' so it agrees with §51.2")
    if "The in-process allowance ends at M7" not in adr_blocks(dec).get(111, ""):
        D.add("semantic documentation", "in-process hosting bound",
              "ADR-111 must state that the in-process allowance ends at M7")
    # AGENTS.md may not instruct agents to run a certification entry point that
    # the working tree does not contain (tools/verify.sh|.ps1 are M0 deliverables).
    agents_path = os.path.join(root, "AGENTS.md")
    if os.path.exists(agents_path):
        agents = open(agents_path, encoding="utf-8").read()
        for entry in ("tools/verify.sh", "tools/verify.ps1"):
            if entry in agents and not os.path.exists(os.path.join(root, entry)) \
                    and "do not exist yet in this documentation-only repository" not in agents:
                D.add("semantic documentation", "certification entry point",
                      f"AGENTS.md instructs running `{entry}`, which is absent from the working tree, without "
                      "stating that it is an M0 deliverable and naming the present gate (verify_contract_graph.py + harness)")
    # A capacity verdict on time exists only for a user-declared bound and must
    # say so next to the enum (TA §69.3); README may not describe a count stop.
    if "exceeds_declared_time_bound" in ta and \
            "never terminates, degrades, or blocks a goal that has no declared bound" not in ta:
        D.add("semantic documentation", "declared time bound verdict",
              "TA §69.3 defines exceeds_declared_time_bound without stating that it applies only to a "
              "user-declared bound and never stops an unbounded goal (ADR-218)")
    readme_path = os.path.join(root, "README.md")
    if os.path.exists(readme_path):
        readme = open(readme_path, encoding="utf-8").read()
        for token in ("failed three times", "three times and stops", "after three attempts"):
            if token in readme:
                D.add("semantic documentation", "README stop wording",
                      f"README says {token!r}: Nirman stops on recurring failure after materially different repairs, not on a count (ADR-218)")
        if "28 canonical command kinds" in readme:
            D.add("semantic documentation", "command registry cardinality",
                  "README states 28 canonical command kinds; §76.1 registers twenty-nine including conversation.continue")
    # Approval expiry has exactly two rules (BS §26.13); every statement of it
    # must carry both so no document reads as clock-only or context-only.
    if "A pending approval request expires in exactly two ways, whichever comes first" not in bs:
        D.add("semantic documentation", "approval expiry rules",
              "BS §26.13 must define the two approval-expiry rules (context expiry and the §80.3 clock, whichever comes first)")
    for token, label in (("Approval requests must expire under the two build spec §26.13 rules", "TA §7.3"),
                         ("or when the build spec §80.3 approval-expiry clock elapses, whichever comes first", "TA §23.6")):
        if token not in ta:
            D.add("semantic documentation", "approval expiry rules",
                  f"{label} states approval expiry without both §26.13 rules (context expiry and the §80.3 clock)")
    exp_row = re.search(r"^\| Approval expiry \| ([^|]+) \| ([^|]+) \|", bs, re.M)
    if not exp_row or exp_row.group(1).strip() != "24 hours" or exp_row.group(2).strip() != "1-168 hours":
        D.add("semantic documentation", "approval expiry rules",
              "§80.3 approval-expiry row must read default 24 hours, range 1-168 hours (the values §26.13 and TA §7.3 cite)")
    # Execution profiles: BS §26.5 is the canonical set; TA §9.1 must list the
    # identical profile names (schema-style parity for the profile tables).
    def _profile_names(text, heading, nxt):
        seg = text.split(heading, 1)[-1].split(nxt, 1)[0]
        return [m.strip() for m in re.findall(r"^\| ([^|`]+?) \| ", seg, re.M) if m.strip() not in ("Profile", "---")]
    bs_profiles = _profile_names(bs, "### 26.5 Sandbox profiles and operating-system isolation", "### 26.6")
    ta_profiles = _profile_names(ta, "### 9.1 Execution profiles", "### 9.2")
    if not bs_profiles or not ta_profiles:
        D.add("semantic documentation", "execution profile set", "BS §26.5 or TA §9.1 profile table not found")
    elif bs_profiles != ta_profiles:
        D.add("semantic documentation", "execution profile set",
              f"TA §9.1 profiles {ta_profiles} differ from the canonical BS §26.5 set {bs_profiles}")
    if "This table is the canonical execution-profile set: exactly these five profiles exist" not in bs:
        D.add("semantic documentation", "execution profile set",
              "BS §26.5 must declare itself the canonical execution-profile set")
    # Platform fixtures run on the Windows host only (BS §2, ADR-108): a
    # fixture, exit gate, or example that requires a Linux/macOS/non-Windows
    # host describes a lane Nirman cannot execute.
    for label, text in (("build spec", bs), ("architecture", ta), ("development plan", dev)):
        for token in ("host = Linux", "host_platform:             linux", "Linux host →", "on a non-Windows host",
                      "from a non-Windows host", "at least two host platforms", "macOS host"):
            if token in text:
                D.add("semantic documentation", f"non-Windows host fixture in {label}",
                      f"{token!r}: Nirman runs only on Windows; platform fixtures vary the validation environment, not the host OS (BS §79.13)")
    if "a fixture that requires a non-Windows host cannot run inside Nirman's certification lane" not in bs:
        D.add("semantic documentation", "platform fixture host rule",
              "BS §79.13 must state that every TEST-PLAT-001 fixture executes on the Windows host")
    if "\nCandidateBranch\n- branchId\n" not in ta:
        D.add("semantic documentation", "CandidateBranch schema",
              "architecture lacks the CandidateBranch field block that BS §65.2 defines (TA §88.2)")

    # CONTRACT.RUNTIME.LOCALIZATION is regression localization (BS §62,
    # ADR-147). Any sentence that makes it the authority for locales,
    # translation, or i18n resources is the homonym error this rule pins.
    i18n_claims = (
        r"CONTRACT\.RUNTIME\.LOCALIZATION`? (?:remains|is) authoritative for locale",
        r"consumes (?:the existing )?`?CONTRACT\.RUNTIME\.LOCALIZATION`? for (?:locale|translation|i18n)",
        r"LOCALIZATION owns the runtime localization mechanism",
        r"consuming existing LOCALIZATION contract",
    )
    for label, text in (("build spec", bs), ("architecture", ta), ("development plan", dev)):
        for pat in i18n_claims:
            if re.search(pat, text):
                D.add("semantic documentation", f"LOCALIZATION homonym in {label}",
                      "CONTRACT.RUNTIME.LOCALIZATION is regression localization (BS §62); "
                      "it must not be cited as the locale/translation authority")
    m164 = adr_blocks(dec).get(164, "")
    if "**Locks:** `CONTRACT.RUNTIME.LOCALIZATION`" in m164:
        D.add("semantic documentation", "ADR-164 lock",
              "ADR-164 (language adapters) must not lock the regression-localization contract")

    # Lifecycle identity (BS §26.14 / §33.2, TA §5.1 / §36.2, AGENTS.md): the
    # task-execution state set is defined once in BS §26.14 and implemented
    # name-for-name in TA §5.1; the session lifecycle is defined once in BS
    # §33.2 and restated verbatim in TA §36.2; every §5.7.2 ProductLifecycleState
    # value appears in the §33.2 mapping table; and exactly one component is
    # named as the committer.
    def _fence_after(text, heading):
        i = text.find(heading)
        if i < 0:
            return None
        m = re.search(r"```text\n(.*?)```", text[i:], re.S)
        return m.group(1) if m else None
    def _states(block):
        return set(re.findall(r"\b[A-Z][A-Z_]{3,}\b", block or ""))
    bs_task = _states(_fence_after(bs, "### 26.14 Continuous execution state machine"))
    ta_task = _states(_fence_after(ta, "### 5.1 Task state machine"))
    if not bs_task or not ta_task:
        D.add("semantic documentation", "task-execution state set",
              "BS §26.14 or TA §5.1 state-machine fence not found")
    elif bs_task != ta_task:
        D.add("semantic documentation", "task-execution state set",
              f"TA §5.1 must implement BS §26.14 name for name (BS only {sorted(bs_task - ta_task)}, "
              f"TA only {sorted(ta_task - bs_task)})")
    bs_sess = _fence_after(bs, "### 33.2 Authoritative lifecycle")
    ta_sess = _fence_after(ta, "### 36.2 Lifecycle authority")
    if not bs_sess or not ta_sess or bs_sess.strip() != ta_sess.strip():
        D.add("semantic documentation", "session lifecycle",
              "TA §36.2 must restate the BS §33.2 session lifecycle verbatim")
    pls = re.search(r"ProductLifecycleState = ((?:[A-Z_]+\s*\|\s*)+[A-Z_]+)", bs)
    sec332 = bs[bs.find("### 33.2 Authoritative lifecycle"):bs.find("### 33.3")]
    if not pls or "| §33.2 session state | §5.7.2 `ProductLifecycleState` |" not in sec332:
        D.add("semantic documentation", "lifecycle mapping",
              "BS §33.2 must carry the session-state ↔ ProductLifecycleState mapping table")
    else:
        mapped = set()
        for row in re.findall(r"^\| `[^\n]*\| ([^\n]*) \|$", sec332, re.M):
            mapped.update(re.findall(r"`([A-Z_]+)`", row))
        for value in re.findall(r"[A-Z_]+", pls.group(1)):
            if value not in mapped:
                D.add("semantic documentation", "lifecycle mapping",
                      f"ProductLifecycleState value {value} is missing from the BS §33.2 mapping table")
    # Schema parity (AGENTS.md: one canonical schema, every duplicate updated
    # together; TA §36.1: BS holds the normative shape, TA the implementation
    # schema, and the two agree field for field). For every schema name whose
    # field block appears in both documents: a CanonicalSchemaRegistry schema
    # must have identical field-name sets; any other duplicated block must
    # carry every build-spec field (TA may add persistence-only fields).
    def _field_blocks(text):
        blocks = {}
        lines = text.split("\n")
        i = 0
        while i < len(lines) - 1:
            name = lines[i].strip()
            if re.fullmatch(r"[A-Z][A-Za-z0-9]+", name) and lines[i + 1].startswith("- "):
                j, fields = i + 1, []
                while j < len(lines) and (lines[j].startswith("- ")
                                          or (lines[j][:1] in (" ", "\t") and lines[j].strip())):
                    if lines[j].startswith("- "):
                        fields.append(re.sub(r"[:(].*", "", lines[j][2:]).strip())
                    j += 1
                blocks.setdefault(name, []).append((i + 1, fields))
                i = j
            else:
                i += 1
        return blocks
    bs_blocks, ta_blocks = _field_blocks(bs), _field_blocks(ta)
    reg_match = re.search(r"```text\nCanonicalSchemaRegistry\n(.*?)```", ta, re.S)
    registry_names = set(reg_match.group(1).split()) if reg_match else set()
    for name in sorted(set(bs_blocks) & set(ta_blocks)):
        canonical = set(bs_blocks[name][0][1])
        occurrences = ([("BS", l, set(f)) for l, f in bs_blocks[name][1:]]
                       + [("TA", l, set(f)) for l, f in ta_blocks[name]])
        if name in registry_names:
            for doc, line, fs in occurrences:
                if fs != canonical:
                    D.add("semantic documentation", f"{name} schema parity",
                          f"registry schema {name} at {doc} line {line} differs from its first "
                          f"build-spec block (extra {sorted(fs - canonical)}, missing {sorted(canonical - fs)})")
                    break
        else:
            for _, line, fs in occurrences:
                missing = canonical - fs
                if missing:
                    D.add("semantic documentation", f"{name} schema parity",
                          f"TA line {line} block for {name} lacks build-spec fields {sorted(missing)}")

    committer_rules = (
        ("Exactly one component commits transitions in either set: `LifecycleAuthority`, which is the pure session reducer", bs, "BS §33.2"),
        ("The only committer of a lifecycle transition is `LifecycleAuthority`, the `SessionReducer` of §45.1", ta, "TA §58.2"),
        ("1. Only `LifecycleAuthority` (the `SessionReducer`, §45.1) commits lifecycle state; `AgentLoopReducer` proposes.", ta, "TA §58.15"),
    )
    for token, text, where in committer_rules:
        if token not in text:
            D.add("semantic documentation", "lifecycle committer",
                  f"{where} must name LifecycleAuthority/SessionReducer as the single lifecycle committer")
    for bad in ("Only `AgentLoopReducer` may commit", "Only the reducer commits lifecycle state."):
        if bad in ta:
            D.add("semantic documentation", "lifecycle committer",
                  f"architecture reintroduces an ambiguous or second committer: {bad!r}")

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
                    "IntegrationState", "SigningState", "DeliveryState", "CompletionState")
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
        "ResourceIntegrityRecord": bs + ta,
        "AgentTrustAssessment": bs + ta,
        "ContextCachePolicy": bs + ta,
        "AndroidRuntimeIntegrityObservation": bs + ta,
        "CAP.ANDROID.RESOURCE_AWARE_AUTONOMY": bs,
        "CAP.ANDROID.TRUSTED_EXTENSIONS": bs,
        "CAP.ANDROID.CONTEXT_GOVERNANCE": bs,
        "CAP.ANDROID.RUNTIME_INTEGRITY": bs,
        "TEST-RESOURCE-001": bs + dev,
        "EV-RESOURCE-001": bs + dev,
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
        "ADR-218": dec,
        "ADR-198": dec,
        "ADR-199": dec,
        "ADR-200": dec,
        "FrontendControlPlaneContract": bs + ta,
        "UICommandRegistry": bs + ta,
        "UICommandEnvelope": bs + ta,
        "ProjectionSnapshot": bs + ta,
        "UIResponseEnvelope": bs + ta,
        "UIErrorEnvelope": bs + ta,
        "EventSubscription": bs + ta,
        "CAP.ANDROID.FRONTEND_CONTROL_PLANE": bs,
        "TEST-FCP-001": bs + dev,
        "EV-FCP-001": bs + dev,
        "M115": dev,
        "ADR-201": dec,
        # ADR-219: attention reliability is measured, placed against, gated, and
        # verified independently of model recall. The schema and its vocabulary
        # must exist on every canonical surface that the decision names.
        "AttentionReliabilityProfile\n- profileId\n- providerProfileId": bs,
        "AttentionReliabilityProfile\n- profileId: string": ta,
        "reliableLiteralSpanTokens": bs + ta,
        "placementPlan": bs + ta + dev,
        "attendabilityMap": bs + ta,
        "recallProbes": bs,
        "attentionReliability": bs + ta,
        "PREMISE_MISMATCH": bs + ta + dev,
        "PlacementPlanner": ta + dev,
        "RecallProbeService": ta + dev,
        "AttentionProfiler": ta + dev,
        "CLAUSE.CONTEXT.ATTENDABILITY_REQUIRED": bs,
        "CLAUSE.CONTEXT.RECALL_EVIDENCE_ONLY": bs,
        "ADR-219": dec,
        "ResourceExecutionProfile\n- planRevision": ta,
        "`ResourceExecutionProfile` (TA §69.3)": bs,
        "ResourceExecutionProfile with honest confidence": dev,
        "recovery-attempt policy (`recoveryAttemptPolicy`) bounds": bs,
        "The recovery-attempt policy (`recoveryAttemptPolicy`) is policy-configurable and bounded": ta,
        "recovery-attempt policies (`recoveryAttemptPolicy`)": dev,
        "**Execution suitability**": bs,
        "maxReasoningTokens: integer? (provider capability metadata only": bs,
        "`maxReasoningTokens` is provider capability metadata": ta,
        "The authoritative Task Ledger is the SQLite execution ledger owned by `NirmanSupervisor.exe`": bs,
        "### 56.1 Asset execution under the canonical UI Worker": ta,
        "compatible cloud-provider requests": dev,
        "scoped asset transaction executed by the canonical UI Worker": dev,
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
        ("## 72. Runtime Resource Integrity Authority", bs, "resource integrity authority"),
        ("## 73. Agent Trust Boundary Authority", bs, "agent trust authority"),
        ("## 74. Context and Cache Governance", bs, "context and cache authority"),
        ("## 75. Android Runtime Integrity Contract", bs, "Android runtime integrity authority"),
        ("## 77. Runtime Resource Integrity Implementation Contract", ta, "resource integrity architecture"),
        ("## 78. Agent Trust Boundary Implementation Contract", ta, "agent trust architecture"),
        ("## 79. Context and Cache Governance Implementation Contract", ta, "context and cache architecture"),
        ("## 80. Android Runtime Integrity Implementation Contract", ta, "Android runtime integrity architecture"),
        ("## M111 — Runtime resource integrity and adaptive execution", dev, "resource integrity milestone"),
        ("## M112 — Agent-layer trust boundary and extension security", dev, "agent trust milestone"),
        ("## M113 — Context compaction and cache governance", dev, "context governance milestone"),
        ("## M114 — Android runtime integrity and honest coverage", dev, "Android integrity milestone"),
        ("## ADR-197: Make cost governance a deterministic resource authority", dec, "superseded cost governance decision (retained for history)"),
        ("## ADR-218: AI usage telemetry is observational and has no execution-authority semantics", dec, "resource integrity decision"),
        ("## ADR-198: Scan and revoke agent-layer extension content", dec, "agent trust decision"),
        ("## ADR-199: Govern context compaction and provider cache reuse", dec, "context governance decision"),
        ("## ADR-200: Report Android runtime integrity as independent applicable signals", dec, "Android integrity decision"),
        ("### 53.11 Attention reliability, placement, and recall verification", bs, "attention reliability authority"),
        ("### 59.12 Recall probes, placement bounds, and attention learning", ta, "attention reliability architecture"),
        ("## ADR-219: Attention reliability is measured per model and context is placed, gated, and verified against it", dec, "attention reliability decision"),
    )
    for anchor, text, subject in governance_sections:
        if anchor not in text:
            D.add("semantic documentation", subject, f"canonical governance anchor is missing: {anchor}")
    governance_tokens = (
        ("resourceRequirements", bs + ta, "physical resource requirements schema"),
        ("pressureResponse", bs + ta, "resource pressure response"),
        ("BLOCKED_NO_SAFE_PATH", bs + ta, "physical exhaustion blocks only without a safe path"),
        ("EvidenceAcquisitionTrigger", bs + ta, "observation-free pass as evidence trigger"),
        ("RepeatedFailureDetector", bs + ta, "anti-thrash repeated-failure detector"),
        ("StrategyChangeRequired", bs + ta, "anti-thrash strategy-change signal"),
        ("ContextCapacityPlanner", ta, "provider context capacity planner"),
        ("staticFindings", bs + ta, "agent trust scan findings"),
        ("revocationState", bs + ta, "agent trust revocation"),
        ("cacheBreakpointPolicy", bs + ta, "cache breakpoint policy"),
        ("cacheInvalidationEvents", bs + ta, "cache invalidation events"),
        ("playIntegrityApplicability", bs + ta, "Play Integrity applicability"),
        ("anrEvidenceIds", bs + ta, "ANR evidence"),
        ("batteryObservationIds", bs + ta, "battery observation"),
        ("dozeObservationIds", bs + ta, "Doze observation"),
        ("`ResourceIntegrityAuthority` (§59) evaluates `resourceRequirements` against currently admissible physical capacity before admission", ta, "resource integrity authority implementation"),
        ("Scanners run in a restricted local process", ta, "trust scanner implementation"),
        ("`ContextGovernance` records selected content", ta, "context governance implementation"),
        ("Runtime collectors observe;", ta, "runtime integrity authority implementation"),
        ("Autonomy-level capability ladder", bs, "autonomy ladder"),
        ("**Compaction.** Compaction output is never the carrier of active constraints, locked decisions, acceptance criteria, or revision identity.", bs, "compaction never carries constraints (§53)"),
        ("Compaction output is never the carrier of active constraints, locked decisions, acceptance criteria, or revision identity: after every compaction they are re-projected from durable state into the DENSE block and verified by a recall probe", bs, "compaction never carries constraints (§74)"),
        ("Post-compaction constraint re-projection verified by a recall probe", dev, "post-compaction probe fixture in M113"),
        ("The `cacheBreakpointPolicy` of §74 places the cache breakpoint before the DENSE block", bs, "cache breakpoint precedes DENSE block"),
        ("evaluates context sufficiency across seven dimensions", ta, "seven confidence dimensions"),
        ("premise check: StructuredPatch anchors and premises match the originating ContextPackage", ta, "premise check gate rung"),
        ("13. Positional literal recall across fill buckets", ta, "positional recall provider fixture"),
        ("14. Post-compaction constraint retention", ta, "post-compaction retention provider fixture"),
        ("a model's statement about what it remembers is inadmissible", bs, "recall self-report inadmissible"),
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
    protocol_sections = (
        ("## 76. Frontend–Control-Plane Protocol Contract", bs, "frontend-control-plane authority"),
        ("## 81. Frontend–Control-Plane Protocol Implementation Contract", ta, "frontend-control-plane architecture"),
        ("## M115 — Frontend–control-plane protocol and generated service adapter", dev, "frontend-control-plane milestone"),
        ("## ADR-201: Make the frontend a typed projection client of the control plane", dec, "frontend-control-plane decision"),
    )
    for anchor, text, subject in protocol_sections:
        if anchor not in text:
            D.add("semantic documentation", subject, f"canonical protocol anchor is missing: {anchor}")
    protocol_tokens = (
        ("### 76.1 UICommandRegistry", bs, "command registry"),
        ("### 76.2 Response and error envelopes", bs, "response and error envelopes"),
        ("UIErrorEnvelope\n- errorId", bs, "error envelope schema"),
        ("### 76.3 Subscription, replay, and snapshot cutover", bs, "event subscription"),
        ("snapshot cutover", bs, "snapshot cutover"),
        ("backpressure", bs, "event backpressure"),
        ("Snapshot-plus-event replay is cursor-atomic", ta, "architecture snapshot cutover"),
        ("AuthoritativeProjectionState", bs + ta, "frontend authoritative projection state"),
        ("OptimisticInputState", bs + ta, "frontend optimistic input state"),
        ("AndroidServiceIntegration", bs + ta, "generated Android service adapter"),
        ("TEST-FCP-001", bs + dev, "frontend-control-plane test identity"),
        ("EV-FCP-001", bs + dev, "frontend-control-plane evidence identity"),
    )
    for token, text, subject in protocol_tokens:
        if token not in text:
            D.add("semantic documentation", subject, f"frontend-control-plane requirement is missing: {token}")
    continuity_tokens = (
        ("## 77. Background Continuity Contract", bs, "background continuity authority"),
        ("## 82. Background Continuity Implementation Contract", ta, "background continuity architecture"),
        ("## M116 — Background continuity and interruption recovery", dev, "background continuity milestone"),
        ("## ADR-202: Canonical background continuity state machine", dec, "background continuity decision"),
        ("BackgroundContinuityRecord", bs + ta, "background continuity schema"),
        ("ContinuityDimensions", bs + ta, "continuity dimensions schema"),
        ("ACTIVE_BACKGROUND | UI_DISCONNECTED | HOST_SUSPENDED", bs + ta, "background continuity state vocabulary"),
        ("unknown outcome", bs + ta, "background continuity reconciliation"),
        ("backgroundContinuityProjection", bs + ta, "background continuity projection wiring"),
    )
    for token, text, subject in continuity_tokens:
        if token not in text:
            D.add("semantic documentation", subject, f"background-continuity requirement is missing: {token}")
    export_tokens = (
        ("## 78. APK Export Provenance Contract", bs, "APK export authority"),
        ("## 83. APK Export Provenance Implementation Contract", ta, "APK export architecture"),
        ("## M117 — Local APK export provenance and delivery admission", dev, "APK export milestone"),
        ("## ADR-203: Make local deployment export profile-bound and provenance-complete", dec, "APK export decision"),
        ("CAP.ANDROID.APK_DELIVERY", bs, "APK delivery capability"),
        ("deploymentDelivery: REQUIRED_APK | DECLARED_AAB_OPTIONAL | SOURCE_ACCESS_ONLY", bs + ta, "deployment delivery distinction"),
        ("destinationKind: LOCAL_WINDOWS_FILESYSTEM | USER_APPROVED_SOURCE_LOCATION", bs + ta, "deployment destination policy"),
        ("signingIdentityBindingId", bs + ta, "export signing lineage"),
        ("ExportVerificationRecord\n- exportId", ta, "export verification schema"),
        ("APKExportRecord", ta, "APK export implementation view"),
        ("`APKExportRecord` is not a registered schema: it is the read-model view of `ExportVerificationRecord`", ta, "APK export view identity"),
        ("deliveryProjection", ta, "export delivery projection wiring"),
        # Anchored to the §83.2 export-copy sentence, not the bare lifecycle
        # token: ADR-203's ExternalEffectRecord generalization (§20.3) also
        # quotes `UNKNOWN → RECONCILING`, so a bare-token check would pass
        # even if the export-specific lifecycle wording were removed.
        ("partially completed follows `UNKNOWN → RECONCILING`", ta, "export reconciliation lifecycle"),
    )
    for token, text, subject in export_tokens:
        if token not in text:
            D.add("semantic documentation", subject, f"APK-export requirement is missing: {token}")

    change_tokens = (
        ("ChangeImpactReport\n- reportId", bs + ta, "change impact report schema"),
        ("ChangeIntelligenceStore", ta, "change intelligence store implementation"),
        ("recommendationBasis", bs + ta, "change recommendation basis"),
        ("recommendationSource", bs + ta, "change recommendation source"),
        ("1. ConstructionTransaction (authoritative for mutation identity, files, revision)", ta, "change impact transaction provenance"),
        ("2. ImpactAnalysis (authoritative for affected surface graph)", ta, "change impact analysis provenance"),
        ("3. ValidationResult (authoritative for verification/test results)", ta, "change impact validation provenance"),
        ("4. PreviewRevision (authoritative for preview impact/currentness)", ta, "change impact preview provenance"),
        ("5. EvidenceAuthority (authoritative for evidence validity/invalidation)", ta, "change impact evidence provenance"),
        ("6. RecoveryAuthority (authoritative for recovery actions)", ta, "change impact recovery provenance"),
    )
    for token, text, subject in change_tokens:
        if token not in text:
            D.add("semantic documentation", subject, f"ChangeImpactReport provenance or requirement is missing: {token}")

    schema_parity_tokens = (
        ("ContentDependency\n- dependencyId\n- contentId\n- dependencyType\n- dependencyIdentity\n- dependencyRevision\n- invalidationPolicy", bs, "BS ContentDependency schema"),
        ("ContentDependency\n- dependencyId\n- contentId\n- dependencyType\n- dependencyIdentity\n- dependencyRevision\n- invalidationPolicy", ta, "TA ContentDependency schema"),
        ("ContentRevision\n- contentRevisionId\n- contentId\n- projectRevisionId\n- requirementIds", bs, "BS ContentRevision requirementIds"),
        ("ContentRevision\n- contentRevisionId\n- contentId\n- projectRevisionId\n- requirementIds", ta, "TA ContentRevision requirementIds"),
        ("MATCH → CONTINUE", bs, "BS Continue state transition"),
        ("ChangeReportRecord\n- recordId", bs, "BS ChangeReportRecord schema"),
        ("ChangeReportRecord\n- recordId", ta, "TA ChangeReportRecord schema"),
        ("COMPLETE → INCOMPLETE", ta, "TA invalid status transition text"),
        ("MutationReportUnit = committed ConstructionTransaction", bs, "BS MutationReportUnit"),
        ("MutationReportUnit = committed ConstructionTransaction", ta, "TA MutationReportUnit"),
    )
    for token, text, subject in schema_parity_tokens:
        if token not in text:
            D.add("semantic documentation", subject, f"schema parity requirement is missing: {token}")

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
    profile_block = re.search(r"\nAndroidCapabilityProfile\n((?:- .*\n)+)", bs)
    profile_fields = profile_block.group(1) if profile_block else ""
    if "- reproducibilityLevel\n" not in profile_fields or "- repositoryTrustRequirement\n" not in profile_fields:
        D.add("semantic documentation", "profile maturity fields",
              "AndroidCapabilityProfile (BS §5.7.1) is missing the reproducibilityLevel or repositoryTrustRequirement field")
    if "attributionStatus" not in ta:
        D.add("semantic documentation", "resource attribution",
              "resource usage lacks explicit parent/child/shared attribution")

    # ADR-218: AI usage is telemetry only. No canonical document may carry an
    # AI-usage budget, reservation, exhaustion outcome, or fixed pass ceiling
    # with execution-authority semantics. The decision log is exempt because it
    # retains superseded/amended historical text by design.
    banned_execution_controls = (
        "BUDGET_EXHAUSTED", "maxToollessPasses", "DeliberationBudget",
        "DeliberationBudgetManager", "CostGovernanceRecord", "CostAuthority",
        "budgetReservationId", "remainingBudget", "tokenBudget", "requestBudget",
        "durationBudget", "costCap", "exhaustionOutcome", "resourceBudget",
        "timeBudget", "CONTRACT.RUNTIME.COST_GOVERNANCE",
        "CLAUSE.COST.EXHAUSTION_EXPLICIT", "CLAUSE.DELIBERATE.RUNTIME_GRANTS_BUDGET",
        "CAP.ANDROID.BUDGETED_AUTONOMY", "TEST-COST-001", "EV-COST-001",
        "Deliberation max passes", "maxPasses", "max_passes",
        "PlanCostEstimate", "ContextBudgetAllocator", "mutation budget",
        "mutation-budget", "retry budget", "Retry budget", "Cost efficiency",
        "lower-cost model",
        # count- or clock-driven stop wording (ADR-218: repetition feeds the
        # recovery ladder; only a user-declared time bound may raise a verdict)
        "occurs three times consecutively, the execution loop is instantly suspended",
        "stop after the configured retry limit", "exceeds_time |", "| exceeds_time",
        "bounded retry limits",
    )
    for label, text in (("build spec", bs), ("architecture", ta), ("development plan", dev)):
        for token in banned_execution_controls:
            if token in text:
                D.add("semantic documentation", f"AI-usage budget vocabulary in {label}",
                      f"{token!r} reintroduces an AI-usage budget as an execution control (ADR-218, BS §72)")
    # Canonical schema identity (TA §36.1): every schema a contract section
    # calls canonical must be registered, the Android capability profile has
    # one name, and the change-report revision/status fields are unambiguous.
    registry_block = re.search(r"```text\nCanonicalSchemaRegistry\n(.*?)```", ta, re.S)
    registered = set(registry_block.group(1).split()) if registry_block else set()
    if not registry_block:
        D.add("semantic documentation", "CanonicalSchemaRegistry",
              "TA §36.1 CanonicalSchemaRegistry block is missing")
    for m in re.finditer(r"^Canonical schemas: (.+)$", ta, re.M):
        for name in re.findall(r"`([A-Za-z]+)`", m.group(1)):
            if name not in registered:
                D.add("semantic documentation", "CanonicalSchemaRegistry",
                      f"{name} is called canonical in the architecture but is not listed in TA §36.1")
            if f"\n{name}\n- " not in ta:
                D.add("semantic documentation", "canonical schema definition",
                      f"{name} is called canonical but has no field block in the architecture")
    for name in ("ContentMutation", "ConversationRebaseRecord", "AndroidCapabilityProfile",
                 "ChangeReportRecord", "ChangeImpactReport", "ExportVerificationRecord",
                 "PackagingProfile", "SkillPackage", "SkillInvocationRecord", "SkillAdmission"):
        if name not in registered:
            D.add("semantic documentation", "CanonicalSchemaRegistry",
                  f"{name} must be listed in TA §36.1")
    if "APKExportRecord" in registered:
        D.add("semantic documentation", "CanonicalSchemaRegistry",
              "APKExportRecord is a view of ExportVerificationRecord (BS §78, TA §74.3) and must not be registered as a schema")
    for name in ("SkillInvocationRecord", "SkillAdmission"):
        if f"\n{name}\n- " not in ta:
            D.add("semantic documentation", "canonical schema definition",
                  f"{name} is a registered M119 ledger record but has no field block in TA §19.1")
    if "ReproducibilityLevel  = " not in bs:
        D.add("semantic documentation", "canonical schema definition",
              "BS §5.7.2 must define the ReproducibilityLevel value set that TA §36.4 names as a separate field")
    if "\nCapabilityProfile\n" in ta or "\nCapabilityProfile\n" in bs:
        D.add("semantic documentation", "capability profile identity",
              "bare 'CapabilityProfile' schema name; the Android capability profile is AndroidCapabilityProfile (BS §5.7.1)")
    for label, text in (("build spec", bs), ("architecture", ta), ("development plan", dev)):
        if "reportStatus" in text:
            D.add("semantic documentation", f"change report status in {label}",
                  "ChangeImpactReport carries projectionStatus; ChangeReportRecord.status is the only lifecycle state")
        if "ChangeReportRecord\n- recordId\n- transactionId\n- projectRevision\n" in text:
            D.add("semantic documentation", f"change report revision in {label}",
                  "ChangeReportRecord must carry projectRevisionAfter, not an ambiguous projectRevision")
    for label, text in (("build spec", bs), ("architecture", ta)):
        for schema in ("ConversationRequirement", "ConversationDecision"):
            block = re.search(rf"\n{schema}\n((?:- .*\n)+)", text)
            if not block or "- sourceEvidenceIds\n" not in block.group(1):
                D.add("semantic documentation", f"{schema} evidence provenance in {label}",
                      f"{schema} must carry sourceEvidenceIds (BS §82: requirements and decisions reference evidence)")
    if "Forward traversal proves that every capability is implemented" in bs:
        D.add("semantic documentation", "documentation certification claim",
              "BS §67.9 must not claim forward traversal proves runtime implementation (§67.6)")
    if "M115 command envelope" in dev:
        D.add("semantic documentation", "M5 sequencing",
              "M5 must reference the canonical command envelope contract, not depend on the M115 milestone")
    # Registry semantics (TA §36.1): one canonical identity per schema, BS
    # holds the normative contract shape, TA the implementation schema, and
    # for the content / conversation / change-intelligence families the two
    # must agree field for field.
    if "| schemaId (normative contract; implementation schema) |" not in ta or "(canonical definition)" in ta:
        D.add("semantic documentation", "CanonicalSchemaRegistry metadata",
              "TA §36.1 metadata must cite a normative contract (BS) and an implementation schema (TA), not a 'canonical definition' in two places")
    def _fields(text, name):
        m = re.search(rf"\n{name}\n((?:- .*\n)+)", text)
        return [re.sub(r":.*", "", ln[2:]).strip() for ln in m.group(1).splitlines()] if m else None
    for name in registered:
        if not name.startswith(("Content", "Conversation", "ChangeReport", "ChangeImpact", "TerminologyProfile")):
            continue
        fb, ft = _fields(bs, name), _fields(ta, name)
        if fb and ft and fb != ft:
            D.add("semantic documentation", f"{name} field parity",
                  f"BS and TA field lists differ (BS only {sorted(set(fb) - set(ft))}, TA only {sorted(set(ft) - set(fb))}, or order)")
    if not _fields(ta, "Content") or "- currentRevisionId" not in ta:
        D.add("semantic documentation", "Content schema",
              "TA §85.1 must define the persisted Content record (BS §81.1)")
    draft = _fields(ta, "ContentRevisionDraft")
    mutation = _fields(ta, "ContentMutation")
    if not draft or not mutation or "proposedContentRevision" not in mutation or "contentRevision" in mutation:
        D.add("semantic documentation", "ContentMutation proposal type",
              "ContentMutation must carry proposedContentRevision: ContentRevisionDraft, never an admitted ContentRevision")
    if draft and set(draft) & {"contentRevisionId", "transactionId", "validationStatus", "approvalState", "sourceEvidenceIds"}:
        D.add("semantic documentation", "ContentRevisionDraft",
              "a draft must not carry authoritative ContentRevision fields")
    for token in ("`conversationRevision` is incremented only when the authoritative `ConversationResolver` commits",
                  "A `RECONCILE/REBASE` changes `expectedProjectRevision` only after the rebase decision",
                  "A `USER_REQUIRED` outcome does not advance `expectedProjectRevision`",
                  "exposes exactly one durable `ChangeReportRecord`",
                  "The owning task may claim completion only when the record is `COMPLETE`"):
        if token not in bs:
            D.add("semantic documentation", "conversation/change lifecycle rule",
                  f"build spec lacks the required rule: {token}")
    if "exposes a complete, valid report" in bs:
        D.add("semantic documentation", "BS §83.4",
              "acceptance must not require a complete report for every committed transaction; INCOMPLETE/UNRESOLVED are permitted states")
    for label, text in (("build spec", bs), ("architecture", ta)):
        rpt = _fields(text, "ChangeImpactReport") or []
        if "causeType" not in rpt or "causeId" not in rpt:
            D.add("semantic documentation", f"change causal provenance in {label}",
                  "ChangeImpactReport must carry the typed causal source causeType/causeId from which why is projected")
    for label, text in (("build spec", bs), ("architecture", ta), ("development plan", dev)):
        if "authoritative persistence implementation" in text:
            D.add("semantic documentation", f"ContentStore authority in {label}",
                  "ContentStore is the canonical persistence implementation; ContentAuthority owns admission and lifecycle")
    if "- Content schema (persisted logical content resource" not in dev:
        D.add("semantic documentation", "M120 deliverables", "M120 must deliver the Content schema")
    # Crash-safety and consistency rules (TA §87.5/§87.6, §86.2; BS §80.9).
    for token, text, subject in (
        ("MUST become durable atomically", bs, "change-report atomicity (BS §83.2)"),
        ("create exactly one `INCOMPLETE` record idempotently", bs, "change-report recovery idempotence (BS §83.2)"),
        ("in the same SQLite transaction that commits the parent `ConstructionTransaction`", ta, "change-report atomicity (TA §87.5)"),
        ("`transactionId` is unique in `ChangeIntelligenceStore`", ta, "change-report uniqueness (TA §87.5)"),
        ("5. Crash between parent commit and projection", ta, "change-report crash recovery (TA §87.6)"),
        ("MUST commit those related records atomically in one SQLite transaction", ta, "atomic Continue resolution (TA §86.2)"),
        ("A partially committed Continue resolution is invalid", ta, "partial Continue recovery (TA §86.2)"),
        ("L. crash immediately after parent commit and before projection", dev, "TEST-CHANGE-001 crash fixture"),
        ("L. crash during Continue resolution after one durable record is written", dev, "TEST-CONV-001 crash fixture"),
    ):
        if token not in text:
            D.add("semantic documentation", subject, f"required rule is missing: {token}")
    if "ChangeReportStatus" in dev or "ChangeReportStatus" in bs or "ChangeReportStatus" in ta:
        D.add("semantic documentation", "change report status name",
              "the lifecycle field is ChangeReportRecord.status (INCOMPLETE | COMPLETE | UNRESOLVED); no ChangeReportStatus type exists")
    if "this criterion\n   is NOT yet satisfied" in bs or "is NOT yet satisfied" in bs:
        D.add("semantic documentation", "BS §80.9 criterion 1",
              "§80.10 records 100% coverage; criterion 1 must not simultaneously claim it is unsatisfied")
    # Certification status vocabulary (BS §67.11): the three terminal values must
    # be defined, and the retired unqualified "PASS (WITH SKIPS)" wording must not
    # be reintroduced anywhere it could be read as complete certification.
    for token, subject in (
        ("| `CERTIFICATION: DOCUMENTATION_CERTIFIED_WITH_RUNTIME_SOURCE_SKIPS` |", "BS §67.11 with-skips status row"),
        ("| `CERTIFICATION: DOCUMENTATION_CERTIFIED` |", "BS §67.11 unqualified status row"),
        ("| `CERTIFICATION: FAIL` |", "BS §67.11 fail status row"),
        ("Exit code 0 means zero defects; it does not by itself mean every check was evaluated.", "BS §67.11 exit-code semantics"),
    ):
        if token not in bs:
            D.add("semantic documentation", subject, f"required rule is missing: {token}")
    if "DOCUMENTATION_CERTIFIED_WITH_RUNTIME_SOURCE_SKIPS" not in dev:
        D.add("semantic documentation", "DP M93 status semantics",
              "M93 must state that runs without crates/ source carry the with-skips status and are not complete evaluation")
    for label, text in (("build spec", bs), ("technical architecture", ta), ("development plan", dev), ("decisions", dec)):
        if "PASS (WITH SKIPS)" in text:
            D.add("semantic documentation", "retired certification status",
                  f"'PASS (WITH SKIPS)' in {label}: the terminal status is DOCUMENTATION_CERTIFIED_WITH_RUNTIME_SOURCE_SKIPS (BS §67.11)")
    # Integration operationality gate (audit follow-up): the TA §74.1
    # `AndroidServiceIntegration.requiredOperationality` field is typed as the
    # BS §5.7.2 IntegrationState and both BS §70 and TA §74.1 state the gate
    # (aggregateState must meet requiredOperationality); BS §5.7.5 owns the
    # comparison order.
    m_asi = re.search(r"\nAndroidServiceIntegration\n((?:- .*\n|[ \t]+.*\n)+)", ta)
    m_asi_fields = m_asi.group(1) if m_asi else ""
    if not re.search(r"^- requiredOperationality: IntegrationState\b", m_asi_fields, re.M):
        D.add("semantic documentation", "integration operationality gate",
              "TA §74.1 AndroidServiceIntegration.requiredOperationality must be typed as IntegrationState (BS §5.7.2)")
    m_ta741 = ta[ta.find("### 74.1 Android service integration"):ta.find("### 74.2", max(ta.find("### 74.1"), 0))]
    if "meets or exceeds `requiredOperationality`" not in m_ta741 or "`IntegrationOperationality.aggregateState`" not in m_ta741:
        D.add("semantic documentation", "integration operationality gate",
              "TA §74.1 must state that IntegrationOperationality.aggregateState meets or exceeds requiredOperationality")
    m_bs70 = bs[bs.find("## 70. Integration Boundary Contract"):bs.find("## 71. ", max(bs.find("## 70. Integration Boundary Contract"), 0))]
    if "meets the integration's declared `requiredOperationality`" not in m_bs70:
        D.add("semantic documentation", "integration operationality gate",
              "BS §70 must gate a service-integration boundary on IntegrationOperationality.aggregateState meeting requiredOperationality")
    m_bs575 = bs[bs.find("### 5.7.5 Required integration operationality"):bs.find("### 5.7.6", max(bs.find("### 5.7.5"), 0))]
    if "`CONFIGURED` < `REACHABLE` < `FUNCTIONAL`" not in m_bs575:
        D.add("semantic documentation", "integration operationality gate",
              "BS §5.7.5 must define the requiredOperationality comparison order (CONFIGURED < REACHABLE < FUNCTIONAL)")
    # AGENTS §8 canonical command chain and TA §81.2 restatement must have the
    # same number of steps and the same anchor concepts in the same order, so
    # the two cannot drift apart (an agent reads AGENTS.md; an implementer
    # reads the architecture).
    # Checkpoint identity (audit: the BS §11.5 record was a 7-field stub while
    # fifteen fields across both documents reference checkpoints by id and
    # the recovery architecture needs validity, known-good and restore
    # semantics). The canonical record must be registered, carry the
    # identity/validity fields, and TA §18's two tiers must declare
    # themselves projections of it.
    m_ckpt = re.search(r"\nCheckpoint\n((?:- .*\n)+)", bs)
    ckpt_fields = set(re.sub(r"[:(].*", "", l[2:]).strip() for l in m_ckpt.group(1).splitlines()) if m_ckpt else set()
    ckpt_required = {"tier", "parentCheckpointId", "workspaceId", "revisionReference", "sourceFingerprint",
                     "restoreReference", "validity", "knownGood", "retentionClass", "evidenceIds"}
    if not ckpt_required <= ckpt_fields:
        D.add("semantic documentation", "checkpoint identity",
              f"BS §11.5 Checkpoint must carry {sorted(ckpt_required - ckpt_fields)}")
    if "Checkpoint" not in registry_names:
        D.add("semantic documentation", "checkpoint identity",
              "Checkpoint must be listed in the TA §36.1 CanonicalSchemaRegistry")
    if "Both tiers are stored as the canonical `Checkpoint` record of build spec §11.5" not in ta:
        D.add("semantic documentation", "checkpoint identity",
              "TA §18 must declare FileCheckpoint and TaskCheckpoint as projections of the canonical Checkpoint record")
    # EvidenceRecord ⊇ BS §5.7.4 (audit: the TA §23.3 record carried only the
    # twelve observation fields while §5.7.4 mandates identity/dependency
    # fields on every evidence node, and the cross-entity invalidation rule
    # cannot run on a record that lacks them).
    ev_fields = set((ta_blocks.get("EvidenceRecord") or [(0, [])])[0][1])
    ev_required = {"sourceEventId", "operationId", "sessionId", "projectRevision", "checkpointId",
                   "toolchainLockId", "environmentIdentityId", "validationPolicyVersion", "freshnessInterval",
                   "dependencyIds", "supersedes", "supersededBy", "invalidationReason"}
    if not ev_required <= ev_fields:
        D.add("semantic documentation", "evidence identity",
              f"TA §23.3 EvidenceRecord must carry the BS §5.7.4 identity fields {sorted(ev_required - ev_fields)}")
    if "these are fields of the canonical `EvidenceRecord` (technical architecture §23.3)" not in bs:
        D.add("semantic documentation", "evidence identity",
              "BS §5.7.4 must bind the evidence-node requirements to the canonical EvidenceRecord fields")
    # Development-plan truthfulness: a milestone work item may state an
    # obligation or record history, but it must not claim that code "now
    # exposes" or "was added" — nothing implemented survives in this
    # documentation-only tree, and a present-tense implementation claim is
    # exactly the "agent claims as evidence" failure the certification model
    # forbids. Scoped to numbered/bulleted work items in the plan.
    dp_claim = re.compile(r"\b(?:now|already) (?:exposes|implements|supports|enforces|provides|includes|ships)\b"
                          r"|\b(?:was|were|has been|have been) (?:added|implemented|wired|shipped|landed)\b", re.I)
    for ln_no, ln in enumerate(dev.split("\n"), 1):
        if re.match(r"^\s*(?:\d+\.|[-*])\s", ln) and dp_claim.search(ln) \
                and "source no longer present" not in ln:
            D.add("semantic documentation", "plan implementation claim",
                  f"development plan line {ln_no} states a present-tense implementation claim "
                  f"({dp_claim.search(ln).group(0)!r}); record it as history or as an obligation")
    # Completion outcome vocabulary: BS §5.7.2 defines CompletionState (the
    # value set of CompletionDecision, containing the NOT_COMPLETE outcome that
    # §5.7.7's CERTIFICATION ≠ COMPLETION rule relies on) and the session
    # record's completionState field is typed with it in both documents.
    m_cs = re.search(r"^CompletionState\s*= ((?:[A-Z_]+\s*\|\s*)+[A-Z_]+)", bs, re.M)
    cs_values = set(re.findall(r"[A-Z_]+", m_cs.group(1))) if m_cs else set()
    if not {"NOT_COMPLETE", "COMPLETED", "INVALIDATED"} <= cs_values:
        D.add("semantic documentation", "completion state vocabulary",
              "BS §5.7.2 must define CompletionState with at least NOT_COMPLETE, COMPLETED and INVALIDATED")
    if "- completionState: CompletionState (§5.7.2)" not in bs or \
            "- completionState: CompletionState (build spec §5.7.2)" not in ta:
        D.add("semantic documentation", "completion state vocabulary",
              "AutonomousAndroidSession.completionState must be typed as CompletionState in BS §29.2 and TA §34")
    if "The outcome is recorded as a `CompletionState`" not in ta:
        D.add("semantic documentation", "completion state vocabulary",
              "TA §36.4 must record the completion outcome as a CompletionState on CompletionDecision")
    # The TA side is checked unconditionally; the AGENTS side only when the
    # file is present (spec-only fixtures may omit it).
    chain_anchors = ("typed ipc client", "envelope", "supervisor", "command registry", "application use case",
                     "authority checks", "sqlite transaction", "event store", "projection projector", "projection ?snapshot")
    chains = [("TA §81.2", re.search(r"### 81\.2 Command-to-domain wiring.*?```text\n(.*?)```", ta, re.S))]
    if os.path.exists(agents_path):
        agents_text = open(agents_path, encoding="utf-8").read()
        chains.append(("AGENTS.md §8", re.search(r"The canonical wiring is:\n\n```text\n(.*?)```", agents_text, re.S)))
    for label, m in chains:
        steps = [s.strip() for s in m.group(1).strip().split("\n")] if m else []
        arrows = [s for s in steps if s.startswith("→")]
        if len(arrows) != 10 or any(not re.search(a, arrows[i], re.I) for i, a in enumerate(chain_anchors)):
            D.add("semantic documentation", "command chain",
                  f"{label} canonical command chain must list the ten steps in order: {', '.join(a.split('|')[0] for a in chain_anchors)}")
    # Schema-parity obligation must be stated normatively in BS §67.11, not
    # only enforced here (audit residual): the build spec names the registry
    # and the identical-field-set rule.
    m22_6711 = bs[bs.find("### 67.11"):bs.find("### 67.12")] if bs.find("### 67.11") >= 0 else ""
    if "`CanonicalSchemaRegistry`" not in m22_6711 or "identical field-name sets" not in m22_6711:
        D.add("semantic documentation", "schema parity statement",
              "BS §67.11 must state the CanonicalSchemaRegistry schema-parity relation (identical field-name sets for registered schemas)")
    # Cross-document consistency (audit M22). Each rule names the canonical
    # owner and rejects the drift that was found in the other document.
    # (a) Disk quota: BS §26.3 owns the 10 GB default; TA §7.2 must repeat it.
    m22_quota = re.search(r"^\| Default task disk quota \|([^|]*)\|", ta, re.M)
    if not m22_quota or "10 GB unless project policy overrides" not in m22_quota.group(1):
        D.add("semantic documentation", "disk quota default",
              "TA §7.2 'Default task disk quota' must state '10 GB unless project policy overrides' (BS §26.3 owns the default)")
    # (b) IPC transport: production SupervisorConnection is named pipes (TA §2);
    # no decision table may present WebSocket as a production alternative.
    m22_ipc = re.search(r"^\| Local IPC \|([^|]*)\|", ta, re.M)
    if not m22_ipc or "named pipes" not in m22_ipc.group(1) or re.search(r"WebSocket or named", m22_ipc.group(1)):
        D.add("semantic documentation", "IPC transport",
              "TA §14 'Local IPC' row must name authenticated named pipes as the production transport, never WebSocket as an alternative")
    # (c) previewMode enumeration is declared on the PreviewRevision field in
    # both canonical blocks and includes CONSERVATIVE_FULL_REINSTALL (TA §73.11).
    m22_modes = ("RN_EXPO_FAST_REFRESH", "COMPOSE_RELOAD", "INCREMENTAL_APK_INSTALL", "FULL_APK_REINSTALL",
                 "CONSERVATIVE_FULL_REINSTALL", "HEADLESS_SMOKE", "DIAGNOSTIC_SOURCE_ONLY", "USER_REQUIRED", "BLOCKED")
    for label, text, where in (("build spec", bs, "BS §69.4"), ("technical architecture", ta, "TA §73.3")):
        pm = re.search(r"^- previewMode:([^\n]*)$", text, re.M)
        values = tuple(v.strip() for v in pm.group(1).split("|")) if pm else ()
        if values != m22_modes:
            D.add("semantic documentation", "previewMode enumeration",
                  f"{where} PreviewRevision.previewMode must enumerate exactly {' | '.join(m22_modes)}")
    # (d) Exactly one ChangeReportRecord per transaction: §87.5 regeneration
    # may not create a second record.
    if "Regeneration never creates a second record" not in ta or "Regeneration creates a new projection version" in ta:
        D.add("semantic documentation", "change report regeneration",
              "TA §87.5 must state that regeneration replaces the single record's report (CLAUSE.CHANGE.EXACTLY_ONE_REPORT), never a second ChangeReportRecord")
    # (e) Ledger completeness: every §36.1 record persisted by the later
    # contracts has a §57.5 table.
    m22_ledger = re.search(r"### 57\.5 SQLite execution ledger.*?```text\n(.*?)```", ta, re.S)
    m22_tables = set(re.split(r"[,\s]+", m22_ledger.group(1).strip())) if m22_ledger else set()
    for table in ("change_report_records", "conversations", "conversation_rebase_records", "content_revisions",
                  "export_verification_records", "environment_capability_records", "skill_invocation_records",
                  "skill_admissions", "construction_transactions"):
        if table not in m22_tables:
            D.add("semantic documentation", "ledger completeness", f"TA §57.5 ledger lacks table {table}")
    # (f) Quality metrics: BS §28.3 and TA §29.2 list the same eleven metrics.
    m22_start = ta.find("### 29.2 Runtime quality metrics")
    m22_end = ta.find("### 29.3", max(m22_start, 0))
    m22_rows = [r for r in ta[m22_start:m22_end].split("\n")
                if r.startswith("| ") and "---" not in r and not r.startswith("| Metric")] if 0 <= m22_start < m22_end else []
    if len(m22_rows) != 11 or "attention reliability (§53.11)" not in bs or "the eleven metrics of TA §29.2" not in bs:
        D.add("semantic documentation", "quality metrics parity",
              f"TA §29.2 lists {len(m22_rows)} metrics; BS §28.3 must name the same eleven including attention reliability (§53.11)")
    # (g) Provider failure taxonomy: TA §38 defers to the eight §24.6 classes.
    if "classified into the eight retry classes of §24.6" not in ta or "provider-unavailable categories" in ta:
        D.add("semantic documentation", "provider failure taxonomy",
              "TA §38 must classify provider failures into the eight retry classes of §24.6, not a second taxonomy")
    # (h) §79 capability vocabulary: DEGRADED is not a §79.4 capability state.
    m22_s79_start = bs.find("## 79. Platform and Target Environment Contract")
    m22_s79_end = bs.find("## 80. Agent-Buildability Contract", max(m22_s79_start, 0))
    m22_s79 = bs[m22_s79_start:m22_s79_end] if m22_s79_start >= 0 and m22_s79_end > m22_s79_start else "DEGRADED"
    if "DEGRADED" in m22_s79:
        D.add("semantic documentation", "capability vocabulary",
              "BS §79 uses DEGRADED, which is not a §79.4 capability state (AVAILABLE | REPAIRABLE | USER_REQUIRED | UNAVAILABLE)")
    # Section-reference resolution (audit LOW): every `§N[.N]` in the build
    # spec and technical architecture must name an existing heading. A
    # reference is same-document unless the words immediately before it
    # qualify it ("build spec §", "BS §", "technical architecture §", "TA §",
    # or a comma-continued list after such a qualifier). Development-plan
    # references are not resolved (its sections are milestone-numbered).
    m22_heads = {k: set(m.group(1) for m in re.finditer(r"^#{2,4}\s+(\d+(?:\.\d+)*)\b", t, re.M))
                 for k, t in (("bs", bs), ("ta", ta))}
    m22_qual = ((r"build[ -]spec(?:ification)?\s+§$", "bs"), (r"\bBS\s+§$", "bs"),
                (r"technical[ -]architecture\s+§$", "ta"), (r"\bTA\s+§$", "ta"),
                (r"development[ -]plan\s+§$", "dev"), (r"\bDP\s+§$", "dev"))
    m22_list = re.compile(r"(build spec|BS|technical architecture|TA|development plan|DP)\s+§\d+(?:\.\d+)*"
                          r"(?:,\s*§\d+(?:\.\d+)*)*,?\s*(?:and\s+)?$")
    m22_doc_of = {"build spec": "bs", "BS": "bs", "technical architecture": "ta", "TA": "ta",
                  "development plan": "dev", "DP": "dev"}
    for label, key, text in (("build spec", "bs", bs), ("technical architecture", "ta", ta)):
        seen = set()
        for m in re.finditer(r"§\s*(\d+(?:\.\d+)*)", text):
            num = m.group(1)
            pre = text[max(0, m.start() - 30):m.start() + 1]
            target = key
            for pat, d in m22_qual:
                if re.search(pat, pre):
                    target = d
                    break
            else:
                lst = m22_list.search(text[max(0, m.start() - 80):m.start()])
                if lst:
                    target = m22_doc_of[lst.group(1)]
            if target == "dev" or num in m22_heads[target] or (target, num) in seen:
                continue
            seen.add((target, num))
            D.add("semantic documentation", "section reference",
                  f"{label} line {text[:m.start()].count(chr(10)) + 1} cites §{num} of the "
                  f"{'build spec' if target == 'bs' else 'technical architecture'}, which has no such heading")
    # §80.2 field-count fidelity (audit LOW): a resolution cell that says
    # "<word> `Schema` fields" must match the schema's actual field block
    # (BS block first, then TA), and the ModelEvent type-count claim must
    # match the enumerated `type` values.
    m22_words = {"two": 2, "three": 3, "four": 4, "five": 5, "six": 6, "seven": 7, "eight": 8, "nine": 9,
                 "ten": 10, "eleven": 11, "twelve": 12, "thirteen": 13, "fourteen": 14, "fifteen": 15,
                 "sixteen": 16, "seventeen": 17, "eighteen": 18, "nineteen": 19, "twenty": 20}
    for _tens, _tv in (("twenty", 20), ("thirty", 30), ("forty", 40)):
        for _ones, _ov in (("one", 1), ("two", 2), ("three", 3), ("four", 4), ("five", 5),
                           ("six", 6), ("seven", 7), ("eight", 8), ("nine", 9)):
            m22_words[f"{_tens}-{_ones}"] = _tv + _ov
    m22_words.update({"thirty": 30, "forty": 40})
    m22_s802 = bs.find("### 80.2")
    m22_e802 = bs.find("### 80.3", max(m22_s802, 0))
    m22_body = bs[m22_s802:m22_e802] if 0 <= m22_s802 < m22_e802 else ""
    for word, schema in re.findall(r"(?<![A-Za-z-])(" + "|".join(sorted(m22_words, key=len, reverse=True)) + r")\s+`([A-Z][A-Za-z0-9]+)`\s+fields", m22_body):
        actual = (bs_blocks.get(schema) or ta_blocks.get(schema) or [(0, None)])[0][1]
        if actual is None or len(actual) != m22_words[word]:
            D.add("semantic documentation", "§80.2 field count",
                  f"§80.2 claims {word} `{schema}` fields; the canonical block has {len(actual) if actual is not None else 'no'} fields")
    m22_ev = re.search(r"\nModelEvent\n(?:- .*\n)*?- type:([^\n]*(?:\n[ \t]+[^\n]*)*)", ta)
    m22_ev_n = len([v for v in re.split(r"\s*\|\s*", m22_ev.group(1).strip()) if v]) if m22_ev else 0
    m22_ev_claim = re.search(r"\| The (" + "|".join(m22_words) + r") event types are the closed set;", m22_body)
    if not m22_ev_claim or m22_words[m22_ev_claim.group(1)] != m22_ev_n:
        D.add("semantic documentation", "§80.2 field count",
              f"§80.2 ModelEvent row must state that the {m22_ev_n} enumerated event types are the closed set")
    # Clause coverage (audit M21): every registered contract owns at least
    # one sealed §67.12 clause, otherwise its normative content is invisible
    # to the contradiction and override checks.
    owned = {meta["contract"] for meta in R["clauses"].values()}
    for cid in sorted(R["contracts"]):
        if cid not in owned:
            D.add("semantic documentation", "clause coverage", f"{cid} owns no §67.12 ClauseId")
    # §80.2 quote fidelity (audit M21): every quoted "should" statement must
    # occur verbatim inside the section it cites, so the resolution table
    # cannot drift from the prose it resolves (paraphrases, vanished quotes,
    # and wrong section numbers are defects).
    def _section_body(text, num):
        pat = re.compile(r"^(#{2,5}) " + re.escape(num) + r"(?:\.| |$)(.*)$", re.M)
        m_ = pat.search(text)
        if m_ is None:
            return None
        rest = text[m_.end():]
        nxt = re.search(r"^#{2," + str(len(m_.group(1))) + r"} ", rest, re.M)
        return rest[:nxt.start()] if nxt else rest
    quote_table = bs.split('### 80.2 "Should" resolution table', 1)[-1].split("### 80.3", 1)[0]
    plain = {"BS": bs.replace("**", ""), "TA": ta.replace("**", "")}
    for doc, num, quote in re.findall(r"^\| (BS|TA) §([0-9.]+) \| \"([^\"]+)\"", quote_table, re.M):
        body = _section_body(plain[doc], num)
        if body is None:
            D.add("semantic documentation", "§80.2 quote fidelity", f"{doc} §{num} does not exist (row {quote[:50]!r})")
            continue
        frags = [f.strip() for f in re.split(r"\.\.\.|…", quote) if f.strip()]
        if not all(f in body for f in frags):
            D.add("semantic documentation", "§80.2 quote fidelity",
                  f"{doc} §{num} does not contain the quoted statement {quote[:70]!r}")
    # §80.10 coverage is machine-derived: the per-scope figures MUST equal the
    # number of §80.2 rows carrying that scope prefix, and the total MUST be
    # their sum. A hand-maintained figure that drifts from the table it
    # summarizes is a defect (the 512-vs-459 drift this rule was added for).
    table = bs.split('### 80.2 "Should" resolution table', 1)[-1].split("### 80.3", 1)[0]
    scope_rows = {}
    for prefix in re.findall(r"^\| (BS|TA|DP|AGENTS)\b", table, re.M):
        scope_rows[prefix] = scope_rows.get(prefix, 0) + 1
    status = bs.split("### 80.10 Resolution coverage status", 1)[-1].split("\n## ", 1)[0]
    declared = {}
    for label, n_rows, n_resolved in re.findall(
            r"^\| (Build spec \(all sections\)|Technical architecture|Development plan|AGENTS\.md) \| (\d+) \| (\d+) \|", status, re.M):
        declared[label] = (int(n_rows), int(n_resolved))
    total = re.search(r"^\| \*\*Total\*\* \| \*\*(\d+)\*\* \| \*\*(\d+)\*\* \|", status, re.M)
    scope_labels = {"BS": "Build spec (all sections)", "TA": "Technical architecture",
                    "DP": "Development plan", "AGENTS": "AGENTS.md"}
    if not table.strip() or not declared or not total:
        D.add("semantic documentation", "§80.10 coverage derivation",
              "§80.2 table or §80.10 coverage table not found; coverage cannot be derived")
    else:
        for prefix, label in scope_labels.items():
            actual = scope_rows.get(prefix, 0)
            if label not in declared:
                D.add("semantic documentation", "§80.10 coverage derivation", f"§80.10 has no row for {label}")
                continue
            rows_n, resolved_n = declared[label]
            if rows_n != actual or resolved_n != actual:
                D.add("semantic documentation", "§80.10 coverage derivation",
                      f"{label}: §80.10 states {rows_n}/{resolved_n} but §80.2 has {actual} rows with prefix {prefix}")
        actual_total = sum(scope_rows.get(p, 0) for p in scope_labels)
        if int(total.group(1)) != actual_total or int(total.group(2)) != actual_total:
            D.add("semantic documentation", "§80.10 coverage derivation",
                  f"§80.10 total states {total.group(1)}/{total.group(2)} but §80.2 has {actual_total} rows")
        if "the contract-graph verifier (§67.11) recomputes them from the §80.2 table on every run" not in status:
            D.add("semantic documentation", "§80.10 coverage derivation",
                  "§80.10 must state that the verifier recomputes the coverage figures from §80.2")
    # Command registry cardinality (BS §76.1): the stated count must equal
    # the number of canonical rows; aliases are not registry entries.
    reg = bs.split("### 76.1 UICommandRegistry", 1)[-1].split("### 76.2", 1)[0]
    rows = re.findall(r"^\| `([a-z_.]+)` \|", reg, re.M)
    if rows and len(rows) != 29:
        D.add("semantic documentation", "command registry cardinality",
              f"§76.1 lists {len(rows)} canonical command kinds; the documented count is twenty-nine")
    if "complete set of twenty-nine canonical command kinds" not in reg:
        D.add("semantic documentation", "command registry cardinality",
              "§76.1 must state the complete set of twenty-nine canonical command kinds and that UI aliases add no registry entries")
    if "conversation.continue" not in rows:
        D.add("semantic documentation", "command registry cardinality",
              "§76.1 must register `conversation.continue`: the §82.1 Continue operation has no other UI entry")
    if "| `nirman-ipc` |" not in ta or "| `nirman-domain` |" not in ta:
        D.add("semantic documentation", "crate layout",
              "TA §57.1 must define the Cargo workspace crate table naming nirman-domain and nirman-ipc (BS §76.1 cites nirman-ipc)")
    for token in ("thirty command kinds", "twenty-eight canonical command kinds"):
        if token in bs:
            D.add("semantic documentation", "command registry cardinality", f"stale count '{token}' in build spec")
    # Migration residue and scope wording (ADR-108 stack, ADR-207 cloud-only, ADR-210 emulator).
    for token, text, why in (
        ("TypeScript and Rust conventions", dev, "the host is C#/.NET WinUI 3 (ADR-108); TypeScript is not a Nirman stack convention"),
        ("TypeScript and Rust conventions", bs, "the host is C#/.NET WinUI 3 (ADR-108); TypeScript is not a Nirman stack convention"),
        ("phone/tablet checks |", dev, "device checks run as layout-profile checks on the managed emulator (ADR-210)"),
        ("compatible cloud services and local runtimes", dec, "ADR-207 excludes local model runtimes; ADR-037 must not name them as supported"),
    ):
        if token in text:
            D.add("semantic documentation", "migration residue", f"{token!r}: {why}")
    # Every H2 heading in the development plan must be unique so sections are addressable.
    h2 = re.findall(r"^## (.+)$", dev, re.M)
    dupes = sorted({h for h in h2 if h2.count(h) > 1})
    if dupes:
        D.add("semantic documentation", "development plan headings", f"duplicate H2 headings: {dupes}")
    # Vocabulary contradicted by an accepted decision must not reappear in the
    # active product, architecture, or milestone documents.
    contradicted_vocabulary = (
        ("BrandAssetWorker", "ADR-049 admits one worker taxonomy; asset work executes under the UI Worker (ADR-103 as amended)"),
        ("Test Engineer", "legacy worker role name (ADR-049)"),
        ("Backend Specialist", "legacy worker role name (ADR-049)"),
        ("Frontend Specialist", "legacy worker role name (ADR-049)"),
        ("local-provider", "local and self-hosted model runtimes are out of scope (ADR-207)"),
        ("stored locally as a structured state file", "the Task Ledger is the SQLite execution ledger (ADR-110); files are projections"),
    )
    for label, text in (("build spec", bs), ("architecture", ta), ("development plan", dev)):
        for token, why in contradicted_vocabulary:
            if token in text:
                D.add("semantic documentation", f"contradicted vocabulary in {label}",
                      f"{token!r}: {why}")
    # Decisions that ADR-218 amends must say so on the record itself, and
    # ADR-217's decision text must not name a second physical-resource authority.
    adrs = adr_blocks(dec)
    for n in (141, 170, 172, 174, 176, 177, 184):
        if "Amended by ADR-218" not in adrs.get(n, ""):
            D.add("semantic documentation", f"ADR-{n} amendment",
                  "ADR-218 amends this decision; the record must carry an explicit "
                  "'Amended by ADR-218' note stating the surviving semantics")
    decision_217 = adrs.get(217, "").split("**Decision:**", 1)[-1].split("**Rationale:**", 1)[0]
    if "CostAuthority" in decision_217:
        D.add("semantic documentation", "ADR-217 authority",
              "ADR-217 names CostAuthority; ResourceIntegrityAuthority is the only physical-resource authority (ADR-218)")
    if "Amended under ADR-049" not in adrs.get(103, ""):
        D.add("semantic documentation", "ADR-103 amendment",
              "ADR-103 must record that the dedicated BrandAssetWorker role is withdrawn under ADR-049")
    for token in ("AI usage telemetry MUST NOT authorize, deny, throttle, degrade, terminate, pause, or complete work",
                  "Usage telemetry is informational and has no execution-authority semantics.",
                  "CLAUSE.RESOURCE.NO_AI_USAGE_AUTHORITY",
                  "CLAUSE.DELIBERATE.RUNTIME_GRANTS_EFFORT"):
        if token not in bs:
            D.add("semantic documentation", "AI usage telemetry rule",
                  f"build spec lacks the ADR-218 telemetry-only requirement: {token}")
    if "No autonomous-goal completion deadline" not in ta:
        D.add("semantic documentation", "task time policy",
              "architecture lacks the no-autonomous-goal-deadline default (TA §7.2)")
    if "**Status:** Superseded\n**Superseded by:** ADR-218" not in dec:
        D.add("semantic documentation", "ADR-197 supersession",
              "ADR-197 must be marked Superseded by ADR-218 while retaining its text")


# Vocabulary that no skill instruction body may carry. The host stack is
# C#/.NET + WinUI 3 + Rust (ADR-108, ADR-117; AGENTS.md §17) and the only
# Android runtime surface is the Nirman-managed local emulator rendered inside
# Nirman's embedded preview (BS §4.4) — a skill body that names a web-wrapper
# shell or a physical device re-introduces an excluded product path.
SKILL_BODY_BANNED = (
    "Tauri", "Electron", "React ", "React/", "TypeScript", "Vite", "WebView",
    "physical device", "physical Android device", "attached device", "USB device",
)


def check_skill_bodies(docs, D, repo_root):
    """Check 15, skill part (reported under "semantic documentation"): skill
    instruction bodies (BS §79.7) exist for every registered platform
    skill and carry neither the excluded host stack nor a physical-device path.

    Skipped (not passed) when crates/ is absent, exactly like check 14: a
    specification-only tree has no bodies to evaluate.
    """
    bs = docs["bs"]
    m = re.search(r"### 79\.7 .*?(?=\n### |\n## )", bs, re.S)
    if not m:
        D.add("semantic documentation", "BS §79.7",
              "platform-skill table (§79.7) not found")
        return
    names = list(dict.fromkeys(re.findall(r"^\| `([a-z][a-z0-9-]*)` \|", m.group(0), re.M)))
    if not names:
        D.add("semantic documentation", "BS §79.7", "platform-skill table lists no skills")
        return
    # Document-only parity (runs even without the skill tree): every id of
    # the BS §79.7 capability vocabulary has a TA §84.1 matrix row.
    doc_vocab = set(re.findall(r"^\| `([A-Z][A-Z_]+)` \| ", m.group(0), re.M))
    ta_rows = set(re.findall(r"^\| `([A-Z][A-Z_]+)` \| windows \| (?:available|environment_dependent|unavailable_by_platform) \|", docs["ta"], re.M))
    for cid in sorted(doc_vocab - ta_rows):
        D.add("semantic documentation", f"capability id {cid}",
              "named in the BS §79.7 skill vocabulary but absent from the TA §84.1 PlatformCapabilityEntry matrix rows")
    skills_root = os.path.join(repo_root, "crates", "nirman-skills", "skills")
    if not os.path.isdir(skills_root):
        D.skip("semantic documentation", "skill bodies",
               "crates/nirman-skills/skills not found; skill bodies cannot be verified")
        return
    bodies = {}
    for dirpath, _dirs, files in os.walk(skills_root):
        if "SKILL.md" in files:
            bodies[os.path.basename(dirpath)] = os.path.join(dirpath, "SKILL.md")
    for name in names:
        path = bodies.get(name)
        if path is None:
            D.add("semantic documentation", f"skill {name}",
                  "registered in BS §79.7 but has no SKILL.md body under crates/nirman-skills/skills")
            continue
        with open(path, encoding="utf-8") as fh:
            body = fh.read()
        for token in SKILL_BODY_BANNED:
            if token in body:
                D.add("semantic documentation", f"skill {name}",
                      f"body carries {token.strip()!r}: excluded host stack or physical-device "
                      f"path (ADR-108, ADR-117, BS §4.4, AGENTS.md §17)")
    for name in sorted(set(bodies) - set(names)):
        D.add("semantic documentation", f"skill {name}",
              "SKILL.md body exists but the skill is not registered in BS §79.7")
    # Manifests (BS §79.7): skill.json beside each body, built_in scope,
    # requiredCapabilities equal to the §79.7 row, drawn from the closed
    # capability-id vocabulary, no permission requests, no ledger state.
    import json as _json
    sec = m.group(0)
    vocab = set(re.findall(r"^\| `([A-Z][A-Z_]+)` \| ", sec, re.M))
    declared = {}
    for row in re.findall(r"^\| `([a-z][a-z0-9-]*)` \| ([^|]*) \|$", sec, re.M):
        declared[row[0]] = set(re.findall(r"`([A-Z][A-Z_]+)`", row[1]))
    if not vocab or not declared:
        D.add("semantic documentation", "BS §79.7", "capability-id vocabulary or per-skill requiredCapabilities table not found")
    for name in names:
        path = bodies.get(name)
        if path is None:
            continue
        mpath = os.path.join(os.path.dirname(path), "skill.json")
        if not os.path.exists(mpath):
            D.add("semantic documentation", f"skill {name}", "no skill.json manifest beside SKILL.md (BS §79.7)")
            continue
        try:
            with open(mpath, encoding="utf-8") as fh:
                man = _json.load(fh)
        except ValueError as exc:
            D.add("semantic documentation", f"skill {name}", f"skill.json is not valid JSON: {exc}")
            continue
        if man.get("skillId") != name:
            D.add("semantic documentation", f"skill {name}", f"manifest skillId {man.get('skillId')!r} differs from the directory and §79.7 id")
        if man.get("scope") != "built_in":
            D.add("semantic documentation", f"skill {name}", "manifest scope must be built_in for a §79.7 skill")
        if man.get("permissionRequests"):
            D.add("semantic documentation", f"skill {name}", "manifest requests permissions; built-in skills are permission-neutral (CLAUSE.SKILL.NO_PERMISSION_GRANT)")
        if man.get("sourcePath") != "SKILL.md":
            D.add("semantic documentation", f"skill {name}", "manifest sourcePath must name the sibling SKILL.md")
        caps = set(man.get("requiredCapabilities") or [])
        for extra in man.get("conditionalCapabilities", {}).values():
            caps |= set(extra)
        # Body gates (BS §79.7): the prose names its gates only in the closed
        # capability-id vocabulary, and only the ids of the skill's own row;
        # legacy lowercase gate ids (android_build, cross_build_windows,
        # native_execution, environment.repair, ...) are rejected.
        with open(path, encoding="utf-8") as fh:
            body_text = fh.read()
        body_upper = set(re.findall(r"`([A-Z][A-Z_]+)`", body_text))
        body_caps = {c for c in body_upper if c in vocab or re.search(r"(TOOLCHAIN|EXECUTION|OBSERVATION|REPAIR)$", c)}
        for cid in sorted(body_caps - vocab):
            D.add("semantic documentation", f"skill {name}",
                  f"body gates on capability id {cid}, which is outside the §79.7 vocabulary")
        for cid in sorted((body_caps & vocab) - caps):
            D.add("semantic documentation", f"skill {name}",
                  f"body gates on {cid}, which is not in the skill's §79.7 row {sorted(caps)}")
        for legacy in sorted(set(re.findall(r"`([a-z]+(?:[_.][a-z]+)+)`", body_text))):
            D.add("semantic documentation", f"skill {name}",
                  f"body uses legacy lowercase gate id `{legacy}`; gates are stated in the §79.7 capability-id vocabulary")
        unknown = sorted(caps - vocab)
        if unknown:
            D.add("semantic documentation", f"skill {name}", f"manifest names capability ids outside the §79.7 vocabulary: {unknown}")
        if name in declared and caps != declared[name]:
            D.add("semantic documentation", f"skill {name}",
                  f"manifest requiredCapabilities {sorted(caps)} differ from the §79.7 row {sorted(declared[name])}")
        for fld in ("scanStatus", "trustStatus", "enabled", "installedAt", "lastUsedAt"):
            if fld in man:
                D.add("semantic documentation", f"skill {name}", f"manifest carries ledger-state field {fld}; the registry owns it")
        for fld in ("name", "description", "version", "compatibleWorkerRoles", "triggerConditions", "requiredTools", "inputSchema", "outputSchema"):
            if fld not in man:
                D.add("semantic documentation", f"skill {name}", f"manifest lacks SkillPackage field {fld}")


def check_section_ownership(R, D):
    """Check 12: SECTION_OWNERSHIP — BS §68 has exactly one authoritative owner
    (`CONTRACT.RUNTIME.DELIBERATION`) and exactly one declared extension
    (of `CONTRACT.RUNTIME.REASONING`).

    §68 is the one section that is simultaneously an authority and an
    extension. That dual role is where a budget authority could be smuggled
    back in as a second owner or a second extension, so the shape is pinned
    explicitly rather than left to the generic checks.
    """
    SEC = 68
    OWNER = "CONTRACT.RUNTIME.DELIBERATION"
    EXTENDS = "CONTRACT.RUNTIME.REASONING"

    # authoritative owners: §67.8 rows whose authority cell is §68, and
    # line-initial "Registry role: authoritative definition of" markers in §68
    registry_owners = sorted(cid for cid, r in R["contracts"].items()
                             if secrefs(r["authority"]) == [SEC])
    marker_owners = sorted(R["authored"].get(SEC, []))
    if registry_owners != [OWNER]:
        D.add("section ownership", f"§{SEC}",
              f"§67.8 assigns authority over §{SEC} to {registry_owners or 'no contract'}; "
              f"expected exactly [{OWNER}]")
    if marker_owners != [OWNER]:
        D.add("section ownership", f"§{SEC}",
              f"§{SEC} carries authoritative markers for {marker_owners or 'no contract'}; "
              f"expected exactly one, for {OWNER}")

    # declared extensions: ExtensionDeclaration blocks in §68, and §67.8 rows
    # listing §68 in their extension column
    declared = sorted(cid for (sec, cid) in R["declarations"] if sec == SEC)
    listed = sorted(cid for cid, r in R["contracts"].items()
                    if SEC in secrefs(r["ext"]))
    if declared != [EXTENDS]:
        D.add("section ownership", f"§{SEC}",
              f"§{SEC} declares extensions of {declared or 'nothing'}; "
              f"expected exactly one, of {EXTENDS}")
    if listed != [EXTENDS]:
        D.add("section ownership", f"§{SEC}",
              f"§67.8 lists §{SEC} as an extension of {listed or 'nothing'}; "
              f"expected exactly one, {EXTENDS}")
    d = R["declarations"].get((SEC, EXTENDS))
    if d and d["authority_contract"] != EXTENDS:
        D.add("section ownership", f"§{SEC}",
              f"extension declaration names authorityContractId {d['authority_contract']!r}, "
              f"expected {EXTENDS}")


def check_structure(docs, R, D):
    """Check 13 (document-structure, BS §67.11 "Structure" row): document-level
    integrity that the contract graph presupposes."""
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
        # The References block is terminal: no numbered section may follow
        # it (a mid-document block splits the contract sections in two), its
        # labels are contiguous from [1], and it links no excluded stack
        # (Electron/Tauri/Playwright are not part of the product).
        ref_pos = text.find("\n## References\n")
        if ref_pos >= 0:
            tail = text[ref_pos:]
            if re.search(r"^##\s+\d+\.\s", tail, re.M):
                D.add("structure", label, "numbered sections follow the References block; References must be the last section")
            labels = [int(n) for n in re.findall(r'^\[(\d+)\]: \S+ "', tail, re.M)]
            if labels != list(range(1, len(labels) + 1)):
                D.add("structure", label, f"References labels {labels} are not contiguous from [1]")
            for excluded in ("electronjs.org", "tauri.app", "playwright.dev"):
                if excluded in tail:
                    D.add("structure", label, f"References links the excluded stack {excluded}")
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
        # Subsection order: within a section, `### N.k` headings ascend, and a
        # `### N.k.j` never precedes its parent `### N.k` (BS §77.1.1 once
        # preceded §77.1). Fenced blocks are ignored.
        fence, cur, last_child = False, None, 0
        for line in text.split("\n"):
            if line.startswith("```"):
                fence = not fence
                continue
            if fence:
                continue
            ms = re.match(r"^##\s+(\d+)\.\s", line)
            if ms:
                cur, last_child = int(ms.group(1)), 0
                continue
            mc = re.match(r"^###\s+(\d+)\.(\d+)(?:\.(\d+))?\s", line)
            if mc and cur is not None and int(mc.group(1)) == cur:
                child = int(mc.group(2))
                if mc.group(3) is not None and child > last_child:
                    D.add("structure", label, f"subsection {mc.group(0).strip()} precedes its parent §{cur}.{child}")
                elif mc.group(3) is None and child < last_child:
                    D.add("structure", label, f"subsection §{cur}.{child} is out of order after §{cur}.{last_child}")
                last_child = max(last_child, child)
    # The §80.2 resolution table is one table: no blank line may split its
    # rows (a split table is two tables to any markdown reader and hides
    # rows from row-count tooling).
    s802 = docs["bs"].find("### 80.2")
    e802 = docs["bs"].find("### 80.3", max(s802, 0))
    if 0 <= s802 < e802:
        rows = docs["bs"][s802:e802].split("\n")
        for i in range(1, len(rows) - 1):
            if rows[i] == "" and rows[i - 1].startswith("| ") and rows[i + 1].startswith("| "):
                D.add("structure", "build spec", f"§80.2 table is split by a blank line after row {rows[i - 1][:40]!r}")
                break

    adrs = adr_blocks(docs["dec"])
    nums = sorted(adrs)
    if nums != list(range(1, max(nums) + 1)):
        gaps = [n for n in range(1, max(nums) + 1) if n not in adrs]
        D.add("structure", "decision log", f"ADR numbering has gaps: {gaps}")
    # ADR blocks appear in ascending numeric order (ADR-002A sits with
    # ADR-002), and the unnumbered "Decision Review Rules" block is not
    # interleaved between ADR blocks.
    order = [int(n) for n in re.findall(r"^## ADR-(\d+)[A-Z]?:", docs["dec"], re.M)]
    for prev, cur in zip(order, order[1:]):
        if cur < prev:
            D.add("structure", "decision log", f"ADR-{cur:03d} appears after ADR-{prev:03d}; ADR blocks must be in ascending order")
            break
    first_adr = docs["dec"].find("\n## ADR-")
    review = docs["dec"].find("\n## Decision Review Rules")
    if first_adr >= 0 and review >= 0 and review > first_adr and "\n## ADR-" in docs["dec"][review:]:
        D.add("structure", "decision log", "'Decision Review Rules' is interleaved between ADR blocks; it belongs after the last ADR")
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
        # The decision log requires every ADR accepted from ADR-209 onward to
        # state the observable evidence that would justify superseding it.
        if n >= 209 and "**Reversal trigger:**" not in body:
            D.add("structure", f"ADR-{n}", "missing Reversal trigger field (required from ADR-209 onward)")

    # registry cardinality sanity
    if len(R["contracts"]) < 2:
        D.add("structure", "§67.8", "registry has fewer than 2 contracts")
    if not R["capabilities"]:
        D.add("structure", "§5.7", "capability registry is empty")
    if not R["clauses"]:
        D.add("structure", "§67.12", "clause registry is empty")


CHECK_ORDER = (
    "duplicate authority", "unregistered contract", "undeclared extension",
    "authority cycle", "clause contradiction", "unversioned override",
    "dangling reference", "forward break", "reverse break", "orphan contract",
    "canonical identity", "section ownership", "structure",
    "semantic documentation", "command payload coverage",
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
    check_section_ownership(R, D)
    check_semantic_documentation(docs, R, D, root)
    check_structure(docs, R, D)
    check_skill_bodies(docs, D, root)
    check_command_payload_field_coverage(docs, R, D, root)
    return R, adj, D


def main():
    dump_registries = "--dump-registries" in sys.argv
    args = [a for a in sys.argv[1:] if a != "--dump-registries"]
    root = args[0] if args else "."
    R, adj, D = verify(root)

    if dump_registries:
        import json
        serializable_R = {
            "contracts": R.get("contracts", {}),
            "capabilities": R.get("capabilities", {}),
            "clauses": R.get("clauses", {}),
            "chain": R.get("chain", {}),
            "milestones": {str(k): v for k, v in R.get("milestones", {}).items()},
        }
        print("REGISTRIES_JSON_BEGIN")
        print(json.dumps(serializable_R))
        print("REGISTRIES_JSON_END")

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
    skips = D.skips_by_check()
    print("\ncheck results")
    for check in CHECK_ORDER:
        hits = grouped.get(check, [])
        skipped = skips.get(check, [])
        if hits:
            status = f"FAIL ({len(hits)})"
        elif skipped:
            status = f"SKIPPED ({len(skipped)})"
        else:
            status = "PASS"
        print(f"  {status:<10} {check}")

    if len(D):
        print("\nDEFECTS")
        for check in CHECK_ORDER:
            for subject, detail in grouped.get(check, []):
                print(f"  [{check}] {subject}: {detail}")
        print("\nCERTIFICATION: FAIL")
        return 1

    total_skips = sum(len(v) for v in skips.values())
    print("\nall twelve §67.11 contract-graph checks pass in both traversal directions; document-structure checks pass")
    print("semantic documentation lint: PASS")
    if total_skips:
        # Skips are an environment state (the input a check needs is absent),
        # not a defect, so the exit code stays 0. The status vocabulary must
        # nevertheless make the unevaluated portion impossible to overlook:
        # every skipped subject is listed and the terminal status is a
        # distinct value, never the unqualified one (BS §67.11, DP M93).
        print(f"\nUNEVALUATED CHECKS ({total_skips}) — required input not present in the working tree:")
        for check in CHECK_ORDER:
            for subject, detail in skips.get(check, []):
                print(f"  [{check}] {subject}: {detail}")
        print("\nimplementation-facing field coverage was NOT evaluated; this run is documentation-scope only")
        print("and is not RUNTIME_CERTIFIED. Exit code 0 reflects zero defects, not complete evaluation.")
        print("\nCERTIFICATION: DOCUMENTATION_CERTIFIED_WITH_RUNTIME_SOURCE_SKIPS")
    else:
        print("command payload coverage: PASS")
        print("\nCERTIFICATION: DOCUMENTATION_CERTIFIED")
    return 0


if __name__ == "__main__":
    sys.exit(main())
