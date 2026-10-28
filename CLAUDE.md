# Contributor guidance

Follow [AGENTS.md](AGENTS.md) for repository instructions.

## Hash / digest / pin prohibition

Do not add, keep or refresh any SHA, digest, hash, checksum, pin, lock, revision,
fingerprint, golden value, stamp or inventory that records, compares against or
gates on a frozen file, version, revision or tool outside Cargo.toml and
Cargo.lock, whatever it is named. A rename, wrapper, helper, derived digest or
move of such a record is the same violation. When you touch code, specs, docs
or comments that contain one, remove it in the same change. Package and tool
versions live in Cargo.toml, Cargo.lock and rust-toolchain.toml only; text points
there or states nothing. Reports and reviews do not record commit SHAs. A question
about a content-identity digest goes to a specification ruling (STD-551), not to
an exemption written here.

For unfinished issue references write `Remaining work: #N`; never put a GitHub closing keyword next to an issue number in a negated sentence, because GitHub may still close it.

Keep prototype bookkeeping minimal. Use ordinary Git history, PR links and brief
test results. Do not create successor draft files for working history, or record every
intermediate step in status/review logs.
Update existing artifacts and tickets for material decisions and remaining work.
Reviews happen at PR readiness.

At substantial PR checkpoints, `/code-review` includes the actual
`agent-skills/rust-review/SKILL.md`; QUOIN `/gap-analysis` is also required.
Choose applicable `/spec-review` analyses rather than automatically running all
seven optional analyses. Rust idioms and code smells remain review requirements.
