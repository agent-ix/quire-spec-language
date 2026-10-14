// SPDX-License-Identifier: AGPL-3.0-or-later
//! The reader of an effective view's parent links (FR-081, `model-complete.md`
//! "Chain-link derivation").
//!
//! A retained `inherit` fact names one direct parent by its effective
//! identity. [`Links::new`] is the strict reader: it refuses a link that is
//! missing, of the wrong kind, owned by another type or derived from another
//! original. [`Links::expand`] is the deterministic expansion of the retained
//! facts into the path-form facts, one inherit fact per distinct inheritance
//! path and one phase-4 fact per redefiner or subsets target carrying the
//! least path to its owner. The binder never encodes or hashes an expanded
//! path; a reader pays for one under its own `max_facts`.
#![allow(
    clippy::result_large_err,
    reason = "cold refusal path; ModelRefusalCause carries DeclarationKeys inline, matching normalize's typed-failure precedent"
)]

use std::collections::{HashMap, HashSet};

use qsl_foundation::diagnostic::Code;

use crate::model::key::{
    DeclarationKey, EffectiveDeclarationPreimage, EffectiveId, RuleRef, RULE_INHERIT, RULE_QUALIFY,
    RULE_REDEFINE, RULE_SUBSET,
};
use crate::model::normalize::{ModelRefusal, ModelRefusalCause, ViewEntry};

/// One path-form derivation fact: `{ordinal, rule, inputs}`, the inputs
/// spelling the whole inheritance path.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PathFact {
    /// Zero-based position in the expanded derivation.
    pub ordinal: usize,
    /// The rule that derived the fact.
    pub rule: RuleRef,
    /// The producer keys the fact consumed, the path first.
    pub inputs: Vec<DeclarationKey>,
}

/// Why an expansion did not complete.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ExpandError {
    /// No declaration of the view has this identity.
    UnknownDeclaration(EffectiveId),
    /// The expansion would hold more than the reader's `max_facts`.
    Exceeds {
        /// The reader's ceiling.
        max_facts: u64,
    },
}

/// A validated set of effective declarations, readable by identity.
pub struct Links<'v> {
    by_id: HashMap<EffectiveId, &'v ViewEntry>,
    /// The owning type of each member's original declaration, from the
    /// declaration that qualifies it.
    owners: HashMap<&'v DeclarationKey, &'v DeclarationKey>,
}

fn mis_wired(entry: &ViewEntry, position: usize) -> ModelRefusal {
    ModelRefusal {
        code: Code::InvalidModelBinding,
        cause: ModelRefusalCause::MisWiredLink {
            original: entry.preimage.original.clone(),
            position,
        },
        detail: format!(
            "{} derivation fact {position} does not link to the declaration it states",
            entry.preimage.original.node
        ),
    }
}

impl<'v> Links<'v> {
    /// Reads `declarations`, refusing the first mis-wired link or repeated
    /// parent in declaration order.
    pub fn new(declarations: &'v [ViewEntry]) -> Result<Self, ModelRefusal> {
        let by_id: HashMap<EffectiveId, &ViewEntry> = declarations
            .iter()
            .map(|entry| (entry.effective_id, entry))
            .collect();
        let mut owners: HashMap<&DeclarationKey, &DeclarationKey> = HashMap::new();
        for entry in declarations {
            let Some(owner) = entry.preimage.owner_effective_type else {
                continue;
            };
            let Some(owner) = by_id.get(&owner) else {
                continue;
            };
            if entry
                .preimage
                .derivation
                .iter()
                .any(|fact| fact.rule == RULE_QUALIFY)
            {
                owners.insert(&entry.preimage.original, &owner.preimage.original);
            }
        }
        let links = Self { by_id, owners };
        for entry in declarations {
            links.validate(entry)?;
        }
        Ok(links)
    }

    fn validate(&self, entry: &ViewEntry) -> Result<(), ModelRefusal> {
        let preimage = &entry.preimage;
        let mut linked: HashMap<EffectiveId, usize> = HashMap::new();
        for (position, fact) in preimage.derivation.iter().enumerate() {
            if fact.rule != RULE_INHERIT {
                if fact.parent.is_some() {
                    return Err(mis_wired(entry, position));
                }
                continue;
            }
            let Some(parent_id) = fact.parent else {
                return Err(mis_wired(entry, position));
            };
            if let Some(earlier) = linked.insert(parent_id, position) {
                return Err(ModelRefusal {
                    code: Code::InvalidModelBinding,
                    cause: ModelRefusalCause::DuplicateParent {
                        original: preimage.original.clone(),
                        earlier,
                        later: position,
                    },
                    detail: format!(
                        "{} derivation links the same parent at positions {earlier} and {position}",
                        preimage.original.node
                    ),
                });
            }
            if !self.link_is_exact(preimage, &fact.inputs, parent_id) {
                return Err(mis_wired(entry, position));
            }
        }
        Ok(())
    }

