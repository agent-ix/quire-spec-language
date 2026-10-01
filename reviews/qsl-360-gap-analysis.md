---
id: SR-950
title: "QSL-360 gap analysis of PR 558: no remaining copy of QSpec's lock or diagnostics data, AC trace"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@13f0b2a6cf2a50af6768366cb754b0c3995ad71a; QSL-360 purpose (read QSpec's lock by reference, hold no copy); FR-093-AC-7; FR-093-AC-17; TC-416 step 9; qsl-semantics/tests/it/complete_value_lock.rs; qsl-semantics/src/check/lowering/tests/leaves.rs; src/linking/composed/definition_source.rs; quire-specification@ce802b78 proposals/quire-v1/definitions/complete-value-selection-vectors.json (context)"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-093
    type: reviews
---
## Summary

Ticket: QSL-360. PR: quire-spec-language#558 at 13f0b2a6.

The ticket's purpose is that QSL holds no copy of QSpec's value lock or diagnostics
catalog data. Checked against the code:

- **Production value-lock data.** Read by reference. `CATALOG`, the selection tables
  and the qsl-bench `text_profile` copy are deleted.
- **Diagnostics catalog in the emitter.** Read by reference
  (`native_diagnostics_catalog`). The `94580e10` literal is gone.
- **FR-093-AC-17 / TC-416 step 9.** Backed by
  `emit::tests::the_lock_selects_the_catalog_definitions`
  (`#[trace("TC-416", "FR-093-AC-7", "FR-093-AC-17")]`). It asserts the wire
  revision and digest equal `native_diagnostics_catalog()`. That alone is the same
  function the emitter calls. The independent oracle is
  `the_native_diagnostics_catalog_reads_its_header`, which recomputes SHA-256 of the
  bytes with `sha2` and checks the header contains the parsed identity and revision.
  Together they back the AC.
- **FR-093-AC-7.** The same test compares wire edition and selections with the
  `DefinitionLock` rows, digests included, and IR admits. Passing in the gate log.

Remaining copies of QSpec data, below. The coder's claim that QSpec defines which
check yields which selection refusal code "only in prose/TC-192" is not right: QSpec
publishes `complete-value-selection-vectors.json` with an `expected_code` per refused
case.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | `vector_text_definition()` in the FR-093 text-leaf vector test is a hand-typed copy of the lock's `text_profile` row (`agent-ix`, `quire.value.text.unicode-17.0.0/v1`, `quire-draft` `1-draft.1`, digest `cd4a985a...`). It is the same copy the PR deleted from `qsl-bench/src/text_cluster.rs`. Replace it with `DefinitionLock::pinned().entry(CatalogRole::TextProfile)...reference()`. A later QSpec bump then fails the FR-093 vectors, which is the right signal. | qsl-semantics/src/check/lowering/tests/leaves.rs:31-43 |
| FND-002 | medium | The two `#[trace("QSpec-TC-192")]` selection tests hand-type their accepted and refused vectors. QSpec publishes them as `complete-value-selection-vectors.json` (7 accepted, 12 refused, each with `expected_code`). The hand copy also omits QSpec's two ordering vectors (`unknown-trigger-before-duplicate-trigger`, `duplicate-trigger-before-unknown-role`). The code orders them correctly today, but nothing tests it. Expose the vectors from the STD-130 crate, which is still open, and drive both tests from it. | qsl-semantics/tests/it/complete_value_lock.rs:107-208 |
| FND-003 | medium | `RegisteredDefinition::Diagnostics` hard-codes `quire.native.diagnostics/v1` at `1-draft.8`. The registry beside it does the same for every other QSpec definition revision. This code predates the PR and is outside the diff. But the PR now reads the same document's revision from its header in `native_diagnostics_catalog`. When QSpec bumps the header, the emitter follows and this registry does not, so QSL holds two disagreeing values for one fact. Record a ticket to read this registry's revisions from QSpec by reference, or at least derive `Diagnostics` from `native_diagnostics_catalog()`. | src/linking/composed/definition_source.rs:248 |

## Verdict

The ticket's main scope, the value lock and the emitter's diagnostics reference, is
done and traced. Three copies of QSpec data remain. FND-001 and FND-002 are in the
same files and subsystem as this PR and are cheap to fix here; FND-002 needs one
constant added to the still-open STD-130 crate. FND-003 predates the PR and needs a
follow-up ticket, but this PR makes the divergence possible.
