// SPDX-License-Identifier: AGPL-3.0-or-later
//! Resolving a request's base directory and crate roots, and reading each
//! root's production Rust source (FR-064 "Inputs" and "Reusable consumer
//! interface").
use std::fs;
use std::path::{Path, PathBuf};

use ix_trace_rs::string_edge;

use crate::detector::has_cfg_test;
use crate::error::{Error, Result};

/// The base directory, resolved: it exists, is a directory, and has no link
/// in its own path.
pub(crate) fn resolve_base(base_dir: &Path) -> Result<PathBuf> {
    let base = fs::canonicalize(base_dir).map_err(|source| Error::BaseDirectory {
        path: base_dir.to_path_buf(),
        source,
    })?;
    if !base.is_dir() {
        return Err(Error::BaseNotDirectory {
            path: base_dir.to_path_buf(),
        });
    }
    Ok(base)
}

/// Each root, resolved against `base` (a relative root is relative to the
/// base; an absolute one is taken as is): a directory inside `base`, not
/// named twice and not inside another root.
pub(crate) fn resolve_roots(base: &Path, roots: &[PathBuf]) -> Result<Vec<PathBuf>> {
    if roots.is_empty() {
        return Err(Error::NoCrateRoots);
    }
    let mut resolved: Vec<(PathBuf, &PathBuf)> = Vec::new();
    for root in roots {
        let canonical =
            fs::canonicalize(base.join(root)).map_err(|source| Error::RootUnresolved {
                path: root.clone(),
                source,
            })?;
        if !canonical.is_dir() {
            return Err(Error::RootNotDirectory { path: root.clone() });
        }
        if !canonical.starts_with(base) {
            return Err(Error::RootOutsideBase { path: root.clone() });
        }
        for (earlier, _) in &resolved {
            if *earlier == canonical {
                return Err(Error::RootDuplicate { path: root.clone() });
            }
            if canonical.starts_with(earlier) {
                return Err(Error::RootOverlap {
                    outer: earlier.clone(),
                    inner: canonical,
                });
            }
            if earlier.starts_with(&canonical) {
                return Err(Error::RootOverlap {
                    outer: canonical,
                    inner: earlier.clone(),
                });
            }
        }
        resolved.push((canonical, root));
    }
    Ok(resolved
        .into_iter()
        .map(|(canonical, _)| canonical)
        .collect())
}

/// The parsed production source of one resolved root: every `.rs` file under
/// it that is not test code, as `(base-relative name in / form, syntax)`,
/// sorted by name.
pub(crate) fn load_root(
    base: &Path,
    root: &Path,
    roots: &[PathBuf],
) -> Result<Vec<(String, syn::File)>> {
    let mut relative_files = Vec::new();
    let mut ancestors = vec![root.to_path_buf()];
    walk(base, roots, root, root, &mut ancestors, &mut relative_files)?;
    relative_files.sort();
    let mut parsed = Vec::with_capacity(relative_files.len());
    for relative in relative_files {
        let full_path = base.join(&relative);
        let source = fs::read_to_string(&full_path).map_err(|source| Error::Read {
            path: full_path.clone(),
            source,
        })?;
        let file = syn::parse_file(&source).map_err(|source| Error::Parse {
            path: full_path,
            source,
        })?;
        parsed.push((relative, file));
    }
    let excluded = cfg_test_module_paths(&parsed);
    parsed.retain(|(name, _)| {
        !excluded
            .iter()
            .any(|test_path| Path::new(name).starts_with(test_path))
    });
    if parsed.is_empty() {
        return Err(Error::NoProductionSources {
            path: root.to_path_buf(),
        });
    }
    Ok(parsed)
}

