// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSL #131 PR 3, M1: the FCD rev a vendored `Source::Fcd` fixture tree is
//! read from must live in exactly one place -- this workspace's own
//! `Cargo.toml` `agent-ix-semantic-ir` git dependency (and `Cargo.lock`'s
//! locked resolution of it) -- never repeated by hand in a vendored tree's
//! `VENDOR.json`. This module reads that one place so `revendor_check` can
//! refuse a `VENDOR.json` `Fcd` source whose `commit` has drifted from it.
use crate::error::{Error, Result};
use std::path::Path;

/// The crate name whose pinned git rev is this repository's single source
/// of truth for every `Source::Fcd` vendoring pin (QSL #131 PR 3's own
/// vendored fixtures are read from the same commit this crate resolves
/// `agent-ix-semantic-ir`/`agent-ix-extraction-frontend` to).
pub const AGENT_IX_SEMANTIC_IR: &str = "agent-ix-semantic-ir";

/// Reads `agent-ix-semantic-ir`'s pinned git rev from `Cargo.toml`'s own
/// dependency line, confirms `Cargo.lock`'s locked resolution names the
/// same rev, and returns it.
///
/// Refuses, with a typed error, if the two disagree (the "one place" this
/// rev lives has split into two) or if either file does not pin
/// `agent-ix-semantic-ir` to a git rev at all.
pub fn read_agent_ix_semantic_ir_rev(workspace_root: &Path) -> Result<String> {
    let cargo_toml_path = workspace_root.join("Cargo.toml");
    let cargo_toml = std::fs::read_to_string(&cargo_toml_path)
        .map_err(|source| Error::io(&cargo_toml_path, source))?;
    let cargo_toml_rev = dependency_rev(&cargo_toml, AGENT_IX_SEMANTIC_IR).ok_or_else(|| {
        Error::CargoPinMissing {
            path: cargo_toml_path.clone(),
            crate_name: AGENT_IX_SEMANTIC_IR,
        }
    })?;

    let cargo_lock_path = workspace_root.join("Cargo.lock");
    let cargo_lock = std::fs::read_to_string(&cargo_lock_path)
        .map_err(|source| Error::io(&cargo_lock_path, source))?;
    let cargo_lock_rev =
        locked_rev(&cargo_lock, AGENT_IX_SEMANTIC_IR).ok_or_else(|| Error::CargoPinMissing {
            path: cargo_lock_path.clone(),
            crate_name: AGENT_IX_SEMANTIC_IR,
        })?;

    if cargo_toml_rev != cargo_lock_rev {
        return Err(Error::CargoPinDisagreement {
            crate_name: AGENT_IX_SEMANTIC_IR,
            cargo_toml_rev,
            cargo_lock_rev,
        });
    }
    Ok(cargo_toml_rev)
}

/// `Cargo.toml`'s own `<crate_name> = { ..., rev = "<rev>", ... }`
/// dependency line's `rev`, read as plain text -- this workspace has no
/// `toml` crate dependency to parse the file structurally, and every git
/// dependency here is declared on one line.
fn dependency_rev(cargo_toml: &str, crate_name: &str) -> Option<String> {
    let prefix = format!("{crate_name} =");
    let line = cargo_toml
        .lines()
        .find(|line| line.trim_start().starts_with(&prefix))?;
    extract_rev_field(line)
}

/// `Cargo.lock`'s own `[[package]]` block for `crate_name`'s `source =
/// "git+<repo>?rev=<rev>#<rev>"` line's `rev` query parameter.
fn locked_rev(cargo_lock: &str, crate_name: &str) -> Option<String> {
    let marker = format!("name = \"{crate_name}\"");
    let mut lines = cargo_lock.lines();
    while let Some(line) = lines.next() {
        if line.trim() != marker {
            continue;
        }
        for line in lines.by_ref() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                return None;
            }
            if let Some(source) = trimmed.strip_prefix("source = \"") {
                let after_rev = source.split_once("?rev=")?.1;
                let rev = after_rev.split(['#', '"']).next()?;
                return Some(rev.to_owned());
            }
        }
        return None;
    }
    None
}

/// The value of a `rev = "..."` field found anywhere in `text`.
fn extract_rev_field(text: &str) -> Option<String> {
    let at = text.find("rev = \"")?;
    let rest = &text[at + "rev = \"".len()..];
    let end = rest.find('"')?;
    Some(rest[..end].to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use ix_trace_rs::trace;

    /// Tracing: QSL #131 PR 3, M1.
    #[trace("TC-152", "NFR-011-AC-4")]
    #[test]
    fn dependency_rev_reads_a_single_line_git_dependency() {
        let cargo_toml = "agent-ix-semantic-ir = { git = \"https://example.invalid\", rev = \"abc123\" }\nother-crate = \"1.0\"\n";
        assert_eq!(
            dependency_rev(cargo_toml, "agent-ix-semantic-ir"),
            Some("abc123".to_owned())
        );
    }

    /// Tracing: QSL #131 PR 3, M1.
    #[trace("TC-152", "NFR-011-AC-4")]
    #[test]
    fn locked_rev_reads_the_source_field_of_the_named_packages_block() {
        let cargo_lock = "[[package]]\nname = \"other\"\nversion = \"0.0.0\"\n\n\
             [[package]]\nname = \"agent-ix-semantic-ir\"\nversion = \"0.0.0\"\n\
             source = \"git+https://example.invalid?rev=abc123#abc123\"\n\n\
             [[package]]\nname = \"another\"\nversion = \"0.0.0\"\n";
        assert_eq!(
            locked_rev(cargo_lock, "agent-ix-semantic-ir"),
            Some("abc123".to_owned())
        );
    }

    /// Tracing: QSL #131 PR 3, M1.
    #[trace("TC-152", "NFR-011-AC-4")]
    #[test]
    fn read_agent_ix_semantic_ir_rev_matches_this_workspaces_own_pin() {
        let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("xtask is one level under the workspace root")
            .to_path_buf();
        let rev = read_agent_ix_semantic_ir_rev(&workspace_root)
            .expect("this workspace's own Cargo.toml/Cargo.lock agree on one rev");
        assert_eq!(rev.len(), 40, "a full commit sha, got {rev:?}");
        assert!(rev.bytes().all(|byte| byte.is_ascii_hexdigit()), "{rev:?}");
    }

    /// Tracing: QSL #131 PR 3, M1.
    #[trace("TC-152", "NFR-011-AC-4")]
    #[test]
    fn read_agent_ix_semantic_ir_rev_refuses_a_toml_lock_disagreement() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("Cargo.toml"),
            "agent-ix-semantic-ir = { git = \"https://example.invalid\", rev = \"aaaa\" }\n",
        )
        .unwrap();
        std::fs::write(
            dir.path().join("Cargo.lock"),
            "[[package]]\nname = \"agent-ix-semantic-ir\"\nversion = \"0.0.0\"\n\
             source = \"git+https://example.invalid?rev=bbbb#bbbb\"\n",
        )
        .unwrap();
        let error = read_agent_ix_semantic_ir_rev(dir.path()).unwrap_err();
        match error {
            Error::CargoPinDisagreement {
                crate_name,
                cargo_toml_rev,
                cargo_lock_rev,
            } => {
                assert_eq!(crate_name, "agent-ix-semantic-ir");
                assert_eq!(cargo_toml_rev, "aaaa");
                assert_eq!(cargo_lock_rev, "bbbb");
            }
            other => panic!("expected CargoPinDisagreement, got {other:?}"),
        }
    }
}
