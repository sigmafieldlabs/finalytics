//! Tax lots and lot relief.

use finalytics_core::{Date, Decimal};
use serde::{Deserialize, Serialize};

use crate::{AccountId, InstrumentId, LotId, LotMethod, LotPick, TransactionId};

/// Units of one instrument acquired together, with their remaining cost basis.
///
/// Costs are totals for the remaining quantity, in the instrument's currency (`cost_local`) and in
/// the portfolio's base currency at the acquisition rate (`cost_base`). Keeping totals rather than
/// unit costs means that splitting a lot never loses or creates cost.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Lot {
    /// The lot's identifier.
    pub id: LotId,
    /// The account holding it.
    pub account: AccountId,
    /// The instrument.
    pub instrument: InstrumentId,
    /// The remaining quantity, greater than zero while the lot is open.
    pub quantity: Decimal,
    /// The remaining cost basis in the instrument's currency, including acquisition fees.
    pub cost_local: Decimal,
    /// The remaining cost basis in the base currency.
    pub cost_base: Decimal,
    /// The acquisition date, which sets the holding period.
    pub acquired: Date,
    /// The transaction that opened the lot (or the earliest one, for lots carried through
    /// corporate actions).
    pub opened_by: TransactionId,
    /// The order in which lots were opened, used to break ties.
    pub sequence: u64,
}

impl Lot {
    /// The cost per unit in the instrument's currency.
    pub fn unit_cost_local(&self) -> Decimal {
        if self.quantity.is_zero() {
            Decimal::ZERO
        } else {
            self.cost_local / self.quantity
        }
    }
}

/// The decimal places kept for amounts and quantities derived by division or multiplication
/// (a share of a lot's cost, a split quantity, an allocated proceed).
///
/// Rounding derived values to a fixed scale keeps every later addition and subtraction exact, so
/// the parts of a lot always add up to the whole.
pub(crate) const DERIVED_DP: u32 = 10;

/// Rounds a derived amount or quantity to [`DERIVED_DP`] decimal places.
pub(crate) fn derived(value: Decimal) -> Decimal {
    value.round_dp(DERIVED_DP)
}

/// The part of a lot taken by a sale or transfer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Relief {
    pub(crate) lot: Lot,
    pub(crate) quantity: Decimal,
    pub(crate) cost_local: Decimal,
    pub(crate) cost_base: Decimal,
}

/// Why lots could not be relieved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ReliefError {
    Insufficient { held: Decimal },
    UnknownLot(LotId),
    LotTooSmall { lot: LotId, held: Decimal },
}

