---
id: SR-342
title: "Base spec review of FR-042 and TC-121 against the producer example's claims"
type: SpecReview
analysis: base
scope: "spec/functional/FR-042-publish-compiled-protocol-artifacts.md; spec/test-cases/TC-121-publish-compiled-protocol-artifacts.md; spec/model-linking/tests.md; examples/protocol-handoff/README.md"
review_set: base
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-042
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-121
    type: references
---

## Summary

Base checklist only, over the FR-042/TC-121 contract this example implements and
the claims its README makes about it. No `type: AssuranceProfile` document exists
in the local specification scope — the only matches in reach are filament
skeletons — so no `review_selection` is enforced and the owner's selection stands:
base set, optional semantic gap extension declined, no analysis lens skills run.
QUOIN's installed SpecReview skeleton and schema supplied this artifact's
contract. No spec, matrix or requirement file changed in this increment.

## Verdict

**CONDITIONAL** — no `high` finding. The example's claims are accurate against
FR-042 and understate rather than overstate coverage. One `medium` gap sits in the
requirement itself: the accepted-inventory transfer that AC-10 depends on has no
owner.

## Base checklist result

| Check | Result |
| --- | --- |
| ID formats (FR/TC/AC), uniqueness, sequence | pass — `FR-042`, `TC-121`, `FR-042-AC-1..AC-10` |
| Validation link integrity | pass — `quire validate --scope <worktree> "spec/**/*.md"` clean, only module-registry first-wins notices |
| FR quality: description, inputs, outputs, behavior, error conditions, criteria | pass — Inputs owns the three caller-supplied revision namespaces; typed refusal vocabulary is named, not paraphrased |
| Six coverage rules | pass with FND-002 — every AC maps to TC-121; the aggregated row cannot express AC-10's distinct state |
| Cross-referencing, terminology | pass — FR-042 ↔ US-004 ↔ TC-121 ↔ IT-001 all resolve |
| Grammar corpus | 398/398 docs grammar-clean (`/tmp/quire-native-handoff-spec.log`) |

## Claims checked against the requirement

- **Baseline.** The README calls the registered Edition artifact (`ix:native`,
  semantic revision `1-draft.2`) "the accepted language baseline" and explicitly
  disclaims it as "C-owned evidence acceptance or proof of an authenticated /
  reproducible binary build". That matches FR-042 Inputs, which sources acceptance
  from the independently supplied inventory rather than a payload flag or
  installed default. Accurate.
- **Revision namespaces.** FR-042 requires that "source-artifact revision labels
  remain separate from the semantic revisions derived from those owners". The
  README states the same separation and the code honours it for source artifacts.
  The one place the code asserts a semantic namespace with a hardcoded value is
  SR-340 FND-002; it is an implementation defect, not a specification ambiguity.
- **AC-10.** The README retains FR-042-AC-10 and B's IT-001 as open and supplies no
  B implementation. TC-121 step 10 already anticipates exactly that disposition.
  No overclaim.
- **Sidecar.** FR-042 Outputs rules out "source stdout, shell commands and
  hand-edited JSON" as the producer handoff. `expected.json` is machine-generated,
  fixture-scoped and explicitly disclaimed as "no production request or manifest
  format", so it does not violate that sentence — but see FND-001.
- **Scope of the authored workflow.** TC-121's positive fixture describes orders
  O1/O2, a shared payment provider, two channels, an await, bounded repeat, join,
  commit and full/partial recovery. This example authors a far smaller subject: a
  predicate, an invariant, a bounded temporal obligation and a single-sequence
  protocol with one check and one finish. The README claims only what it contains.
  No spec text is contradicted, and TC-121's full positive fixture remains owed.

## Findings

| ID      | Severity | Summary                                                                                  | Refs                                                            |
| ------- | -------- | ----------------------------------------------------------------------------------------- | --------------------------------------------------------------- |
| FND-001 | medium   | FR-042 requires an accepted inventory at the reader but no artifact owns how it reaches B  | spec/functional/FR-042-publish-compiled-protocol-artifacts.md:52 |
| FND-002 | low      | TC-121's matrix row aggregates AC-1..AC-10, so the open AC-10 handoff has no distinct state | spec/model-linking/tests.md:111                                  |
| FND-003 | low      | Neither FR-042 nor TC-121 references the delivered producer recipe that step 1 describes   | spec/test-cases/TC-121-publish-compiled-protocol-artifacts.md:31 |

## Finding detail

**FND-001.** FR-042 Inputs says the reader takes "canonical bytes, an
independently selected external artifact reference, that accepted inventory,
exact local dependency bytes/admitted producer views and caller-lowered artifact
limits", and AC-10 requires every "producer/source/profile/dependency selector"
to survive into B's public Rust interface. Both sentences describe in-process
Rust values. Nothing in FR-042 or TC-121 says how an accepted inventory crosses a
repository boundary to B — whether B authors its own selections in Rust from the
same registry, or A exports a selection surface, or an interchange encoding is
specified. The example had to invent a fixture-only `expected.json` to fill that
hole and then spend a README paragraph warning that it is not a format. That
warning is the right call, but the underlying question is a specification gap, and
AC-10 cannot close until it is answered by one of those three options. Naming the
chosen option in FR-042 Inputs or TC-121 step 10 is a small edit that removes the
ambiguity before B builds against a guess.

**FND-002.** Same item as SR-341 FND-003, recorded here because the fix is a
matrix edit rather than a code change: a `🚧 Planned` row covering ten acceptance
criteria reports as fully backed once any TC-121-tagged test exists, so AC-10's
genuinely unmet positive handoff is invisible to `quire coverage`. Splitting
AC-10 into its own row, or marking it blocked on `ix://agent-ix/quire-protocol/IT-001`,
makes the open prerequisite machine-visible.

**FND-003.** TC-121 step 1 describes precisely what this increment now delivers —
admission through the real parser, namespace/definition/model binding, types,
definedness and family admission, then emission through the production entry
point. The delivered recipe and its produced selectors are recorded only in
`examples/protocol-handoff/README.md`. Under the owner's prototype-bookkeeping
restriction this does not warrant a status log or a checksum catalog; one
reference from TC-121 step 1 or FR-042 Dependencies to the example path would be
enough, and it is a material decision rather than routine bookkeeping. Left as
`low` because the Git history and the PR already carry the link.