/// Collects the `.rs` files below `dir` (skipping any directory named
/// `tests`) as base-relative names, spelled through any
/// directory link the walk followed. `resolved_dir` is `dir` with its links
/// resolved, as is every `ancestors` entry; a link that leaves `base` or
/// leads back into an ancestor is an error, so the walk never follows a
/// cycle. A filesystem-walk edge: converts directory-entry names into an
/// include/exclude decision, not a family dispatch.
#[string_edge]
fn walk(
    base: &Path,
    roots: &[PathBuf],
    dir: &Path,
    resolved_dir: &Path,
    ancestors: &mut Vec<PathBuf>,
    files: &mut Vec<String>,
) -> Result<()> {
    let entries = fs::read_dir(dir).map_err(|source| Error::Read {
        path: dir.to_path_buf(),
        source,
    })?;
    for entry in entries {
        let entry = entry.map_err(|source| Error::Read {
            path: dir.to_path_buf(),
            source,
        })?;
        let path = entry.path();
        let name = entry.file_name();
        let name = name.to_string_lossy();
        let metadata = fs::symlink_metadata(&path).map_err(|source| Error::Read {
            path: path.clone(),
            source,
        })?;
        let is_rust = path.extension().is_some_and(|extension| extension == "rs");
        let (is_dir, resolved) = if metadata.is_symlink() {
            match fs::canonicalize(&path) {
                Ok(target) => {
                    if !target.starts_with(base) {
                        return Err(Error::LinkEscapesBase { path });
                    }
                    // The target's source is already scanned under its own
                    // root (or the target holds a selected root); following the link would report it twice.
                    if roots
                        .iter()
                        .any(|root| target.starts_with(root) || root.starts_with(&target))
                        && !ancestors.contains(&target)
                    {
                        return Err(Error::LinkIntoRoot { path });
                    }
                    (target.is_dir(), Some(target))
                }
                // A dangling link to a non-Rust name selects no source.
                Err(_) if !is_rust => continue,
                Err(source) => return Err(Error::Read { path, source }),
            }
        } else {
            (metadata.is_dir(), None)
        };
        if is_dir {
            if name == "tests" {
                continue;
            }
            let resolved_child = resolved.unwrap_or_else(|| resolved_dir.join(&*name));
            if ancestors.contains(&resolved_child) {
                return Err(Error::LinkCycle { path });
            }
            ancestors.push(resolved_child.clone());
            walk(base, roots, &path, &resolved_child, ancestors, files)?;
            ancestors.pop();
        } else if is_rust {
            files.push(relative_name(base, &path));
        }
    }
    Ok(())
}

/// `path` (which lies under `base`) as a base-relative name with `/`
/// separators.
fn relative_name(base: &Path, path: &Path) -> String {
    path.strip_prefix(base)
        .unwrap_or(path)
        .components()
        .map(|component| component.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

/// The paths of every `#[cfg(test)] mod name;` file module declared by one
/// of `files` (FR-064 "excludes test code"): `<dir>/name.rs` (and its
/// `<dir>/name/` children) or `<dir>/name/mod.rs`'s directory. `<dir>` is
/// the declaring file's directory for `mod.rs`/`lib.rs`/`main.rs`, else the
/// directory named after the declaring file.
fn cfg_test_module_paths(files: &[(String, syn::File)]) -> Vec<PathBuf> {
    let mut excluded = Vec::new();
    for (name, parsed) in files {
        let declared: Vec<String> = parsed
            .items
            .iter()
            .filter_map(|item| match item {
                syn::Item::Mod(module)
                    if module.content.is_none() && has_cfg_test(&module.attrs) =>
                {
                    Some(module.ident.to_string())
                }
                _ => None,
            })
            .collect();
        if declared.is_empty() {
            continue;
        }
        let relative = Path::new(name);
        let parent = relative.parent().unwrap_or_else(|| Path::new(""));
        let stem = relative.file_stem().map(PathBuf::from).unwrap_or_default();
        let base = if is_module_root_stem(&stem) {
            parent.to_path_buf()
        } else {
            parent.join(&stem)
        };
        for name in declared {
            excluded.push(base.join(format!("{name}.rs")));
            excluded.push(base.join(&name));
        }
    }
    excluded
}

/// `mod.rs`, `lib.rs` and `main.rs` own their own directory; any other file
/// `foo.rs` owns `foo/`. A filesystem-name edge, not a family dispatch.
#[string_edge]
fn is_module_root_stem(stem: &Path) -> bool {
    matches!(stem.to_str(), Some("mod" | "lib" | "main"))
}
