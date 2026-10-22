// SPDX-License-Identifier: AGPL-3.0-or-later
//! ADR-018 LA-3 and LA-5: the vocabulary of EN-1, QSL's explicit-state
//! model checker, that the core owns: its run limits ([`limits`]), its
//! outcome ([`outcome`]), the item it decides ([`item`]), the counterexample it returns ([`counterexample`]),
//! the counterexample's replay ([`replay`], FR-128) and the one settlement
//! map from its outcome to a terminal record ([`settle`], FR-127), and the
//! outcome of a state-graph item instance ([`state_graph`], FR-168) and its
//! settlement map ([`graph_settle`], FR-169), the outcome of an HP-5 item
//! instance ([`existential_outcome`], FR-181), the counterexample ([`hyper_counterexample`]) and outcome
//! ([`hyper_outcome`]) of an EN-1 run over an HP-2 hyperproperty (FR-176) and
//! its replay ([`hyper_replay`], FR-183), and the settlement of a per-step
//! simulation record ([`step_settle`], FR-146). The engine itself is
//! `qsl-analyze`'s; nothing here names it.

pub mod counterexample;
pub mod existential_outcome;
pub mod graph_settle;
pub mod hyper_counterexample;
pub mod hyper_outcome;
pub mod hyper_replay;
pub mod hyper_settle;
pub mod identity;
pub mod item;
pub mod limits;
pub mod outcome;
pub mod refinement_replay;
pub mod refinement_settle;
pub mod replay;
pub mod settle;
pub mod state_graph;
pub mod step_settle;
