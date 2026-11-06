# Untangle decisions

Workflow: https://raw.githubusercontent.com/caldoai/skills/refs/heads/main/skills/untangle-codebase/SKILL.md

Scope: final review and structural Rust refactor; preserve the existing interface and saved data. User selected both optional comment and test cleanup.

## 1. Immutable events and an owned workspace

Keep raw JSON private, derive projections directly from raw data, and expose read-only collections. A workspace owns append/replace/clear, revision, query caches, summaries, ordering, and correlation.

Problems addressed:
- Public raw JSON can disagree with its OnceLock projection.
- Public event/file/error vectors can bypass cache invalidation; same-length replacements are undetectable.
- CLI, sample, worker completion and restore each implement dataset commitment.
- Timestamp sorting is duplicated and puts unknown dates first in timelines.
- Slow-request semantics are repeated in filtering, summaries and labels.

## 2. Plain query and timeline vocabulary

Keep serializable filter/query types independent of Rune. Convert edit buffers into one query value at the application boundary; saved views persist that query with source/correlation excluded. Give inspector tabs and correlation kinds names and stable automation keys. Query changes invalidate pagination at one boundary.

Problems addressed:
- Filters/query keys duplicate TextEdit query/start/end strings.
- Save/load manually synchronizes those representations.
- Paging resets are scattered and absent for search/date edits.
- Correlation strings, optional field parameters, boolean timeline selection, and numeric inspector tabs encode conventions across files.
- Session request grouping lives in UI and searches accumulated groups quadratically.

## 3. Cohesive import lifecycle

One importer handles local file reading and batch formation for CLI and background imports. Jobs own cancellation, channels and completion/failure; workspace assigns committed source identities.

Problems addressed:
- App state knows reading, parsing, threads, token and channel internals.
- Empty and disconnected channels are conflated, leaving a perpetual pending import after unexpected worker exit.
- Source indices depend on the caller's dataset length at job creation.
- Cancellation tests inspect implementation fields rather than observable completion.

## 4. Explicit persistence boundary

Inject a Store, resolve legacy storage once, report corrupt/unreadable data, validate restored metadata, and use unique private temporary files for atomic saves. Rendering consumes saved-session availability rather than probing disk.

Problems addressed:
- State::default reads the real user's storage and swallows failures.
- UI knows session filenames and performs filesystem reads during every rebuild.
- A fixed temporary filename lets simultaneous app instances interfere.
- Persisted filter values are accepted without validation (including arithmetic-overflow-prone time windows).

## 5. Feature-owned presentation and production shell

Keep entrypoint launch-only; share the production AppSpec with runtime tests. Separate shared presentation, sidebar, explorer and inspector views. Centralize the palette and canonical logo asset; use explicit actual font sizes. Controls return styled elements, and callers attach real handlers.

Problems addressed:
- One UI file mixes controls, branding, domain grouping and unrelated views.
- Action pass-through wrappers and replaced no-op callbacks add no behavior.
- SVG/native logo definitions and color copies can drift.
- Font helpers silently clamp requested sizes.
- Import diagnostics permanently hide errors after the first 100.
- Runtime tests bypass production poll/key hooks and manually drive state.

## 6. Regression tests and comments

Retain tests with observable failure modes. Replace representation assertions (Rc allocation identity, cancellation callback counts, token/worker fields) with behavior checks. Use isolated temporary stores and shared runtime setup; retain tests for format handling, normalization, search, cancellation, persistence, cache freshness, scrolling and dialogs. Remove implementation-restating comments; retain external compatibility constraints.

## Intentional special cases retained

Field aliases and format detection represent accepted log inputs. Cooperative read/parse cancellation boundaries represent responsiveness requirements. Legacy storage fallback and Unix file permissions represent compatibility/platform requirements. The sample fixture remains an explicit demonstration mode. Headless input uses Rune's virtual animation clock; asynchronous tests wait for actual background completion with a deadline.

## Verification

The refactor is implemented. There were no pre-existing Git commits available for change-history analysis.

Follow-up reviews caught pagination missing dataset changes and an incomplete corrupt-view recovery path. Navigation now keys off workspace generation plus query, the application exposes only immutable workspace access, and repaired views are reloaded before editing. A duplicated sidebar activity predicate now uses the shared query rule.

Verification: 32 behavior-focused tests pass; strict Clippy and formatting pass; the headless suite renders eight PNG/tree pairs without Rune warnings. Both review agents found no remaining actionable structural or correctness issues after the final fixes.

Dependency direction: shell depends on application/UI/storage; application depends on workspace/query/importer/storage; workspace depends on query/log; importer and storage share the workspace batch representation. Log/query/workspace/storage/import code does not depend on Rune or presentation modules. Only the application adapter and shell/presentation layers know Rune controls.

The optional cleanup removed representation-only Rc identity checks, private token/worker assertions and parser callback-count bounds. Runtime tests now exercise production hooks with isolated stores, and all remaining tests cover observable behavior or the native/external logo consistency contract.