/// Takes `quantity` units from `lots` (all of one instrument in one account), returning what was
/// taken from each lot. `lots` is updated in place; emptied lots are removed.
pub(crate) fn relieve(
    lots: &mut Vec<Lot>,
    quantity: Decimal,
    method: LotMethod,
    picks: &[LotPick],
) -> Result<Vec<Relief>, ReliefError> {
    let held: Decimal = lots.iter().map(|l| l.quantity).sum();
    if quantity > held {
        return Err(ReliefError::Insufficient { held });
    }
    let mut taken = Vec::new();
    match method {
        LotMethod::AverageCost => {
            if quantity == held {
                for lot in lots.iter_mut() {
                    let all = lot.quantity;
                    taken.push(take(lot, all));
                }
            } else {
                let mut remaining = quantity;
                for lot in lots.iter_mut() {
                    let share = derived(lot.quantity * quantity / held)
                        .min(remaining)
                        .min(lot.quantity);
                    if !share.is_zero() {
                        remaining -= share;
                        taken.push(take(lot, share));
                    }
                }
                // Decimal division can leave a sliver; take it from the first lots that still
                // hold units, so the total is exact.
                for lot in lots.iter_mut() {
                    if remaining.is_zero() {
                        break;
                    }
                    let share = remaining.min(lot.quantity);
                    if !share.is_zero() {
                        remaining -= share;
                        taken.push(take(lot, share));
                    }
                }
            }
        }
        LotMethod::SpecificId => {
            for pick in picks {
                let lot = lots
                    .iter_mut()
                    .find(|l| l.id == pick.lot)
                    .ok_or_else(|| ReliefError::UnknownLot(pick.lot.clone()))?;
                if pick.quantity > lot.quantity {
                    return Err(ReliefError::LotTooSmall {
                        lot: lot.id.clone(),
                        held: lot.quantity,
                    });
                }
                taken.push(take(lot, pick.quantity));
            }
        }
        _ => {
            let mut order: Vec<usize> = (0..lots.len()).collect();
            match method {
                LotMethod::Fifo => order.sort_by_key(|&i| (lots[i].acquired, lots[i].sequence)),
                LotMethod::Lifo => {
                    order.sort_by_key(|&i| std::cmp::Reverse((lots[i].acquired, lots[i].sequence)))
                }
                LotMethod::HighestCost => order.sort_by(|&a, &b| {
                    lots[b]
                        .unit_cost_local()
                        .cmp(&lots[a].unit_cost_local())
                        .then(
                            (lots[a].acquired, lots[a].sequence)
                                .cmp(&(lots[b].acquired, lots[b].sequence)),
                        )
                }),
                LotMethod::LowestCost => order.sort_by(|&a, &b| {
                    lots[a]
                        .unit_cost_local()
                        .cmp(&lots[b].unit_cost_local())
                        .then(
                            (lots[a].acquired, lots[a].sequence)
                                .cmp(&(lots[b].acquired, lots[b].sequence)),
                        )
                }),
                LotMethod::AverageCost | LotMethod::SpecificId => unreachable!("handled above"),
            }
            let mut remaining = quantity;
            for i in order {
                if remaining.is_zero() {
                    break;
                }
                let share = remaining.min(lots[i].quantity);
                remaining -= share;
                taken.push(take(&mut lots[i], share));
            }
        }
    }
    lots.retain(|l| !l.quantity.is_zero());
    Ok(taken)
}

