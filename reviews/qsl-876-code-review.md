---
id: SR-12173
title: "Code review of quire-spec-language PR #1010: FR-336 AC-1 population domain and AC-4 model request"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@14cde82acd99ffe92f31fcdb44cf6bb87c1c74de; PR #1010 diff against merge-base 4978f722a: qsl-semantics/src/check/{mod,refusal,temporal_clause}.rs; qsl-semantics/src/check/temporal_clause/tests.rs; qsl-semantics/tests/it/{main,model_subject_bound,temporal_fairness}.rs; qsl-replay/src/spine.rs; qsl-route/src/{lib,model_request}.rs; qsl-analyze/tests/it/{main,universe_obligation}.rs; qsl-package/src/emit/tests/temporal_clause.rs"
review_set: subset
---
# Code review of quire-spec-language PR #1010

## Summary

Ticket: QSL-876. One code-review pass with the rust-review lane folded in,
over `git diff origin/main...HEAD` (13 files, +540/-24; the diff includes a
merge of main carrying #1008). Read-only: nothing was built or run, at the
dispatcher's instruction. QSpec FR-361-AC-8 read at its current main, cited
by id.

Soundness questions from the brief, each traced through the code:

1. **No universe in the emitted record.** `TemporalClaim::requirements`
   takes the clause node, the parameter nodes and an optional population
   key. Its domains come from `parameter_domains` (checked types),
   `infinite_trace` (the profile) and `population` (the checked `over`
   type against the admitted models' populations). No argument or field
   carries a universe, and `PackageDeclarations::check` (mod.rs 2457-2469)
   calls it with `population_domain_key(&object_types, &object_nodes, ..)`
   only. The real-emission test
   `the_emitted_record_carries_the_population_domain_and_no_universe`
   asserts exactly two domains: the population and the clause node's
   infinite-trace domain. Confirmed by reading the body, not the signature.
2. **AmbiguousPopulation.** It fires only from `population_of`'s `Err`,
   which needs two or more `(ordinal, key, member, bound)` hits across the
   admitted models. The single-population case is the `[one]` arm and
   returns `Ok(Some)`. It is the same helper and the same rule state
   clauses, reachabilities and compensation obligations already use (FR-104,
   FR-365), so no new spurious case. It cannot be skipped: the `?` returns
   the refusal for every `over` type that is a `Reference`, before
   `property_form` and the claim are built, whether or not the population
   is bounded (FR-380: "FR-104's and FR-365's rules ... for refusing two,
   are the same for a bounded population"). The temporal `over` type is not
   covered by any assembly-level population check (assemble.rs checks state
   clause contexts and state-graph claims), so the path is reachable. It
   has no test (FND-001).
3. **Bound present means no population domain.** `.filter(|domain|
   domain.maximum.is_none())` drops a bounded population. FR-380 "Meaning":
   a bounded population "is not listed among a claim's unbounded domains",
   and FR-336 Behavior bullet 1 speaks only of "a population with no
   declared maximum". This matches `frame_record`'s `continue` on
   `maximum.is_some()` for state clauses. A domain-package population
   declares no maximum (QSpec FR-153), so only a `bound` sets one.
   Matches. A state clause also keeps its bounded populations'
   `AuthoredBound`s (`population_bounds()`) for the FR-380 evaluator
   refusal; a temporal clause drops them. FR-380 states that refusal for
   observation evaluation, not for model checking, so no finding here.
4. **One key builder.** `population_domain_key` is a pure move of the old
   `CheckedGraph::population_domain` body; `population_domain` and the new
   public `population_key` both call it, and so does the emitter. The AC-1
   test checks that `graph.population_key(claim.population())` equals
   `graph.population_domain(Config, config_history)`.
5. **ModelRequest.** Index alignment holds: the inner `RequestWriter` is
   private and only `authored` pushes, so `obligations[i]` is the
   obligation of item `i` (`RequestWriter::push` assigns
   `RequestIndex::new(items.len())`). Dedup is by obligation equality. It
   has no production caller (FND-002).

Every `#[trace]` in the diff:

| Test | Tag | Verdict |
| --- | --- | --- |
| `model_subject_bound::an_unbounded_population_stays_in_the_requirement_record` | TC-846, FR-336-AC-1 | correct |
| `model_subject_bound::a_bounded_population_joins_no_domain` | FR-336 | loose (FND-005) |
| `emit::tests::temporal_clause::the_emitted_record_carries_the_population_domain_and_no_universe` | TC-846, FR-336-AC-1 | correct, strongest AC-1 evidence |
| `universe_obligation::two_universes_are_two_obligations_and_each_result_joins_its_own_index` | TC-846, FR-336-AC-4 | over-claims the universe half (SR-12174 FND-001) |

The edits to `temporal_clause/tests.rs` and `temporal_fairness.rs` only
add the new third argument; their tags are unchanged.

**Merge risk with PR #1011 (QSL-877, head 5f9c2fbb6 at review time, being
reworked to STD-193).** #1011 adds a `fairness: BTreeSet<FairnessKind>`
field to `TemporalClaim`, sets it in `check_clause`'s constructor, and
rewrites `requirements`'s tail to `Requirements::new(..).with_fairness(..)`.
This PR adds `population` at the same two places and inserts the population
domain just before that tail. Three conflicts follow:

- Textual: the struct field after `infinite_trace`, the constructor field
  after `infinite_trace: !bounded`, and the `Ok(...)` tail. The resolution
  must keep both the population insert and `.with_fairness(..)`.
- Compile: #1011's new test `a_clauses_requirements_carry_its_fairness_kinds`
  in `qsl-semantics/tests/it/temporal_fairness.rs` calls `requirements`
  with two arguments.
- Silent runtime: that test's clauses are over `Config::ConfigVersion`, whose
  `config_history` has no maximum, so `claim.population()` is `Some`. The
  obvious fix of passing `None` makes `requirements` return
  `population-domain-keyed` and the test panics at `.expect("the record is
  keyed")`. The resolver must pass `claim.population().map(..)` a key, as
  this PR does in `clauses_over_a_model_subject_classify_into_their_property_form`.

Whichever PR lands second owns this resolution.

## Verdict

Mergeable. No high finding. The AC-1 change is sound: the emitted record
carries exactly the population and infinite-trace domains, no universe can
reach it, the bound rule matches FR-380, and the ambiguity refusal reuses
the established one-covering-population rule. The mediums are an untested
new refusal path and a public `ModelRequest` with no production caller.
Neither breaks a result.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | `TemporalCause::AmbiguousPopulation` and its qsl-replay message are new, reachable refusal paths with no test. No test checks a temporal clause whose `over` type two populations cover, its code (`ambiguous_declaration`/`ambiguous-name`), its span, or the replay text. | qsl-semantics/src/check/temporal_clause.rs:644-660; qsl-semantics/src/check/refusal.rs:788-796; qsl-replay/src/spine.rs:888-890 |
| FND-002 | medium | `qsl_route::model_request::ModelRequest` is a public API with no production caller; only `qsl-analyze/tests/it/universe_obligation.rs` uses it. Its `S: Clone + Eq` bound does not fit the real subject type either: `qsl_eval::ModelSubject` wraps `ModelSimulationRequest<'a>`, which derives `Clone` only. Nothing in production writes a model-subject request yet. This follows the `deadlock.rs` precedent (FR-124), and the driver is blocked with AC-2 on EN-1. Wire it into the EN-1 request path or record the owning ticket. | qsl-route/src/model_request.rs:20-81; qsl-eval/src/model_subject.rs:25-31 |
| FND-003 | low | `TemporalClaim::requirements(.., population: Option<DomainKey>)` faults on `None` when the claim has a population, but silently drops a `Some` key when it has none. A caller can pass a key and have it ignored. Taking the key builder (or the graph) instead of a pre-built key would remove the mismatch. | qsl-semantics/src/check/temporal_clause.rs:293-330 |
| FND-004 | low | `TemporalCause::AmbiguousPopulation { name: String }` joins `key.node` of each covering population into one string. `AssemblyCause::AmbiguousPopulation` and `ObligationCause::AmbiguousPopulation` keep `Vec<DeclarationKey>`. This one drops the package part of each key and is stringly typed, so two populations with one node name in two packages read "x, x". | qsl-semantics/src/check/refusal.rs:788-796; qsl-semantics/src/check/temporal_clause.rs:647-655 |
| FND-005 | low | `a_bounded_population_joins_no_domain` is tagged bare `FR-336`, but what it checks is FR-380 "Meaning" (a bounded population is not an unbounded domain) applied to a temporal clause. FR-336's ACs speak only of the no-maximum case. Tag it FR-380. | qsl-semantics/tests/it/model_subject_bound.rs:106-127 |
