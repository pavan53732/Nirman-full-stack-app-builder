# Nirman Architecture Decision Log

## Purpose

This document records significant product and engineering decisions for Nirman. It prevents important choices from disappearing into chat history and makes future changes deliberate. A decision may be revised when new evidence appears, but the reason for revision must be recorded.

**Canonical ownership:** The Build Spec owns product contracts, invariants, and capability/contract registries. The Technical Architecture owns implementation schemas, protocols, and module boundaries; the field blocks of both documents are held in `nirman-schemas.md`, each under the section that owns it (ADR-220). The milestone document (`nirman-milestones.md`) owns sequencing, milestones, fixtures, and exit gates. The ADR document (`nirman-adrs.md`) owns accepted decisions, rationale, and supersession; `nirman-decisions.md` owns only the decision process. The README, `INDEX.md`, and `GLOSSARY.md` are explanatory only. AGENTS defines agent operating constraints only. The verifier certifies documentation and semantic checks only; it is never a runtime authority.

## Decision Status Values

| Status | Meaning |
|---|---|
| Proposed | Under discussion and not yet implemented |
| Accepted | Approved direction for implementation |
| Deferred | Intentionally postponed until a later milestone |
| Superseded | Replaced by a newer decision |
| Rejected | Considered and not selected |

---

## ADR format

Every ADR accepted from ADR-209 onward MUST carry a **Reversal trigger:** field in addition to Locks, Status, Decision, Rationale, and Consequences. The field states the specific observable evidence that would justify superseding the decision — a measurement, a failure mode, a capability change, or a contract conflict. It is not a statement of doubt; it is the condition under which revisiting is correct rather than churn.

A reversal trigger of "none foreseeable" is permitted where a decision is structural, but it must be stated explicitly rather than omitted.

Reversal triggers do not weaken a decision or make it provisional. An accepted ADR remains binding until an explicit superseding ADR is accepted per BS §67.12. The trigger records what should prompt that supersession; it never performs it.

ADR-001 through ADR-208 predate this requirement and are not retrofitted. Absence of the field in those records is not a defect.

---

## Decision records

The decision records themselves — ADR-001 through the current ceiling — live in `nirman-adrs.md`, in numeric order and exactly as accepted (ADR-220). This document governs how a decision is written, labelled, and reviewed; it holds no decision. A new ADR is appended to that document after the last record, follows the format above, and is cited everywhere by its `ADR-nnn` identifier, which never changes.

---

## Decision Review Rules

Every major change to the master specification, technical architecture, security model, or execution permissions should add or update a decision record. Rejected alternatives should remain documented when they explain an important trade-off.

A decision should be reviewed when a milestone exposes a failed assumption, a security test fails, a new operating-system constraint appears, or the product scope changes materially.
