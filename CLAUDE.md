# Contributor guidance

Follow [AGENTS.md](AGENTS.md) for repository instructions.

## Hash / digest / pin antipattern: present, slated for removal

This repository still contains hashes, digests, SHAs, pins, checksum catalogs and
records that track files, versions or tools. That is an antipattern and it is slated
for removal. Do not introduce any new use of it. When you touch code or specs that
contain one, remove it as part of the change. The only hash that stays is a canonical
identity digest that binds a proof to the exact content it proved. Package versions
live in Cargo.toml / package.json and their lockfiles only; reports name the app
version they ran.

For unfinished issue references write `Remaining work: #N`; never put a GitHub closing keyword next to an issue number in a negated sentence, because GitHub may still close it.

Keep prototype bookkeeping minimal. Use ordinary Git history, PR links and brief
test results. Do not maintain manual SHA inventories or per-document checksum
catalogs, refresh hashes for routine draft edits, create successor draft files
for working history, or record every intermediate step in status/review logs.
Update existing artifacts and tickets for material decisions and remaining work.
Reviews happen at PR readiness.

At substantial PR checkpoints, `/code-review` includes the actual
`agent-skills/rust-review/SKILL.md`; QUOIN `/gap-analysis` is also required.
Choose applicable `/spec-review` analyses rather than automatically running all
seven optional analyses. Rust idioms and code smells remain review requirements.
