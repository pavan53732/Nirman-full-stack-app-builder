#!/usr/bin/env python3
"""
Mutation battery for tools/verify_contract_graph.py.

Each case copies the four canonical documents to a temp dir, injects one
mutation, and asserts the verifier exits 1 reporting the EXPECTED defect class.
A case that passes proves the corresponding §67.11 check is not vacuous.

Run: python3 tools/test_verify_contract_graph.py
"""
import os
import re
import shutil
import subprocess
import sys
import tempfile

HERE = os.path.dirname(os.path.abspath(__file__))
REPO = os.path.dirname(HERE)
TOOL = os.path.join(HERE, "verify_contract_graph.py")
DOCS = ("nirman-build-spec.md", "nirman-technical-architecture.md",
        "nirman-decisions.md", "nirman-development-plan.md")
BS, TA, DEC, DEV = DOCS

# label -> (doc, find, replace, expected defect check)
CASES = {
    # ---- check 1: duplicate authority
    "two contracts claim one section": (
        BS, "| CONTRACT.RUNTIME.DEBUGGER | BS §63 |",
        "| CONTRACT.RUNTIME.DEBUGGER | BS §55 |", "duplicate authority"),
    "one contract names two authorities": (
        BS, "| CONTRACT.RUNTIME.PROFILING | BS §64 |",
        "| CONTRACT.RUNTIME.PROFILING | BS §64, BS §63 |", "duplicate authority"),

    # ---- check 2: unregistered identifier
    "unregistered contract id": (
        TA, "`CONTRACT.RUNTIME.PROFILING`", "`CONTRACT.RUNTIME.BOGUS`",
        "unregistered contract"),
    "unregistered clause id": (
        BS, "- nonOverriddenClauses: CLAUSE.WORKSPACE.SINGLE_WRITER",
        "- nonOverriddenClauses: CLAUSE.WORKSPACE.INVENTED", "unregistered contract"),

    # ---- check 3: undeclared / inconsistent extension
    "extension declaration removed": (
        BS, "**ExtensionDeclaration:**\n- authorityContractId: CONTRACT.RUNTIME.MEMORY",
        "**Note:** removed\n- authorityContractId: CONTRACT.RUNTIME.MEMORY",
        "undeclared extension"),
    "declaration disagrees with registry": (
        BS, "- authoritySection: §38\n- extendingSection: §53",
        "- authoritySection: §37\n- extendingSection: §53", "undeclared extension"),
    "invalid extension type": (
        BS, "- extensionType: adds_clauses\n- extendedClauses: CLAUSE.CONTEXT.CONSTRAINT_PRIORITY",
        "- extensionType: rewrites_everything\n- extendedClauses: CLAUSE.CONTEXT.CONSTRAINT_PRIORITY",
        "undeclared extension"),
    "bare authoritative marker": (
        BS, "**Registry role:** authoritative definition of `CONTRACT.RUNTIME.RECONCILIATION` (see §67.8)",
        "**Registry role:** authoritative definition (see §67.8)", "undeclared extension"),

    # ---- check 4: authority cycle
    "two-cycle in authority graph": (
        BS, "| CONTRACT.RUNTIME.CONTEXT | BS §53 | — |",
        "| CONTRACT.RUNTIME.CONTEXT | BS §53 | BS §38 |", "authority cycle"),

    # ---- check 5: clause contradiction
    "sealed clause listed as extended": (
        BS, "- extendedClauses: CLAUSE.CONTEXT.CONSTRAINT_PRIORITY, CLAUSE.CONTEXT.SOURCE_REQUIRED",
        "- extendedClauses: CLAUSE.MEMORY.RETENTION_AUTHORITY", "clause contradiction"),
    "clause authority contradicts contract authority": (
        BS, "| CLAUSE.RECONCILE.USER_PRECEDENCE | CONTRACT.RUNTIME.RECONCILIATION | §55 |",
        "| CLAUSE.RECONCILE.USER_PRECEDENCE | CONTRACT.RUNTIME.RECONCILIATION | §63 |",
        "clause contradiction"),
    "clause belongs to unregistered contract": (
        BS, "| CLAUSE.DEBUG.READ_ONLY | CONTRACT.RUNTIME.DEBUGGER |",
        "| CLAUSE.DEBUG.READ_ONLY | CONTRACT.RUNTIME.PHANTOM |", "clause contradiction"),

    # ---- check 6: unversioned override
    "deprecated without superseding contract": (
        BS, "| CONTRACT.RUNTIME.PROFILING | BS §64 | — | TA §69 | ADR-153 | M90 | INTERNAL |",
        "| CONTRACT.RUNTIME.PROFILING | BS §64 | — | TA §69 | ADR-153 | M90 | DEPRECATED |",
        "unversioned override"),

    # ---- check 7: dangling reference
    "dangling ADR": (
        BS, "| ADR-153 | M90 |", "| ADR-999 | M90 |", "dangling reference"),
    "dangling architecture section": (
        BS, "| TA §65 | ADR-150 |", "| TA §995 | ADR-150 |", "dangling reference"),
    "dangling schema subsection": (
        BS, "| TA §62 | TA §62.2 |", "| TA §62 | TA §62.97 |", "dangling reference"),
    # THE WRONG-DOCUMENT CASES. BS §62.2 and BS §23.1 both genuinely EXIST, so a
    # resolver that accepts a reference found in either document passes these.
    # They fail only under domain-exact resolution.
    "schema points at BS when edge addresses TA": (
        BS, "| TA §62 | TA §62.2 |", "| TA §62 | BS §62.2 |", "dangling reference"),
    "persistence points at BS when edge addresses TA": (
        BS, "| BS §33 | TA §23.1 |", "| BS §33 | BS §23.1 |", "dangling reference"),
    "architecture points at BS when edge addresses TA": (
        BS, "| TA §21 | TA §27.1 |", "| BS §21 | TA §27.1 |", "dangling reference"),
    "requirement points at TA when edge addresses BS": (
        BS, "| CONTRACT.RUNTIME.TRIGGER | CAP.ANDROID.AUTOMATED_START | BS §60 |",
        "| CONTRACT.RUNTIME.TRIGGER | CAP.ANDROID.AUTOMATED_START | TA §60 |",
        "dangling reference"),
    "unqualified reference rejected": (
        BS, "| TA §70 | TA §70.4 |", "| TA §70 | §70.4 |", "dangling reference"),
    "dangling capability in twelve-edge row": (
        BS, "| CONTRACT.RUNTIME.TRIGGER | CAP.ANDROID.AUTOMATED_START |",
        "| CONTRACT.RUNTIME.TRIGGER | CAP.ANDROID.NOSUCH |", "dangling reference"),

    # ---- check 8: forward break
    "twelve-edge cell emptied": (
        BS, "| CONTRACT.RUNTIME.DEBUGGER | CAP.ANDROID.LIVE_STEER | BS §63 | BS §63 | TA §67 |",
        "| CONTRACT.RUNTIME.DEBUGGER | CAP.ANDROID.LIVE_STEER | BS §63 | BS §63 | — |",
        "forward break"),
    "capability requires no contract": (
        BS, "| CAP.ANDROID.SECURE_RELEASE | Produce a packaged artifact with verified dependencies and provenance | CONTRACT.RUNTIME.SUPPLY_CHAIN |",
        "| CAP.ANDROID.SECURE_RELEASE | Produce a packaged artifact with verified dependencies and provenance | none |",
        "forward break"),

    # ---- check 9: reverse break
    "ADR Locks field removed": (
        DEC, "**Locks:** `CONTRACT.RUNTIME.SPECULATION`\n\n", "", "reverse break"),
    "milestone mapping loses its contract": (
        DEV, "| M91 | CONTRACT.RUNTIME.TRIGGER |", "| M91 | |", "reverse break"),
    "milestone mapping loses test id": (
        DEV, "| M86 | CONTRACT.RUNTIME.LOCALIZATION | ADR-147 | TEST-LOC-001 |",
        "| M86 | CONTRACT.RUNTIME.LOCALIZATION | ADR-147 |  |", "reverse break"),

    # ---- check 10: orphan contract (the false-negative case)
    "orphan contract with a VALID class": (
        BS, "| CONTRACT.RUNTIME.INVARIANTS | BS §67 | — | all | ADR-157 | M93 | FOUNDATIONAL |",
        "| CONTRACT.RUNTIME.INVARIANTS | BS §67 | — | all | ADR-157 | M93 | FOUNDATIONAL |\n"
        "| CONTRACT.RUNTIME.DEAD_TEST | BS §64 | — | TA §69 | ADR-153 | M90 | INTERNAL |",
        "orphan contract"),
    "cross-cutting contract unreachable from any capability": (
        BS, "| CAP.ANDROID.USER_COEDIT | Let the user edit project files during an active autonomous run | CONTRACT.RUNTIME.RECONCILIATION |",
        "| CAP.ANDROID.USER_COEDIT | Let the user edit project files during an active autonomous run | CONTRACT.RUNTIME.MEMORY |",
        "orphan contract"),

    # ---- Step 2: reasoning contract + namespace migration integrity
    "reasoning clause listed as extended not adopted": (
        BS, "- nonOverriddenClauses: CLAUSE.AUTHORITY.MODEL_PROPOSES, CLAUSE.AUTHORITY.NO_SELF_ELEVATION\n\nThis section extends §33",
        "- nonOverriddenClauses: CLAUSE.AUTHORITY.NO_SELF_ELEVATION\n\nThis section extends §33",
        "undeclared extension"),
    "reasoning contract loses its milestone": (
        BS, "| CONTRACT.RUNTIME.REASONING | BS §66 | BS §68 | TA §71 | ADR-167, ADR-168, ADR-169, ADR-170, ADR-171 | M94 |",
        "| CONTRACT.RUNTIME.REASONING | BS §66 | BS §68 | TA §71 | ADR-167, ADR-168, ADR-169, ADR-170, ADR-171 | M999 |",
        "dangling reference"),
    "reasoning architecture points at BS": (
        BS, "| CONTRACT.RUNTIME.REASONING | CAP.ANDROID.AUTONOMOUS_REASONING | BS §66 | BS §66 | TA §71 |",
        "| CONTRACT.RUNTIME.REASONING | CAP.ANDROID.AUTONOMOUS_REASONING | BS §66 | BS §66 | BS §66 |",
        "dangling reference"),
    "reasoning ADR loses its Locks": (
        DEC, "**Locks:** `CONTRACT.RUNTIME.REASONING`\n\n**Status:** Accepted\n\n**Decision:** Autonomous work will be driven",
        "**Status:** Accepted\n\n**Decision:** Autonomous work will be driven",
        "reverse break"),
    "migration regression: BS cert ref reverted to 66": (
        BS, "### 67.8 Registered contract identifiers", "### 66.8 Registered contract identifiers",
        "FATAL"),

    # ---- Step 3: deliberation contract
    "deliberation drops an inherited sealed clause": (
        BS, "CLAUSE.REASONING.MODE_WITHIN_POLICY, CLAUSE.REASONING.CHILD_CAPABILITY_CEILING",
        "CLAUSE.REASONING.MODE_WITHIN_POLICY", "undeclared extension"),
    "deliberation contract loses its architecture section": (
        BS, "| CONTRACT.RUNTIME.DELIBERATION | BS §68 | — | TA §72 |",
        "| CONTRACT.RUNTIME.DELIBERATION | BS §68 | — | TA §972 |", "dangling reference"),
    "deliberation schema edge points at BS": (
        BS, "| CONTRACT.RUNTIME.DELIBERATION | CAP.ANDROID.DEEP_PROBLEM_SOLVING | BS §68 | BS §68 | TA §72 | TA §72.3 |",
        "| CONTRACT.RUNTIME.DELIBERATION | CAP.ANDROID.DEEP_PROBLEM_SOLVING | BS §68 | BS §68 | TA §72 | BS §68.3 |",
        "dangling reference"),
    "deliberation ADR loses its Locks": (
        DEC, "**Locks:** `CONTRACT.RUNTIME.DELIBERATION`\n\n**Status:** Accepted\n\n**Decision:** Reasoning effort will be budgeted",
        "**Status:** Accepted\n\n**Decision:** Reasoning effort will be budgeted", "reverse break"),
    "deliberation capability unregistered": (
        BS, "| CAP.ANDROID.DEEP_PROBLEM_SOLVING | Spend additional bounded reasoning",
        "| CAP.ANDROID.DEEP_THINKING | Spend additional bounded reasoning", "unregistered contract"),
    "M95 mapping loses its contract": (
        DEV, "| M95 | CONTRACT.RUNTIME.DELIBERATION |", "| M95 | |", "reverse break"),

    "causal-escalation clause unregistered": (
        BS, "CLAUSE.DELIBERATE.CAUSAL_ESCALATION, CLAUSE.DELIBERATE.NO_MUTATION_IN_PASS",
        "CLAUSE.DELIBERATE.CAUSAL_TRIGGER, CLAUSE.DELIBERATE.NO_MUTATION_IN_PASS",
        "unregistered contract"),
    "no-mutation clause loses its contract": (
        BS, "| CLAUSE.DELIBERATE.NO_MUTATION_IN_PASS | CONTRACT.RUNTIME.DELIBERATION |",
        "| CLAUSE.DELIBERATE.NO_MUTATION_IN_PASS | CONTRACT.RUNTIME.PHANTOM |",
        "clause contradiction"),

    # ---- structure
    "ADR numbering gap": (
        DEC, "## ADR-150:", "## ADR-1500:", "structure"),
    "duplicate References section": (
        TA, "## References", "## References\n\n## References", "structure"),
}


