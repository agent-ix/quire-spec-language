// SPDX-License-Identifier: AGPL-3.0-or-later
//! Charge the canonical writer and its separate output before either grows.

use std::cell::RefCell;

use quire_canonical::{Encode, Error, Limits, Reservation, ReservationDenied, Sink};

use crate::monitor_root::progress::charge::Charge;
use crate::monitor_root::PlanAuthorityRefusal;

use super::{ModelStepOrder, ModelStepOrderRefusal, ModelStepOrderRefusalCode};

struct Accounting<'call, 'budget> {
    charge: RefCell<&'call mut Charge<'budget>>,
    refusal: RefCell<Option<PlanAuthorityRefusal>>,
}

impl Accounting<'_, '_> {
    fn reserve(&self, bytes: usize) -> Result<(), ReservationDenied> {
        let mut charge = self.charge.borrow_mut();
        let result = charge.work(1).and_then(|()| charge.storage(bytes as u128));
        match result {
            Ok(()) => Ok(()),
            Err(error) => {
                // The writer stops at its first denied growth. Retain the
                // original fixed-size native denial, not an encoder surrogate.
                if self.refusal.borrow().is_none() {
                    *self.refusal.borrow_mut() = Some(error);
                }
                Err(ReservationDenied)
            }
        }
    }
}

struct WriterBudget<'account, 'call, 'budget>(&'account Accounting<'call, 'budget>);

impl Reservation for WriterBudget<'_, '_, '_> {
    fn reserve(&mut self, bytes: usize) -> Result<(), ReservationDenied> {
        self.0.reserve(bytes)
    }
}

struct Output<'account, 'call, 'budget> {
    accounting: &'account Accounting<'call, 'budget>,
    bytes: Vec<u8>,
}

impl Sink for Output<'_, '_, '_> {
    fn write_bytes(&mut self, bytes: &[u8]) -> Result<(), Error> {
        // A metered writer retains its complete output and calls the sink
        // only after finishing it. The sink is outside the writer's own
        // reservation contract, so its owned copy needs separate permission.
        self.accounting
            .reserve(bytes.len())
            .map_err(|ReservationDenied| Error::ReservationDenied {
                requested: bytes.len(),
            })?;
        {
            let mut charge = self.accounting.charge.borrow_mut();
            if let Err(error) = charge.work(bytes.len() as u128) {
                *self.accounting.refusal.borrow_mut() = Some(error);
                return Err(Error::ReservationDenied {
                    requested: bytes.len(),
                });
            }
        }
        self.bytes
            .try_reserve_exact(bytes.len())
            .map_err(|_| Error::Allocation {
                requested: bytes.len(),
            })?;
        self.bytes.extend_from_slice(bytes);
        Ok(())
    }
}

pub(super) fn encode(
    value: &impl Encode,
    limits: Limits,
    charge: &mut Charge<'_>,
) -> Result<ModelStepOrder, PlanAuthorityRefusal> {
    // The output container itself is retained by ModelStepOrder. The two
    // callbacks borrow this one ledger and run sequentially, never owning or
    // resetting an allowance; RefCell holds only their shared mutable borrow.
    charge.storage(1)?;
    let accounting = Accounting {
        charge: RefCell::new(charge),
        refusal: RefCell::new(None),
    };
    let mut reservation = WriterBudget(&accounting);
    let mut output = Output {
        accounting: &accounting,
        bytes: Vec::new(),
    };
    let result =
        quire_canonical::encode_with_reservation(&mut output, value, limits, &mut reservation);
    if let Some(error) = accounting.refusal.borrow_mut().take() {
        return Err(error);
    }
    result.map_err(|encoding| ModelStepOrderRefusal {
        code: ModelStepOrderRefusalCode::Encoding,
        encoding: Some(encoding),
    })?;
    Ok(ModelStepOrder {
        bytes: output.bytes,
    })
}

#[cfg(test)]
mod tests {
    use ix_trace_rs::trace;
    use quire_contract_model::{
        NativeJoinBudget, NativeJoinChargeKind, NativeJoinLocus, NativeJoinLocusSource,
    };

