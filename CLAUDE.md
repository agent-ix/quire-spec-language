# Contributor guidance

Follow [AGENTS.md](AGENTS.md) for repository instructions.

For unfinished issue references write `Remaining work: #N`; never put a GitHub closing keyword next to an issue number in a negated sentence, because GitHub may still close it.

Keep prototype bookkeeping minimal. Use ordinary Git history, PR links and brief
test results. Do not maintain manual SHA inventories or per-document checksum
catalogs, refresh hashes for routine draft edits, create successor draft files
for working history, or record every intermediate step in status/review logs.
Update existing artifacts and tickets for material decisions and remaining work.
Reviews happen at PR readiness. Preserve digests required by actual runtime
identities, dependency selections and interchange contracts; those are product
semantics.
Remove this prototype-only bookkeeping restriction when the repository reaches
a stable release.

At substantial PR checkpoints, `/code-review` includes the actual
`agent-skills/rust-review/SKILL.md`; QUOIN `/gap-analysis` is also required.
Choose applicable `/spec-review` analyses rather than automatically running all
seven optional analyses. Rust idioms and code smells remain review requirements.
