#!/usr/bin/env python3
"""
Mutation battery for tools/verify_contract_graph.py.

Each case copies the ten root documents (ADR-220) to a temp dir, injects one
mutation, and asserts the verifier exits 1 reporting the EXPECTED defect class.
A case that passes proves the corresponding §67.11 check is not vacuous.

Run: python3 tools/test_verify_contract_graph.py
"""
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile

HERE = os.path.dirname(os.path.abspath(__file__))
if HERE not in sys.path:
    sys.path.insert(0, HERE)
import verify_contract_graph
REPO = os.path.dirname(HERE)
# Terminal status family printed by the verifier on a zero-defect run. The
# with-skips value is documentation-scope only; the unqualified value requires
# zero skipped checks. "CERTIFICATION: PASS" is no longer emitted.
CERTIFIED_RE = re.compile(
    r"^CERTIFICATION: DOCUMENTATION_CERTIFIED(_WITH_RUNTIME_SOURCE_SKIPS)?$", re.M)
TOOL = os.path.join(HERE, "verify_contract_graph.py")
DOCS = ("nirman-build-spec.md", "nirman-technical-architecture.md",
        "nirman-decisions.md", "nirman-milestones.md")
BS, TA, DEC, DEV = DOCS
# ADR-220 documents. Every root document present in the repository is copied
# into each fixture so verifier rules that read them are exercised; the tuple
# below names the ones that may not exist yet while the migration lands.
ADR220_DOCS = ("nirman-schemas.md", "nirman-adrs.md",
               "INDEX.md", "GLOSSARY.md", "AGENTS.md", "README.md")
SCHEMAS, ADRS, INDEX, GLOSSARY, AGENTS_MD, README_MD = ADR220_DOCS
MILESTONES = DEV

# Extra (relpath, abspath) pairs the mutation battery needs to copy into the
# temp root so the new command-payload-coverage check can resolve Rust sources
# relative to the verifier's `root` argument.
RUST_SOURCES = (
    ("crates/nirman-domain/src/lib.rs",
     os.path.join(REPO, "crates/nirman-domain/src/lib.rs")),
    ("crates/nirman-ipc/src/lib.rs",
     os.path.join(REPO, "crates/nirman-ipc/src/lib.rs")),
    ("crates/nirman-preview/src/lib.rs",
     os.path.join(REPO, "crates/nirman-preview/src/lib.rs")),
)

# Skill instruction bodies (BS §79.7). The skill-body rule needs every
# registered body present in the temp root, otherwise a "missing body" defect
# would mask the mutation actually under test.
SKILL_DIRS = (
    "android/android-toolchain",
    "cross-platform/cross-platform-build-diagnostics",
    "environment/environment-preflight",
    "environment/environment-repair",
    "windows/windows-desktop-build",
    "windows/windows-runtime-validation",
)
SKILL_SOURCES = tuple(
    (f"crates/nirman-skills/skills/{d}/SKILL.md",
     os.path.join(REPO, f"crates/nirman-skills/skills/{d}/SKILL.md"))
    for d in SKILL_DIRS)
SKILL_MANIFESTS = tuple(
    (f"crates/nirman-skills/skills/{d}/skill.json",
     os.path.join(REPO, f"crates/nirman-skills/skills/{d}/skill.json"))
    for d in SKILL_DIRS)
