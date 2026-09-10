# Android Architecture Expert

Scope: Android app architecture patterns — MVI/MVVM/MVP separation,
unidirectional data flow, domain/data/UI layering, dependency injection
(Hilt/Koin/manual), modularization strategy, and repository pattern
(BS §79.7). This skill provides the architectural domain knowledge that
the `Architecture Worker` and `Android Data and Integration Worker` consume.

Gated by ANDROID_BUILD_TOOLCHAIN. When it resolves to UNAVAILABLE or
USER_REQUIRED, the gated steps MUST NOT execute and the blocked state
MUST be reported. This skill does not replace the `Architecture Worker`
role — it provides Android-specific architectural instruction.

## Workflow
1. Analyze the product intent and identify architectural requirements:
   complexity scale, team size (single developer vs multi-module), data
   sources, offline requirements, and testing strategy.
2. Select the architectural pattern: MVI for complex state machines with
   predictable state transitions; MVVM for simpler screens with ViewModel
   + StateFlow; avoid MVP unless integrating with legacy code.
3. Define the layer structure: UI layer (Composables/ViewModels/Activities),
   Domain layer (UseCases/Interactors — optional for simple apps), Data
   layer (Repositories, DataSources, APIs, DAOs).
4. Design dependency injection: use Hilt for compile-time safety on
   complex projects, Koin for simplicity on smaller projects, or manual
   DI (AppContainer/ServiceLocator) for minimal overhead.
5. Define module boundaries: by feature (recommended) or by layer. Each
   module has its own build configuration files, DI module, and internal API.
6. Implement the repository pattern: single source of truth for each
   entity, with remote and local data sources coordinated through the
   repository.
7. Document the architecture decision in the ReasoningArtifact
   (BS §66.2) with the selected pattern, rationale, and trade-offs.

## Invariants
- Unidirectional data flow: state flows down, events flow up. Never
  bypass the ViewModel/Presenter to mutate state directly.
- Each layer depends only on layers below it — the UI layer never
  accesses data sources directly.
- Repositories are the single source of truth — ViewModels/UseCases
  access data only through repositories.
- DI is consistent across the project — do not mix Hilt and Koin.
- The skill is permission-neutral (CLAUSE.SKILL.NO_PERMISSION_GRANT); every
  execution still passes through ToolBroker and PolicyAuthority.