/// Removes `quantity` units from `lot`, with the matching share of its costs. The costs removed
/// and left always add up to the costs before.
fn take(lot: &mut Lot, quantity: Decimal) -> Relief {
    let (cost_local, cost_base) = if quantity == lot.quantity {
        (lot.cost_local, lot.cost_base)
    } else {
        (
            derived(lot.cost_local * quantity / lot.quantity),
            derived(lot.cost_base * quantity / lot.quantity),
        )
    };
    let before = lot.clone();
    lot.quantity -= quantity;
    lot.cost_local -= cost_local;
    lot.cost_base -= cost_base;
    Relief {
        lot: before,
        quantity,
        cost_local,
        cost_base,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    fn lot(id: &str, acquired: &str, quantity: i64, cost: i64, sequence: u64) -> Lot {
        Lot {
            id: LotId::new(id).unwrap(),
            account: AccountId::new("a").unwrap(),
            instrument: InstrumentId::new("X").unwrap(),
            quantity: Decimal::from(quantity),
            cost_local: Decimal::from(cost),
            cost_base: Decimal::from(cost * 2),
            acquired: acquired.parse().unwrap(),
            opened_by: TransactionId::new(id).unwrap(),
            sequence,
        }
    }

    fn book() -> Vec<Lot> {
        vec![
            lot("t1", "2024-01-10", 10, 1000, 0), // 100 per unit
            lot("t2", "2024-06-10", 10, 1500, 1), // 150
            lot("t3", "2025-01-10", 10, 1200, 2), // 120
        ]
    }

    fn ids(relief: &[Relief]) -> Vec<(&str, i64)> {
        relief
            .iter()
            .map(|r| (r.lot.id.as_str(), r.quantity.try_into().unwrap()))
            .collect()
    }

    #[test]
    fn each_method_picks_its_lots() {
        let cases = [
            (LotMethod::Fifo, vec![("t1", 10), ("t2", 5)]),
            (LotMethod::Lifo, vec![("t3", 10), ("t2", 5)]),
            (LotMethod::HighestCost, vec![("t2", 10), ("t3", 5)]),
            (LotMethod::LowestCost, vec![("t1", 10), ("t3", 5)]),
        ];
        for (method, expected) in cases {
            let mut lots = book();
            let relief = relieve(&mut lots, Decimal::from(15), method, &[]).unwrap();
            assert_eq!(ids(&relief), expected, "{method:?}");
            assert_eq!(lots.len(), 2);
        }
    }

    #[test]
    fn average_cost_reduces_every_lot_in_proportion() {
        let mut lots = book();
        let relief = relieve(&mut lots, Decimal::from(15), LotMethod::AverageCost, &[]).unwrap();
        let cost: Decimal = relief.iter().map(|r| r.cost_local).sum();
        assert_eq!(cost, Decimal::from(1850)); // 15 units at the average of 3700 / 30
        assert!(lots.iter().all(|l| l.quantity == Decimal::from(5)));
    }

    #[test]
    fn specific_identification_and_its_errors() {
        let mut lots = book();
        let picks = [
            LotPick {
                lot: LotId::new("t3").unwrap(),
                quantity: Decimal::from(4),
            },
            LotPick {
                lot: LotId::new("t1").unwrap(),
                quantity: Decimal::from(1),
            },
        ];
        let relief = relieve(&mut lots, Decimal::from(5), LotMethod::SpecificId, &picks).unwrap();
        assert_eq!(ids(&relief), [("t3", 4), ("t1", 1)]);
        let unknown = [LotPick {
            lot: LotId::new("t9").unwrap(),
            quantity: Decimal::ONE,
        }];
        assert_eq!(
            relieve(&mut book(), Decimal::ONE, LotMethod::SpecificId, &unknown),
            Err(ReliefError::UnknownLot(LotId::new("t9").unwrap()))
        );
        let too_many = [LotPick {
            lot: LotId::new("t1").unwrap(),
            quantity: Decimal::from(11),
        }];
        assert!(matches!(
            relieve(
                &mut book(),
                Decimal::from(11),
                LotMethod::SpecificId,
                &too_many
            ),
            Err(ReliefError::LotTooSmall { .. })
        ));
    }

    #[test]
    fn cannot_take_more_than_is_held() {
        assert_eq!(
            relieve(&mut book(), Decimal::from(31), LotMethod::Fifo, &[]),
            Err(ReliefError::Insufficient {
                held: Decimal::from(30)
            })
        );
    }

    proptest! {
        #[test]
        fn quantity_and_cost_are_conserved(
            sizes in proptest::collection::vec((1i64..1000, 1i64..100_000), 1..8),
            take_pct in 1u32..=100,
            method in prop_oneof![
                Just(LotMethod::Fifo), Just(LotMethod::Lifo), Just(LotMethod::HighestCost),
                Just(LotMethod::LowestCost), Just(LotMethod::AverageCost)
            ],
        ) {
            let mut lots: Vec<Lot> = sizes.iter().enumerate()
                .map(|(i, &(q, c))| lot(&format!("t{i}"), "2025-01-01", q, c, i as u64))
                .collect();
            let held: Decimal = lots.iter().map(|l| l.quantity).sum();
            let cost: Decimal = lots.iter().map(|l| l.cost_local).sum();
            let quantity = (held * Decimal::from(take_pct) / Decimal::from(100)).round_dp(6).max(Decimal::new(1, 6)).min(held);
            let relief = relieve(&mut lots, quantity, method, &[]).unwrap();
            let taken_qty: Decimal = relief.iter().map(|r| r.quantity).sum();
            let taken_cost: Decimal = relief.iter().map(|r| r.cost_local).sum();
            let left_qty: Decimal = lots.iter().map(|l| l.quantity).sum();
            let left_cost: Decimal = lots.iter().map(|l| l.cost_local).sum();
            prop_assert_eq!(taken_qty, quantity);
            prop_assert_eq!(taken_qty + left_qty, held);
            prop_assert_eq!(taken_cost + left_cost, cost);
            prop_assert!(lots.iter().all(|l| l.quantity > Decimal::ZERO));
        }
    }
}
