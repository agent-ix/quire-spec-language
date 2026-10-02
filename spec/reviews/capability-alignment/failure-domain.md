---
id: SR-491
title: "failure-domain review of FR-057 capability-kind admission"
type: SpecReview
analysis: failure-domain
scope: "spec/functional/FR-057-admit-shared-capability-kinds.md, spec/functional/FR-036-link-composed-native-packages.md, spec/test-cases/TC-115-preserve-composed-admission-stages.md, spec/test-cases/TC-153-admit-exact-capability-kinds.md, spec/test-cases/TC-154-refuse-unsupported-capability-vocabulary-version.md, spec/test-cases/TC-155-keep-admission-backend-independent.md, spec/model-linking/tests.md, spec/spec.md"
review_set: all
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-057
    type: reviews
---

## Summary

Reviewed the #229 change at quire-spec-language commit cd4f71a
(`origin/main...HEAD`): new FR-057, amendments to FR-036, TC-115,
`spec/model-linking/tests.md` and `spec/spec.md`, and new TC-153, TC-154 and
TC-155. Checked against the #229 acceptance bullets, QSpec FR-290, AD-010,
AD-016 and the QSpec native diagnostic catalog
(`proposals/quire-v1/definitions/native-diagnostics.md`) at QSpec `origin/main`.

The owner rulings hold throughout. The six FR-290 labels are the only admitted
set, with no alias. Backend absence settles `unsupported` with a warning and
is never a refusal or hold. CG `negotiate_*` is the single negotiation point.
Other versions and unknown kinds are refused, never mapped. No finding asks for
a mapping or other compatibility path.

The gaps are unstated failure modes at the edges of the admission and
negotiation split. Label identity is "exact bytes", but the carrier is JSON,
and the spec does not say whether matching runs on raw or decoded bytes.
Duplicate requested pairs have no stated identity rule. A version-refused
carrier conflicts with FR-036's rule that every requested pair stays in the
report. `invalid_capability` is not in the closed diagnostic catalog. The
registry side has three gaps: what happens to a backend advertisement that is
not a v1 kind, which registry state one negotiation reads, and how routing
picks between two backends that both settle `supported`. Four low findings
cover the refused-pair aggregate rule, warning identity, the undefined "hold"
and the FR-290 revision that v1 denotes.