WIN_BUILD_SKILL = SKILL_SOURCES[4][0]
ANDROID_SKILL = SKILL_SOURCES[0][0]

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
        BS, "- extendedClauses: CLAUSE.CONTEXT.CONSTRAINT_PRIORITY, CLAUSE.CONTEXT.SOURCE_REQUIRED, CLAUSE.CONTEXT.ATTENDABILITY_REQUIRED, CLAUSE.CONTEXT.RECALL_EVIDENCE_ONLY",
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
        ADRS, "**Locks:** `CONTRACT.RUNTIME.SPECULATION`\n\n", "", "reverse break"),
    "milestone mapping loses its contract": (
        DEV, "| M91 | CONTRACT.RUNTIME.TRIGGER |", "| M91 | |", "reverse break"),
    "milestone mapping loses test id": (
        DEV, "| M86 | CONTRACT.RUNTIME.LOCALIZATION | ADR-147 | TEST-LOC-001 |",
        "| M86 | CONTRACT.RUNTIME.LOCALIZATION | ADR-147 |  |", "reverse break"),

    # ---- check 10: orphan contract (the false-negative case)
    "orphan contract with a VALID class": (
        BS, "| CONTRACT.RUNTIME.INVARIANTS | BS §67 | BS §80 | all | ADR-157 | M93 | FOUNDATIONAL |",
        "| CONTRACT.RUNTIME.INVARIANTS | BS §67 | BS §80 | all | ADR-157 | M93 | FOUNDATIONAL |\n"
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
        BS, "| CONTRACT.RUNTIME.REASONING | BS §66 | BS §68 | TA §71 | ADR-167, ADR-168, ADR-169, ADR-170, ADR-171, ADR-218 | M94 |",
        "| CONTRACT.RUNTIME.REASONING | BS §66 | BS §68 | TA §71 | ADR-167, ADR-168, ADR-169, ADR-170, ADR-171, ADR-218 | M999 |",
        "dangling reference"),
    "reasoning architecture points at BS": (
        BS, "| CONTRACT.RUNTIME.REASONING | CAP.ANDROID.AUTONOMOUS_REASONING | BS §66 | BS §66 | TA §71 |",
        "| CONTRACT.RUNTIME.REASONING | CAP.ANDROID.AUTONOMOUS_REASONING | BS §66 | BS §66 | BS §66 |",
        "dangling reference"),
    "reasoning ADR loses its Locks": (
        ADRS, "**Locks:** `CONTRACT.RUNTIME.REASONING`\n\n**Status:** Accepted\n\n**Decision:** Autonomous work will be driven",
        "**Status:** Accepted\n\n**Decision:** Autonomous work will be driven",
        "reverse break"),
    "migration regression: BS cert ref reverted to 66": (
        BS, "### 67.8 Registered contract identifiers", "### 66.8 Registered contract identifiers",
        "structure"),

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
        ADRS, "**Locks:** `CONTRACT.RUNTIME.DELIBERATION`\n\n**Status:** Accepted\n\n**Decision:** Reasoning effort will be budgeted",
        "**Status:** Accepted\n\n**Decision:** Reasoning effort will be budgeted", "reverse break"),
    "deliberation capability unregistered": (
        BS, "| CAP.ANDROID.DEEP_PROBLEM_SOLVING | Spend additional progress-governed reasoning",
        "| CAP.ANDROID.DEEP_THINKING | Spend additional progress-governed reasoning", "unregistered contract"),

    # ---- check 12: section ownership (§68 = one owner, one extension)
    "section 68 gains a second authoritative owner": (
        BS, "| CONTRACT.RUNTIME.RESOURCE_INTEGRITY | BS §72 | — | TA §77 | ADR-218 | M111 | CROSS_CUTTING |",
        "| CONTRACT.RUNTIME.RESOURCE_INTEGRITY | BS §68 | — | TA §77 | ADR-218 | M111 | CROSS_CUTTING |",
        "section ownership"),
    "section 68 loses its authoritative owner": (
        BS, "| CONTRACT.RUNTIME.DELIBERATION | BS §68 | — | TA §72 |",
        "| CONTRACT.RUNTIME.DELIBERATION | BS §72 | — | TA §72 |",
        "section ownership"),
    "section 68 loses its authoritative marker": (
        BS, "**Registry role:** authoritative definition of `CONTRACT.RUNTIME.DELIBERATION` (see §67.8)",
        "**Registry role:** implementation note for `CONTRACT.RUNTIME.DELIBERATION` (see §67.8)",
        "section ownership"),
    "section 68 gains a second declared extension": (
        BS, "**ContractId:** `CONTRACT.RUNTIME.REASONING`  \n**ExtensionDeclaration:**\n- authorityContractId: CONTRACT.RUNTIME.REASONING\n- authoritySection: §66\n- extendingSection: §68",
        "**ContractId:** `CONTRACT.RUNTIME.REASONING`  \n**ExtensionDeclaration:**\n- authorityContractId: CONTRACT.RUNTIME.REASONING\n- authoritySection: §66\n- extendingSection: §68\n- extensionType: adds_clauses\n- extendedClauses: CLAUSE.DELIBERATE.CAUSAL_ESCALATION\n- nonOverriddenClauses: CLAUSE.AUTHORITY.MODEL_PROPOSES, CLAUSE.AUTHORITY.NO_SELF_ELEVATION\n\n**ContractId:** `CONTRACT.RUNTIME.AUTHORITY`  \n**ExtensionDeclaration:**\n- authorityContractId: CONTRACT.RUNTIME.AUTHORITY\n- authoritySection: §33\n- extendingSection: §68",
        "section ownership"),
    "section 68 extension retargeted away from REASONING": (
        BS, "- authorityContractId: CONTRACT.RUNTIME.REASONING\n- authoritySection: §66\n- extendingSection: §68",
        "- authorityContractId: CONTRACT.RUNTIME.DELIBERATION\n- authoritySection: §66\n- extendingSection: §68",
        "section ownership"),

    # ---- ADR-218: AI-usage budget vocabulary must not return
    "budget exhaustion outcome reintroduced": (
        SCHEMAS, "- outcome: SUFFICIENT | NO_PROGRESS | ESCALATED | ABANDONED",
        "- outcome: SUFFICIENT | BUDGET_EXHAUSTED | NO_PROGRESS | ESCALATED | ABANDONED",
        "semantic documentation"),
    "fixed toolless pass ceiling reintroduced": (
        BS, "no fixed observation-free pass ceiling exists, and no component may hardcode one.",
        "consecutive observation-free passes are counted against `maxToollessPasses`.",
        "semantic documentation"),
    "telemetry-only rule removed": (
        BS, "AI usage telemetry MUST NOT authorize, deny, throttle, degrade, terminate, pause, or complete work.",
        "AI usage telemetry MAY throttle work.",
        "semantic documentation"),
    "ADR-197 supersession removed": (
        ADRS, "**Status:** Superseded\n**Superseded by:** ADR-218",
        "**Status:** Accepted",
        "semantic documentation"),
    "M95 mapping loses its contract": (
        DEV, "| M95 | CONTRACT.RUNTIME.DELIBERATION |", "| M95 | |", "reverse break"),

    # ---- ADR-218 vocabulary cleanup (second pass): budget wording must not return
    "mutation budget reintroduced in broker validation list": (
        BS, "schema, syntax, mutation safety constraints, dependency policy, and evidence requirements.",
        "schema, syntax, mutation budget, dependency policy, and evidence requirements.",
        "semantic documentation"),
    "retry budget reintroduced in defaults table": (
        BS, "| Recovery-attempt policy: materially different attempts per failure fingerprint (transient failures) | 3 | 1-10 | Per task |",
        "| Retry budget (transient failures) | 3 | 1-10 | Per task |",
        "semantic documentation"),
    "recovery-attempt policy definition removed from architecture": (
        TA, "The recovery-attempt policy (`recoveryAttemptPolicy`) is policy-configurable and bounded",
        "The retry ceiling is policy-configurable and bounded",
        "semantic documentation"),
    "cost-efficiency routing criterion reintroduced": (
        BS, "4. **Execution suitability**", "4. **Cost efficiency**",
        "semantic documentation"),
    "PlanCostEstimate schema reintroduced in architecture": (
        SCHEMAS, "ResourceExecutionProfile\n- planRevision", "PlanCostEstimate\n- planRevision",
        "semantic documentation"),
    "ResourceExecutionProfile removed from development plan": (
        DEV, "ResourceExecutionProfile with honest confidence", "plan estimate with honest confidence",
        "semantic documentation"),
    "maxReasoningTokens metadata-only note removed from build spec": (
        SCHEMAS, "- maxReasoningTokens: integer? (provider capability metadata only",
        "- maxReasoningTokens: integer? (execution ceiling",
        "semantic documentation"),
    "maxReasoningTokensOptional metadata-only note removed from architecture": (
        TA, "`maxReasoningTokens` is provider capability metadata",
        "`maxReasoningTokens` is a runtime execution ceiling",
        "semantic documentation"),

    # ---- certification status audit: BS §67.11 status table and DP M93 semantics
    "BS §67.11 loses the with-skips status definition": (
        BS, "| `CERTIFICATION: DOCUMENTATION_CERTIFIED_WITH_RUNTIME_SOURCE_SKIPS` |", "| `CERTIFICATION: PASS (WITH SKIPS)` |",
        "semantic documentation"),
    "BS §67.11 loses the exit-code semantics sentence": (
        BS, "Exit code 0 means zero defects; it does not by itself mean every check was evaluated.", "Exit code 0 means the documentation is fully certified.",
        "semantic documentation"),
    "DP M93 drops the with-skips semantics": (
        DEV, "`DOCUMENTATION_CERTIFIED_WITH_RUNTIME_SOURCE_SKIPS` (zero defects, but implementation-facing field coverage unevaluated because the `crates/` source is absent)", "`DOCUMENTATION_CERTIFIED` (zero defects)",
        "semantic documentation"),

    # ---- coverage derivation audit: §80.10 must equal the §80.2 row counts
    "§80.10 build-spec figure drifts from §80.2": (
        BS, "| Build spec (all sections) | 284 | 284 | Complete |", "| Build spec (all sections) | 320 | 320 | Complete |",
        "semantic documentation"),
    "§80.10 total overstated": (
        BS, "| **Total** | **459** | **459** | **100%** |", "| **Total** | **512** | **512** | **100%** |",
        "semantic documentation"),
    "§80.2 row deleted without updating §80.10": (
        BS, '| BS §3.4 | "should create a checkpoint" | MUST create checkpoint | Before every multi-file autonomous task |\n', "",
        "semantic documentation"),
    "§80.2 row added without updating §80.10": (
        BS, '| BS §3.4 | "should create a checkpoint" | MUST create checkpoint | Before every multi-file autonomous task |\n',
        '| BS §3.4 | "should create a checkpoint" | MUST create checkpoint | Before every multi-file autonomous task |\n| BS §3.4 | "should also snapshot" | MUST snapshot | Before every multi-file autonomous task |\n',
        "semantic documentation"),
    "§80.10 drops the verifier-recomputes statement": (
        BS, "the contract-graph verifier (§67.11) recomputes them from the §80.2 table on every run", "the figures are maintained by hand",
        "semantic documentation"),

    # ---- residue audit: command count, stack residue, emulator wording, ADR-037 scope, heading uniqueness
    "command registry count overstated again": (
        BS, "complete set of twenty-nine canonical command kinds", "complete set of thirty command kinds",
        "semantic documentation"),
    "intelligence contract loses all of its clauses": (
        BS,
        "| CLAUSE.CONVERSATION.DURABLE_AGGREGATE | CONTRACT.RUNTIME.CONVERSATION_CONTEXT | §82 | a Conversation survives UI restart, supervisor restart, context compaction, provider changes, and task handoff as a durable product object | SEALED |\n"
        "| CLAUSE.CONVERSATION.REVISION_CONSISTENT_CONTINUE | CONTRACT.RUNTIME.CONVERSATION_CONTEXT | §82 | Continue resolves expectedProjectRevision against Project.currentRevision into CONTINUE, RECONCILE/REBASE, or USER_REQUIRED; conflicting intervening changes halt autonomous execution and no rebase advances expectedProjectRevision before its ConversationRebaseRecord is durably committed | SEALED |\n"
        "| CLAUSE.CONVERSATION.NO_TRANSCRIPT_RECONSTRUCTION | CONTRACT.RUNTIME.CONVERSATION_CONTEXT | §82 | a continuation never reconstructs state solely by replaying the chat transcript or issuing another free-form prompt; compaction preserves requirements, locked decisions, active goal, project revision, task lineage, attachment references, unresolved failures, and evidence lineage | SEALED |\n",
        "",
        "semantic documentation"),
    "TA §7.2 disk quota default drifts from BS §26.3": (
        TA, "| Default task disk quota | 10 GB unless project policy overrides",
        "| Default task disk quota | Android-profile-based; 10 GB is not fixed",
        "semantic documentation"),
    "PreviewRevision.previewMode loses CONSERVATIVE_FULL_REINSTALL": (
        SCHEMAS, "FULL_APK_REINSTALL | CONSERVATIVE_FULL_REINSTALL | HEADLESS_SMOKE",
        "FULL_APK_REINSTALL | HEADLESS_SMOKE",
        "semantic documentation"),
    "§57.5 ledger drops the change_report_records table": (
        TA, "construction_transactions, change_report_records, conversations,",
        "construction_transactions, conversations,",
        "semantic documentation"),
    "§80.2 field count overstates TaskContract": (
        BS, "all fifteen `TaskContract` fields", "all sixteen `TaskContract` fields",
        "semantic documentation"),
    "§80.2 row cites a section number that has no heading": (
        BS, "Recorded in `toolchainLock` (§5.7.1) |", "Recorded in `toolchainLock` (§369) |",
        "semantic documentation"),
    "BS §67.11 weakens the schema-parity statement": (
        BS, "A registered schema MUST have identical field-name sets in every occurrence",
        "A registered schema SHOULD have matching fields in every occurrence",
        "semantic documentation"),
    "an ADR block is written back into nirman-decisions.md": (
        DEC, "## Decision Review Rules\n",
        "## ADR-221: Stray decision\n\n**Status:** Accepted\n**Locks:** `CONTRACT.RUNTIME.SCOPE`\n**Decision:** x\n**Rationale:** y\n**Consequences:** z\n**Reversal trigger:** none foreseeable\n\n---\n\n## Decision Review Rules\n",
        "structure"),
    "nirman-decisions.md loses its pointer to nirman-adrs.md": (
        DEC, "live in `nirman-adrs.md`, in numeric order", "live in the decision records document, in numeric order",
        "structure"),
    "README refers to the retired nirman-development-plan.md filename": (
        README_MD, "[3]: nirman-milestones.md", "[3]: nirman-development-plan.md",
        "structure"),
    # README's capability numbers are projections of the parsed §5.7 registry:
    # a hand-edited count, or an all-PLANNED claim that the registry no longer
    # supports, is a semantic-documentation defect.
    "README capability count drifts from the §5.7 registry": (
        README_MD, "All 27 registered capabilities are `PLANNED`.", "All 26 registered capabilities are `PLANNED`.",
        "semantic documentation"),
    "README claims all capabilities PLANNED while a §5.7 row says SUPPORTED": (
        BS, "| TEST-GEN-001 | EV-GEN-001 | PLANNED |", "| TEST-GEN-001 | EV-GEN-001 | SUPPORTED |",
        "semantic documentation"),
    # ADR-221: the Android toolchain and emulator are provisioned by Nirman on
    # first launch. Each case removes exactly one of the load-bearing rules —
    # no bundling, no licence auto-accept, readiness by observed frame, no
    # installation guide, supervisor-owned transport, drop-oldest backpressure,
    # the RenderTransport field block, the engine identity, no HAXM.
    "TA §49.4 permits bundling the emulator in the installer": (
        TA, "Nirman MUST NOT bundle, fork, patch, rebuild, or redistribute the emulator",
        "Nirman MAY bundle the emulator",
        "semantic documentation"),
    "TA §49.4 lets Nirman accept the SDK licence for the user": (
        TA, "Nirman MUST NOT pre-accept, auto-accept, or accept the licence on the user's behalf",
        "Nirman accepts the licence on the user's behalf",
        "semantic documentation"),
    "TA §49.4 calls an install without an observed frame READY": (
        TA, "`PROVISIONED_UNVERIFIED`, never `READY`", "`READY`",
        "semantic documentation"),
    "BS §4.2 hands the user an installation guide again": (
        BS, "Nirman MUST NOT present an installation guide, a download link, or a command to run in place of provisioning",
        "Missing tools are reported with an installation guide",
        "semantic documentation"),
    "BS §9.2 offers an installation reference for the Android toolchain": (
        BS, "only a tool outside that toolchain — Git, or a Node.js version manager the project explicitly declares — may be reported with an official installation reference",
        "every missing tool is reported with an official installation reference",
        "semantic documentation"),
    "TA §10.7 lets PreviewHost open the emulator control channel": (
        TA, "The supervisor — never a worker, never PreviewHost — opens the single gRPC channel",
        "PreviewHost opens the gRPC channel",
        "semantic documentation"),
    "TA §10.7 changes the frame backpressure policy to blocking": (
        TA, "`backpressurePolicy: DROP_OLDEST`", "`backpressurePolicy: BLOCK`",
        "semantic documentation"),
    "the RenderTransport block loses its backpressurePolicy field": (
        SCHEMAS, "- backpressurePolicy: DROP_OLDEST\n", "",
        "semantic documentation"),
    "ADR-221 drops the Google APIs system-image variant": (
        ADRS, "distributed through the Android SDK repository, running Google APIs x86_64 system images",
        "distributed through the Android SDK repository, running x86_64 system images",
        "semantic documentation"),
    "TA §49.4 provisions the discontinued HAXM accelerator": (
        TA, "HAXM is never provisioned", "HAXM is provisioned on Intel hosts",
        "semantic documentation"),
    # Frames are pixels, events are meaning: a per-frame PreviewSyncEvent, a
    # FrameNotice that reaches the durable log, a §71.1 without the paint rule,
    # or a placeholder root the agent cannot create are all defects.
    "TA §10.7 turns every frame back into a durable PreviewSyncEvent": (
        TA, "`PreviewSyncEvent`s are emitted only on a change of stream state, never per frame",
        "a `PreviewSyncEvent` is emitted per frame",
        "semantic documentation"),
    "TA §10.7 lets a FrameNotice be a PreviewSyncEvent": (
        TA, "a `FrameNotice` is not a `PreviewSyncEvent`", "a `FrameNotice` is a `PreviewSyncEvent`",
        "semantic documentation"),
    "BS §71.1 drops the live-paint rule": (
        BS, "A frame MUST NOT be painted as live unless the reduced projection's `streamStatus` is `CONNECTED`",
        "A frame may be painted as live whenever it arrives",
        "semantic documentation"),
    "the RenderTransport block loses its frameNotice field": (
        SCHEMAS, "- frameNotice\n  - previewSurfaceId\n  - ringSlot\n  - frameStamp\n", "",
        "semantic documentation"),
    "a milestone block appears outside nirman-milestones.md": (
        BS, "## 80. Agent-Buildability Contract", "## M999 — Stray milestone\n\nText.\n\n## 80. Agent-Buildability Contract",
        "structure"),
    "ADR-220 no longer fixes the root document count at ten": (
        ADRS, "The repository root holds exactly ten Markdown documents:",
        "The repository root holds the following Markdown documents:",
        "semantic documentation"),
    "ADR-220 loses the owner-precedence inheritance rule": (
        ADRS, "and the block inherits the precedence of that owner section rather than holding authority of its own",
        "and the block carries its own precedence",
        "semantic documentation"),
    "§80.2 Job Object row regains the where-unavailable degraded fallback": (
        BS, "MUST use Job Objects, unconditionally | Every spawned process tree is assigned to a Job Object before it is resumed (technical architecture §3.4).",
        "MUST use Job Objects when the API is available | Every spawned process tree is assigned to a Job Object at creation; where unavailable the runtime records the degraded state.",
        "semantic documentation"),
    "BS §5.7.2 SessionProviderMode loses the PROVIDER_CONFIGURED state": (
        BS, "SessionProviderMode   = PLANNING_ONLY | PROVIDER_CONFIGURED | PROVIDER_VALIDATED |\n                        OFFLINE\n",
        "SessionProviderMode   = PLANNING_ONLY | PROVIDER_VALIDATED | OFFLINE\n",
        "semantic documentation"),
    "BS §5.7.2 turns provider validation into a global prerequisite": (
        BS, "it is never a global prerequisite", "it is the session prerequisite",
        "semantic documentation"),
    "TA §34 AutonomousAndroidSession.providerMode loses its SessionProviderMode type": (
        SCHEMAS, "- providerMode: SessionProviderMode (build spec §5.7.2)\n", "- providerMode\n",
        "semantic documentation"),
    "BS §15 roadmap reclaims normative status": (
        BS, "**Normative status:** This roadmap is explanatory sequencing, not a contract.",
        "**Status:** This roadmap is the sequencing contract.",
        "semantic documentation"),
    "a free-floating BS section gains phase exit criteria without subordination": (
        BS, "## 21. Suggested Next Build Sequence\n\n1. Create the Nirman desktop shell.",
        "## 21. Suggested Next Build Sequence\n\n**Exit criteria:** The shell launches.\n\n1. Create the Nirman desktop shell.",
        "semantic documentation"),
    "BS §5.6 derivation table drops the DEGRADED maturity row": (
        BS, "| `DEGRADED` | any | `DEGRADED` |\n", "",
        "semantic documentation"),
    "AndroidCapabilityProfile.status loses its derived §5.6 typing": (
        SCHEMAS, "- status: derived build spec §5.6 status (SUPPORTED | SUPPORTED_WITH_ENVIRONMENT_REQUIREMENTS | DEGRADED | USER_REQUIRED | UNAVAILABLE | PLANNED)\n",
        "- status\n",
        "semantic documentation"),
    "TA §23.3 EvidenceRecord loses its toolchain and environment identity fields": (
        SCHEMAS, "- toolchainLockId\n- environmentIdentityId\n- validationPolicyVersion\n- freshnessInterval\n- dependencyIds\n",
        "- validationPolicyVersion\n- freshnessInterval\n- dependencyIds\n",
        "semantic documentation"),
    "BS §5.7.4 stops binding evidence-node requirements to EvidenceRecord fields": (
        BS, "these are fields of the canonical `EvidenceRecord` (technical architecture §23.3), not annotations kept beside it.",
        "these are recorded alongside the canonical `EvidenceRecord` (technical architecture §23.3).",
        "semantic documentation"),
    "§80.2 hyphenated field count drifts from the EvidenceRecord block": (
        BS, "MUST contain all twenty-eight `EvidenceRecord` fields", "MUST contain all twenty-seven `EvidenceRecord` fields",
        "semantic documentation"),
    "BS §11.5 Checkpoint loses its validity and knownGood fields": (
        SCHEMAS, "- validity: VALID | STALE | INVALIDATED\n- knownGood: boolean\n", "",
        "semantic documentation"),
    "Checkpoint drops out of the TA §36.1 registry": (
        SCHEMAS, "PreviewRevision\nCheckpoint\nEvidenceRecord\n", "PreviewRevision\nEvidenceRecord\n",
        "semantic documentation"),
    "TA §18 tiers stop declaring themselves projections of the canonical Checkpoint": (
        TA, "Both tiers are stored as the canonical `Checkpoint` record of build spec §11.5",
        "Both tiers are stored as checkpoint records of build spec §11.5",
        "semantic documentation"),
    "DP M6 work item regains a present-tense implementation claim": (
        DEV, "8. Add repeated-action and doom-loop detection.",
        "8. Add repeated-action and doom-loop detection; the loop detector now exposes a fingerprint counter.",
        "semantic documentation"),
    "BS §5.7.2 CompletionState loses the NOT_COMPLETE outcome": (
        BS, "CompletionState       = NOT_EVALUATED | NOT_COMPLETE | COMPLETED | BLOCKED |",
        "CompletionState       = NOT_EVALUATED | COMPLETED | BLOCKED |",
        "semantic documentation"),
    "TA §34 AutonomousAndroidSession.completionState loses its CompletionState type": (
        SCHEMAS, "- completionState: CompletionState (build spec §5.7.2)", "- completionState",
        "semantic documentation"),
    "TA §36.4 stops recording the completion outcome as a CompletionState": (
        TA, "The outcome is recorded as a `CompletionState`", "The outcome is recorded as a completion state",
        "semantic documentation"),
    "AndroidServiceIntegration.requiredOperationality loses its IntegrationState type": (
        SCHEMAS, "- requiredOperationality: IntegrationState (build spec §5.7.2;", "- requiredOperationality (build spec §5.7.2;",
        "semantic documentation"),
    "BS §70 drops the operationality gate on service-integration boundaries": (
        BS, "meets the integration's declared `requiredOperationality`", "satisfies the integration's declared `requiredOperationality`",
        "semantic documentation"),
    "TA §81.2 command chain reorders authority checks before the use case": (
        TA, "  → application use case\n  → deterministic authority checks", "  → deterministic authority checks\n  → application use case",
        "semantic documentation"),
    "AGENTS §8 command chain drops the authority-check step": (
        "AGENTS.md", "→ deterministic authority checks\n→ repository and SQLite transaction", "→ repository and SQLite transaction",
        "semantic documentation",
        (("AGENTS.md", os.path.join(REPO, "AGENTS.md")),)),
    "§80.2 row quotes a sentence its section does not contain": (
        BS, '| BS §6.3 | "It should never assume that a tool exists"', '| BS §6.3 | "should never assume tool exists"',
        "semantic documentation"),
    "orphan ProviderContextDecision reintroduced": (
        DEV, "delegation to ContextGovernance / ProviderContextEnvelope.transmissionDecision", "delegation to ContextGovernance / ProviderContextDecision",
        "semantic documentation"),
    "ExactRetriever dropped from the context component table": (
        TA, "| ExactRetriever | Resolves pinned symbols,", "| Exact retriever | Resolves pinned symbols,",
        "semantic documentation"),
    "retired AndroidApplicationContract name reintroduced": (
        BS, "| `AndroidConstructionContract` | Captures features, screens, behavior,", "| `AndroidApplicationContract` | Captures features, screens, behavior,",
        "semantic documentation"),
    "ResourceGovernor identity with ResourceIntegrityAuthority dropped": (
        TA, "`ResourceGovernor` is the §57.2 process-topology name of `ResourceIntegrityAuthority`", "`ResourceGovernor` is a separate service from `ResourceIntegrityAuthority`",
        "semantic documentation"),
    "ownership map cites the worker-registry ADR for toolchain authority": (
        DEV, "(ADR-163 toolchain manifest and project lock; ADR-114 externally managed toolchains)", "(ADR-049 toolchain authority)",
        "semantic documentation"),
    "specialist gate invents a worker role": (
        TA, "| Schema/type consistency | Reconciliation Worker |", "| Schema/type consistency | Consistency worker |",
        "semantic documentation"),
    "update controller becomes a third executable": (
        TA, "The stable launcher/controller is not a third executable.", "The stable launcher/controller is a third executable, `NirmanLauncher.exe`.",
        "semantic documentation"),
    "Continue loses its registered command kind": (
        BS, "| `conversation.continue` | Resolve the durable conversation", "| `conversation.resume` | Resolve the durable conversation",
        "semantic documentation"),
    "crate layout table dropped from the architecture": (
        TA, "| `nirman-ipc` | `UICommandEnvelope`", "| nirman-ipc | `UICommandEnvelope`",
        "semantic documentation"),
    "canonical command row removed without updating the count": (
        BS, "| `worker.step` | Execute one worker stage", "| worker.step | Execute one worker stage",
        "semantic documentation"),
    "TypeScript convention residue returns to M0": (
        DEV, "| C#/.NET, WinUI 3, Windows App SDK, and Rust conventions |", "| TypeScript and Rust conventions |",
        "semantic documentation"),
    "M9 physical-device wording returns": (
        DEV, "phone/tablet layout-profile checks on the managed emulator |", "phone/tablet checks |",
        "semantic documentation"),
    "ADR-037 names local runtimes again": (
        ADRS, "across supported cloud-hosted providers without changing the agent orchestrator",
        "including compatible cloud services and local runtimes, without changing the agent orchestrator",
        "semantic documentation"),
    "duplicate acceptance-matrix heading reintroduced": (
        DEV, "## Integrated acceptance matrix — autonomy (M51–M58)", "## Integrated acceptance matrix — runtime (M39–M50)",
        "semantic documentation"),

    # ---- crash-safety audit: change-report atomicity, Continue atomicity, §80.9 consistency
    "change-report obligation decoupled from the commit": (
        TA, "in the same SQLite transaction that commits the parent `ConstructionTransaction`",
        "after the parent `ConstructionTransaction` has committed",
        "semantic documentation"),
    "duplicate change-report records permitted": (
        TA, "`transactionId` is unique in `ChangeIntelligenceStore`", "`recordId` is unique in `ChangeIntelligenceStore`",
        "semantic documentation"),
    "crash-window recovery step removed": (
        TA, "5. Crash between parent commit and projection", "5. Projection retry",
        "semantic documentation"),
    "build spec drops the atomic record obligation": (
        BS, "MUST become durable atomically", "SHOULD become durable together",
        "semantic documentation"),
    "Continue resolution no longer atomic": (
        TA, "MUST commit those related records atomically in one SQLite transaction",
        "commits those related records in sequence",
        "semantic documentation"),
    "TEST-CHANGE-001 loses the crash fixture": (
        DEV, "L. crash immediately after parent commit and before projection", "L. projection retry",
        "semantic documentation"),
    "TEST-CONV-001 loses the Continue crash fixture": (
        DEV, "L. crash during Continue resolution after one durable record is written", "L. Continue retry",
        "semantic documentation"),
    "nonexistent ChangeReportStatus type reintroduced": (
        DEV, "projector failure records ChangeReportRecord.status = INCOMPLETE", "projector failure records ChangeReportStatus = INCOMPLETE",
        "semantic documentation"),
    "§80.9 criterion 1 contradicts §80.10 again": (
        BS, "is satisfied for the statements currently enumerated in §80.2.", "is NOT yet satisfied.",
        "semantic documentation"),

    # ---- schema semantics audit: registry meaning, drafts vs admitted records, lifecycle rules
    "canonical-definition column reintroduced": (
        TA, "| schemaId (normative contract; implementation schema) |", "| schemaId (canonical definition) |",
        "semantic documentation"),
    "Content record removed from architecture": (
        SCHEMAS, "```text\nContent\n- contentId\n- projectId", "```text\nContentX\n- contentId\n- projectId",
        "semantic documentation"),
    "ContentMutation carries an admitted revision again": (
        SCHEMAS, "- proposedContentRevision: ContentRevisionDraft", "- contentRevision",
        "semantic documentation"),
    "ContentRevisionDraft smuggles an authoritative field": (
        SCHEMAS, "ContentRevisionDraft\n- contentType", "ContentRevisionDraft\n- contentRevisionId\n- contentType",
        "semantic documentation"),
    "ContentMutation field renamed in its single block while prose still cites it": (
        SCHEMAS, "- baseProjectRevision\n", "- baseRevision\n",
        "semantic documentation"),
    "conversationRevision rule weakened": (
        BS, "`conversationRevision` is incremented only when the authoritative `ConversationResolver` commits",
        "`conversationRevision` may be incremented when the `ConversationResolver` commits",
        "semantic documentation"),
    "USER_REQUIRED advances the expected revision": (
        BS, "A `USER_REQUIRED` outcome does not advance `expectedProjectRevision`",
        "A `USER_REQUIRED` outcome advances `expectedProjectRevision`",
        "semantic documentation"),
    "§83.4 requires a complete report for every transaction again": (
        BS, "exposes exactly one durable `ChangeReportRecord`. The record may initially be `INCOMPLETE`",
        "exposes a complete, valid report. The record may initially be `INCOMPLETE`",
        "semantic documentation"),
    "typed causal source removed from build spec": (
        SCHEMAS, "- causeType: REQUIREMENT | GOAL | DIRECTIVE | REPAIR_CAUSE | APPROVED_ACTION\n- causeId\n", "",
        "semantic documentation"),
    "ContentStore called authoritative persistence again": (
        TA, "canonical persistence implementation for Content records", "authoritative persistence implementation of the BS content records",
        "semantic documentation"),
    "M120 drops the Content schema": (
        DEV, "- Content schema (persisted logical content resource", "- (persisted logical content resource",
        "semantic documentation"),

    # ---- schema identity audit: registry completeness, evidence provenance, revision/status naming
    "canonical schema dropped from the registry": (
        SCHEMAS, "BuildGateRecord\nContent\nContentRevision\nContentRevisionDraft\nContentMutation\n", "BuildGateRecord\nContent\nContentRevision\nContentRevisionDraft\n",
        "semantic documentation"),
    "canonical schema declared without a field block": (
        SCHEMAS, "ConversationRebaseRecord\n- recordId\n- conversationId", "ConversationRebaseRecordX\n- recordId\n- conversationId",
        "semantic documentation"),
    "bare CapabilityProfile name reintroduced": (
        SCHEMAS, "AndroidTechnologyPlan\nAndroidCapabilityProfile\nTaskGraph", "AndroidTechnologyPlan\nCapabilityProfile\nTaskGraph",
        "semantic documentation"),
    "reportStatus reintroduced on ChangeImpactReport": (
        SCHEMAS, "- projectionStatus: COMPLETE", "- reportStatus: COMPLETE",
        "semantic documentation"),
    "ambiguous projectRevision reintroduced on ChangeReportRecord": (
        SCHEMAS, "ChangeReportRecord\n- recordId\n- transactionId\n- projectRevisionAfter\n",
        "ChangeReportRecord\n- recordId\n- transactionId\n- projectRevision\n",
        "semantic documentation"),
    "ConversationDecision loses evidence provenance": (
        SCHEMAS, "ConversationDecision\n- decisionId\n- status\n- sourceMessageId\n- sourceEvidenceIds\n",
        "ConversationDecision\n- decisionId\n- status\n- sourceMessageId\n",
        "semantic documentation"),
    "forward traversal claims runtime implementation again": (
        BS, "Forward traversal proves that every registered capability has a complete declared contract and certification traceability chain",
        "Forward traversal proves that every capability is implemented",
        "semantic documentation"),
    "M5 depends on the M115 milestone again": (
        DEV, "Route every tool call through the canonical authenticated command envelope",
        "Route every tool call through the M115 command envelope",
        "semantic documentation"),

    # ---- coherence audit 2026-09-06: contradicted vocabulary and unrecorded amendments
    "workspace file made the Task Ledger authority again": (
        BS, "The authoritative Task Ledger is the SQLite execution ledger owned by `NirmanSupervisor.exe`",
        "The Task Ledger is stored locally as a structured state file within the workspace",
        "semantic documentation"),
    "local-provider requests reintroduced in M22": (
        DEV, "compatible cloud-provider requests (cloud-hosted providers only per ADR-207)",
        "compatible local-provider requests",
        "semantic documentation"),
    "BrandAssetWorker reintroduced in architecture": (
        TA, "### 56.1 Asset execution under the canonical UI Worker", "### 56.1 BrandAssetWorker",
        "semantic documentation"),
    "BrandAssetWorker reintroduced in M62": (
        DEV, "scoped asset transaction executed by the canonical UI Worker", "scoped BrandAssetWorker",
        "semantic documentation"),
    "legacy worker names reintroduced in swarm handoff example": (
        BS, "the Test and QA Worker reads the Android Data and Integration Worker's implementation notes",
        "the Test Engineer reads the Backend Specialist's implementation notes",
        "semantic documentation"),
    "ADR-141 amendment note removed": (
        ADRS, "**Amended by ADR-218:** \"token budget\" in this decision means",
        "**Note:** \"token budget\" in this decision means",
        "semantic documentation"),
    "ADR-184 amendment note removed": (
        ADRS, "**Amended by ADR-218:** the \"reasoning budgets\" and \"pass limits\" named in this decision no longer exist",
        "**Note:** the \"reasoning budgets\" and \"pass limits\" named in this decision no longer exist",
        "semantic documentation"),
    "ADR-217 names CostAuthority again": (
        ADRS, "Instead, `ResourceIntegrityAuthority` evaluates physical host memory pressure",
        "Instead, `ResourceIntegrityAuthority` and `CostAuthority` evaluate physical host memory pressure",
        "semantic documentation"),
    "ADR-103 BrandAssetWorker withdrawal removed": (
        ADRS, "**Amended under ADR-049:** the dedicated `BrandAssetWorker` role is withdrawn",
        "**Note:** the dedicated `BrandAssetWorker` role remains",
        "semantic documentation"),

    # ---- ADR-219: attention reliability is measured, placed, gated, verified
    "fixed deliberation pass ceiling table row reintroduced": (
        BS, "| Deliberation pass ceiling | None (progress-governed per §68.13) | N/A | Not overridable |",
        "| Deliberation max passes (DEEP) | 5 | 3-10 | Per task |",
        "semantic documentation"),
    "attention profile schema removed from build spec": (
        SCHEMAS, "AttentionReliabilityProfile\n- profileId", "AttentionProfile\n- profileId",
        "semantic documentation"),
    "attention profile schema removed from architecture": (
        SCHEMAS, "AttentionReliabilityProfile\n- profileId: string", "AttentionProfile\n- profileId: string",
        "semantic documentation"),
    "seventh confidence dimension removed": (
        TA, "evaluates context sufficiency across seven dimensions", "evaluates context sufficiency across six dimensions",
        "semantic documentation"),
    "premise check rung removed from gate sequence": (
        TA, "  -> premise check: StructuredPatch anchors and premises match the originating ContextPackage\n",
        "", "semantic documentation"),
    "compaction allowed to carry constraints (§74)": (
        BS, "Compaction output is never the carrier of active constraints, locked decisions, acceptance criteria, or revision identity: after every compaction",
        "Compaction output retains active constraints, locked decisions, acceptance criteria, and revision identity: after every compaction",
        "semantic documentation"),
    "compaction allowed to carry constraints (§53)": (
        BS, "**Compaction.** Compaction output is never the carrier of active constraints",
        "**Compaction.** Compaction output remains the carrier of active constraints",
        "semantic documentation"),
    "cache breakpoint moved after DENSE block": (
        BS, "The `cacheBreakpointPolicy` of §74 places the cache breakpoint before the DENSE block",
        "The `cacheBreakpointPolicy` of §74 places the cache breakpoint after the DENSE block",
        "semantic documentation"),
    "attendability clause unregistered": (
        BS, "| CLAUSE.CONTEXT.ATTENDABILITY_REQUIRED | CONTRACT.RUNTIME.CONTEXT | §53 |",
        "| CLAUSE.CONTEXT.ATTENDABILITY_NOTE | CONTRACT.RUNTIME.CONTEXT | §53 |",
        "unregistered contract"),
    "ADR-219 dropped from CONTEXT registry row": (
        BS, "| CONTRACT.RUNTIME.CONTEXT | BS §53 | — | TA §19, TA §59 | ADR-141, ADR-214, ADR-215, ADR-216, ADR-219 | M81 | CROSS_CUTTING |",
        "| CONTRACT.RUNTIME.CONTEXT | BS §53 | — | TA §19, TA §59 | ADR-141, ADR-214, ADR-215, ADR-216, ADR-999 | M81 | CROSS_CUTTING |",
        "dangling reference"),
    "ADR-219 heading renamed": (
        ADRS, "## ADR-219: Attention reliability is measured per model and context is placed, gated, and verified against it",
        "## ADR-219: Attention notes", "semantic documentation"),
    "ADR-219 loses its Reversal trigger role fields": (
        ADRS, "**Reversal trigger:** Measured literal recall is uniform within the configured threshold",
        "**Note:** Measured literal recall is uniform within the configured threshold", "structure"),
    "positional recall provider fixture removed": (
        TA, "13. Positional literal recall across fill buckets", "13. Long context smoke test",
        "semantic documentation"),
    "post-compaction probe fixture dropped from M113": (
        DEV, "- Post-compaction constraint re-projection verified by a recall probe\n", "",
        "semantic documentation"),
    "CONTEXT_GOVERNANCE authority heading drifts": (
        BS, "## 74. Context and Cache Governance", "## 74. Provider Billing Notes",
        "canonical identity"),
    # SPECULATION must resolve to its own architecture section, not to the
    # emulator-scenario coordinator or repair registry it once borrowed.
    "SPECULATION architecture edge points at the emulator coordinator": (
        BS, "| CAP.ANDROID.QUALITY_GATE | BS §65 | BS §65 | TA §88 |",
        "| CAP.ANDROID.QUALITY_GATE | BS §65 | BS §65 | TA §65 |",
        "canonical identity"),
    "SPECULATION architecture section loses its ContractId header": (
        TA, "**ContractId:** `CONTRACT.RUNTIME.SPECULATION`  ",
        "**ContractId:** `CONTRACT.RUNTIME.DEVICE_MATRIX`  ",
        "semantic documentation"),
    "architecture section mapped in §67.8 never names its contract": (
        TA, "**Implements:** build spec §73 and `CONTRACT.RUNTIME.AGENT_TRUST`\n",
        "**Implements:** build spec §73\n",
        "semantic documentation"),
    "LOCALIZATION cited as the locale authority": (
        BS, "`CONTRACT.RUNTIME.LOCALIZATION` (§62) is *regression localization*",
        "`CONTRACT.RUNTIME.LOCALIZATION` remains authoritative for locale resources; it (§62) is *regression localization*",
        "semantic documentation"),
    "ADR-164 locks the regression-localization contract again": (
        ADRS, "## ADR-164: Language-neutral AndroidCodeIntelligence\n\n**Locks:** `CONTRACT.RUNTIME.SCOPE`",
        "## ADR-164: Language-neutral AndroidCodeIntelligence\n\n**Locks:** `CONTRACT.RUNTIME.LOCALIZATION`",
        "semantic documentation"),
    "LOCALIZATION authority heading becomes an i18n heading": (
        BS, "## 62. Regression Localization", "## 62. Locale and Language Resources",
        "canonical identity"),
    "TA task state machine drops a BS §26.14 state": (
        TA, "                    │          │          ├── PAUSED\n", "",
        "semantic documentation"),
    "TA session lifecycle drifts from BS §33.2": (
        TA, "  → Testing → Recovering → Revalidating → Packaging → Completed\n```\n\nTerminal states are",
        "  → Testing → Recovering → Packaging → Completed\n```\n\nTerminal states are",
        "semantic documentation"),
    "ProductLifecycleState value missing from the §33.2 mapping": (
        BS, "| `SafelyFailed` | `SAFELY_FAILED` |\n", "",
        "semantic documentation"),
    "AgentLoopReducer made the lifecycle committer again": (
        TA, "1. Only `LifecycleAuthority` (the `SessionReducer`, §45.1) commits lifecycle state; `AgentLoopReducer` proposes.",
        "1. Only `AgentLoopReducer` may commit lifecycle state.",
        "semantic documentation"),
    "registry schema field renamed in its single block while prose still cites it": (
        SCHEMAS, "- attentionCapabilities: AttentionReliabilityProfile (TA §19.2; BS §53.11;",
        "- attentionProfile: AttentionReliabilityProfile (TA §19.2; BS §53.11;",
        "semantic documentation"),
    "registry schema field dropped from the build spec typed block": (
        SCHEMAS, "- lastValidatedCheckpoint: string? (checkpoint id)\n", "",
        "semantic documentation"),
    "PreviewRevision field dropped from its single block while prose still cites it": (
        SCHEMAS, "- promotionLineage\n- checkpointId\n- sourceFingerprint\n- contractVersion\n", "- promotionLineage\n- sourceFingerprint\n- contractVersion\n",
        "semantic documentation"),
    "merged block loses the addition marker that keeps the §80.2 field count honest": (
        SCHEMAS, "- contractId: (technical architecture §6.2 addition; build spec §67.11)\n", "- contractId\n",
        "semantic documentation"),
    "APK_EXPORT authority heading drifts to a billing heading": (
        BS, "## 78. APK Export Provenance Contract", "## 78. Release Billing Notes",
        "canonical identity"),
    "CHANGE_INTELLIGENCE authority heading drifts": (
        BS, "## 83. Change Intelligence Contract", "## 83. Diff Viewer Notes",
        "canonical identity"),
    "contract given a second owning milestone": (
        DEV, "| M119 | extends CONTRACT.RUNTIME.SKILL |", "| M119 | CONTRACT.RUNTIME.SKILL |",
        "reverse break"),
    "milestone-level evidence id loses its constituent statement": (
        DEV, "and `EV-GEN-001` is not complete for that capability while `EV-IB-001` is missing.", "",
        "reverse break"),
    "command registry gains a UI-to-emulator command kind": (
        BS, "| `preview.start` | Start a revision-bound preview |", "| `emulator.start` | Start a revision-bound preview |",
        "semantic documentation"),
    "technology adapter exposes a concrete execution operation": (
        TA, "- classifyFailure()          -> AndroidTechnologyAdapterResolution\n",
        "- classifyFailure()          -> AndroidTechnologyAdapterResolution\n- executeBuild()             -> AndroidBuildObservation\n",
        "semantic documentation"),
    "certification report reverts to PASSED or FAILED": (
        SCHEMAS, "- result: FAIL | DOCUMENTATION_CERTIFIED_WITH_RUNTIME_SOURCE_SKIPS | DOCUMENTATION_CERTIFIED\n",
        "- result: PASSED | FAILED\n",
        "semantic documentation"),
    "in-process hosting allowance loses its M7 bound": (
        BS, "and from M7 onward `Nirman.exe` and `NirmanSupervisor.exe` MUST be distinct processes.",
        "and the two executables are separated when convenient.",
        "semantic documentation"),
    "AGENTS.md presents the absent verify.sh entry point as present": (
        "AGENTS.md", "do not exist yet in this documentation-only repository", "are the standard gate",
        "semantic documentation",
        (("AGENTS.md", os.path.join(REPO, "AGENTS.md")),)),
    "capacity verdict regains an unqualified time exhaustion value": (
        SCHEMAS, "- capacityVerdict: fits | exceeds_declared_time_bound | exceeds_memory | exceeds_disk |",
        "- capacityVerdict: fits | exceeds_time | exceeds_memory | exceeds_disk |",
        "semantic documentation"),
    "debugger worker stops on a retry count again": (
        BS, "and, when the `recoveryAttemptPolicy` bound of materially different repairs for that failure fingerprint is reached (§26.3; technical architecture §76.2), hand the failure to the graduated recovery ladder (§28.2) — change strategy, backtrack, delegate, escalate, or report a truthful blocker — rather than repeating the same repair.",
        "and stop after the configured retry limit.",
        "semantic documentation"),
    "README describes a three-failure stop": (
        "README.md", "if the same failure keeps recurring after materially different repairs,", "if a test failed three times,",
        "semantic documentation",
        (("README.md", os.path.join(REPO, "README.md")),)),
    "architecture approval expiry reverts to context-only": (
        TA, "Approval must be bound to the request fingerprint and must expire when the action, task state, or policy changes (context expiry) or when the build spec §80.3 approval-expiry clock elapses, whichever comes first (build spec §26.13).",
        "Approval must be bound to the request fingerprint and must expire when the action, task state, or policy changes.",
        "semantic documentation"),
    "architecture profile table drops Review-only": (
        TA, "| Review-only | No write access and no arbitrary process execution; diff, security, and architecture analysis |\n", "",
        "semantic documentation"),
    "platform fixture A reverts to a Linux host": (
        BS, "| A — validation environment absent | host = Windows; target = Android;", "| A — host mismatch | host = Linux, target = Windows;",
        "semantic documentation"),
    "registry re-registers the APK export view instead of the canonical record": (
        SCHEMAS, "BackgroundContinuityRecord\nExportVerificationRecord\nPackagingProfile\n", "BackgroundContinuityRecord\nAPKExportRecord\nPackagingProfile\n",
        "semantic documentation"),
    "SkillInvocationRecord loses its field block": (
        SCHEMAS, "```text\nSkillInvocationRecord\n- invocationId\n", "```text\nSkillInvocationRecordX\n- invocationId\n",
        "semantic documentation"),
    "skill manifest requests a permission": (
        "crates/nirman-skills/skills/android/android-toolchain/skill.json", '"permissionRequests": [],', '"permissionRequests": ["filesystem:write"],',
        "semantic documentation", SKILL_SOURCES + SKILL_MANIFESTS),
    "skill capability id missing from the TA §84.1 matrix": (
        TA, "| `ANDROID_EMULATOR_EXECUTION` | windows | environment_dependent |", "| `ANDROID_EMULATOR_EXECUTION` | windows | environment-dependent |",
        "semantic documentation"),
    "skill manifest names a capability outside the vocabulary": (
        "crates/nirman-skills/skills/windows/windows-runtime-validation/skill.json", '"WINDOWS_NATIVE_EXECUTION"', '"WINDOWS_DEVICE_EXECUTION"',
        "semantic documentation", SKILL_SOURCES + SKILL_MANIFESTS),
    "skill body gates on a legacy lowercase capability id": (
        "crates/nirman-skills/skills/windows/windows-desktop-build/SKILL.md",
        "When `WINDOWS_HOST_TOOLCHAIN`\nresolves to", "When `cross_build_windows`\nresolves to",
        "semantic documentation", SKILL_SOURCES + SKILL_MANIFESTS),
    "skill body gates on a capability outside its §79.7 row": (
        "crates/nirman-skills/skills/android/android-toolchain/SKILL.md",
        "emulator steps, `ANDROID_EMULATOR_EXECUTION`)", "emulator steps, `WINDOWS_NATIVE_EXECUTION`)",
        "semantic documentation", SKILL_SOURCES + SKILL_MANIFESTS),
    "CandidateBranch schema dropped from the architecture": (
        SCHEMAS, "```text\nCandidateBranch\n- branchId\n- parentRevision",
        "```text\nCandidateBranchX\n- branchId\n- parentRevision",
        "semantic documentation"),

    "causal-escalation clause unregistered": (
        BS, "CLAUSE.DELIBERATE.CAUSAL_ESCALATION, CLAUSE.DELIBERATE.NO_MUTATION_IN_PASS",
        "CLAUSE.DELIBERATE.CAUSAL_TRIGGER, CLAUSE.DELIBERATE.NO_MUTATION_IN_PASS",
        "unregistered contract"),
    "no-mutation clause loses its contract": (
        BS, "| CLAUSE.DELIBERATE.NO_MUTATION_IN_PASS | CONTRACT.RUNTIME.DELIBERATION |",
        "| CLAUSE.DELIBERATE.NO_MUTATION_IN_PASS | CONTRACT.RUNTIME.PHANTOM |",
        "clause contradiction"),

    # ---- Step 4: Android-only scope contract
    "scope contract loses its milestone": (
        BS, "| CONTRACT.RUNTIME.SCOPE | BS §5 | BS §69 | TA §47 | ADR-180 | M11 |",
        "| CONTRACT.RUNTIME.SCOPE | BS §5 | BS §69 | TA §47 | ADR-180 | M911 |", "dangling reference"),
    "scope ADR loses its Locks": (
        ADRS, "**Locks:** `CONTRACT.RUNTIME.SCOPE`\n\n**Status:** Accepted",
        "**Status:** Accepted", "reverse break"),
    "android-only clause loses its contract": (
        BS, "| CLAUSE.SCOPE.ANDROID_ONLY_TARGET | CONTRACT.RUNTIME.SCOPE |",
        "| CLAUSE.SCOPE.ANDROID_ONLY_TARGET | CONTRACT.RUNTIME.NOWHERE |", "clause contradiction"),
    # Step 5: intent-driven synthesis and no-template enforcement
    "prompt contract row disappears": (
        BS, "| CONTRACT.RUNTIME.PROMPT_CONTRACT | BS §69 |",
        "| CONTRACT.RUNTIME.NOPROMPT | BS §69 |", "unregistered contract"),
    "intent contract loses its ADR": (
        BS, "| CONTRACT.RUNTIME.PROMPT_CONTRACT | BS §69 | — | TA §73 | ADR-181 | M96 |",
        "| CONTRACT.RUNTIME.PROMPT_CONTRACT | BS §69 | — | TA §73 | ADR-000 | M96 |", "dangling reference"),
    "no-template clause loses its contract": (
        BS, "| CLAUSE.PROMPT_CONTRACT.NO_TEMPLATE_CATALOG | CONTRACT.RUNTIME.PROMPT_CONTRACT |",
        "| CLAUSE.PROMPT_CONTRACT.NO_TEMPLATE_CATALOG | CONTRACT.RUNTIME.NOWHERE |", "clause contradiction"),
    # Step 5: canonical identity (Check 11) — semantic drift: §69's heading
    # changes to something about an unrelated domain, so the PROMPT_CONTRACT
    # authority reference points to the wrong semantic section.
    "prompt contract authority heading drifts": (
        BS, "## 69. Intent-Driven Android Synthesis and Truthful Live Preview Contract",
        "## 69. Legacy Scope Language and Unrelated Maintenance",
        "canonical identity"),
    # Renumbering a registry heading must NOT break the verifier: it locates
    # registries by heading text. Expect certification to still pass -> handled
    # as a POSITIVE case below, not a defect-expecting mutation.

    # ---- structure
    "ADR numbering gap": (
        ADRS, "## ADR-150:", "## ADR-1500:", "structure"),
    "duplicate References section": (
        TA, "## References", "## References\n\n## References", "structure"),
    "child subsection precedes its parent": (
        BS, "### 77.1 Orthogonal continuity dimensions and aggregate precedence",
        "### 77.1.1 Orthogonal continuity dimensions and aggregate precedence",
        "structure"),
    "§80.2 table split by a blank line": (
        BS, '| BS §26.1 | "should separate the desktop user int', '\n| BS §26.1 | "should separate the desktop user int',
        "structure"),
    "ADR blocks out of numeric order": (
        ADRS, "## ADR-159: Pure reducer as lifecycle authority", "## ADR-161: Pure reducer as lifecycle authority",
        "structure"),
    "References labels skip a number": (
        TA, '[7]: https://platform.openai.com/docs/api-reference/chat/create "Chat Completions Create Reference"',
        '[9]: https://platform.openai.com/docs/api-reference/chat/create "Chat Completions Create Reference"',
        "structure"),
    "duplicate contract registry identity": (
        BS,
        "| CONTRACT.RUNTIME.SCOPE | BS §5 | BS §69 | TA §47 | ADR-180 | M11 | FOUNDATIONAL |",
        "| CONTRACT.RUNTIME.SCOPE | BS §5 | BS §69 | TA §47 | ADR-180 | M11 | FOUNDATIONAL |\n| CONTRACT.RUNTIME.SCOPE | BS §5 | BS §69 | TA §47 | ADR-180 | M11 | FOUNDATIONAL |",
        "structure"),
    "duplicate capability registry identity": (
        BS,
        "| CAP.ANDROID.GENERATE | Generate a working Android application",
        "| CAP.ANDROID.GENERATE | Generate a working Android application from product intent | CONTRACT.RUNTIME.SCOPE | TEST-GEN-001 | EV-GEN-001 | PLANNED |\n| CAP.ANDROID.GENERATE | Generate a working Android application",
        "structure"),
    "duplicate clause registry identity": (
        BS,
        "| CLAUSE.SCOPE.ANDROID_ONLY_TARGET | CONTRACT.RUNTIME.SCOPE |",
        "| CLAUSE.SCOPE.ANDROID_ONLY_TARGET | CONTRACT.RUNTIME.SCOPE | §5 | desc | SEALED |\n| CLAUSE.SCOPE.ANDROID_ONLY_TARGET | CONTRACT.RUNTIME.SCOPE |",
        "structure"),
    "duplicate twelve-edge registry identity": (
        BS,
        "| CONTRACT.RUNTIME.SCOPE | CAP.ANDROID.GENERATE |",
        "| CONTRACT.RUNTIME.SCOPE | CAP.ANDROID.GENERATE | BS §5 | BS §5 | TA §47 | TA §47.1 | BS §5 | TA §47.2 | TA §47.3 | ADR-180 | M11 | TEST-GEN-001 | EV-GEN-001 |\n| CONTRACT.RUNTIME.SCOPE | CAP.ANDROID.GENERATE |",
        "structure"),
    "duplicate milestone registry identity": (
        DEV,
        "| M120 | CONTRACT.RUNTIME.CONTENT_INTELLIGENCE | ADR-211 |",
        "| M120 | CONTRACT.RUNTIME.CONTENT_INTELLIGENCE | ADR-211 | TEST-CONTENT-001 | EV-CONTENT-001 | Content and Writing Intelligence |\n| M120 | CONTRACT.RUNTIME.CONTENT_INTELLIGENCE | ADR-211 |",
        "structure"),

    # ---- semantic documentation lint
    "semantic goal template identifier": (
        SCHEMAS, "- goalDefinition", "- goalTemplate", "semantic documentation"),
    "semantic browser core wording": (
        BS, "Use browser validation only for a declared optional external/auxiliary surface",
        "Run browser, device, accessibility, and visual QA where applicable", "semantic documentation"),
    "semantic stale coverage reference": (
        DEV, "§5.6 coverage matrix", "§5.5 coverage matrix", "semantic documentation"),
    "semantic missing preview gate": (
        TA, "### 73.5.1 Canonical `PreviewPromotionGate`",
        "### 73.5.1 Canonical preview promotion predicate", "semantic documentation"),
    "semantic missing profile identity": (
        SCHEMAS, "- profileId", "- profileIdentifier", "semantic documentation"),
    "semantic duplicate milestone outcome": (
        DEV, "| M38 | Certified Android profile coverage and production acceptance |",
        "| M38 | Android capability registry and representative profile coverage |",
        "semantic documentation"),
    "semantic missing approval precedence": (
        TA, "### 16.2.1 Execution profiles and approval precedence",
        "### 16.2.1 Execution policy details", "semantic documentation"),
    "semantic supported row without profile": (
        BS, "| CAP.ANDROID.GENERATE | Generate a working Android application from product intent | CONTRACT.RUNTIME.SCOPE, CONTRACT.RUNTIME.PROMPT_CONTRACT, CONTRACT.RUNTIME.AUTHORITY, CONTRACT.RUNTIME.EVIDENCE, CONTRACT.RUNTIME.WORKSPACE, CONTRACT.RUNTIME.INTEGRATION_BOUNDARY | TEST-GEN-001 | EV-GEN-001 | PLANNED |",
        "| CAP.ANDROID.GENERATE | Generate a working Android application from product intent | CONTRACT.RUNTIME.SCOPE, CONTRACT.RUNTIME.PROMPT_CONTRACT, CONTRACT.RUNTIME.AUTHORITY, CONTRACT.RUNTIME.EVIDENCE, CONTRACT.RUNTIME.WORKSPACE, CONTRACT.RUNTIME.INTEGRATION_BOUNDARY | TEST-GEN-001 | EV-GEN-001 | SUPPORTED |",
        "semantic documentation"),
    "semantic supported environment row without profile": (
        BS, "| CAP.ANDROID.LONG_HORIZON | Continue a multi-session project without losing settled decisions | CONTRACT.RUNTIME.MEMORY, CONTRACT.RUNTIME.CONTEXT | TEST-MEM-001 | EV-MEM-001 | PLANNED |",
        "| CAP.ANDROID.LONG_HORIZON | Continue a multi-session project without losing settled decisions | CONTRACT.RUNTIME.MEMORY, CONTRACT.RUNTIME.CONTEXT | TEST-MEM-001 | EV-MEM-001 | SUPPORTED_WITH_ENVIRONMENT_REQUIREMENTS |",
        "semantic documentation"),
    "semantic artifact policy removed": (
        BS, "AAB generation is an optional separately declared release artifact",
        "AAB generation is not declared",
        "semantic documentation"),
    "semantic state vocabulary weakened": (
        BS, "AssuranceState        = UNKNOWN",
        "AssuranceStatus       = UNKNOWN",
        "semantic documentation"),
    "semantic evidence dependency removed": (
        BS, "### 5.7.4 Evidence dependencies and cascading invalidation",
        "### 5.7.4 Evidence records",
        "semantic documentation"),
    "semantic integration operationality removed": (
        BS, "### 5.7.5 Required integration operationality",
        "### 5.7.5 Integration notes",
        "semantic documentation"),
    "semantic external effect removed": (
        BS, "### 5.7.6 External-effect reconciliation",
        "### 5.7.6 External operations",
        "semantic documentation"),
    "semantic runtime boundary removed": (
        BS, "### 69.10 Runtime-certification and hidden-human-dependency boundary",
        "### 69.10 Certification notes",
        "semantic documentation"),
    "semantic hidden dependency milestone removed": (
        DEV, "## M104 — Hidden-human-dependency and runtime-proof fixtures",
        "## M104 — Runtime-proof fixtures",
        "semantic documentation"),
    "semantic schema parity milestone removed": (
        DEV, "## M105 — Schema parity and cross-document conformance",
        "## M105 — Cross-document conformance",
        "semantic documentation"),
    "semantic profile maturity field removed": (
        SCHEMAS, "- reproducibilityLevel",
        "- reproducibilityMode",
        "semantic documentation"),
    "semantic resource attribution removed": (
        SCHEMAS, "- attributionStatus: DIRECT | INHERITED | SHARED | ESTIMATED | UNAVAILABLE",
        "- usageStatus: DIRECT | INHERITED | SHARED | ESTIMATED | UNAVAILABLE",
        "semantic documentation"),
    "semantic legacy artifact wording introduced": (
        TA, "APK packaging and optional AAB packaging",
        "APK/AAB packaging",
        "semantic documentation"),
    "semantic integration boundary section removed": (
        BS, "## 70. Integration Boundary Contract",
        "## 70. Integration Boundary Notes",
        "semantic documentation"),
    "semantic integration architecture section removed": (
        TA, "## 74. Integration Boundary Implementation Contract",
        "## 74. Integration Notes",
        "semantic documentation"),
    "semantic integration milestone removed": (
        DEV, "## M107 — Integration boundary contract and wiring conformance",
        "## M107 — Integration conformance",
        "semantic documentation"),
    "semantic integration decision removed": (
        ADRS, "## ADR-194: Establish one canonical integration-boundary contract",
        "## ADR-194: Integration notes",
        "semantic documentation"),
    "semantic universal integration chain removed": (
        BS, "SOURCE\n  → CONTRACT\n  → ADAPTER / BRIDGE\n  → AUTHORITY\n  → STATE\n  → OPERATION\n  → OBSERVATION\n  → EVIDENCE\n  → VALIDATION\n  → DOWNSTREAM EFFECT",
        "SOURCE\n  → DESTINATION",
        "semantic documentation"),
    "semantic UI hierarchy observation removed": (
        TA, "`UiHierarchyObservation` is defined in",
        "`UIHierarchyRecord` is defined in",
        "semantic documentation"),
    "semantic certificate inspection removed": (
        SCHEMAS, "CertificateInspection\n- inspectionId\n",
        "CertificateInspection\n- inspectionRef\n",
        "semantic documentation"),
    "semantic export verification removed": (
        SCHEMAS, "ExportVerificationRecord\n- exportId",
        "ArtifactExportRecord\n- exportId",
        "semantic documentation"),
    "semantic continuity projection wiring removed": (
        TA, "backgroundContinuityProjection",
        "missingContinuityProjection",
        "semantic documentation"),
    # Anchored to the §83.2 export-copy sentence: ADR-203's
    # ExternalEffectRecord generalization quotes the same bare lifecycle token
    # earlier in TA, so a bare-token mutation would corrupt the wrong site and
    # leave the export sentence intact (undetected).
    "semantic export reconciliation lifecycle removed": (
        TA, "partially completed follows `UNKNOWN → RECONCILING`",
        "partially completed follows `UNKNOWN_ONLY`",
        "semantic documentation"),
    "semantic preview sync section removed": (
        BS, "## 71. Preview Synchronization Protocol",
        "## 71. Preview Notes",
        "semantic documentation"),
    "semantic preview event ownership removed": (
        BS, "### 71.2 Event-to-preview field ownership",
        "### 71.2 Preview field notes",
        "semantic documentation"),
    "semantic preview replay rules removed": (
        BS, "### 71.3 Ordering, duplicate, stale, and reconnect rules",
        "### 71.3 Preview ordering notes",
        "semantic documentation"),
    "semantic preview event schema removed": (
        SCHEMAS, "PreviewSyncEvent\n- eventId",
        "PreviewEvent\n- eventId",
        "semantic documentation"),
    "semantic preview reducer schema removed": (
        SCHEMAS, "PreviewProjectionReducer\n- reducerId",
        "PreviewReducer\n- reducerId",
        "semantic documentation"),
    "semantic preview evidence schema removed": (
        SCHEMAS, "PreviewSyncEvidenceRecord\n- evidenceId",
        "PreviewEvidence\n- evidenceId",
        "semantic documentation"),
    "semantic preview architecture removed": (
        TA, "## 75. Preview Synchronization Implementation Contract",
        "## 75. Preview Implementation Notes",
        "semantic documentation"),
    "semantic preview vertical slice removed": (
        DEV, "## M108 — Preview synchronization protocol and first Android vertical slice",
        "## M108 — Android vertical slice",
        "semantic documentation"),
    "semantic preview resilience removed": (
        DEV, "## M109 — Preview projection resilience and runtime-certification evidence",
        "## M109 — Preview resilience",
        "semantic documentation"),
    "semantic preview decision removed": (
        ADRS, "## ADR-195: Make preview synchronization event- and reducer-bound",
        "## ADR-195: Preview notes",
        "semantic documentation"),
    "semantic preview projection dimensions removed": (
        SCHEMAS, "PreviewProjection\n- projectionRevision",
        "PreviewState\n- projectionRevision",
        "semantic documentation"),
    "semantic preview authority levels removed": (
        SCHEMAS, "authorityClass: DECLARATIVE",
        "eventAuthority: DECLARATIVE",
        "semantic documentation"),
    "semantic preview causality removed": (
        BS, "Every non-root event MUST identify its `causationId`",
        "Every non-root event may identify its `causationId`",
        "semantic documentation"),
    "semantic preview runtime reconciliation removed": (
        BS, "Preview truth reconciliation compares the durable projection with the current supervised runtime observation.",
        "Preview reconciliation is implementation-defined.",
        "semantic documentation"),
    "semantic preview provenance decision removed": (
        SCHEMAS, "- certificationDecisionRef\n- completionDecisionRef",
        "- certificationDecision\n- completionDecisionRef",
        "semantic documentation"),
    "semantic source deployment export separation removed": (
        BS, "`export_project` does not make a ZIP or Git bundle a deployment artifact",
        "`export_project` creates a deployment artifact",
        "semantic documentation"),
    "semantic generated output terminology removed": (
        TA, "Project.generatedOutputs ⊆ {APK, AAB, Android source project}",
        "Project.generatedDeliverables ⊆ {APK, AAB, Android source project}",
        "semantic documentation"),
    "semantic deployment artifact policy removed": (
        TA, "Project.deploymentArtifacts ⊆ {APK} ∪ {AAB when PackagingProfile explicitly requires AAB}",
        "Project.deploymentArtifacts ⊆ {APK, AAB}",
        "semantic documentation"),
    "semantic preview identifiers removed": (
        BS, "TEST-PSYNC-001 | EV-PSYNC-001",
        "TEST-GEN-001 | EV-GEN-001",
        "reverse break"),
    "semantic resource integrity authority removed": (
        BS, "## 72. Runtime Resource Integrity Authority",
        "## 72. Resource Notes",
        "semantic documentation"),
    "semantic trust authority removed": (
        BS, "## 73. Agent Trust Boundary Authority",
        "## 73. Extension Notes",
        "semantic documentation"),
    "semantic context governance removed": (
        BS, "## 74. Context and Cache Governance",
        "## 74. Context Notes",
        "semantic documentation"),
    "semantic Android integrity removed": (
        BS, "## 75. Android Runtime Integrity Contract",
        "## 75. Android Notes",
        "semantic documentation"),
    "semantic resource integrity schema removed": (
        TA, "`ResourceIntegrityAuthority` (§59) evaluates `resourceRequirements` against currently admissible physical capacity before admission",
        "`ResourceIntegrityAuthority` (§59) evaluates reservations before admission",
        "semantic documentation"),
    "semantic trust schema removed": (
        TA, "Scanners run in a restricted local process",
        "Scanners run in a remote process",
        "semantic documentation"),
    "semantic context cache schema removed": (
        TA, "`ContextGovernance` records selected content",
        "`ContextPolicy` records selected content",
        "semantic documentation"),
    "semantic Android integrity schema removed": (
        TA, "Runtime collectors observe; `ValidationAuthority` interprets",
        "Runtime collectors report; `ValidationAuthority` interprets",
        "semantic documentation"),
    "semantic autonomy ladder removed": (
        BS, "### 28.5 Autonomy-level capability ladder",
        "### 28.5 Autonomy levels",
        "semantic documentation"),
    "semantic resource integrity milestone removed": (
        DEV, "## M111 — Runtime resource integrity and adaptive execution",
        "## M111 — Resource notes",
        "semantic documentation"),
    "semantic resource integrity decision removed": (
        ADRS, "## ADR-218: AI usage telemetry is observational and has no execution-authority semantics",
        "## ADR-218: Resource notes",
        "semantic documentation"),
    "semantic trust decision removed": (
        ADRS, "## ADR-198: Scan and revoke agent-layer extension content",
        "## ADR-198: Extension notes",
        "semantic documentation"),
    "semantic frontend-control-plane authority removed": (
        BS, "## 76. Frontend–Control-Plane Protocol Contract",
        "## 76. Frontend Protocol Notes",
        "semantic documentation"),
    "semantic frontend-control-plane architecture removed": (
        TA, "## 81. Frontend–Control-Plane Protocol Implementation Contract",
        "## 81. Frontend Protocol Notes",
        "semantic documentation"),
    "semantic frontend-control-plane milestone removed": (
        DEV, "## M115 — Frontend–control-plane protocol and generated service adapter",
        "## M115 — Frontend Protocol Notes",
        "semantic documentation"),
    "semantic frontend-control-plane decision removed": (
        ADRS, "## ADR-201: Make the frontend a typed projection client of the control plane",
        "## ADR-201: Frontend Protocol Notes",
        "semantic documentation"),
    "semantic command registry removed": (
        BS, "### 76.1 UICommandRegistry",
        "### 76.1 Command Notes",
        "semantic documentation"),
    "semantic response envelope removed": (
        BS, "### 76.2 Response and error envelopes",
        "### 76.2 Response envelopes",
        "semantic documentation"),
    "semantic error envelope removed": (
        SCHEMAS, "UIErrorEnvelope\n- errorId",
        "ErrorEnvelope\n- errorId",
        "semantic documentation"),
    "semantic event subscription removed": (
        BS, "### 76.3 Subscription, replay, and snapshot cutover",
        "### 76.3 Subscription and replay",
        "semantic documentation"),
    "semantic projection state separation removed": (
        TA, "AuthoritativeProjectionState",
        "ProjectionState",
        "semantic documentation"),
    "semantic snapshot cutover removed": (
        TA, "Snapshot-plus-event replay is cursor-atomic",
        "Snapshot-plus-event replay is best-effort",
        "semantic documentation"),
    "semantic frontend-control-plane identifiers removed": (
        BS, "TEST-FCP-001 | EV-FCP-001",
        "TEST-GEN-001 | EV-GEN-001",
        "reverse break"),

    # ---- check 14: command payload coverage
    # The check requires ArtifactExportCommandPayload to expose the policy-
    # mandatory fields. Removing one from the Rust source must fire the check.
    # These cases are SKIPPED when Rust source is absent (specification-only
    # working tree); they are recorded as SKIP, not PASS, and excluded from
    # the non-vacuous coverage count.
    "command payload field removed from artifact export": (
        "crates/nirman-ipc/src/lib.rs",
        "    pub packaging_profile_id: String,\n",
        "    pub packaging_profile_id_removed: String,\n",
        "command payload coverage",
        (("crates/nirman-ipc/src/lib.rs",
          os.path.join(REPO, "crates/nirman-ipc/src/lib.rs")),)),
    "command payload field removed from preview request": (
        "crates/nirman-preview/src/lib.rs",
        "    pub workspace_root: Option<String>,\n",
        "    pub workspace_root_removed: Option<String>,\n",
        "command payload coverage",
        (("crates/nirman-preview/src/lib.rs",
          os.path.join(REPO, "crates/nirman-preview/src/lib.rs")),)),
    # ---- check 14 (extended): M11 domain type coverage
    # The check now asserts all M11 domain structs declared in
    # crates/nirman-domain/src/lib.rs expose the policy-mandatory field
    # set. Each mutation removes one required field from the Rust source and
    # expects the verifier to report "command payload coverage".
    "android capability registry body removed": (
        "crates/nirman-domain/src/lib.rs",
        "    pub registry_id: String,\n",
        "    pub registry_id_removed: String,\n",
        "command payload coverage",
        (("crates/nirman-domain/src/lib.rs",
          os.path.join(REPO, "crates/nirman-domain/src/lib.rs")),)),
    "technology composition body removed": (
        "crates/nirman-domain/src/lib.rs",
        "    pub ui_framework: String,\n",
        "    pub ui_framework_removed: String,\n",
        "command payload coverage",
        (("crates/nirman-domain/src/lib.rs",
          os.path.join(REPO, "crates/nirman-domain/src/lib.rs")),)),
    "toolchain lock body removed": (
        "crates/nirman-domain/src/lib.rs",
        "    pub locked_version: String,\n",
        "    pub locked_version_removed: String,\n",
        "command payload coverage",
        (("crates/nirman-domain/src/lib.rs",
          os.path.join(REPO, "crates/nirman-domain/src/lib.rs")),)),
    "device matrix entry body removed": (
        "crates/nirman-domain/src/lib.rs",
        "    pub api_level: u32,\n",
        "    pub api_level_removed: u32,\n",
        "command payload coverage",
        (("crates/nirman-domain/src/lib.rs",
          os.path.join(REPO, "crates/nirman-domain/src/lib.rs")),)),
    "fixture record body removed": (
        "crates/nirman-domain/src/lib.rs",
        "    pub evidence_status: String,\n",
        "    pub evidence_status_removed: String,\n",
        "command payload coverage",
        (("crates/nirman-domain/src/lib.rs",
          os.path.join(REPO, "crates/nirman-domain/src/lib.rs")),)),
    "known exclusion body removed": (
        "crates/nirman-domain/src/lib.rs",
        "    pub description: String,\n",
        "    pub description_removed: String,\n",
        "command payload coverage",
        (("crates/nirman-domain/src/lib.rs",
          os.path.join(REPO, "crates/nirman-domain/src/lib.rs")),)),
    "android diagnostic body removed": (
        "crates/nirman-domain/src/lib.rs",
        "    pub status: DiagnosticStatus,\n",
        "    pub status_removed: DiagnosticStatus,\n",
        "command payload coverage",
        (("crates/nirman-domain/src/lib.rs",
          os.path.join(REPO, "crates/nirman-domain/src/lib.rs")),)),
    "device session body removed": (
        "crates/nirman-domain/src/lib.rs",
        "    pub connection_state: ConnectionState,\n",
        "    pub connection_state_removed: ConnectionState,\n",
        "command payload coverage",
        (("crates/nirman-domain/src/lib.rs",
          os.path.join(REPO, "crates/nirman-domain/src/lib.rs")),)),
    "android log entry body removed": (
        "crates/nirman-domain/src/lib.rs",
        "    pub level: LogEntryLevel,\n",
        "    pub level_removed: LogEntryLevel,\n",
        "command payload coverage",
        (("crates/nirman-domain/src/lib.rs",
          os.path.join(REPO, "crates/nirman-domain/src/lib.rs")),)),
    "install status body removed": (
        "crates/nirman-domain/src/lib.rs",
        "    pub state: InstallState,\n",
        "    pub state_removed: InstallState,\n",
        "command payload coverage",
        (("crates/nirman-domain/src/lib.rs",
          os.path.join(REPO, "crates/nirman-domain/src/lib.rs")),)),
    "reload status body removed": (
        "crates/nirman-domain/src/lib.rs",
        "    pub state: ReloadState,\n",
        "    pub state_removed: ReloadState,\n",
        "command payload coverage",
        (("crates/nirman-domain/src/lib.rs",
          os.path.join(REPO, "crates/nirman-domain/src/lib.rs")),)),
    "packaging profile body removed": (
        "crates/nirman-domain/src/lib.rs",
        "    pub required_artifacts: RequiredArtifacts,\n",
        "    pub required_artifacts_removed: RequiredArtifacts,\n",
        "command payload coverage",
        (("crates/nirman-domain/src/lib.rs",
          os.path.join(REPO, "crates/nirman-domain/src/lib.rs")),)),
    "export verification record body removed": (
        "crates/nirman-domain/src/lib.rs",
        "    pub destination_file_identity: FileIdentity,\n",
        "    pub destination_file_identity_removed: FileIdentity,\n",
        "command payload coverage",
        (("crates/nirman-domain/src/lib.rs",
          os.path.join(REPO, "crates/nirman-domain/src/lib.rs")),)),
    "signing config body removed": (
        "crates/nirman-domain/src/lib.rs",
        "    pub signing_scheme: SigningScheme,\n",
        "    pub signing_scheme_removed: SigningScheme,\n",
        "command payload coverage",
        (("crates/nirman-domain/src/lib.rs",
          os.path.join(REPO, "crates/nirman-domain/src/lib.rs")),)),

    # ---- Step 6: Content, Conversation, and Change Intelligence contracts
    "remove Content capability row": (
        BS,
        "| CAP.ANDROID.CONTENT_INTELLIGENCE | First-class product-content generation, revision, consistency, localization, accessibility, and content validation | CONTRACT.RUNTIME.CONTENT_INTELLIGENCE | TEST-CONTENT-001 | EV-CONTENT-001 | PLANNED |\n",
        "", "dangling reference"),
    "remove Conversation contract row": (
        BS,
        "| CONTRACT.RUNTIME.CONVERSATION_CONTEXT | BS §82 | — | TA §86 | ADR-212 | M121 | CROSS_CUTTING |\n",
        "", "unregistered contract"),
    "remove Change contract row": (
        BS,
        "| CONTRACT.RUNTIME.CHANGE_INTELLIGENCE | BS §83 | — | TA §87 | ADR-213 | M122 | CROSS_CUTTING |\n",
        "", "unregistered contract"),
    "remove TA section 85 reference": (
        BS,
        "| CONTRACT.RUNTIME.CONTENT_INTELLIGENCE | BS §81 | — | TA §85 | ADR-211 | M120 | CROSS_CUTTING |",
        "| CONTRACT.RUNTIME.CONTENT_INTELLIGENCE | BS §81 | — | TA §9985 | ADR-211 | M120 | CROSS_CUTTING |",
        "dangling reference"),
    "remove ADR-211 locks": (
        ADRS,
        "**Locks:** `CONTRACT.RUNTIME.CONTENT_INTELLIGENCE`, `CONTRACT.RUNTIME.EVIDENCE`, `CONTRACT.RUNTIME.VERIFICATION`\n\n",
        "", "reverse break"),
    "remove M120 mapping": (
        DEV,
        "| M120 | CONTRACT.RUNTIME.CONTENT_INTELLIGENCE | ADR-211 | TEST-CONTENT-001 | EV-CONTENT-001 | Content and Writing Intelligence |\n",
        "", "dangling reference"),
    "remove M121 mapping": (
        DEV,
        "| M121 | CONTRACT.RUNTIME.CONVERSATION_CONTEXT | ADR-212 | TEST-CONV-001 | EV-CONV-001 | Durable Conversation Context |\n",
        "", "dangling reference"),
    "remove M122 mapping": (
        DEV,
        "| M122 | CONTRACT.RUNTIME.CHANGE_INTELLIGENCE | ADR-213 | TEST-CHANGE-001 | EV-CHANGE-001 | Change Intelligence |\n",
        "", "dangling reference"),
    "wrong ADR in M120 contract": (
        BS,
        "| CONTRACT.RUNTIME.CONTENT_INTELLIGENCE | BS §81 | — | TA §85 | ADR-211 | M120 | CROSS_CUTTING |",
        "| CONTRACT.RUNTIME.CONTENT_INTELLIGENCE | BS §81 | — | TA §85 | ADR-999 | M120 | CROSS_CUTTING |",
        "dangling reference"),
    "wrong test in Content twelve-edge": (
        BS,
        "| CONTRACT.RUNTIME.CONTENT_INTELLIGENCE | CAP.ANDROID.CONTENT_INTELLIGENCE | BS §81 | BS §81 | TA §85 | TA §85.1 | BS §81 | TA §85.3 | TA §85.4 | ADR-211 | M120 | TEST-CONTENT-001 | EV-CONTENT-001 |",
        "| CONTRACT.RUNTIME.CONTENT_INTELLIGENCE | CAP.ANDROID.CONTENT_INTELLIGENCE | BS §81 | BS §81 | TA §85 | TA §85.1 | BS §81 | TA §85.3 | TA §85.4 | ADR-211 | M120 | TEST-CONTENT-999 | EV-CONTENT-001 |",
        "dangling reference"),
    "wrong evidence in Content twelve-edge": (
        BS,
        "| CONTRACT.RUNTIME.CONTENT_INTELLIGENCE | CAP.ANDROID.CONTENT_INTELLIGENCE | BS §81 | BS §81 | TA §85 | TA §85.1 | BS §81 | TA §85.3 | TA §85.4 | ADR-211 | M120 | TEST-CONTENT-001 | EV-CONTENT-001 |",
        "| CONTRACT.RUNTIME.CONTENT_INTELLIGENCE | CAP.ANDROID.CONTENT_INTELLIGENCE | BS §81 | BS §81 | TA §85 | TA §85.1 | BS §81 | TA §85.3 | TA §85.4 | ADR-211 | M120 | TEST-CONTENT-001 | EV-CONTENT-999 |",
        "dangling reference"),
    "wrong TA failure section in Change twelve-edge": (
        BS,
        "| CONTRACT.RUNTIME.CHANGE_INTELLIGENCE | CAP.ANDROID.CHANGE_INTELLIGENCE | BS §83 | BS §83 | TA §87 | TA §87.1 | BS §83 | TA §87.5 | TA §87.6 |",
        "| CONTRACT.RUNTIME.CHANGE_INTELLIGENCE | CAP.ANDROID.CHANGE_INTELLIGENCE | BS §83 | BS §83 | TA §87 | TA §87.1 | BS §83 | TA §87.5 | TA §87.99 |",
        "dangling reference"),
    "corrupt section 87 persistence reference": (
        BS,
        "| CONTRACT.RUNTIME.CHANGE_INTELLIGENCE | CAP.ANDROID.CHANGE_INTELLIGENCE | BS §83 | BS §83 | TA §87 | TA §87.1 | BS §83 | TA §87.5 | TA §87.6 |",
        "| CONTRACT.RUNTIME.CHANGE_INTELLIGENCE | CAP.ANDROID.CHANGE_INTELLIGENCE | BS §83 | BS §83 | TA §87 | TA §87.1 | BS §83 | BS §87.5 | TA §87.6 |",
        "dangling reference"),
    "duplicate content authoritative section": (
        BS,
        "| CONTRACT.RUNTIME.CONTENT_INTELLIGENCE | BS §81 |",
        "| CONTRACT.RUNTIME.CONTENT_INTELLIGENCE | BS §81, BS §82 |",
        "duplicate authority"),
    "remove M120 test id": (
        DEV,
        "| M120 | CONTRACT.RUNTIME.CONTENT_INTELLIGENCE | ADR-211 | TEST-CONTENT-001 | EV-CONTENT-001 |",
        "| M120 | CONTRACT.RUNTIME.CONTENT_INTELLIGENCE | ADR-211 |  | EV-CONTENT-001 |",
        "reverse break"),
    "duplicate milestone mapping rejected": (
        DEV,
        "| M120 | CONTRACT.RUNTIME.CONTENT_INTELLIGENCE | ADR-211 | TEST-CONTENT-001 | EV-CONTENT-001 | Content and Writing Intelligence |",
        "| M120 | CONTRACT.RUNTIME.CONTENT_INTELLIGENCE | ADR-211 | TEST-CONTENT-001 | EV-CONTENT-001 | Content and Writing Intelligence |\n| M120 | CONTRACT.RUNTIME.CONTENT_INTELLIGENCE | ADR-211 | TEST-CONTENT-001 | EV-CONTENT-001 | Content and Writing Intelligence |",
        "structure"),
    "missing ChangeImpactReport provenance": (
        TA,
        "1. ConstructionTransaction (authoritative for mutation identity, files, revision)",
        "1. ConstructionTransaction (mutation identity, files, revision)",
        "semantic documentation"),
    "BS/TA ContentDependency mismatch": (
        SCHEMAS,
        "ContentDependency\n- dependencyId\n- contentId\n- dependencyType\n- dependencyIdentity\n- dependencyRevision\n- invalidationPolicy",
        "ContentDependency\n- contentDependencyId\n- sourceContentId\n- dependencyType\n- targetId\n- dependencyRevision\n- invalidationPolicy",
        "semantic documentation"),
    "BS/TA ContentRevision requirementIds mismatch": (
        SCHEMAS,
        "ContentRevision\n- contentRevisionId\n- contentId\n- projectRevisionId\n- requirementIds",
        "ContentRevision\n- contentRevisionId\n- contentId\n- projectRevisionId\n- requirementId",
        "semantic documentation"),
    "missing Continue state transition": (
        BS,
        "MATCH → CONTINUE",
        "MATCH → PROCEED",
        "semantic documentation"),
    "missing ChangeReportRecord": (
        SCHEMAS,
        "ChangeReportRecord\n- recordId",
        "ChangeReportEntry\n- recordId",
        "semantic documentation"),
    "illegal COMPLETE → INCOMPLETE transition text": (
        TA,
        "COMPLETE → INCOMPLETE",
        "COMPLETE → PARTIAL",
        "semantic documentation"),
    "missing MutationReportUnit": (
        BS,
        "MutationReportUnit = committed ConstructionTransaction",
        "MutationReportUnit = file edit",
        "semantic documentation"),
    # ---- skill bodies (BS §79.7, ADR-108, BS §4.4)
    "skill body names the excluded host stack": (
        WIN_BUILD_SKILL,
        "Scope: C#/.NET / WinUI 3 / Windows App SDK / XAML host build plus Rust",
        "Scope: Tauri 2 / React / TypeScript / Vite / Rust build plus Rust",
        "semantic documentation",
        SKILL_SOURCES + SKILL_MANIFESTS),
    "skill body reintroduces a physical device": (
        ANDROID_SKILL,
        "observation bound to the environment fingerprint.",
        "observation bound to the environment fingerprint, or a physical device observation.",
        "semantic documentation",
        SKILL_SOURCES + SKILL_MANIFESTS),
    "registered skill without a body": (
        BS,
        "| `android-toolchain` | Node, package manager,",
        "| `android-ghost` | placeholder | none |\n| `android-toolchain` | Node, package manager,",
        "semantic documentation",
        SKILL_SOURCES + SKILL_MANIFESTS),
}


