// SPDX-License-Identifier: AGPL-3.0-or-later
//! ADR-018 LA-3 and LA-5: the vocabulary of EN-1, QSL's explicit-state
//! model checker, that the core owns: its run limits ([`limits`]), its
//! outcome ([`outcome`]), the item it decides ([`item`]), the counterexample it returns ([`counterexample`]),
//! the counterexample's replay ([`replay`], FR-128) and the one settlement
//! map from its outcome to a terminal record ([`settle`], FR-127), and the
//! outcome of a state-graph item instance ([`state_graph`], FR-168) and its
//! settlement map ([`graph_settle`], FR-169). The engine itself is
//! `qsl-analyze`'s; nothing here names it.

pub mod counterexample;
pub mod graph_settle;
pub mod identity;
pub mod item;
pub mod limits;
pub mod outcome;
pub mod replay;
pub mod settle;
pub mod state_graph;
