---
id: SR-1709001
title: "Code review of quire-spec-language PR #1709: bare-clock collection counts and Clock parameters and results (580p)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@68e00d6d6; PR #1709 diff origin/qsl-580o-clock-positions...origin/qsl-580p-clock-positions: qsl-eval/src/simulation/model{.rs,/encode.rs,/symbolic.rs,/timed.rs}; qsl-semantics/src/check/{assemble.rs,mod.rs,timed_model.rs,timed_model/quantified.rs,timed_model/reading.rs}; qsl-semantics/src/model/{intake.rs,operation.rs}; tests timed_subject, clock_places"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-230
    type: reviews
---
# Code review of quire-spec-language PR #1709

## Summary

Ticket: QSL-580, slice 580p. The Rust review lane is folded into this
review. I reviewed only `git diff origin/qsl-580o-clock-positions...origin/qsl-580p-clock-positions`
at 68e00d6d6. The specs are FR-230 as amended by #1689 ("Operation
parameters", "Results", "Initial element count", AC-39..48, TC-685 steps
29–32) and FR-120.

Gates: `cargo test -p qsl-semantics -p qsl-eval -p qsl-analyze -p qsl-replay --all-features`
passed at 68e00d6d6 (EXIT=0, no failures). I did not run clippy.

I re-measured each claim:

- **Counts.** A collection with the shape `[Field, Slot]` and no data part
  becomes a `ClockCount` member of its owner type and of every subtype. Its
  domain is `Int[min, max]`. A `Set` or `OrderedSet` is capped at 1, and a
  collection with no maximum has domain `Integer`. `with_clock_counts`
  ranges each holder's count over `min..=max` and gives no state when
  `min > max`. `TimedSystem::unbounded_counts` ascends by total added
  elements while the live clocks stay within `max_clocks`, plus one state
  past the bound. This matches "Initial element count" and AC-46/47/48.
- **Granted collection with no maximum.** It stays `RequiresBound`. That is
  right: FR-230 says the `[min, ∞)` range "does not return
  `NotSimulated::RequiresBound`, which FR-120 applies only to `domains()` of
  an unbounded frame-granted value". So a granted one is exactly FR-120's
  case, and FR-120's collection rule makes an unmaxed `Sequence`/`Bag` of
  clocks unbounded. `Set<Clock>` with no maximum is capped at 1 and stays
  finite (2 values), as FR-120-AC-26 says.
- **`carries_clocks` exemption.** It applies only to the exact path
  `[Field, Slot]`. A bare-clock slot holds no data, so `source_slots` keeps
  positions, and a set holds at most one element, so no carry can arise. A
  set of data-bearing elements is still refused.
- **State key.** `object_enc` appends one `{"type":"tuple","name":"Clock","components":[]}`
  shell per element under the collection's name, after the declared fields.
  `typing()` carries the counts. It is used by `model_key`, the
  symmetry sort key and the timed key. Only the SMT operation encoding uses
  `value_typing()`, which is no state key. The count is the only eval-side
  member in object state, so no other member is missing from the key.
- **Clock parameters.** Assembly drops a `[1, 1]` `Clock` (or alias)
  parameter from the data parameters and keeps its declared position. S3
  expands `c` into one alternative place per field in its range, each
  tagged with the parameter. The alternatives have no premise, so a
  constraint is `∨_f (x_f op b)` and its negation is `∨_f ¬(x_f op b)`.
  `bound()` then keeps, per instance, only the parts on the bound field, so
  both polarities and conjunctions such as `c >= 1 and c <= 3` read exactly
  the bound place. A reset of `c` with an empty range refuses `clock-write`.
- **Interim argument.** `{"type":"clock_place","object":<reference>,"field":<declaration>,"path":[...]}`
  names the object, the clock field and every slot index, so it is
  one-to-one with the runtime place. Its `clock_place` tag decodes as no
  data type. Positions are a separate problem (FND-002).

## Verdict