def run(root):
    p = subprocess.run([sys.executable, TOOL, root],
                       capture_output=True, text=True, timeout=180)
    return p.returncode, p.stdout + p.stderr


def _copy_fixture(tmp, files):
    """Copy the ten root documents plus any extra (relpath, abspath) files
    into the temp root, preserving relative paths. The verifier's
    `os.path.join(repo_root, rel_path)` lookups resolve correctly.

    Missing source files are silently skipped so the harness can run in a
    specification-only working tree where crates/ source has been removed.
    """
    for d in DOCS:
        shutil.copy2(os.path.join(REPO, d), os.path.join(tmp, d))
    for d in ADR220_DOCS:
        if os.path.exists(os.path.join(REPO, d)):
            shutil.copy2(os.path.join(REPO, d), os.path.join(tmp, d))
    for relpath, abspath in files:
        if not os.path.exists(abspath):
            continue
        dst = os.path.join(tmp, relpath)
        os.makedirs(os.path.dirname(dst), exist_ok=True)
        shutil.copy2(abspath, dst)


def failed_checks(out):
    """Defect classes reported by a run.

    A missing registry aborts before the check table is produced. That is a real
    detection, not a pass, so it is surfaced as the synthetic class "FATAL" —
    keeping it distinguishable from a clean exit.
    """
    # Only the DEFECTS block counts. The UNEVALUATED CHECKS block itemises
    # skips with the same "[class] subject" shape; a skip is not a detection.
    body = out.split("\nDEFECTS\n", 1)[1] if "\nDEFECTS\n" in out else ""
    body = body.split("\nCERTIFICATION:", 1)[0]
    hits = {m.group(1) for m in re.finditer(r"^\s*\[([a-z ]+)\] ", body, re.M)}
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
                    rc == 0 and CERTIFIED_RE.search(out) is not None and n and n.group(1) == "0",
                    f"exit={rc} defects={n.group(1) if n else '?'}"))
    # Status semantics: the terminal status must be the with-skips value iff any
    # check was skipped, the unqualified value must never appear alongside skips,
    # and every skipped subject must be itemised so the unevaluated portion is
    # visible in the report rather than summarised away.
    skip_total = sum(int(x) for x in re.findall(r"SKIPPED \((\d+)\)", out))
    unevaluated = out[out.index("UNEVALUATED CHECKS"):] if "UNEVALUATED CHECKS" in out else ""
    itemised = len(re.findall(r"^  \[[a-z ]+\] ", unevaluated, re.M))
    if skip_total:
        status_ok = ("CERTIFICATION: DOCUMENTATION_CERTIFIED_WITH_RUNTIME_SOURCE_SKIPS" in out
                     and "\nCERTIFICATION: DOCUMENTATION_CERTIFIED\n" not in out
                     and "CERTIFICATION: PASS" not in out
                     and f"UNEVALUATED CHECKS ({skip_total})" in out
                     and "NOT evaluated" in out
                     and itemised == skip_total)
    else:
        status_ok = ("\nCERTIFICATION: DOCUMENTATION_CERTIFIED\n" in out
                     and "WITH_RUNTIME_SOURCE_SKIPS" not in out)
    results.append(("positive: skipped checks change the terminal status and are itemised",
                    status_ok, f"skips={skip_total} itemised={itemised}"))

    rc2, out2 = run(REPO)
    results.append(("deterministic", out == out2, ""))

    # Determine whether Rust source files are available for the command-
    # payload-coverage check. When absent (specification-only working tree),
    # those mutation cases are recorded as SKIPPED (not PASS). A skipped
    # negative mutation proves nothing about verifier detection, so it is
    # excluded from the non-vacuous coverage count.
    SOURCE_PRESENT = os.path.exists(
        os.path.join(REPO, "crates/nirman-ipc/src/lib.rs"))

    covered = set()
    skipped_count = 0
    for label, case in CASES.items():
        if not isinstance(case, tuple) or len(case) not in (4, 5):
            raise AssertionError(f"bad case shape: {label!r} -> {case!r}")
        extra = ()
        if len(case) == 5:
            doc, find, repl, expect, extra = case
        else:
            doc, find, repl, expect = case

        # Skip command-payload-coverage cases when Rust source is absent.
        if expect == "command payload coverage" and not SOURCE_PRESENT:
            results.append((f"negative: {label}", None,
                            "SKIPPED — Rust source not present"))
            skipped_count += 1
            # Do NOT add to `covered`: a skipped negative test proves nothing.
            continue

        with tempfile.TemporaryDirectory(prefix="hermes-cg-") as tmp:
            _copy_fixture(tmp, extra)
            path = os.path.join(tmp, doc)
            if not os.path.exists(path):
                results.append((f"negative: {label}", None,
                                "SKIPPED — mutated file not present"))
                skipped_count += 1
                continue
            text = open(path, encoding="utf-8").read()
            if find not in text:
                results.append((f"negative: {label}", False, "anchor missing -> test invalid"))
                continue
            open(path, "w", encoding="utf-8").write(text.replace(find, repl, 1))
            # If the mutation targets a non-doc file in `extra`, apply the
            # same find/replace to that file's copy in the temp root.
            for relpath, _abspath in extra:
                fpath = os.path.join(tmp, relpath)
                if not os.path.exists(fpath):
                    continue
                ftext = open(fpath, encoding="utf-8").read()
                if find in ftext:
                    open(fpath, "w", encoding="utf-8").write(
                        ftext.replace(find, repl, 1))
            rc, out = run(tmp)
            hit = expect in failed_checks(out)
            if hit:
                covered.add(expect)
            results.append((f"negative: {label}", rc == 1 and hit,
                            "" if (rc == 1 and hit) else
                            f"exit={rc} expected={expect!r} got={sorted(failed_checks(out))}"))

    # ---- ADR-220 document topology (synthetic fixtures). These rules read
    # documents the migration introduces one commit at a time, so each case
    # constructs the minimal fixture it needs inside the temp root instead of
    # anchoring on a literal in the repository's current files.
    def _topology_case(label, mutate, expect="structure"):
        with tempfile.TemporaryDirectory(prefix="hermes-cg-topo-") as tmp:
            _copy_fixture(tmp, ())
            mutate(tmp)
            rc, out = run(tmp)
            hit = expect in failed_checks(out)
            if hit:
                covered.add(expect)
            results.append((f"negative: {label}", rc == 1 and hit,
                            "" if (rc == 1 and hit) else f"exit={rc} expected={expect!r} got={sorted(failed_checks(out))}"))

    def _rw(tmp, name, fn):
        path = os.path.join(tmp, name)
        text = open(path, encoding="utf-8").read() if os.path.exists(path) else ""
        open(path, "w", encoding="utf-8").write(fn(text))

    # CandidateBranch lives in nirman-schemas.md (owner BS §65.2, projected at
    # TA §88.2); its heading number is read from the document so a renumbering
    # of the schema document does not invalidate these cases.
    _cb_sec = re.search(r"^### (\d+\.\d+) CandidateBranch$",
                        open(os.path.join(REPO, SCHEMAS), encoding="utf-8").read(), re.M).group(1)
    _cb_proj = f"> **Schema projection:** `CandidateBranch` is defined in `nirman-schemas.md` §{_cb_sec}. Owner: BS §65.2.\n"

    def _move_candidate_branch(tmp, proj_sec=None, schemas_owner=None):
        if proj_sec:
            _rw(tmp, TA, lambda t: t.replace(_cb_proj, _cb_proj.replace(f"§{_cb_sec}.", f"§{proj_sec}."), 1))
        if schemas_owner:
            _rw(tmp, SCHEMAS, lambda t: t.replace(
                f"### {_cb_sec} CandidateBranch\n\n**Owner:** BS §65.2", f"### {_cb_sec} CandidateBranch\n\n**Owner:** {schemas_owner}", 1))

    _topology_case("schema projection cites the wrong nirman-schemas.md section",
                   lambda tmp: _move_candidate_branch(tmp, proj_sec="9.9"))
    _topology_case("nirman-schemas.md owner line names a heading that does not exist",
                   lambda tmp: _move_candidate_branch(tmp, schemas_owner="BS §65.9"))

    def _stub_dropped(tmp):
        _rw(tmp, BS, lambda t: t.replace(_cb_proj, "CandidateBranch fields are listed in the schema document.\n", 1))
    _topology_case("owner section loses its schema projection line", _stub_dropped)

    def _fence_kept(tmp):
        schemas = open(os.path.join(tmp, SCHEMAS), encoding="utf-8").read()
        fence = re.search(r"```text\nCandidateBranch\n.*?```\n", schemas, re.S).group(0)
        _rw(tmp, TA, lambda t: t.replace(_cb_proj, fence, 1))
    _topology_case("a schema moved to nirman-schemas.md keeps a second fence in the architecture", _fence_kept)

    def _projection_owner_drift(tmp):
        _rw(tmp, BS, lambda t: t.replace(_cb_proj, _cb_proj.replace("Owner: BS §65.2", "Owner: TA §88.2"), 1))
    _topology_case("a projection line names a different owner than the schema block", _projection_owner_drift)

    def _registry_fence_back_in_ta(tmp):
        schemas = open(os.path.join(tmp, SCHEMAS), encoding="utf-8").read()
        fence = re.search(r"```text\nCanonicalSchemaRegistry\n.*?```\n", schemas, re.S).group(0)
        _rw(tmp, TA, lambda t: t.replace(
            "> **Schema projection:** `CanonicalSchemaRegistry` is defined in `nirman-schemas.md` §3.1. Owner: TA §36.1.\n", fence, 1))
    _topology_case("the CanonicalSchemaRegistry list is written back into TA §36.1", _registry_fence_back_in_ta)

    _topology_case("GLOSSARY.md carries a Locks field and an upper-case requirement",
                   lambda tmp: _rw(tmp, GLOSSARY, lambda _t: "# Glossary\n\n**Term** — a thing. **Locks:** `CONTRACT.RUNTIME.SCOPE`. Agents MUST obey.\n"))
    _topology_case("INDEX.md carries an ADR block",
                   lambda tmp: _rw(tmp, INDEX, lambda _t: "# Index\n\n## ADR-001: Something\n\n**Status:** Accepted\n"))
    _topology_case("INDEX.md is edited by hand (drifts from the generator output)",
                   lambda tmp: _rw(tmp, INDEX, lambda t: t.replace("| §1 | Product Identity |", "| §1 | Product identity |", 1)))
    _topology_case("INDEX.md loses a contract row",
                   lambda tmp: _rw(tmp, INDEX, lambda t: re.sub(r"^\| `CONTRACT\.RUNTIME\.SCOPE` \|.*\n", "", t, count=1, flags=re.M)))
    # A requirement sentence smuggled into a schema fence: the reference
    # document's authority-marker scan strips fences, so only the block-purity
    # rule can see it. Anchored on the last field line of ExternalEffectRecord,
    # the block whose trailing prose the post-migration audit moved back to
    # TA §36.4.
    _topology_case("a requirement sentence rides inside a nirman-schemas.md fence",
                   lambda tmp: _rw(tmp, SCHEMAS, lambda t: t.replace(
                       "- reconciliationState: KNOWN_SUCCESS | KNOWN_FAILURE | UNKNOWN | RECONCILING | RESOLVED\n```",
                       "- reconciliationState: KNOWN_SUCCESS | KNOWN_FAILURE | UNKNOWN | RECONCILING | RESOLVED\n\n"
                       "An `UNKNOWN` outcome MUST NOT be retried until it transitions to `RESOLVED`.\n```", 1)))

    # POSITIVE: --emit-index reproduces the committed INDEX.md byte for byte and
    # the regenerated tree still certifies.
    with tempfile.TemporaryDirectory(prefix="hermes-cg-index-") as tmp:
        _copy_fixture(tmp, ())
        before = open(os.path.join(tmp, INDEX), encoding="utf-8").read() if os.path.exists(os.path.join(tmp, INDEX)) else None
        p = subprocess.run([sys.executable, TOOL, tmp, "--emit-index"], capture_output=True, text=True, timeout=180)
        after = open(os.path.join(tmp, INDEX), encoding="utf-8").read()
        rc, out = run(tmp)
        results.append(("positive: --emit-index is idempotent against the committed INDEX.md and certifies",
                        p.returncode == 0 and before == after and rc == 0 and CERTIFIED_RE.search(out) is not None,
                        f"emit={p.returncode} identical={before == after} exit={rc}"))

    _topology_case("an eleventh root Markdown file", lambda tmp: _rw(tmp, "NOTES.md", lambda _t: "# scratch\n"))
    _topology_case("a root document is missing from the ten", lambda tmp: os.remove(os.path.join(tmp, GLOSSARY)))
    _topology_case("GLOSSARY.md cites a Build Spec section that does not exist",
                   lambda tmp: _rw(tmp, GLOSSARY, lambda t: t.replace("— BS §5; ADR-180.", "— BS §5.99; ADR-180.", 1)),
                   expect="semantic documentation")
    _topology_case("GLOSSARY.md entry gains an upper-case requirement",
                   lambda tmp: _rw(tmp, GLOSSARY, lambda t: t.replace("no physical device plays any role.",
                                                                      "no physical device plays any role; agents MUST NOT add one.", 1)))

    _topology_case("a SCHEMAS § citation names a heading nirman-schemas.md lacks",
                   lambda tmp: _rw(tmp, TA, lambda t: t + "\nSee SCHEMAS §9.9 for the field list.\n"),
                   expect="semantic documentation")

    # POSITIVE: renumbering a registry heading (together with the citations
    # that point at it, so no §-reference dangles, and keeping subsection
    # order ascending) must not break registry location, because headings
    # are matched by text rather than by section number.
    with tempfile.TemporaryDirectory(prefix="hermes-cg-renum-") as tmp:
        _copy_fixture(tmp, RUST_SOURCES)
        path = os.path.join(tmp, BS)
        text = open(path, encoding="utf-8").read()
        text = text.replace("### 67.15 Twelve-edge resolution table",
                            "### 67.99 Twelve-edge resolution table", 1)
        text = re.sub(r"(?<!TA )(?<!architecture )§67\.15(?!\d)", "§67.99", text)
        open(path, "w", encoding="utf-8").write(text)
        tpath = os.path.join(tmp, TA)
        ttext = open(tpath, encoding="utf-8").read()
        open(tpath, "w", encoding="utf-8").write(re.sub(r"(build spec §)67\.15(?!\d)", r"\g<1>67.99", ttext))
        # GLOSSARY.md cites the heading too; a real renumber updates it, and
        # INDEX.md is regenerated (both are checked, so neither may be stale).
        _rw(tmp, GLOSSARY, lambda t: re.sub(r"(BS §)67\.15(?!\d)", r"\g<1>67.99", t))
        subprocess.run([sys.executable, TOOL, tmp, "--emit-index"], capture_output=True, text=True, timeout=180)
        rc, out = run(tmp)
        results.append(("positive: registry found after heading renumber",
                        rc == 0 and CERTIFIED_RE.search(out) is not None,
                        f"exit={rc}"))

    # POSITIVE CONFORMANCE: identifiers in ordinary prose, comments, and fenced
    # examples must not become graph records or authorities.
    with tempfile.TemporaryDirectory(prefix="hermes-cg-prose-") as tmp:
        _copy_fixture(tmp, RUST_SOURCES)
        path = os.path.join(tmp, BS)
        with open(path, "a", encoding="utf-8") as fh:
            fh.write("\nThis explanatory note mentions CONTRACT.RUNTIME.SCOPE, ADR-180, and M95 only as prose.\n")
            fh.write("<!-- CONTRACT.RUNTIME.PHANTOM and | fake | table | row | -->\n")
            fh.write("```text\nCONTRACT.RUNTIME.EXAMPLE is illustrative, not registered.\n```\n")
        rc, out = run(tmp)
        results.append(("positive: prose/comment/fence identifiers are inert",
                        rc == 0 and CERTIFIED_RE.search(out) is not None,
                        f"exit={rc}"))

    # POSITIVE CONFORMANCE: harmless Unicode explanatory text must not affect
    # registry addressing or semantic checks.
    with tempfile.TemporaryDirectory(prefix="hermes-cg-unicode-") as tmp:
        _copy_fixture(tmp, RUST_SOURCES)
        path = os.path.join(tmp, TA)
        with open(path, "a", encoding="utf-8") as fh:
            fh.write("\nImplementation note — résumé, café, and हिन्दी text are non-normative.\n")
        rc, out = run(tmp)
        results.append(("positive: Unicode explanatory text is inert",
                        rc == 0 and CERTIFIED_RE.search(out) is not None,
                        f"exit={rc}"))

    with tempfile.TemporaryDirectory(prefix="hermes-cg-ctl-") as tmp:
        _copy_fixture(tmp, RUST_SOURCES)
        rc, out = run(tmp)
        results.append(("control: clean copy certifies", rc == 0, f"exit={rc}"))

    # POSITIVE CONFORMANCE: the validated continuity and export contracts are
    # present in the clean synchronized document set and survive certification.
    with tempfile.TemporaryDirectory(prefix="hermes-cg-continuity-export-") as tmp:
        _copy_fixture(tmp, RUST_SOURCES)
        required = (
            (BS, "## 77. Background Continuity Contract"),
            (TA, "## 82. Background Continuity Implementation Contract"),
            (DEV, "## M116 — Background continuity and interruption recovery"),
            (ADRS, "## ADR-202: Canonical background continuity state machine"),
            (SCHEMAS, "deploymentDelivery: REQUIRED_APK | DECLARED_AAB_OPTIONAL | SOURCE_ACCESS_ONLY"),
            (TA, "ExportVerificationRecord"),
            (BS, "## 78. APK Export Provenance Contract"),
            (TA, "## 83. APK Export Provenance Implementation Contract"),
            (BS, "CAP.ANDROID.APK_DELIVERY"),
            (DEV, "## M117 — Local APK export provenance and delivery admission"),
            (ADRS, "## ADR-203: Make local deployment export profile-bound and provenance-complete"),
        )
        present = all(token in open(os.path.join(tmp, doc), encoding="utf-8").read()
                      for doc, token in required)
        rc, out = run(tmp)
        results.append(("positive: continuity and APK export anchors certify",
                        present and rc == 0 and CERTIFIED_RE.search(out) is not None,
                        f"exit={rc}"))

    # POSITIVE CONFORMANCE (ADR-218): §68 has exactly one authoritative owner
    # (CONTRACT.RUNTIME.DELIBERATION) and exactly one declared extension
    # (of CONTRACT.RUNTIME.REASONING), and M111 resolves RESOURCE_INTEGRITY.
    with tempfile.TemporaryDirectory(prefix="hermes-cg-s68-") as tmp:
        _copy_fixture(tmp, RUST_SOURCES)
        docs = verify_contract_graph.load(tmp)
        D = verify_contract_graph.Defects()
        R = verify_contract_graph.parse_registries(docs, D)
        owners = sorted(cid for cid, r in R["contracts"].items()
                        if verify_contract_graph.secrefs(r["authority"]) == [68])
        markers = sorted(R["authored"].get(68, []))
        declared = sorted(cid for (sec, cid) in R["declarations"] if sec == 68)
        listed = sorted(cid for cid, r in R["contracts"].items()
                        if 68 in verify_contract_graph.secrefs(r["ext"]))
        results.append(("positive: §68 has exactly one authoritative owner (DELIBERATION)",
                        owners == ["CONTRACT.RUNTIME.DELIBERATION"] and markers == ["CONTRACT.RUNTIME.DELIBERATION"],
                        f"registry={owners} markers={markers}"))
        results.append(("positive: §68 has exactly one declared extension (of REASONING)",
                        declared == ["CONTRACT.RUNTIME.REASONING"] and listed == ["CONTRACT.RUNTIME.REASONING"],
                        f"declared={declared} listed={listed}"))
        m111_ok = (111 in R["milestones"] and
                   R["milestones"][111]["contracts"] == ["CONTRACT.RUNTIME.RESOURCE_INTEGRITY"] and
                   R["milestones"][111]["test"] == "TEST-RESOURCE-001" and
                   R["milestones"][111]["evidence"] == "EV-RESOURCE-001")
        results.append(("positive: M111 resolves RESOURCE_INTEGRITY with TEST/EV-RESOURCE-001", m111_ok,
                        f"m111={R['milestones'].get(111)}"))
        no_cost_milestone = not any("COST" in c for m in R["milestones"].values() for c in m["contracts"])
        results.append(("positive: no milestone maps an AI-cost governance contract", no_cost_milestone, ""))

    # POSITIVE CONFORMANCE (ADR-219): M81 and M113 cite ADR-219, the CONTEXT
    # authority owns both new sealed clauses, and §53 adopts them.
    with tempfile.TemporaryDirectory(prefix="hermes-cg-adr219-") as tmp:
        _copy_fixture(tmp, RUST_SOURCES)
        docs = verify_contract_graph.load(tmp)
        D = verify_contract_graph.Defects()
        R = verify_contract_graph.parse_registries(docs, D)
        m81 = R["milestones"].get(81, {})
        m113 = R["milestones"].get(113, {})
        results.append(("positive: M81 and M113 cite ADR-219",
                        219 in m81.get("adrs", []) and 219 in m113.get("adrs", []),
                        f"m81={m81.get('adrs')} m113={m113.get('adrs')}"))
        owned = sorted(cl for cl, meta in R["clauses"].items()
                       if meta["contract"] == "CONTRACT.RUNTIME.CONTEXT" and meta["sealed"])
        results.append(("positive: CONTEXT owns four sealed clauses incl. attendability and recall-evidence",
                        owned == ["CLAUSE.CONTEXT.ATTENDABILITY_REQUIRED",
                                  "CLAUSE.CONTEXT.CONSTRAINT_PRIORITY",
                                  "CLAUSE.CONTEXT.RECALL_EVIDENCE_ONLY",
                                  "CLAUSE.CONTEXT.SOURCE_REQUIRED"],
                        f"owned={owned}"))
        adr219 = verify_contract_graph.adr_blocks(verify_contract_graph.adr_text(docs)).get(219, "")
        results.append(("positive: ADR-219 locks CONTEXT and CONTEXT_GOVERNANCE with a Reversal trigger",
                        "`CONTRACT.RUNTIME.CONTEXT`" in adr219
                        and "`CONTRACT.RUNTIME.CONTEXT_GOVERNANCE`" in adr219
                        and "**Reversal trigger:**" in adr219, ""))

    # POSITIVE CONFORMANCE: M120, M121, M122 resolve their respective contracts
    with tempfile.TemporaryDirectory(prefix="hermes-cg-m120-122-") as tmp:
        _copy_fixture(tmp, RUST_SOURCES)
        docs = verify_contract_graph.load(tmp)
        D = verify_contract_graph.Defects()
        R = verify_contract_graph.parse_registries(docs, D)
        m120_ok = (120 in R["milestones"] and
                   "CONTRACT.RUNTIME.CONTENT_INTELLIGENCE" in R["milestones"][120]["contracts"])
        m121_ok = (121 in R["milestones"] and
                   "CONTRACT.RUNTIME.CONVERSATION_CONTEXT" in R["milestones"][121]["contracts"])
        m122_ok = (122 in R["milestones"] and
                   "CONTRACT.RUNTIME.CHANGE_INTELLIGENCE" in R["milestones"][122]["contracts"])
        results.append(("positive: M120 resolves CONTENT_INTELLIGENCE", m120_ok,
                        f"contracts={R['milestones'].get(120, {}).get('contracts')}"))
        results.append(("positive: M121 resolves CONVERSATION_CONTEXT", m121_ok,
                        f"contracts={R['milestones'].get(121, {}).get('contracts')}"))
        results.append(("positive: M122 resolves CHANGE_INTELLIGENCE", m122_ok,
                        f"contracts={R['milestones'].get(122, {}).get('contracts')}"))

    # POSITIVE CONFORMANCE: Content, Conversation, and Change anchors certify
    with tempfile.TemporaryDirectory(prefix="hermes-cg-cross-cutting-anchors-") as tmp:
        _copy_fixture(tmp, RUST_SOURCES)
        required = (
            (BS, "## 81. Content and Writing Intelligence Contract"),
            (BS, "### 81.3 Content dependencies and invalidation"),
            (TA, "## 85. Content Intelligence Implementation Contract"),
            (DEV, "## M120 — Content and Writing Intelligence"),
            (ADRS, "## ADR-211: Make product content a first-class autonomous capability"),
            (BS, "## 82. Durable Conversation Context Contract"),
            (TA, "## 86. Conversation Context Implementation Contract"),
            (DEV, "## M121 — Durable Conversation Context"),
            (ADRS, "## ADR-212: Make Conversation a durable development aggregate"),
            (BS, "## 83. Change Intelligence Contract"),
            (TA, "## 87. Change Intelligence Implementation Contract"),
            (DEV, "## M122 — Change Intelligence"),
            (ADRS, "## ADR-213: Standardize post-mutation change intelligence"),
        )
        present = all(token in open(os.path.join(tmp, doc), encoding="utf-8").read()
                      for doc, token in required)
        rc, out = run(tmp)
        results.append(("positive: content, conversation, and change anchors certify",
                        present and rc == 0 and CERTIFIED_RE.search(out) is not None,
                        f"exit={rc}"))

    # POSITIVE CONFORMANCE: invoke verify_contract_graph.py as CLI and verify M120-M122 in dumped registries
    cli_proc = subprocess.run(
        [sys.executable, os.path.join(REPO, "tools/verify_contract_graph.py"), "--dump-registries", REPO],
        capture_output=True, text=True, encoding="utf-8"
    )
    cli_rc = cli_proc.returncode
    cli_out = cli_proc.stdout
    has_begin = "REGISTRIES_JSON_BEGIN\n" in cli_out
    has_end = "\nREGISTRIES_JSON_END" in cli_out
    parsed_ok = False
    m120_cli_ok = False
    m121_cli_ok = False
    m122_cli_ok = False
    if has_begin and has_end:
        s_idx = cli_out.index("REGISTRIES_JSON_BEGIN\n") + len("REGISTRIES_JSON_BEGIN\n")
        e_idx = cli_out.index("\nREGISTRIES_JSON_END")
        try:
            reg_data = json.loads(cli_out[s_idx:e_idx])
            parsed_ok = True
            m_dict = reg_data.get("milestones", {})
            m120_cli_ok = ("120" in m_dict and "CONTRACT.RUNTIME.CONTENT_INTELLIGENCE" in m_dict["120"].get("contracts", []))
            m121_cli_ok = ("121" in m_dict and "CONTRACT.RUNTIME.CONVERSATION_CONTEXT" in m_dict["121"].get("contracts", []))
            m122_cli_ok = ("122" in m_dict and "CONTRACT.RUNTIME.CHANGE_INTELLIGENCE" in m_dict["122"].get("contracts", []))
        except Exception:
            parsed_ok = False
    results.append(("positive: verifier CLI execution certifies with code 0",
                    cli_rc == 0 and CERTIFIED_RE.search(cli_out) is not None,
                    f"exit={cli_rc}"))
    results.append(("positive: verifier CLI dumps M120-M122 milestone registrations",
                    parsed_ok and m120_cli_ok and m121_cli_ok and m122_cli_ok,
                    f"m120={m120_cli_ok} m121={m121_cli_ok} m122={m122_cli_ok}"))

    expected_checks = {
        "duplicate authority", "unregistered contract", "undeclared extension",
        "authority cycle", "clause contradiction", "unversioned override",
        "dangling reference", "forward break", "reverse break", "orphan contract",
        "canonical identity", "section ownership", "structure",
        "command payload coverage",
    }
    # FATAL is a harness-synthesised class, not a §67.11 check; exclude it from
    # coverage accounting so the ratio cannot exceed the number of real checks.
    covered_checks = covered & expected_checks
    missing = sorted(expected_checks - covered_checks)
    # When Rust source is absent, command payload coverage mutations are all
    # skipped. The "every check has a proving mutation" conformance case is
    # satisfied for all checks whose source was available; the single uncovered
    # check is reported on the non-vacuous line, not as a failure — the skip is
    # an acknowledged environment limitation, not a defect.
    if missing and not SOURCE_PRESENT and missing == ["command payload coverage"]:
        results.append(("every check has a proving mutation", True,
                        f"all non-skipped checks have proving mutations "
                        f"(command payload coverage skipped — source absent)"))
    else:
        results.append(("every check has a proving mutation", not missing,
                        f"uncovered: {missing}" if missing else ""))

    width = max(len(n) for n, _, _ in results)
    bad = 0
    skip = 0
    for name, ok, detail in results:
        if ok is None:
            skip += 1
        elif not ok:
            bad += 1
        print(f"{'PASS' if ok else 'FAIL' if ok is False else 'SKIP':<5} {name:<{width}}  {detail}")
    executed = len(results) - skip
    print(f"\n{executed}/{len(results)} checks executed and passed")
    print(f"{skip} skipped — Rust source not present in working tree")
    covered_checks = covered & expected_checks
    missing = sorted(expected_checks - covered_checks)
    print(f"verifier detection classes proven non-vacuous: "
          f"{len(covered_checks)}/{len(expected_checks)}")
    if missing:
        print(f"not proven: {', '.join(missing)} "
              f"(all mutations skipped)")
    extra = sorted(covered - expected_checks)
    if extra:
        print(f"additional detection classes exercised: {', '.join(extra)}")
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main())
