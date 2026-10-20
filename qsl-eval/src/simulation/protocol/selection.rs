// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-219 "Request selection" and "Identity" (ADR-025 MM-2, MM-3, MM-5;
//! ADR-027 SE-4): the resolved memory model of each outermost `parallel`,
//! the request's selection when it names one, else the source default.
//!
//! A request entry names a `parallel` by its scoped path: the protocol's
//! name, then the names of the controls from the `run` root down to the
//! `parallel`, joined by `::` (`SB::Both` for a `run parallel Both`). The
//! resolved models are written for every outermost `parallel`, a defaulted
//! one as explicitly as a selected one, so two requests that resolve every
//! `parallel` alike carry one [`ResolvedMemory`] whether the models came
//! from source or from the request.

use std::collections::BTreeMap;

use qsl_foundation::diagnostic::Code;
use qsl_foundation::MemoryModelName;
use qsl_semantics::check::{CheckedJoinPolicy, CheckedProtocol, ProtocolNodeId};

/// One entry of a request's `memory` member (QSpec FR-331).
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct MemorySelection {
    /// The selected `parallel`'s scoped path.
    pub parallel: String,
    /// The selected model.
    pub model: MemoryModelName,
}

/// The resolved model of one outermost `parallel`.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct ResolvedParallel {
    /// The `parallel`'s scoped path.
    pub parallel: String,
    /// Its resolved model.
    pub model: MemoryModelName,
}

/// The resolved model of every outermost `parallel` of a protocol subject,
/// in ascending scoped path: the member of the subject, and so of the
/// obligation identity, the memory model enters through (MM-5).
#[derive(Clone, Debug, Default, Eq, Ord, PartialEq, PartialOrd)]
pub struct ResolvedMemory {
    parallels: Vec<ResolvedParallel>,
}

impl ResolvedMemory {
    /// FR-219 "Request selection": resolve every outermost `parallel` of
    /// `protocol` to the model `selection` names for it, else its source
    /// default.
    ///
    /// # Errors
    ///
    /// [`MemorySelectionRefusal`] for an entry that names no `parallel`, a
    /// nested `parallel`, or a `parallel` another entry names.
    pub fn resolve(
        protocol: &CheckedProtocol,
        selection: &[MemorySelection],
    ) -> Result<Self, MemorySelectionRefusal> {
        let tree = Tree::of(protocol);
        let mut chosen: BTreeMap<ProtocolNodeId, (usize, MemoryModelName)> = BTreeMap::new();
        for (entry, selected) in selection.iter().enumerate() {
            let named = || selected.parallel.clone();
            let Some(node) = tree.paths.get(&selected.parallel).copied() else {
                return Err(MemorySelectionRefusal::NotAParallel {
                    entry,
                    parallel: named(),
                });
            };
            if !tree.defaults.contains_key(&node) {
                return Err(MemorySelectionRefusal::NotAParallel {
                    entry,
                    parallel: named(),
                });
            }
            if tree.outermost_of(node) != node {
                return Err(MemorySelectionRefusal::Nested {
                    entry,
                    parallel: named(),
                });
            }
            if let Some((first, _)) = chosen.get(&node) {
                return Err(MemorySelectionRefusal::Duplicate {
                    first: *first,
                    second: entry,
                    parallel: named(),
                });
            }
            chosen.insert(node, (entry, selected.model));
        }
        let mut parallels: Vec<ResolvedParallel> = tree
            .outermost()
            .map(|(node, default)| ResolvedParallel {
                parallel: tree.scoped(node).to_owned(),
                model: chosen.get(&node).map_or(default, |(_, model)| *model),
            })
            .collect();
        parallels.sort();
        Ok(Self { parallels })
    }

    /// The resolved models, in ascending scoped path.
    pub fn parallels(&self) -> &[ResolvedParallel] {
        &self.parallels
    }

    /// The resolved model of the outermost `parallel` at `parallel`.
    pub fn model_of(&self, parallel: &str) -> Option<MemoryModelName> {
        self.parallels
            .iter()
            .find(|resolved| resolved.parallel == parallel)
            .map(|resolved| resolved.model)
    }

    /// The model governing each `parallel` of `protocol`, nested ones
    /// included: its outermost `parallel`'s resolved model (MM-3).
    ///
    /// # Errors
    ///
    /// [`MemorySelectionRefusal::Unresolved`] when this resolution names no
    /// model for an outermost `parallel` of `protocol`, and
    /// [`MemorySelectionRefusal::NotAParallel`] when it names a path that is
    /// no outermost `parallel` of it.
    pub fn governing(
        &self,
        protocol: &CheckedProtocol,
    ) -> Result<BTreeMap<ProtocolNodeId, MemoryModelName>, MemorySelectionRefusal> {
        let tree = Tree::of(protocol);
        for (entry, resolved) in self.parallels.iter().enumerate() {
            let outermost = tree
                .paths
                .get(&resolved.parallel)
                .is_some_and(|node| tree.outermost_of(*node) == *node);
            if !outermost {
                return Err(MemorySelectionRefusal::NotAParallel {
                    entry,
                    parallel: resolved.parallel.clone(),
                });
            }
        }
        let mut governing = BTreeMap::new();
        for node in tree.defaults.keys() {
            let outer = tree.outermost_of(*node);
            let path = tree.scoped(outer);
            let model = self
                .model_of(path)
                .ok_or_else(|| MemorySelectionRefusal::Unresolved {
                    parallel: path.to_owned(),
                })?;
            governing.insert(*node, model);
        }
        Ok(governing)
    }
}

