---
id: SR-12159
title: "Code review of quire-spec-language PR #996: FR-134 registry filter by fairness kind"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@ad74f49e477767a0b97002d1c963c6e5965eb974; qsl-route/src/lib.rs, qsl-route/tests/it/fairness_kinds.rs, qsl-route/tests/it/main.rs"
review_set: subset
---
# Code review of quire-spec-language PR #996

## Summary

Ticket: QSL-534. Code review with the rust-review lane, scoped to
`git diff origin/main...HEAD` (3 files). Static review only: no cargo or make
was run, per the dispatch brief.

Examined:
- `BackendDescriptor`: the new `fairness_kinds` member, `with_fairness_kinds`,
  `fairness_kinds`, and both constructors (`new`, `descriptor_from_manifest`).
- `Registry::candidates` (now a delegate) and `Registry::candidates_for_fairness`.
- `CandidateSet`: the `fairness` side map, `fairness_kinds`,
  `missing_fairness_kinds`.
- `Registry::register` equality semantics with the new member.
- `qsl-route/tests/it/fairness_kinds.rs`: all six tests and their `#[trace]`
  tags.
- Context only: `qsl-route/src/request.rs` `items_from_requirements`,
  `qsl-semantics/src/check/temporal_clause.rs` (`CheckedTemporalClause::fairness`),
  `qsl-semantics/src/family/requirements.rs` (`Requirements`),
  `qsl-route/Cargo.toml` (ADR-011 layer R dependency table, TC-390), and
  QSpec FR-368 "Fairness kinds".

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | `descriptor_from_manifest` is documented as the sole manifest conversion entry point for linked and process providers (FR-288, C-28). It takes no `fairness_kinds` input and always builds an empty set. QSpec FR-368 makes `fairness_kinds` a manifest descriptor member. Nothing in the doc says the member is not read, or that the driver must chain `with_fairness_kinds`. No wrong result today: no production caller passes a non-empty fairness set, because `items_from_requirements` calls `candidates`. It is a latent silent filter, though. The first caller that passes an item's fairness set will drop every manifest-registered backend, EN-1 included, and report `Strong`/`Weak` as missing. Minimum fix: one doc line on `descriptor_from_manifest` naming the typed-builder path. Better fix: read the closed `{weak, strong}` label set and refuse an unknown label. | qsl-route/src/lib.rs:167-209 |
| FND-002 | low | The `register` doc still defines an equal repeat as "same identity, manifest digest and advertised pairs". Equality now also covers `fairness_kinds`, as it already covered origin and domains. A same-digest repeat that differs only in fairness kinds conflicts. `registrations_differing_in_fairness_kinds_conflict` tests that, but the doc omits it. | qsl-route/src/lib.rs:594-596 |

## Verdict

PASS, no major finding. The filter is sound and the API shape is right:

- **Every kind, never partial.** `partition(|d| required.is_subset(&d.fairness_kinds))`
  excludes a backend that lacks any required kind. `every_kind_in_the_fairness_set_must_be_advertised`
  fails if the filter became "any" instead of "every".
- **An empty requirement filters nothing.** `candidates` delegates with an
  empty set. `is_subset` of the empty set is always true, so every existing
  caller gets the same candidates as before. The output is still sorted, and
  `requested_backend` and the named/unknown arms are unchanged.
- **`missing_fairness_kinds`.** It is computed only when the filter emptied
  the set. It is the union of the kinds each capable backend lacks. It stays
  empty for backend absence, both unnamed and named-but-not-capable, so an
  FR-076 empty set stays distinct from a filtered one.
- **Equality change is safe.** Every descriptor on `main` is built with an
  empty set, so equal repeats stay equal (FR-290-AC-9) and no existing
  registration starts to conflict. A same-digest pair that differs only in
  kinds follows the existing one-refusal-per-digest path.
- **`Candidate` stays identity-only** (ADR-013 O-19). Fairness kinds sit in a
  `CandidateSet` side map keyed by `Candidate`. The keys are unique because
  `held` is keyed by `BackendId`.
- **Layering** (ADR-011 layer R, TC-390). No dependency was added.
  `FairnessKind` comes from `quire-semantic-value` (layer SV), which the crate
  already depends on.
- **Rust lane.** No new panic surface, `unsafe`, integer conversion or lock.
  `with_fairness_kinds` is `#[must_use]`. The `Vec`/`partition` allocations
  are per call over the registry size, which is fine.
- **`#[trace]` tags.** The two AC-1 tags can fail on the registry filter and
  on carrying kinds unchanged. The FR-134-AC-2 tag overclaims (the brief said
  AC-2 was untagged; it is tagged). That is recorded in SR-12160 FND-001 and
  not repeated here.