**Changes requested.** One frame-scope defect and one argument-position
defect.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | A `Clock` parameter's reset range ignores the frame's scope. `clock_parameters` builds the range from `written_fields()`, which merges every-object and receiver-scoped entries, and `instances` binds the parameter to range places on every object. My probe uses two calls with `work(c: Clock)`, `post { c = 0 ms }` and the receiver-scoped frame `modifies self.x` (`modifies_receiver`). It gives 4 steps, and receiver `c2`'s instance resets `c1.x` (receiver c2 → reset c1). FR-230 "Operation parameters" limits the range to "the places of the clock fields the operation's frame (FR-103) names". A receiver-scoped entry names only the receiver's field, so this writes a clock outside the frame. Bind a receiver-scoped field's places on the receiver only (the eval range), and test it. | qsl-semantics/src/check/timed_model.rs clock_parameters; qsl-eval/src/simulation/model/timed.rs:1301-1342 |
| FND-002 | medium | Timed transitions place each `clock_place` argument at the parameter's declared position (`instance_transition`). The other consumers index arguments by data-parameter position. A hyper `ArgumentRead.index` is the position in `declaration.parameters()`, which holds data parameters only (hyper_clause.rs:2305-2310, step_value.rs:51), so `op(c: Clock, a: Int, b: Int)` read for `b` decodes `a`'s argument, a wrong value with no refusal. `footprints` requires `parameters().len() == arguments().len()` and returns `UnknownTransition` for every instance. Until 580q's place id lands, put clock arguments after the data arguments, or make every reader index by declared position, and test a data parameter after a `Clock` one. | qsl-eval/src/simulation/model/timed.rs:1350-1378; qsl-semantics/src/check/hyper_clause.rs:2305-2321; qsl-eval/src/hyper/step_value.rs:51; qsl-eval/src/simulation/model/footprints.rs:516 |
| FND-003 | low | For a `Set`/`OrderedSet` with `min > 1`, the count domain is `Int[min.min(max), max]`, which is `[1, 1]`. `with_clock_counts` correctly gives no initial state. But a granted holder, or a created object of that type, enumerates count 1, below the declared `min`. Give such a collection an empty domain, or refuse it, instead of `[1, 1]`. | qsl-eval/src/simulation/model.rs:700-716 |
| FND-004 | low | `permute_transition` (FR-152 symmetry) renames only reference and option-of-reference arguments, so a `clock_place` argument keeps the old object. A lifted EN-6 symmetry counterexample of a `Clock`-parameter step then names the wrong place unless it is re-matched against an expansion. Note it for 580q: the place-id argument must be renamed under a permutation. | qsl-eval/src/simulation/model/symmetry.rs:293-330 |

## Dispositions

Round 1, fix commit cc106f1bf, head 81fa6e048.

Gates (locked-build, jobs 4, debug info off, `--all-features`, QSPEC_DIR 308596b0):
- qsl-eval `timed_subject`: 55/55.
- qsl-analyze `zone`: 105/105.
- qsl-semantics `timed`: 3 + 67.
- clippy on qsl-eval, qsl-analyze and qsl-semantics with `--tests -D warnings`: exit 0.

I re-applied probe_1709.diff on the head. It was not committed.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | `ClockParameter.receiver` holds the range fields that are written but not in the population-wide `modifies` (that is, only in `receiver_modifies`). The `instances` filter adds `!receiver_only.contains(clock) \|\| place.holds(receiver)`. The re-run probe gives 2 steps: receiver `c1` resets only `c1`'s `x`, and `c2` resets only `c2`'s `x`. Before the fix, `c2` reset `c1.x`. |
| FND-002 | fixed | `instance_transition` now pushes the place arguments after the data arguments. `footprint_selecting` accepts either `parameters` or `parameters + clock_parameters` arguments and zips only the data parameters. `a_data_argument_after_a_clock_parameter_keeps_its_index` checks `work(c: Clock, a, b)`: the arguments are `[a, b, place]`, `b` is `true` on both steps under `pre { b }`, and the footprint resolves. |
| FND-003 | fixed | `field_domain` gives an empty `Listed` domain when a pure-clock count has `max < min`, and frames.rs uses it for created objects. I checked the claim that the created-object path is unreachable, and it holds. An object of the type cannot exist, because a `Set<Clock>[2, ..]` field has no value, so there is no initial state (`en6_settles_the_bare_clock_counts` asserts `NoInitialState`). So no receiver of that type can create one. A creator of another object type would need a second object-type model node, which `AmbiguousTimedModel` refuses (check/mod.rs: `object_type_model_node_keys` must be one). The guard is correct and has no reachable test. |
| FND-004 | fixed | `permute_transition` renames the object inside a `clock_place` argument through `split_clock_place` and keeps the slot path. `a_permutation_renames_a_clock_place_argument` asserts that the image names the swapped object. Without the rename, the image of (c1, place c2.x) would be (c2, place c2.x), which is also an expanded transition. So the `named(&image, to)` assertion is the one that catches the defect, and it does. |
