// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-315 "Hyper self-composition queries" and "Product identity": the
//! request of `encode_hyper_query`, the query it returns, and the identity of
//! the HP-2 product and instance a query encodes.

use std::collections::BTreeSet;

use qsl_eval::model_subject::ModelSubject;
use qsl_eval::simulation::model::ModelSystem;
use qsl_eval::smt::{
    encode_hyper_script, HyperScriptRequest, Refusal, SmtEncodingLimits, SmtEncodingRefusal,
    SmtQueryKind,
};
use qsl_foundation::source::provenance::OccurrenceKey;
use qsl_semantics::check::{CheckedHyperClause, HyperForm};
use quire_exact::{Meter, ValueType};

use crate::model_check::subject_identity::SubjectIdentity;

/// FR-315's request for one instance of an HP-2 clause.
///
/// The request has these five members and no other: the clause and the
/// subjects carry the checked nodes the encoder reads, so it needs no
/// evaluator seam.
pub struct HyperQueryRequest<'a> {
    /// The checked hyper clause (FR-172).
    pub clause: &'a CheckedHyperClause,
    /// One subject per alias, in `clause.aliases()` order.
    pub subjects: &'a [ModelSubject<'a>],
    /// One `(parameter, key)` pair per object parameter, in parameter order
    /// (FR-183's binding).
    pub instance: &'a [(String, String)],
    /// The budget for evaluating `μ_U` while the query is encoded.
    pub meter: &'a mut Meter,
    /// The request's encoding limits.
    pub encoding: SmtEncodingLimits,
}

/// An encoded query of an HP-2 product.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HyperQuery {
    /// The product and instance the script encodes.
    pub product: HyperProductIdentity,
    /// The query kind.
    pub kind: SmtQueryKind,
    /// The canonical SMT-LIB 2 script.
    pub script: Vec<u8>,
}

/// QSL's identity of the HP-2 product and instance a hyper query encodes.
///
/// It is made of existing canonical identities only, has no digest of its
/// own and no wire form, and only [`HyperProductIdentity::of`] and
/// `encode_hyper_query` make one: a caller cannot write one as a struct
/// literal, serialize one or deserialize one (FR-315-AC-14).
///
/// ```compile_fail,E0451
/// use qsl_replay::HyperProductIdentity;
///
/// fn minted() -> HyperProductIdentity {
///     HyperProductIdentity {
///         clause: todo!(),
///         subjects: Vec::new(),
///         instance: Vec::new(),
///     }
/// }
/// ```
///
/// ```compile_fail,E0277
/// use qsl_replay::HyperProductIdentity;
///
/// fn serializes<T: serde::Serialize>() {}
/// serializes::<HyperProductIdentity>();
/// ```
///
/// ```compile_fail,E0277
/// use qsl_replay::HyperProductIdentity;
///
/// fn deserializes<T: serde::de::DeserializeOwned>() {}
/// deserializes::<HyperProductIdentity>();
/// ```
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct HyperProductIdentity {
    clause: OccurrenceKey,
    subjects: Vec<SubjectIdentity>,
    instance: Vec<(String, String)>,
}

impl HyperProductIdentity {
    /// The identity of the product and instance `request` names. It
    /// evaluates no `μ_U` conjunct and charges nothing to the request's
    /// meter.
    ///
    /// # Errors
    ///
    /// [`SmtEncodingRefusal`] on the first of `hyper-form`,
    /// `subject-binding`, `instance-binding` and `product-identity` that
    /// applies, in that order.
    pub fn of(request: &HyperQueryRequest<'_>) -> Result<Self, SmtEncodingRefusal> {
        let clause = request.clause;
        if !matches!(clause.form(), Some(HyperForm::Universal)) {
            return Err(Refusal::HyperForm {
                clause: clause.name().to_owned(),
                form: form_name(clause.form()),
            }
            .into());
        }
        check_subjects(clause, request.subjects)?;
        check_instance(clause, request.subjects, request.instance)?;
        let package = request.subjects[0].simulation.package;
        let no_identity = || Refusal::ProductIdentity {
            clause: clause.name().to_owned(),
        };
        let occurrence = clause.assembled_occurrence().ok_or_else(no_identity)?;
        let subjects = request
            .subjects
            .iter()
            .map(SubjectIdentity::of)
            .collect::<Vec<_>>();
        if !package
            .graph()
            .hyper_clauses()
            .iter()
            .any(|declared| declared.assembled_occurrence() == Some(occurrence))
        {
            return Err(no_identity().into());
        }
        Ok(Self {
            clause: occurrence.clone(),
            subjects,
            instance: request.instance.to_vec(),
        })
    }

    /// The clause's `hyperproperty` claim occurrence in the checked-package
    /// v2 emission.
    pub const fn clause(&self) -> &OccurrenceKey {
        &self.clause
    }

    /// FR-336's projection of each subject, in `clause.aliases()` order.
    pub fn subjects(&self) -> &[SubjectIdentity] {
        &self.subjects
    }