/// The `parallel` controls of a protocol, by scoped path, with their
/// parents.
struct Tree {
    paths: BTreeMap<String, ProtocolNodeId>,
    scoped: BTreeMap<ProtocolNodeId, String>,
    parent: BTreeMap<ProtocolNodeId, ProtocolNodeId>,
    defaults: BTreeMap<ProtocolNodeId, MemoryModelName>,
}

impl Tree {
    fn of(protocol: &CheckedProtocol) -> Self {
        let mut paths = BTreeMap::new();
        let mut scoped = BTreeMap::new();
        let mut parent = BTreeMap::new();
        for control in &protocol.controls.controls {
            let path = std::iter::once(protocol.name.as_str())
                .chain(control.path.iter().map(String::as_str))
                .collect::<Vec<_>>()
                .join("::");
            paths.insert(path.clone(), control.node);
            scoped.insert(control.node, path);
            for child in &control.children {
                parent.insert(*child, control.node);
            }
        }
        let defaults = protocol
            .memory
            .parallels
            .iter()
            .map(|parallel| (parallel.node, parallel.memory))
            .collect();
        Self {
            paths,
            scoped,
            parent,
            defaults,
        }
    }

    fn scoped(&self, node: ProtocolNodeId) -> &str {
        self.scoped.get(&node).map_or("", String::as_str)
    }

    /// The outermost `parallel` at or above the `parallel` `node`.
    fn outermost_of(&self, node: ProtocolNodeId) -> ProtocolNodeId {
        let mut outermost = node;
        let mut below = node;
        while let Some(&above) = self.parent.get(&below) {
            if self.defaults.contains_key(&above) {
                outermost = above;
            }
            below = above;
        }
        outermost
    }

    /// Every outermost `parallel` with its source default.
    fn outermost(&self) -> impl Iterator<Item = (ProtocolNodeId, MemoryModelName)> + '_ {
        self.defaults
            .iter()
            .filter(|(node, _)| self.outermost_of(**node) == **node)
            .map(|(node, model)| (*node, *model))
    }
}

/// Why a request's `memory` member, or a subject's resolved models, refuse
/// (`invalid_runtime_input`/`invalid-value`, QSpec FR-434).
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum MemorySelectionRefusal {
    /// The entry names a control that is not a `parallel`, or no control.
    #[error(
        "invalid_runtime_input/invalid-value: memory entry {entry} names no parallel {parallel}"
    )]
    NotAParallel {
        /// The entry's position in the member.
        entry: usize,
        /// The scoped path it names.
        parallel: String,
    },
    /// The entry names a `parallel` nested in another.
    #[error("invalid_runtime_input/invalid-value: memory entry {entry} names the nested parallel {parallel}")]
    Nested {
        /// The entry's position in the member.
        entry: usize,
        /// The scoped path it names.
        parallel: String,
    },
    /// Two entries name one `parallel`.
    #[error("invalid_runtime_input/invalid-value: memory entries {first} and {second} both name {parallel}")]
    Duplicate {
        /// The first entry's position.
        first: usize,
        /// The second entry's position.
        second: usize,
        /// The scoped path both name.
        parallel: String,
    },
    /// A subject resolves no model for an outermost `parallel`.
    #[error("invalid_runtime_input/invalid-value: no resolved memory model for {parallel}")]
    Unresolved {
        /// The `parallel`'s scoped path.
        parallel: String,
    },
}

impl MemorySelectionRefusal {
    /// The refusal's catalog code: `invalid_runtime_input`.
    pub fn code(&self) -> Code {
        Code::InvalidRuntimeInput
    }

    /// The refusal's catalog cause: `invalid-value`.
    pub fn cause(&self) -> &'static str {
        "invalid-value"
    }
}

/// FR-219 "Request selection" (ADR-025 MM-3): the first `parallel`, in
/// declaration order, whose governing model is weak and whose join is not
/// `all` with no `outstanding` clause.
pub(crate) fn weak_join(
    protocol: &CheckedProtocol,
    governing: &BTreeMap<ProtocolNodeId, MemoryModelName>,
) -> Option<ProtocolNodeId> {
    protocol
        .parallels
        .iter()
        .find(|parallel| {
            governing
                .get(&parallel.node)
                .is_some_and(|model| model.is_weak())
                && (parallel.policy != CheckedJoinPolicy::All || parallel.outstanding.is_some())
        })
        .map(|parallel| parallel.node)
}

/// The memory model a protocol system runs a subject under, by its resolved
/// models: [`super::Tso`] when some outermost `parallel` resolves to `tso`,
/// else [`super::Sc`]. A subject that resolves a `parallel` to `ra` gets
/// `Sc`, which refuses it when the system is built (FR-222 is not stepped).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SubjectMemory {
    /// Every `parallel` resolves to `sc`.
    Sc,
    /// Some `parallel` resolves to `tso`, with its store buffer bound.
    Tso(super::Tso),
}

impl SubjectMemory {
    /// The memory model for `resolved`, with `max_store_buffer` entries per
    /// buffer, the published default 4 when `None` (ADR-025 MB-2).
    pub fn of(resolved: &ResolvedMemory, max_store_buffer: Option<u64>) -> Self {
        if resolved
            .parallels()
            .iter()
            .any(|parallel| parallel.model == MemoryModelName::Tso)
        {
            Self::Tso(super::Tso::new(
                max_store_buffer.unwrap_or(super::DEFAULT_MEMORY_BOUND),
            ))
        } else {
            Self::Sc
        }
    }
}
