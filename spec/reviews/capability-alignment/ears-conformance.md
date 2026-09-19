---
id: SR-497
title: "EARS conformance review of FR-057 capability-kind admission"
type: SpecReview
analysis: ears-conformance
scope: "spec/functional/FR-057-admit-shared-capability-kinds.md, spec/functional/FR-036-link-composed-native-packages.md (changed lines), spec/test-cases/TC-115-preserve-composed-admission-stages.md, spec/test-cases/TC-153-admit-exact-capability-kinds.md, spec/test-cases/TC-154-refuse-unsupported-capability-vocabulary-version.md, spec/test-cases/TC-155-keep-admission-backend-independent.md, spec/model-linking/tests.md, spec/spec.md"
review_set: all
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-057
    type: reviews
---
# SR-497: EARS conformance review of FR-057

## Summary

Reviewed commit `cd4f71a` (branch `task/229-capability-spec`), the diff against
`origin/main`. EARS scope is FR-057 and the one changed FR-036 statement. The
TCs, `tests.md` and `spec.md` are out of EARS scope, but are read for the
statement each one verifies. `quire validate --strict` reports 2/2 docs
grammar-clean with 0 engine findings. FR-057 has 12 `SHALL` statements. Five
are clean. The main defect is subject naming. Two statements name an artifact
or a data form as the subject, one uses a pronoun, and the negotiation rule
names a process instead of the system that owns it. Several obligations that
the acceptance criteria test are written as plain indicative sentences with no
`SHALL`. So they are not requirement statements. No finding changes what gets
built. The owner rulings (FR-290 six kinds, absence settles `unsupported`,
single `negotiate_*` point, admission-only Capability, no compatibility path)
are applied correctly and are not flagged.

Verdict: ACCEPT WITH FINDINGS

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The absence rule reads "When no registered backend advertises a requested kind, negotiation SHALL settle that item `unsupported` …". "Negotiation" is a process, not a named system. "No backend advertises" is an unwanted state, not an event, so `When` is the wrong pattern. The Behavior section names quire-contract-codegen's `negotiate_*` as the single negotiation point. But Dependencies says "which repository runs the negotiation over the registry" is decided in #210, so the subject looks open. Fix: rewrite as "If no registered backend advertises a requested kind, then quire-contract-codegen's `negotiate_*` SHALL settle that item `unsupported` with a warning naming the kind's exact label." In Dependencies, limit #210 to per-family applicability of each kind. | FR-057 Behavior (Absence), Dependencies · FR-057-AC-6 · TC-155 |
| FND-002 | medium | Obligations that ACs test are written without `SHALL`, so they are not requirement statements: "A missing label is never defaulted …" and "A refused label is never mapped, aliased or replaced … carries no suggested replacement" (AC-2); "Two values are equal exactly when their labels are byte-equal" and "A set of kinds compares as a set" (AC-4); "Solver or backend absence is never a refusal and never a hold. No stage waits …" and "absence of one item's backend does not delay any other item's settlement" (AC-6). Fix: restate each as a subject + `SHALL`/`SHALL NOT` statement. For example: "The compiler SHALL NOT default an absent label to any kind." "The compiler SHALL NOT map, alias or suggest a replacement for a refused label." "The compiler SHALL compare `Capability` values by label byte equality." "quire-contract-codegen's `negotiate_*` SHALL NOT hold or refuse an item because no backend advertises its kind." Keep the current sentences only as explanation. | FR-057 Behavior · FR-057-AC-2, FR-057-AC-4, FR-057-AC-6 |
| FND-003 | medium | "Any serialized artifact that carries capability labels SHALL declare capability vocabulary version `quire.capability-kind/v1`" names an artifact as the subject. No system is obligated, and "any" reads as binding other repositories' producers. Fix: "When the compiler emits a serialized carrier of capability labels, the compiler SHALL declare capability vocabulary version `quire.capability-kind/v1` in it." The existing If/then rule already covers the read side. | FR-057 Behavior (Serialization and version) |
| FND-004 | low | "The canonical serialized form of a `Capability` value SHALL be its exact FR-290 label as a JSON string" names a data form as the subject. Fix: "The compiler SHALL serialize a `Capability` value as its exact FR-290 label, as a JSON string." | FR-057 Behavior (Serialization and version) · FR-057-AC-1 |
| FND-005 | low | "It SHALL retain every admitted requested pair …" uses a pronoun as the subject. The antecedent sits in a separate indicative sentence. Fix: "The QSL composed linker SHALL retain every admitted requested pair, with its declaration, kind and `required` flag, in the handoff FR-036 defines." | FR-057 Behavior (Stage ownership) |
| FND-006 | low | The unsupported-version rule packs two `SHALL` into one statement: "SHALL refuse the carrier …, and SHALL read none of its labels". Fix: split into "If … then the compiler SHALL refuse the carrier with `invalid_capability`/`unsupported-version`, naming the received version or its absence." and "The compiler SHALL NOT read any label of a refused carrier." | FR-057 Behavior (Serialization and version) · FR-057-AC-3 |
| FND-007 | low | The statement and its AC differ in scope. The statement bans only "backend support": "The QSL composed linker SHALL NOT consult backend support when admitting". AC-5 tests the wider "The linker reads no backend state", and the next sentence also names solver presence and registration state. Fix: "The QSL composed linker SHALL NOT read backend support, solver presence or registration state when admitting a requested pair." | FR-057 Behavior (Stage ownership) · FR-057-AC-5 |
| FND-008 | low | The Description says "the compiler SHALL admit". Stage ownership gives admission to "the QSL composed linker". Two subjects name the same admission step. Fix: use one subject throughout, for example "the QSL composed linker (the compiler's admission stage)" once, then "the linker" in every admission statement. | FR-057 Description, Behavior |
| FND-009 | low | The changed FR-036 line "Each requested capability SHALL name exactly one capability kind that FR-057 admits" names a data item as the subject. No system is obligated. Fix: "The compiler SHALL retain a requested capability only when its label names exactly one capability kind that FR-057 admits." | FR-036 Behavior |
| FND-010 | low | `tests.md` says "FR-036-AC-6 now names FR-057 capability kinds". "Now" narrates a change, and specs state current design. Fix: "FR-036-AC-6 names FR-057 capability kinds." | spec/model-linking/tests.md (Composed language admission) |