Verdict: ACCEPT WITH FINDINGS. No high findings. Seven medium and three low
findings, each fixable by adding sentences to FR-057 (and one TC step) without
changing an owner or a ruling.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Label byte identity is unclear on a JSON carrier. FR-057 matches "by exact byte equality" and serializes labels "as a JSON string". It does not say whether matching runs on the raw JSON token or on the decoded string. `"global-conformance"` decodes to an admitted label, and its raw bytes do not match. The spec also does not classify an empty string `""`, JSON `null`, or a non-string value (number, array, object) as `absent-kind` or `unknown-kind`. The refusal echoes "the exact received label bytes" with no length bound, so an arbitrarily long or non-UTF-8 label is copied into the diagnostic. Fix: state that matching runs on the decoded JSON string's UTF-8 bytes. State that a missing member is `absent-kind` and that `null`, `""` and any non-string value are `unknown-kind`, carrying the received JSON value. State that received label bytes are charged under FR-036's supplied-bytes limit. Add the escaped, empty, `null` and non-string cases to TC-153 step 3. | FR-057 §Spelling, identity and order, §Serialization and version, §Outputs · FR-057-AC-2 · TC-153 |
| FND-002 | medium | Duplicate requested pairs have no identity rule. FR-057 says a set of kinds "compares as a set" and that request order is the request index. It does not say what happens when one declaration carries the same kind twice, or twice with different `required` flags. The linker could keep both, merge them, or refuse one. Negotiation then settles and warns twice or once, and the aggregate rule can differ. The same gap applies to a carrier that lists one label twice. Fix: state the requested-pair key (declaration, kind), and state one rule, either that each duplicate is retained at its own index and settled separately, or that a repeated (declaration, kind) key is refused with a named cause. Add a TC-153 step for a duplicate pair and a duplicate with a conflicting `required` flag. | FR-057 §Spelling, identity and order, §Inputs · FR-036 requested-pair retention · FR-057-AC-4 |
| FND-003 | medium | A version-refused carrier conflicts with FR-036 retention, and the effect of a refused pair on the report is unstated. FR-057 refuses the whole carrier and "SHALL read none of its labels". FR-036 retains every requested pair in the request report and checks inventory against it. If the pairs arrive only in that carrier, there is no inventory to retain, and the spec does not say whether a request report exists. For a single refused pair, "A refused request enters no later stage" does not say that the pair stays in the report at its index, or how a refused optional pair affects the aggregate. Fix: state that an `unsupported-version` refusal is atomic for the carrier and produces no request report and no admitted pair. State that a pair refused `absent-kind` or `unknown-kind` stays in the request report at its request index with its refusal. State that a refused required pair makes complete aggregate success unavailable and that a refused optional pair does not. Add the optional-pair case to TC-153. | FR-057 §Serialization and version, §Outputs · FR-036 Behavior, FR-036-AC-6 · TC-154 |
| FND-004 | medium | `invalid_capability` is not a catalogued diagnostic code. The QSpec native diagnostic catalog (`quire.native.diagnostics/v1`, revision `1-draft.4`) is closed. It states that a new code or cause needs a new catalog revision, and that a reader refuses to interpret an unknown code. It has no `invalid_capability` and no `absent-kind`, `unknown-kind` or `unsupported-version` cause. Its only capability entry, `unsupported_projection`/`unsupported-requested-capability`, retains a selected backend. As written, #213 would emit a code that a strict catalog reader refuses. Fix: in FR-057 §Outputs and §Dependencies, name the catalog revision that adds `invalid_capability` with its three causes and their required payloads (received label or version, request index), and add that revision as a dependency. State that `unsupported_projection` is never produced at QSL admission. | FR-057 §Outputs, §Dependencies · QSpec `proposals/quire-v1/definitions/native-diagnostics.md` (Selection and compatibility; `unsupported_projection` row) |
| FND-005 | medium | Backend advertisement failure is unstated at the registration extension point. FR-057 says each backend "advertises the kinds it can discharge" under the AD-010 contract. It does not say what happens when an advertisement names a label outside v1 (for example `FiniteReplay`), declares another vocabulary version, repeats a kind, or is empty. The registry could accept the backend, drop the bad kind, or refuse the backend. Each choice changes which items settle `unsupported`. Fix: state that an advertisement is parsed by the same admission as a requested label, under `quire.capability-kind/v1`. State that an advertisement with an unknown kind or another version refuses that backend's registration with `invalid_capability`, and that the backend is then absent for every kind. That absence settles `unsupported` with the warning. Add the case to TC-155. | FR-057 §Stage ownership · QSpec AD-010, AD-016 · TC-155 |
| FND-006 | medium | The registry state that negotiation reads is not pinned. FR-057 says no stage waits for a registration and each item settles independently. It does not say whether one negotiation reads one registry snapshot. If a backend registers while a request is being negotiated, two items of the same kind can settle differently, and a rerun over the same inputs can give a different result. Fix: state that negotiation of one request reads one immutable registry snapshot. State that the snapshot identity (registered backend identities and their advertised kinds) is retained as assessment provenance, separate from static meaning. State that a later registration affects only later requests. | FR-057 §Stage ownership, §Absence, unsupported, refusal, timeout and hold · FR-057-AC-6 · FR-036 assessment provenance |
| FND-007 | medium | Target selection among several `supported` backends has no rule. FR-057 forbids dependence on registration order, display text and ambient state. It gives no positive rule for choosing between two registered backends that both settle an item `supported`. #229 scope asks the spec to define "which QSL component selects a target", and without a rule #185 must invent one. Fix: state the selection rule in FR-057, for example an explicit caller-selected backend identity when present and otherwise the lowest backend identity by byte order. Or state that more than one `supported` candidate with no caller selection is settled by a named, typed outcome that retains every candidate identity. Add a two-candidate case to TC-155. | FR-057 §Stage ownership · #229 Scope bullet 3 · TC-155 step 2 |
| FND-008 | low | Warning identity is unstated. FR-057 requires "a warning naming the kind's exact label" per item. It does not say whether two items with the same absent kind give two warnings or one, or which structured field carries the label. TC-155 assumes "a structured field". Fix: state one warning per `unsupported` item, carrying the item's request index and the kind label in a typed field, never only in message text. | FR-057 §Absence, unsupported, refusal, timeout and hold · FR-057-AC-6 · TC-155 |
| FND-009 | low | "Hold" is named but not defined. The case table covers absence, unknown kind, version, backend absence, unsupported claim, `requires-bound`, `invalid-request` and timeout, and the prose says several cases are "never a hold". No row says what a hold is or whether any stage may produce one. #229 asks for the distinction to be defined. Fix: add a row or sentence that defines a hold (settlement deferred to a later event) and states that no admission, negotiation or run stage in this requirement produces one. | FR-057 §Absence, unsupported, refusal, timeout and hold · #229 Scope bullet 4 |
| FND-010 | low | `quire.capability-kind/v1` does not name the FR-290 revision it denotes. FR-057 mints the version string in QSL, and FR-290 has no version of its own. TC-153 reads from a committed copy of the FR-290 Values table, and FR-057 does not say which revision that copy must be. If FR-290's Values change, v1 could denote two different sets. Fix: state that `quire.capability-kind/v1` denotes exactly the FR-290 Values table at a named QSpec revision, and that TC-153's copy is pinned to that revision. | FR-057 §Admitted vocabulary, §Dependencies · TC-153 step 1 · QSpec FR-290 |

