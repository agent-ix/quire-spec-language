# Source-backed probability certificate checker

`qsl-probability-check` reads a JSON request from standard input or its one
file argument, recompiles the supplied native source through S1–S3 and checks
its exact reachability certificate in the qualified core. Its normal dependency
closure contains no analyzer. Tests use EN-5 as a development dependency to
produce evidence, then launch the built checker executable in a fresh process.

The request contains `source` (`authority`, `identity`, exact UTF-8 `content`),
`package_name`, structural `item` (`node_id`, `model`, `ordinal`), source-keyed
`byte_provision` triples (`authority`, `identity`, byte array), domain/snapshot
`documents` as byte arrays, `initial` document references, `universes` keyed by
original `{package,node}` population declarations, the exact `certificate`
JSON text as a string, and `limits`. Certificate text is preserved exactly.
Every request object rejects unknown members.

`limits` requires `bytes`, `states`, `edges`, `scalar_bits`, `expansion`, and
`stages` (a map of the existing FR-255 stage/accounting setting names to caller
ceilings). Unspecified stage/accounting settings retain their published defaults.
`--max-input-bytes N` bounds the whole request before JSON decoding (default
16777216); `--cancel` supplies a pre-cancelled caller handle. A file name of
`-` selects standard input. Output is one JSON result; exit 0 means accepted,
1 means certificate rejection and 2 means no accepted evidence was produced.

This source facade admits one native unit without library imports, one initial
state and at most one claim binding. The closed document profile covers exact,
untimed, acyclic, unbounded eventual reachability with empty fairness. The
checker independently derives the target keys and the checked claim's
minimum, maximum or resolved-workload operator, verifies complete key/value/rank
coverage, positive-value rank progress, Bellman equality and the threshold.
Unsupported configurations produce no acceptance token.

The Link domain/source/snapshot fixtures are exported data from the existing
`qsl-analyze/tests/it/prob_world.rs` Link fixture; tests compile those bytes with
the production spine rather than importing that test fixture's `World` adapter.