    /// Whether `parent_id` is the declaration an inherit fact with `inputs`
    /// on `preimage` names: for a type, a type derived from the named
    /// supertype; for a member, the named supertype's member of the same
    /// original, the supertype being a direct parent of the member's owner.
    fn link_is_exact(
        &self,
        preimage: &EffectiveDeclarationPreimage,
        inputs: &[DeclarationKey],
        parent_id: EffectiveId,
    ) -> bool {
        let Some(parent) = self.by_id.get(&parent_id) else {
            return false;
        };
        match (
            preimage.owner_effective_type,
            parent.preimage.owner_effective_type,
        ) {
            (None, None) => matches!(inputs, [supertype] if *supertype == parent.preimage.original),
            (Some(owner), Some(parent_owner)) => {
                let [supertype, member] = inputs else {
                    return false;
                };
                let (Some(owner_entry), Some(parent_owner_entry)) =
                    (self.by_id.get(&owner), self.by_id.get(&parent_owner))
                else {
                    return false;
                };
                *member == preimage.original
                    && *member == parent.preimage.original
                    && *supertype == parent_owner_entry.preimage.original
                    && owner_entry
                        .preimage
                        .derivation
                        .iter()
                        .any(|fact| fact.rule == RULE_INHERIT && fact.parent == Some(parent_owner))
            }
            _ => false,
        }
    }

    /// The path-form derivation of the declaration `id`: its retained facts
    /// expanded through the parent identities this reader holds, in phase,
    /// input and rule order with multiplicity.
    pub fn expand(&self, id: &EffectiveId, max_facts: u64) -> Result<Vec<PathFact>, ExpandError> {
        let entry = self
            .by_id
            .get(id)
            .ok_or(ExpandError::UnknownDeclaration(*id))?;
        let mut expanded: Vec<(u8, PathFact)> = Vec::new();
        let mut budget = Budget::new(max_facts);
        for fact in &entry.preimage.derivation {
            if fact.rule == RULE_QUALIFY {
                budget.take(1)?;
                expanded.push((2, path_fact(fact.rule.clone(), fact.inputs.clone())));
            } else if fact.rule == RULE_INHERIT {
                self.expand_inherit(entry, fact.parent, &mut expanded, &mut budget)?;
            } else {
                budget.take(1)?;
                let path = self.least_path(entry, fact.inputs.first());
                let mut inputs = path;
                inputs.extend(fact.inputs.iter().cloned());
                expanded.push((4, path_fact(fact.rule.clone(), inputs)));
            }
        }
        expanded.sort_by(|(left_phase, left), (right_phase, right)| {
            left_phase
                .cmp(right_phase)
                .then_with(|| left.inputs.cmp(&right.inputs))
                .then_with(|| rule_rank(&left.rule).cmp(&rule_rank(&right.rule)))
        });
        Ok(expanded
            .into_iter()
            .enumerate()
            .map(|(ordinal, (_, fact))| PathFact { ordinal, ..fact })
            .collect())
    }

    /// Every distinct inheritance path from `entry` through `parent`, in
    /// ascending-key depth-first order (explicit stack). A type's every
    /// prefix of a path is a fact; a member's path runs to the declaration
    /// that owns it.
    fn expand_inherit(
        &self,
        entry: &ViewEntry,
        parent: Option<EffectiveId>,
        out: &mut Vec<(u8, PathFact)>,
        budget: &mut Budget,
    ) -> Result<(), ExpandError> {
        let member = entry
            .preimage
            .owner_effective_type
            .map(|_| &entry.preimage.original);
        let Some(first) = parent.and_then(|id| self.by_id.get(&id)).copied() else {
            return Ok(());
        };
        // Each frame is a declaration on the current path and the position of
        // the next inherit fact to follow from it.
        let mut stack: Vec<(&ViewEntry, usize)> = vec![(first, 0)];
        let mut types: Vec<DeclarationKey> = vec![self.supertype_key(first, member)];
        emit_inherit(first, &types, member, out, budget)?;
        while let Some(&(at, next)) = stack.last() {
            let step = at
                .preimage
                .derivation
                .iter()
                .enumerate()
                .skip(next)
                .find(|(_, fact)| fact.rule == RULE_INHERIT);
            let Some((position, fact)) = step else {
                stack.pop();
                types.pop();
                continue;
            };
            if let Some(frame) = stack.last_mut() {
                frame.1 = position + 1;
            }
            let Some(child) = fact.parent.and_then(|id| self.by_id.get(&id)).copied() else {
                continue;
            };
            types.push(self.supertype_key(child, member));
            stack.push((child, 0));
            emit_inherit(child, &types, member, out, budget)?;
        }
        Ok(())
    }