    use super::*;

    struct Value;

    impl Encode for Value {
        fn encode_into<S: Sink + ?Sized>(
            &self,
            writer: &mut quire_canonical::Writer<'_, S>,
        ) -> Result<(), Error> {
            writer.begin_object()?;
            writer.name("source")?;
            writer.string("dense")?;
            writer.name("kind")?;
            writer.string("model-step")?;
            writer.end_object()
        }
    }

    fn locus() -> NativeJoinLocus {
        NativeJoinLocus {
            source: NativeJoinLocusSource::ReachedRow(37),
            form: "qsl_authority_selection",
        }
    }

    fn materialize(budget: &mut NativeJoinBudget) -> Result<ModelStepOrder, PlanAuthorityRefusal> {
        budget.resume();
        encode(
            &Value,
            Limits::new(1 << 16),
            &mut Charge {
                budget,
                locus: locus(),
            },
        )
    }

    #[test]
    #[trace("TC-1081", "FR-251-AC-49")]
    fn writer_reservations_and_output_copy_share_one_original_ledger() {
        let mut budget = NativeJoinBudget::new(1 << 20, 1 << 20);
        budget.resume();
        budget
            .accounted_charge(NativeJoinChargeKind::Work, 7, locus())
            .unwrap();
        budget
            .accounted_charge(NativeJoinChargeKind::Storage, 11, locus())
            .unwrap();
        let result = materialize(&mut budget).unwrap();
        assert_eq!(result.bytes(), br#"{"kind":"model-step","source":"dense"}"#);
        let spent = budget.consumed();
        assert!(spent.work > 7 + result.bytes().len() as u64);
        assert!(spent.storage > 11 + result.bytes().len() as u64);
        let mut fresh = NativeJoinBudget::new(1 << 20, 1 << 20);
        assert_eq!(materialize(&mut fresh).unwrap().bytes(), result.bytes());
        assert_eq!(spent.work, fresh.consumed().work + 7);
        assert_eq!(spent.storage, fresh.consumed().storage + 11);
    }

    #[test]
    #[trace("TC-1081", "FR-251-AC-49")]
    fn exact_writer_allowances_succeed_and_one_short_retains_the_native_denial() {
        let mut ample = NativeJoinBudget::new(1 << 20, 1 << 20);
        let expected = materialize(&mut ample).unwrap();
        let total = ample.consumed();
        let mut exact = NativeJoinBudget::new(total.work, total.storage);
        assert_eq!(materialize(&mut exact).unwrap().bytes(), expected.bytes());
        assert_eq!(exact.consumed(), total);
        for kind in [NativeJoinChargeKind::Work, NativeJoinChargeKind::Storage] {
            let mut short = NativeJoinBudget::new(
                total.work - u64::from(kind == NativeJoinChargeKind::Work),
                total.storage - u64::from(kind == NativeJoinChargeKind::Storage),
            );
            let error = materialize(&mut short).unwrap_err();
            let PlanAuthorityRefusal::Budget {
                denied,
                locus: unpaid_locus,
                configured_bound,
                consumed,
            } = error
            else {
                panic!("writer returned an encoder surrogate instead of a native denial");
            };
            assert_eq!(denied.kind, kind);
            assert_eq!(Some(denied), short.first_unpaid());
            assert_eq!(unpaid_locus, Some(locus()));
            assert_eq!(consumed, short.consumed());
            assert_eq!(
                configured_bound,
                match kind {
                    NativeJoinChargeKind::Work => short.work_limit(),
                    NativeJoinChargeKind::Storage => short.storage_limit(),
                }
            );
        }
        let mut zero = NativeJoinBudget::new(1 << 20, 0);
        assert!(matches!(
            materialize(&mut zero),
            Err(PlanAuthorityRefusal::Budget { .. })
        ));
        assert_eq!(zero.consumed().storage, 0);
        assert_eq!(zero.consumed().work, 0);
    }
}
