// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-174 "Counterexamples and verdicts" (ADR-023 CS-4): a path of the
//! quotient by the copy group as the concrete joint behaviour it stands for.
//!
//! A stored state is one member of its orbit, and each retained edge keeps the
//! permutation its target was canonicalised by. The concrete tuple a path
//! stands for starts at the stored root, which is itself a tuple of initial
//! states, and follows the edges: where a stored state's component `m` stands
//! for concrete component `k`, the edge's permutation says which stored
//! component of the target the same behaviour continues in. The path is
//! repeated until the concrete tuple closes, at most the order of the
//! permutation it composes to.

use qsl_eval::hyper::{copy_population, HyperState, HyperTransition};
use qsl_eval::simulation::Permutation;
use qsl_foundation::digest::DigestRecord;

use crate::model_check::check::fault;
use crate::model_check::graph::{ProductStateId, RetainedEdge};
use crate::model_check::ModelCheckRefusal;

use super::{Explored, Run};

/// A path of retained edges as the concrete joint behaviour it stands for.
pub(super) struct Lifted {
    /// The concrete state at each joint position of the path, the root first.
    pub(super) states: Vec<HyperState>,
    /// The joint step into each position after the first, with the digest of
    /// each component's post-state.
    pub(super) steps: Vec<(HyperTransition, Vec<DigestRecord>)>,
    /// Component `k` of the last concrete state is component `positions[k]` of
    /// the stored state it stands for.
    pub(super) positions: Vec<usize>,
}

/// `items` read through `positions`: element `k` is `items[positions[k]]`.
/// Elements past `positions` stay in place, as an HP-3 step's existential
/// components do.
pub(super) fn through<T: Clone>(items: &[T], positions: &[usize]) -> Vec<T> {
    positions
        .iter()
        .filter_map(|from| items.get(*from))
        .chain(items.iter().skip(positions.len()))
        .cloned()
        .collect()
}

impl Run<'_, '_> {
    /// The stored component each stored component of an edge's source goes
    /// to in its target: `slots[m]` is the position of the variable that
    /// `permutation` moves component `m`'s variable to.
    fn slots(
        &self,
        permutation: &Permutation,
        components: usize,
    ) -> Result<Vec<usize>, ModelCheckRefusal> {
        let population = copy_population();
        let quantifiers = self.request.clause.quantifiers();
        let names: Vec<&str> = quantifiers
            .iter()
            .take(components)
            .map(|quantifier| quantifier.variable.as_str())
            .collect();
        names
            .iter()
            .map(|name| {
                let moved = permutation.image(&population, name);
                names
                    .iter()
                    .position(|candidate| *candidate == moved)
                    .ok_or_else(|| fault("a-copy-permutation-moves-a-variable-of-the-clause"))
            })
            .collect()
    }

    /// `edges`, a path from `root`, as the concrete behaviour it stands for.
    pub(super) fn lift(
        &self,
        run: &Explored<'_, '_>,
        root: ProductStateId,
        edges: &[&RetainedEdge<HyperTransition>],
    ) -> Result<Lifted, ModelCheckRefusal> {
        let stored = |id: ProductStateId| {
            run.graph
                .state(id)
                .map(|state| &state.state)
                .ok_or_else(|| fault("a-path-runs-through-retained-states"))
        };
        let reorder = |state: &HyperState, positions: &[usize]| HyperState {
            components: through(&state.components, positions),
            property: state.property.clone(),
        };
        let root_state = stored(root)?;
        let mut positions: Vec<usize> = (0..root_state.components.len()).collect();
        let mut states = vec![root_state.clone()];
        let mut steps = Vec::with_capacity(edges.len());
        for edge in edges {
            let target = stored(edge.target)?;
            let slots = self.slots(&edge.permutation, target.components.len())?;
            let step = HyperTransition(through(&edge.transition.0, &positions));
            positions = positions
                .iter()
                .map(|from| {
                    slots
                        .get(*from)
                        .copied()
                        .ok_or_else(|| fault("a-slot-for-every-component"))
                })
                .collect::<Result<_, _>>()?;
            let concrete = reorder(target, &positions);
            steps.push((step, self.digests(&concrete)?));
            states.push(concrete);
        }
        Ok(Lifted {
            states,
            steps,
            positions,
        })
    }

    /// How many times the loop `edges` is traversed for the concrete tuple to
    /// close: the order of the permutation of components the loop composes
    /// to. One under the trivial group.
    pub(super) fn repeats(
        &self,
        edges: &[&RetainedEdge<HyperTransition>],
        components: usize,
    ) -> Result<usize, ModelCheckRefusal> {
        let identity: Vec<usize> = (0..components).collect();
        let mut once = identity.clone();
        for edge in edges {
            let slots = self.slots(&edge.permutation, components)?;
            once = once
                .iter()
                .map(|from| {
                    slots
                        .get(*from)
                        .copied()
                        .ok_or_else(|| fault("a-slot-for-every-component"))
                })
                .collect::<Result<_, _>>()?;
        }
        let mut repeats = 1;
        let mut composed = once.clone();
        while composed != identity {
            composed = composed.iter().map(|at| once[*at]).collect();
            repeats += 1;
        }
        Ok(repeats)
    }
}