    /// The supertype `parent` stands for on a path: the type itself, or the
    /// owner of the member `parent` is.
    fn supertype_key(&self, parent: &ViewEntry, member: Option<&DeclarationKey>) -> DeclarationKey {
        match (parent.preimage.owner_effective_type, member) {
            (Some(owner), Some(_)) => self.by_id.get(&owner).map_or_else(
                || parent.preimage.original.clone(),
                |owner| owner.preimage.original.clone(),
            ),
            _ => parent.preimage.original.clone(),
        }
    }

    /// The types from `entry`'s own type (exclusive) to the owner of
    /// `declaring` (inclusive): the first path in ascending-key depth-first
    /// order, empty when the owner is the type itself.
    fn least_path(
        &self,
        entry: &ViewEntry,
        declaring: Option<&DeclarationKey>,
    ) -> Vec<DeclarationKey> {
        let Some(target) = declaring.and_then(|key| self.owners.get(key)) else {
            return Vec::new();
        };
        let start = match entry.preimage.owner_effective_type {
            Some(owner) => self.by_id.get(&owner).copied(),
            None => Some(entry),
        };
        let Some(start) = start else {
            return Vec::new();
        };
        if &start.preimage.original == *target {
            return Vec::new();
        }
        let mut dead: HashSet<EffectiveId> = HashSet::new();
        let mut stack: Vec<(&ViewEntry, usize)> = vec![(start, 0)];
        while let Some(&(at, next)) = stack.last() {
            let step = at
                .preimage
                .derivation
                .iter()
                .enumerate()
                .skip(next)
                .find(|(_, fact)| fact.rule == RULE_INHERIT);
            let Some((position, fact)) = step else {
                dead.insert(at.effective_id);
                stack.pop();
                continue;
            };
            if let Some(frame) = stack.last_mut() {
                frame.1 = position + 1;
            }
            let Some(parent) = fact.parent.and_then(|id| self.by_id.get(&id)) else {
                continue;
            };
            if &parent.preimage.original == *target {
                let mut path: Vec<DeclarationKey> = stack
                    .iter()
                    .skip(1)
                    .map(|(visited, _)| visited.preimage.original.clone())
                    .collect();
                path.push(parent.preimage.original.clone());
                return path;
            }
            if !dead.contains(&parent.effective_id) {
                stack.push((parent, 0));
            }
        }
        Vec::new()
    }
}

/// Records the inherit fact for the path `types` ending at `at`.
fn emit_inherit(
    at: &ViewEntry,
    types: &[DeclarationKey],
    member: Option<&DeclarationKey>,
    out: &mut Vec<(u8, PathFact)>,
    budget: &mut Budget,
) -> Result<(), ExpandError> {
    if member.is_some()
        && at
            .preimage
            .derivation
            .iter()
            .any(|fact| fact.rule == RULE_INHERIT)
    {
        return Ok(());
    }
    budget.take(1)?;
    out.push((3, path_fact(RULE_INHERIT, closed(types, member))));
    Ok(())
}

fn path_fact(rule: RuleRef, inputs: Vec<DeclarationKey>) -> PathFact {
    PathFact {
        ordinal: 0,
        rule,
        inputs,
    }
}

/// The types of a path followed by the member it ends at, if any.
fn closed(types: &[DeclarationKey], member: Option<&DeclarationKey>) -> Vec<DeclarationKey> {
    let mut inputs = types.to_vec();
    inputs.extend(member.cloned());
    inputs
}

/// Phase-4 rules of equal inputs order by the rule identity's bytes, which
/// puts `redefine` before `subset`.
fn rule_rank(rule: &RuleRef) -> u8 {
    if *rule == RULE_REDEFINE {
        0
    } else if *rule == RULE_SUBSET {
        1
    } else {
        2
    }
}

/// The facts a reader still admits.
struct Budget {
    remaining: u64,
    max_facts: u64,
}

impl Budget {
    fn new(max_facts: u64) -> Self {
        Self {
            remaining: max_facts,
            max_facts,
        }
    }

    fn take(&mut self, facts: u64) -> Result<(), ExpandError> {
        self.remaining = self
            .remaining
            .checked_sub(facts)
            .ok_or(ExpandError::Exceeds {
                max_facts: self.max_facts,
            })?;
        Ok(())
    }
}
