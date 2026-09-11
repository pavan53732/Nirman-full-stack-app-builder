# Android App Shortcuts Expert

Scope: Launcher and assistant entry points — static, dynamic, and pinned
shortcuts through ShortcutManager, capability-based assistant entry, deep
link targets with back-stack correctness, shortcut limits and ranking, and
keeping shortcuts consistent with app state (BS §79.7). This skill provides the domain knowledge
that the `UI Worker` and `Android Data and Integration Worker` consume.

Gated by ANDROID_BUILD_TOOLCHAIN and ANDROID_SOURCE_ENGINEERING. When it resolves to UNAVAILABLE or USER_REQUIRED,
the gated steps MUST NOT execute and the blocked state MUST be
reported. This skill does not replace a worker role — it provides
domain-specific instruction that the worker executes within its
scoped asset transaction (BS §50).

## Workflow
1. Choose the shortcut kind: static for entry points that never change,
   dynamic for context-dependent destinations, and pinned only where the
   user explicitly asks to keep one.
2. Give every shortcut a stable id, a short and long label, and an icon
   that is a recognizable adaptive icon rather than a cropped launcher
   glyph.
3. Bind each shortcut to a deep link into a real destination, and make
   that destination stand alone: it must be reachable directly, with the
   correct back stack synthesized, not only by in-app navigation.
4. Keep dynamic shortcuts current: update or remove them as the
   underlying data changes, so a shortcut never points at something that
   no longer exists.
5. Respect the published limits on how many dynamic and pinned shortcuts
   the launcher will show, and rank the most likely destinations first,
   because the surplus is dropped.
6. Declare capabilities for assistant entry where a voice or assistant
   invocation is a natural way to reach the same destination, and map
   them to the same deep-link targets.
7. Verify on the Nirman-managed emulator: long-press to reveal
   shortcuts, launch each one cold, confirm the destination and the back
   stack, and confirm a stale shortcut is removed or updated.

## Invariants
- Every shortcut launches a destination that stands alone with a
  correct synthesized back stack.
- Shortcut ids are stable; a rebuilt id is a new shortcut and loses its
  rank and pin.
- Dynamic shortcuts track app state; a shortcut pointing at deleted
  data is removed, never left dangling.
- Published count limits are respected and the most likely destinations
  are ranked first.
- Shortcuts are verified by cold launch, not merely declared.
- The skill is permission-neutral (CLAUSE.SKILL.NO_PERMISSION_GRANT); every
  execution still passes through ToolBroker and PolicyAuthority.