## Method

- Checklist, applied to a vocabulary admission requirement and its split into
  admission, registration, negotiation and routing:
  - Trust boundaries: the requested-pair input, the serialized carrier, and
    backend advertisements at the AD-010 registration contract.
  - Entity identity: label byte identity on a JSON carrier, the requested-pair
    key, version identity and diagnostic code identity.
  - Evaluation purity: whether admission reads backend state (it must not, and
    FR-057 says so), and which registry state negotiation reads.
  - Topology: no graph algorithm is introduced. Ordering was checked instead:
    kind non-order, request index order and routing tie order.
- Settled inputs were not flagged: FR-290's six kinds as the authority, backend
  absence as warned `unsupported`, CG `negotiate_*` as the single negotiation
  point, QSL `Capability` as language admission only, refusal of any other
  version or unknown kind with no mapping, and the #213, #185, #222, #210 and
  #211 ownership split.
- Evidence read: `git diff origin/main...HEAD` at cd4f71a; FR-057, FR-036,
  TC-115, TC-153, TC-154 and TC-155 at HEAD; `src/linking/composed/requests.rs`
  (current four-member `Capability` at :36 and caller-declared `Backend`);
  QSpec `origin/main` FR-290, AD-016 (capability rows) and
  `proposals/quire-v1/definitions/native-diagnostics.md`. A search for
  `invalid_capability` across the QSL worktree and QSpec found it only in
  FR-057, TC-153 and TC-154.

## Round 2 dispositions

Checked against the current tree: cd4f71a plus the uncommitted edits. The
upstream is quire-specification `046d1bd`.

| Finding | Disposition | Evidence |
| --- | --- | --- |
| FND-001 | resolved | Matching runs on the decoded UTF-8 bytes (FR-057:95-96). A missing label or JSON `null` is `absent-kind` (FR-057:141-143); FR-290 fixes the `null` case, and FR-057 follows it. `""` and non-string values are `unknown-kind` (FR-057:147-150). Received bytes are charged to the FR-036 supplied-bytes limit (FR-057:114-115). TC-153 step 3 includes `""` and the number `1` (TC-153:25-27). |
| FND-002 | resolved | Duplicate pairs stay at their own indices with their own flags and are never merged (FR-057:109-112, AC-4 at FR-057:309). TC-153 step 5 covers this (TC-153:31-33, 53-55). A carrier that repeats a label is a carrier-format question for #211 (FR-057:136-137). |
| FND-003 | resolved | A refused pair stays at its index (FR-057:73-74). A refused carrier produces no request report (FR-057:76-77). A required refusal makes admission unavailable; an optional one does not (FR-057:155-157). TC-153 step 3 marks one refused pair required and one not (TC-153:27, 49-52). |
| FND-004 | resolved | `invalid_capability` and its causes are catalogued at `1-draft.5` (FR-057:87-91, 327-329), which matches `native-diagnostics.md` at `046d1bd`. `unsupported_projection` is never produced at admission (FR-057:89-91). |
| FND-005 | resolved | The registry admits advertised kinds under the same rules as a requested pair. It refuses an absent kind, an unknown kind, an unknown mode or a duplicate identity, and the refused backend advertises nothing (FR-057:212-219). TC-155 step 3 covers this (TC-155:31-33, 51-54). |
| FND-006 | resolved | Candidates are computed from one immutable snapshot, which is retained as provenance. A registration made during a request affects only later requests (FR-057:221-228). |
| FND-007 | resolved | There is no preference order. Several candidates with no named backend settle `ambiguous-backend` (FR-057:243, 252-253). TC-155 step 4 covers two capable backends (TC-155:34-36, 55-57). |
| FND-008 | resolved | Each backend-absence item gets one warning naming its kind and any named backend (FR-057:264). The typed payload, request index and kind are the FR-272 `unsupported-requested-capability` payload that FR-057 cites (FR-057:327-329). |
| FND-009 | resolved | "Hold" is now defined, and no stage produces one (FR-057:271-274). |
| FND-010 | resolved | `quire.capability-kind/v1` is the FR-290 Values table at quire-specification `046d1bd` (FR-057:38-40, 319-320). |

No new failure-domain defects.

Round 2 verdict: ACCEPT
