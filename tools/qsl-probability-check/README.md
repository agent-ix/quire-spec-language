# Source-backed probability certificate checker

`qsl-probability-check` reads a JSON request from standard input or its one
file argument, recompiles the supplied native source through S1–S3 and checks
its exact reachability certificate in the qualified core. Its normal dependency
closure contains no analyzer. Tests use EN-5 as a development dependency to
produce evidence, then launch the built checker executable in a fresh process.

For accepted EN-5 proofs, `TerminalRecord::exact_certificate_document()` exposes
the canonical bytes retained by the qualified checker's opaque acceptance token.
Both direct analysis and provider results carry that same checked document.

Inside each decoded six-component key, `model` is the parsed
`["quire.simulation.state-key/v1", model_form]` array, with `model_form` exactly
the bare form returned by `ModelSystem::state_key`. There is no inner base64
layer. A formula monitor is `["formula", parsed_complete_FR338_key]`; contextual
monitor/state membership remains the graph checker's responsibility.
`ProbabilityProductKey::for_model_state` uses finite 1 GiB byte and cumulative
working-storage defaults. `for_model_state_with_limits` and `for_formula_state`
take explicit `ProbabilityDocumentIoLimits` and cancellation; model/monitor
parsing, canonical validation and output share the working-storage reservation.
Exhaustion names `certificate_check.max_certificate_bytes` or
`certificate_check.max_working_bytes` with its configured bound and refused
required amount. The two `link.model-key-*.json` vectors were captured directly
from the production Link model's `ModelSystem::state_key` output.

The request contains `source` (`authority`, `identity`, exact UTF-8 `content`),
`package_name`, structural `item` (`node_id`, `model`, `ordinal`), source-keyed
`byte_provision` triples (`authority`, `identity`, byte array), domain/snapshot
`documents` as byte arrays, `initial` document references, `universes` keyed by
original `{package,node}` population declarations, the exact `certificate`
JSON text as a string, and `limits`. Certificate text is preserved exactly.
Every request object rejects unknown members.

`limits` requires `bytes`, `storage_bytes`, `states`, `edges`, `scalar_bits`, `expansion`, and
`stages` (a map of the existing FR-255 stage/accounting setting names to caller
ceilings). Unspecified stage/accounting settings retain their published defaults.
`--max-input-bytes N` bounds the whole request before JSON decoding (default
16777216); `--cancel` supplies a pre-cancelled caller handle. A file name of
`-` selects standard input. Output is one JSON result; exit 0 means accepted,
1 means certificate rejection and 2 means no accepted evidence was produced.

This source facade admits one native unit without library imports, one initial
state and at most one claim binding. The closed document profile covers exact,
untimed, acyclic, unbounded eventual reachability with empty fairness, plus
infinite-trace `holds(A) until holds(B)` and `always holds(P)` over checked
state predicates. The latter uses the separate state-predicate/v1 objective:
its event, success keys and absorbing failure keys are independently derived.
The checker establishes original model acyclicity before stopping at boundaries;
terminal stutter preserves the last state predicates. These additions admit
proof documents only; native until/always refutations remain unsupported. The
checker independently derives the target keys and the checked claim's
minimum, maximum or resolved-workload operator, verifies complete key/value/rank
coverage, positive-value rank progress, Bellman equality and the threshold.
Unsupported configurations produce no acceptance token.

The analyzer's additive `ExactProbLimits.document` API carries independent
`bytes` and `storage_bytes` ceilings, each defaulting to 1073741824 bytes.
The former stops as `certificate_check.max_certificate_bytes`; canonical-reader
writer/reader working storage stops as `certificate_check.max_working_bytes`. The
writer uses the qualified canonical event encoder without wire row vectors or a
JSON tree. It charges cumulative requested capacity before canonical buffers,
identity traversal, transport strings and the output destination grow. Base64
lengths are checked against the byte budget before encoding; decimal formatting
reserves a digit bound derived from the existing integer's bit count first.
Source callers
configure the corresponding `ProbabilityDocumentLimits` fields directly.
Graph/rational limits and the subject's scalar accounting also apply. Decimal
text is checked before large integer construction, and normalization and
Bellman arithmetic use the kernel's metered exact-rational operations.

The Link domain/source/snapshot fixtures are exported data from the existing
`qsl-analyze/tests/it/prob_world.rs` Link fixture; tests compile those bytes with
the production spine rather than importing that test fixture's `World` adapter.
