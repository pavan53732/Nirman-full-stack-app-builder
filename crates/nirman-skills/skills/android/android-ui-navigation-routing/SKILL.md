# Android UI Navigation and Routing

Scope: navigation and routing in a generated Android application — route structure and deep linking, back and up behaviour, state preserved across navigation, and guards on restricted destinations (BS §79.7).

## Trigger
A route is added or changed, a deep link opens the wrong place or nothing, back
behaviour surprises the user, or a restricted destination is reachable without its
condition.

## Required capabilities
- `HOST_TOOL_OBSERVATION`

## Preconditions
- The current EnvironmentCapabilityRecord is available and not stale; capability
  is read from the record, never assumed.
- The route table and the guard conditions per protected destination are known.

## Context requirements
- The route table, including parameters and their types.
- Which destinations require a condition to be met.
- Revision, artifact digest where one exists, and the environment fingerprint of
  the host, so every record binds to them.
- Which destinations are reachable from an external link.

## Allowed tools
- static_analyzer
- navigation_probe

## Procedure
1.  Review the route structure and confirm each route is reachable by a stable path and
   that parameters are typed rather than parsed ad hoc at the destination.
2. Verify every externally reachable destination resolves from a cold start, not only
   when the app is already running.
3. Verify a deep link to a destination that requires a condition: the condition is met,
   or the user is taken through it and then on to the destination, rather than dropped
   at a default screen.
4. Verify back behaviour: back leaves the app or the section only at the top level, and
   never traps the user in a loop between two screens.
5. Verify up behaviour differs from back where the hierarchy requires it, and that the
   two are not conflated.
6. Verify guard enforcement: a destination whose condition is unmet is not reachable by
   typing its route or restoring a saved one, and the user is told why.
7. Verify state survives navigation where the contract requires it, and that returning to
   a screen restores its scroll position and input rather than resetting.
8. Verify an unknown route shows a stated error with a way onward instead of a blank
   screen.

## Evidence
- Route inventory with parameter types and external reachability.
- Cold-start deep link results per externally reachable destination.
- Back and up behaviour per screen, including the top-level case.
- Guard results: each protected destination attempted without its condition.
- State preservation results for scroll position and input.
- Unknown-route behaviour.

- A record of each procedure step that executed, with the outcome observed and
  the step that produced it, bound to the source revision and the environment
  fingerprint; a step that ran and recorded nothing is not evidence that it
  succeeded.
- Invariant claims this skill must leave observable, each a statement the
  evidence above has to support:
  - Every externally reachable destination resolves from a cold start.
  - No destination is reachable without the condition that guards it.
  - Back navigation exits at the top level and never cycles between screens.
- Every claim reduced to an observable: what was seen, on which device or host,
  at which revision; never a statement of intent, and never an inference about
  target behaviour drawn from a host observation.

## Failure classification
- - BLOCKED — a required capability resolves to UNAVAILABLE or USER_REQUIRED;
  the gated steps MUST NOT execute and the blocked state MUST be reported.
- DEEP_LINK_UNRESOLVED — a link opened the wrong destination or nothing from a cold
  start.
- GUARD_BYPASSED — a protected destination was reached without its condition.
- BACK_TRAP — back navigation cycles between screens without exiting.
- STATE_LOST_NAVIGATING — returning to a screen reset state the contract preserves.

## Recovery
- A bypassed guard is enforced at the destination, not only by hiding the entry point
  that leads to it.
- An unresolved deep link is fixed in the route resolution, not by adding a redirect
  from the default screen after the fact.
- A back trap is fixed by correcting the stack, not by intercepting back and forcing an
  exit from an arbitrary depth.
- - One retry is permitted after a repair that materially changed the input; an
  identical action is never re-run against unchanged evidence.

## Output contract
Emits `NavigationResult` (§23 SkillPackage contract):

- routes: inventory with parameter types and external reachability
- deepLinks: cold-start resolution per destination
- guards: protected destinations attempted without their condition
- backAndUp: behaviour per screen, including the top level
- classification: one of the classes above, null on success
- evidenceRefs: identifiers of the captured navigation evidence

## Fixtures
- Deep link resolves from a cold start
- Protected destination refuses a direct route
- Back exits at the top level
- Returning to a screen restores scroll and input
- Unknown route shows an error with a way onward
- Capability UNAVAILABLE — blocked, nothing reviewed

This skill is permission-neutral (CLAUSE.SKILL.NO_PERMISSION_GRANT); every
execution still passes through ToolBroker and PolicyAuthority.
