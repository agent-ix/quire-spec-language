---
id: SR-12335
title: "Spec review of the FR-290 amendments in QSL-977 (PR #1222)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language PR #1222 head c0c037a7e; spec/functional/FR-290-settle-plugin-results-as-typed-terminal-records.md diff against origin/main, judged against FR-071, FR-098, ADR-029 PL-7 and QSpec FR-331"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-290
    type: reviews
---
# SR-12335: Spec review of the FR-290 amendments in QSL-977 (PR #1222)

## Summary

The PR makes three amendments to FR-290's "Replay context". `make check-spec-validation` exits 0 and reports no FR-290 warning.

1. **`profile_selections` is empty.** Accepted. The lock's `profile_selections` holds one row per clause-profile law (`qsl-package/src/emit.rs` module doc, `split_laws`). Those rows are not FR-071 semantic profile selections. `ReplayRequest::decode` only checks each selection against the closed set `quire.profile.v1`, and the FR-098 recompile (`execute.rs` `recompile` / `run_spine`) never reads them. The existing request builder in `call_site.rs` also sends `[]`. Copying the lock rows would have made decode refuse with `unknown_profile`.
2. **`dependencies` carries the resolved library closure in ascending identity order.** Accepted. FR-098's recompile applies D-4 rules 6 and 7 against the recompiled `dependency_selections()`, which is the closure (`qsl-package/src/checked.rs:531`). It also needs every transitive library's source to resolve imports, as the existing test `transitive_only_library_source_reaches_replay` shows. Rule 1 requires strictly ascending identities, and the `BTreeMap<LibraryName, _>` order satisfies it. Measured end to end with A importing B importing C: the plugin request replays.
3. **`Encoding` refusal.** See FND-002.

The "declaration occurrence of a Boolean function only" rule is in the merged "Replay context" text. #1217 put it there, not this PR. I judged it against FR-098 and ADR-013 O-09:

- **Not too wide.** FR-098 `replay` decides only `Claim::Predicate`, which says a Boolean function returns `true`. A value-validity item's claim is about an application, not the owner's result. For `p(x) = x + 1 > 3`, `p(0)` is `false` while the claim holds, so predicate replay would wrongly refute. Every state-clause and `operation-contract` item is excluded (measured).
- **Not too narrow.**
  - A Boolean function with non-Boolean parameters is replayable, which is correct: `replay` converts each argument by its declared type.
  - A pure helper predicate's declaration item, such as `u(x) = h(x)`, is replayable and settles `refuted` at `x = 0` (measured). That is also correct.
  - Library functions are out: `CheckedGraph`'s function table holds the unit's own declarations, and `selected_function` names a function of the recompiled unit.
  - A Boolean function with an object parameter (for example `hasNext(n: M::Node)`) builds a request with no object environment. `replay` refuses it, so the item settles `inconclusive`/`replay_refused`. That is safe.
- QSpec FR-331 agrees. Canonical assignments carry one FR-181 typed value per parameter node, and a plugin proof is `trusted` (FR-331-AC-23).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The amended table now says "resolved library selection", but older text still says "lock `dependency_selections` entry". That text is step 3 of `request`'s order, the Behavior bullet ("a lock dependency has no resolved source"), and the `UnresolvedDependency` / `DependencySelection` rustdoc. Reword them to match the table. | FR-290 "Replay context" step 3, Behavior; plugin_replay.rs:41-48, 282-291 |
| FND-002 | low | The `Encoding` paragraph comes after the ordered list and contradicts step 4 ("the encoder can refuse nothing else"). It gives no FR-272 code: the code uses `invalid_runtime_input`/`invalid-value`, which the spec never states. The Behavior ordered-refusal bullet also omits it. The value test applies here: the path cannot be reached, because the ordinal is a `u32`, the limits are unbounded and `DuplicateKey` is caught first. So drop the public variant and treat an encoder refusal as an internal fault that settles `failed`. Otherwise, add it as step 5 with its code. | FR-290 "Replay context"; plugin_replay.rs:299-304 |
| FND-003 | low | AC-14's clause "`UnknownOccurrence` ... comes before `NotReplayable` for the same call" can never be tested. `NotReplayable` requires a known occurrence, so no single call can meet both conditions. The order that can be observed is `NotReplayable` before `DuplicateBoundKey` (SR-12333 FND-001), so the AC should state that order instead. | FR-290-AC-14 |

Verdict: the amendments and both assumptions are sound. There are no major or medium findings.