## Round 2 dispositions

Checked against the current tree: cd4f71a plus the uncommitted edits.

| Finding | Disposition | Evidence |
| --- | --- | --- |
| FND-001 | resolved | FR-057 no longer places a QSL `SHALL` on negotiation. It states that "Negotiation is not a QSL stage" and describes `negotiate_*` settling in plain indicative sentences (FR-057:232-247). FR-290-AC-4 is cited as the owner (FR-057:322-323). #210 is limited to deciding which family records which requirements (FR-057:336). |
| FND-002 | resolved | Each tested obligation is now stated as a `SHALL` or `SHALL NOT`: no default (FR-057:145); no mapping or suggested replacement (FR-057:152-153); byte-equal equality (FR-057:102-103); set comparison (FR-057:107); routing never converts or delays (FR-057:276-277). |
| FND-003 | resolved | "When QSL emits a serialized carrier of capability labels, QSL SHALL declare …" (FR-057:124-125). |
| FND-004 | resolved | "The QSL composed linker SHALL serialize a `Capability` value as its exact FR-290 label as a JSON string" (FR-057:119-120). |
| FND-005 | resolved | The subject is now named explicitly (FR-057:198-200). |
| FND-006 | resolved | The refusal and the no-read rule are two statements (FR-057:127-131). |
| FND-007 | resolved | The statement now covers backend support, solver presence and registration state (FR-057:202-204), which matches AC-5 (FR-057:310). |
| FND-008 | resolved | Admission statements use "the QSL composed linker" throughout (FR-057:27-28, 83, 95-155, 189, 198-204). The carrier rules say "QSL" (FR-057:124-131) because #211 assigns the reader stage (FR-057:136-137). That split is deliberate. |
| FND-009 | open | FR-036:97 still reads "Each requested capability SHALL name exactly one capability kind that FR-057 admits", with a data item as the subject. Fix, unchanged from round 1: "The compiler SHALL retain a requested capability only when its label names exactly one capability kind that FR-057 admits." |
| FND-010 | resolved | `tests.md:244` reads "FR-036-AC-6 names FR-057 capability kinds", without "now". |
| FND-011 (new) | open, low | Two criteria have no `SHALL` statement behind them. For FR-057-AC-10, the claim-form rule is written in the indicative: "Each requested item is one claim with exactly one kind, which the FR-290 claim-form assignment table gives …" (FR-057:161-164). For the tool-absence half of FR-057-AC-9, "A tool-absence result keeps the item's `supported` disposition, runs no other candidate and falls back to no other mode" (FR-057:281-283) is also indicative. Fix: for AC-10, add "The QSL composed linker SHALL record for each requested item exactly the one kind the claim-form table gives its clause's claim form, and SHALL record no kind for an expression nested in it." For AC-9, add a routing statement: "The routing SHALL NOT route a `supported` item to another candidate or mode after a tool-absence result." SR-494 FND-007 sets the scope of that fix. |

Round 2 verdict: ACCEPT WITH FINDINGS
