// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-366: the role of every type-level clause of a domain package and the
//! effective invariants of every type.
//!
//! A clause a domain package attaches to a type is an invariant, a guard or
//! an unevaluated clause. The role is a function of the admitted package
//! alone: the same package gives the same roles in any document order.

use std::collections::{BTreeMap, BTreeSet};

use crate::model::domain_package::{DomainPackage, DomainPackageRecord};
use crate::model::intake::member_identity_name;
use crate::model::key::DeclarationKey;
use crate::model::normalize::EffectiveView;

/// What a type-level clause is (FR-366 "Classification").
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ClauseRole {
    /// A `quire` clause no transition names as its guard: evaluated for
    /// every value of its type.
    Invariant,
    /// A `quire` clause a transition of its state machine names as its
    /// `guard`: not an invariant, not conjoined to any, not evaluated here.
    Guard,
    /// A clause in another language: kept, never parsed, checked or
    /// evaluated.
    Unevaluated,
}

/// Every type-level clause's [`ClauseRole`] and every type's effective
/// invariants, read from one admitted package.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct TypeClauseRoles {
    roles: BTreeMap<DeclarationKey, ClauseRole>,
    effective: BTreeMap<DeclarationKey, Vec<DeclarationKey>>,
}

impl TypeClauseRoles {
    /// The roles and effective invariants of `package`.
    #[qsl_attrs::string_edge]
    pub fn of(package: &DomainPackage) -> Self {
        let mut guarded: BTreeSet<(&DeclarationKey, &str)> = BTreeSet::new();
        let mut supertypes: BTreeMap<&DeclarationKey, &[DeclarationKey]> = BTreeMap::new();
        for record in &package.records {
            match record {
                DomainPackageRecord::StateMachine(machine) => {
                    supertypes.insert(&machine.key, &machine.supertypes);
                    for guard in machine
                        .transitions
                        .iter()
                        .filter_map(|transition| transition.guard.as_deref())
                    {
                        guarded.insert((&machine.key, guard));
                    }
                }
                DomainPackageRecord::ObjectType(object) => {
                    supertypes.insert(&object.key, &object.supertypes);
                }
                DomainPackageRecord::RecordValueType(record) => {
                    supertypes.insert(&record.key, &record.supertypes);
                }
                DomainPackageRecord::EventType(event) => {
                    supertypes.insert(&event.key, &event.supertypes);
                }
                DomainPackageRecord::Process(process) => {
                    supertypes.insert(&process.key, &process.supertypes);
                }
                _ => {}
            }
        }
        let mut roles = BTreeMap::new();
        let mut own: BTreeMap<&DeclarationKey, Vec<&DeclarationKey>> = BTreeMap::new();
        for record in &package.records {
            let DomainPackageRecord::Clause(clause) = record else {
                continue;
            };
            let role = if clause.language != "quire" {
                ClauseRole::Unevaluated
            } else if member_identity_name(&clause.owner.node, &clause.key.node)
                .is_some_and(|name| guarded.contains(&(&clause.owner, name)))
            {
                ClauseRole::Guard
            } else {
                own.entry(&clause.owner).or_default().push(&clause.key);
                ClauseRole::Invariant
            };
            roles.insert(clause.key.clone(), role);
        }
        for keys in own.values_mut() {
            keys.sort();
        }
        let mut effective = BTreeMap::new();
        for (ty, direct) in &supertypes {
            let mut ancestors = BTreeSet::new();
            let mut pending: Vec<&DeclarationKey> = direct.iter().collect();
            while let Some(ancestor) = pending.pop() {
                if ancestors.insert(ancestor) {
                    pending.extend(supertypes.get(ancestor).copied().unwrap_or(&[]));
                }
            }
            ancestors.remove(*ty);
            let list: Vec<DeclarationKey> = std::iter::once(*ty)
                .chain(ancestors)
                .flat_map(|owner| own.get(owner).into_iter().flatten())
                .map(|key| (*key).clone())
                .collect();
            if !list.is_empty() {
                effective.insert((*ty).clone(), list);
            }
        }
        Self { roles, effective }
    }

    /// The role of the clause `key`; `None` when `key` names no type-level
    /// clause of the package.
    pub fn role(&self, key: &DeclarationKey) -> Option<ClauseRole> {
        self.roles.get(key).copied()
    }

    /// Every type that has an effective invariant, ascending by key, with
    /// the list [`Self::effective_invariants`] gives it.
    pub fn types(&self) -> impl Iterator<Item = (&DeclarationKey, &[DeclarationKey])> {
        self.effective
            .iter()
            .map(|(ty, invariants)| (ty, invariants.as_slice()))
    }

    /// The effective invariants of the type `ty`: its own `Invariant` clauses
    /// followed by those of each ancestor, ancestors ascending by key, each
    /// group ascending by clause key. A guard is in no list.
    pub fn effective_invariants(&self, ty: &DeclarationKey) -> &[DeclarationKey] {
        self.effective.get(ty).map_or(&[], Vec::as_slice)
    }
}

impl EffectiveView {
    /// FR-366: the role of every type-level clause of this package and the
    /// effective invariants of its types.
    pub fn type_clause_roles(&self) -> TypeClauseRoles {
        TypeClauseRoles::of(self.domain_package())
    }
}