def run(root):
    p = subprocess.run([sys.executable, TOOL, root],
                       capture_output=True, text=True, timeout=180)
    return p.returncode, p.stdout + p.stderr


def failed_checks(out):
    """Defect classes reported by a run.

    A missing registry aborts before the check table is produced. That is a real
    detection, not a pass, so it is surfaced as the synthetic class "FATAL" —
    keeping it distinguishable from a clean exit.
    """
    hits = set(re.findall(r"^\s*FAIL \(\d+\)\s+(.+)$", out, re.M))
    if "FATAL:" in out:
        hits.add("FATAL")
    return hits


def main():
    results = []

    rc = subprocess.run([sys.executable, "-m", "py_compile", TOOL],
                        capture_output=True, text=True).returncode
    results.append(("verifier compiles", rc == 0, ""))

    rc, out = run(REPO)
    n = re.search(r"defects\s*:\s*(\d+)", out)
    results.append(("positive: repo certifies",
                    rc == 0 and "CERTIFICATION: PASS" in out and n and n.group(1) == "0",
                    f"exit={rc} defects={n.group(1) if n else '?'}"))

    rc2, out2 = run(REPO)
    results.append(("deterministic", out == out2, ""))

    covered = set()
    for label, (doc, find, repl, expect) in CASES.items():
        with tempfile.TemporaryDirectory(prefix="hermes-cg-") as tmp:
            for d in DOCS:
                shutil.copy2(os.path.join(REPO, d), os.path.join(tmp, d))
            path = os.path.join(tmp, doc)
            text = open(path, encoding="utf-8").read()
            if find not in text:
                results.append((f"negative: {label}", False, "anchor missing -> test invalid"))
                continue
            open(path, "w", encoding="utf-8").write(text.replace(find, repl, 1))
            rc, out = run(tmp)
            hit = expect in failed_checks(out)
            if hit:
                covered.add(expect)
            results.append((f"negative: {label}", rc == 1 and hit,
                            "" if (rc == 1 and hit) else
                            f"exit={rc} expected={expect!r} got={sorted(failed_checks(out))}"))

    with tempfile.TemporaryDirectory(prefix="hermes-cg-ctl-") as tmp:
        for d in DOCS:
            shutil.copy2(os.path.join(REPO, d), os.path.join(tmp, d))
        rc, out = run(tmp)
        results.append(("control: clean copy certifies", rc == 0, f"exit={rc}"))

    expected_checks = {
        "duplicate authority", "unregistered contract", "undeclared extension",
        "authority cycle", "clause contradiction", "unversioned override",
        "dangling reference", "forward break", "reverse break", "orphan contract",
        "structure",
    }
    # FATAL is a harness-synthesised class, not a §67.11 check; exclude it from
    # coverage accounting so the ratio cannot exceed the number of real checks.
    covered_checks = covered & expected_checks
    missing = sorted(expected_checks - covered_checks)
    results.append(("every check has a proving mutation", not missing,
                    f"uncovered: {missing}" if missing else ""))

    width = max(len(n) for n, _, _ in results)
    bad = 0
    for name, ok, detail in results:
        if not ok:
            bad += 1
        print(f"{'PASS' if ok else 'FAIL'}  {name:<{width}}  {detail}")
    print(f"\n{len(results) - bad}/{len(results)} checks passed")
    print(f"§67.11 checks proven non-vacuous: "
          f"{len(covered_checks)}/{len(expected_checks)}")
    extra = sorted(covered - expected_checks)
    if extra:
        print(f"additional detection classes exercised: {', '.join(extra)}")
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main())
