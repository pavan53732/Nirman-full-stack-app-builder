# UI Form Validation

Scope: form validation in Nirman apps — when validation fires, whether messages are specific
and correctly placed, how server errors map back to fields, and whether a disabled
submission states why (BS §79.7).

## Trigger
A form is built or changed, or users report validation that fires too early, messages
that do not say what is wrong, or a submit button that is disabled with no explanation.

## Required capabilities
- `HOST_TOOL_OBSERVATION`

## Preconditions
- The current EnvironmentCapabilityRecord is available and not stale; capability
  is read from the record, never assumed.
- The validation rules per field are known, including which are enforced on the server
  and are therefore authoritative.

## Context requirements
- The fields, their rules, and which rules the server enforces.
- The server's error shape, so responses can be mapped to fields.
- Revision, artifact digest where one exists, and the environment fingerprint of
  the host, so every record binds to them.
- Whether the form is long enough for progressive validation to matter.

## Allowed tools
- static_analyzer
- ui_inspector

## Procedure
1. Confirm each rule is enforced on the server, and treat the client check as
   convenience rather than as the enforcement point.
2. Set when validation fires per field: not while the user is still typing the first
   time, and promptly once a field has been left or the form submitted.
3. Write each message so it says what is wrong and what would satisfy it, rather than
   naming the rule that failed.
4. Place each message adjacent to its field and associate it programmatically, so a
   screen reader announces it with the field.
5. Map server errors back to the field they concern, and surface any error that maps to
   no field at the form level rather than dropping it.
6. Verify the disabled submit state always states what is missing or invalid, rather
   than being inert with no explanation.
7. Verify focus moves to the first invalid field on a failed submit, and that the error
   is announced rather than only coloured.
8. Verify values the user entered survive a failed submit rather than being cleared.

## Evidence
- Rule inventory, with which rules the server enforces.
- Validation timing observed per field.
- Message text per rule, reviewed for specificity.
- Placement and programmatic association per message.
- Server-error mapping results, including unmapped errors.
- Disabled-state reason, and focus and announcement behaviour on failed submit.

## Failure classification
- - BLOCKED — a required capability resolves to UNAVAILABLE or USER_REQUIRED;
  the gated steps MUST NOT execute and the blocked state MUST be reported.
- CLIENT_ONLY_RULE — a rule is enforced only on the client.
- MESSAGE_UNSPECIFIC — a message names the rule rather than what is wrong.
- SERVER_ERROR_UNMAPPED — a server error reached no field and was dropped.
- UNEXPLAINED_DISABLED_STATE — submission is blocked with no stated reason.

## Recovery
- A client-only rule is enforced on the server as well, rather than trusting the client
  because the UI prevents the input.
- An unspecific message is rewritten to say what would satisfy the rule, not to restate
  the rule more politely.
- An unmapped server error is surfaced at the form level immediately, not logged for
  later while the user sees nothing.
- - One retry is permitted after a repair that materially changed the input; an
  identical action is never re-run against unchanged evidence.

## Output contract
Emits `FormValidationResult` (§23 SkillPackage contract):

- rules: inventory and whether the server enforces each
- timing: when validation fires per field
- messages: text per rule, reviewed for specificity
- mapping: server errors to fields, with unmapped ones named
- classification: one of the classes above, null on success
- evidenceRefs: identifiers of the captured form evidence

## Fixtures
- Server rejects what the client check rejects
- Message says what would satisfy the rule
- Server error maps to its field
- Unmapped server error surfaced at the form level
- Submit disabled with a stated reason
- Capability UNAVAILABLE — blocked, nothing reviewed

This skill is permission-neutral (CLAUSE.SKILL.NO_PERMISSION_GRANT); every
execution still passes through ToolBroker and PolicyAuthority.