    /// The request's instance, in parameter order.
    pub fn instance(&self) -> &[(String, String)] {
        &self.instance
    }
}

/// The form a clause has, as `hyper-form` names it.
const fn form_name(form: Option<&HyperForm>) -> &'static str {
    match form {
        None => "a clause under a profile other than infinite-trace",
        Some(HyperForm::StepRelation) => "HP-1",
        Some(HyperForm::Universal) => "HP-2",
        Some(HyperForm::ForallExistsSafety) => "HP-3",
        Some(HyperForm::Other { .. }) => "HP-4",
        Some(HyperForm::SingleExistential) => "HP-5",
        Some(HyperForm::ProjectionAligned) => "HP-6",
    }
}

/// `subject-binding`: one subject per alias, all over the first subject's
/// checked package. The checked graph keeps no alias-to-model map, so the
/// model of an alias is read as the package it is declared in.
fn check_subjects(
    clause: &CheckedHyperClause,
    subjects: &[ModelSubject<'_>],
) -> Result<(), SmtEncodingRefusal> {
    let aliases = clause.aliases();
    let binding = |alias: Option<&str>| Refusal::SubjectBinding {
        alias: alias.map(str::to_owned),
    };
    let Some(first) = subjects.first() else {
        return Err(binding(aliases.first().map(|alias| alias.as_str())).into());
    };
    for (index, alias) in aliases.iter().enumerate() {
        let Some(subject) = subjects.get(index) else {
            return Err(binding(Some(alias.as_str())).into());
        };
        let (one, other) = (first.simulation.package, subject.simulation.package);
        if !std::ptr::eq(one, other) {
            return Err(binding(Some(alias.as_str())).into());
        }
    }
    if subjects.len() > aliases.len() {
        return Err(binding(None).into());
    }
    Ok(())
}

/// `instance-binding`: one pair per parameter, in parameter order, each key
/// in that parameter's universe: the keys of the populations of its alias's
/// subject that admit the parameter's object type. A subject that FR-120 does
/// not admit lists no populations, so its key is read against the keys of all
/// of the subject's universes, and its admission is refused by
/// `encode_hyper_query` after the identity checks. A subject that admits and
/// has no listable universe for the parameter refuses `instance-binding`.
///
/// # Errors
///
/// A refusal as above; a fault when a parameter names an alias that is not
/// an alias of the clause.
fn check_instance(
    clause: &CheckedHyperClause,
    subjects: &[ModelSubject<'_>],
    instance: &[(String, String)],
) -> Result<(), SmtEncodingRefusal> {
    let binding = |parameter: &str| Refusal::InstanceBinding {
        parameter: parameter.to_owned(),
    };
    for (index, parameter) in clause.parameters().iter().enumerate() {
        let slot = clause
            .aliases()
            .iter()
            .position(|alias| *alias == parameter.alias)
            .ok_or_else(|| SmtEncodingRefusal::fault("a parameter names an alias of the clause"))?;
        let Some(subject) = subjects.get(slot) else {
            return Err(binding(&parameter.name).into());
        };
        let Some((name, key)) = instance.get(index) else {
            return Err(binding(&parameter.name).into());
        };
        if *name != parameter.name {
            return Err(binding(&parameter.name).into());
        }
        let listed = match ModelSystem::new(subject.simulation.clone()) {
            Ok(system) => system
                .instances(&ValueType::Reference(parameter.population.object()))
                .map_err(|_| binding(&parameter.name))?
                .into_iter()
                .map(|instance| instance.key)
                .collect::<BTreeSet<_>>(),
            Err(_) => subject
                .simulation
                .universes
                .iter()
                .flat_map(|universe| universe.keys.iter().cloned())
                .collect(),
        };
        if !listed.contains(key) {
            return Err(binding(&parameter.name).into());
        }
    }
    if instance.len() > clause.parameters().len() {
        return Err(binding(&instance[clause.parameters().len()].0).into());
    }
    Ok(())
}

/// Encode the query `kind` of the HP-2 product and instance `request` names
/// (FR-315 "Hyper self-composition queries").
///
/// The identity checks run first, in the order of
/// [`HyperProductIdentity::of`], which this call computes before it writes
/// any script and returns as [`HyperQuery::product`]. The encoding's own
/// stages follow; `request.meter` is charged by `μ_U` only.
///
/// # Errors
///
/// [`SmtEncodingRefusal`], with the code of FR-315's table, naming the
/// construct and its locus; no script is written.
pub fn encode_hyper_query(
    request: HyperQueryRequest<'_>,
    kind: SmtQueryKind,
) -> Result<HyperQuery, SmtEncodingRefusal> {
    let product = HyperProductIdentity::of(&request)?;
    let HyperQueryRequest {
        clause,
        subjects,
        instance,
        meter,
        encoding,
    } = request;
    let script = encode_hyper_script(HyperScriptRequest {
        clause,
        subjects,
        instance,
        meter,
        encoding,
        kind,
    })?;
    Ok(HyperQuery {
        product,
        kind,
        script,
    })
}
