use serde::{Deserialize, Serialize};

use crate::double_ma::{Trade, TradeStats};

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
pub struct TradeRecord {
    pub col: u32,
    pub entry_t: u32,
    pub exit_t: u32,
    pub direction: i32,
    pub pnl: f64,
    pub bars: u32,
    pub open: bool,
}

impl TradeRecord {
    pub fn try_from_trade(col: u32, trade: &Trade) -> Result<Self, String> {
        let entry_t: u32 = trade
            .entry_t
            .try_into()
            .map_err(|_| "trade.entry_t overflow".to_string())?;
        let exit_t: u32 = trade
            .exit_t
            .try_into()
            .map_err(|_| "trade.exit_t overflow".to_string())?;
        Ok(Self {
            col,
            entry_t,
            exit_t,
            direction: trade.direction,
            pnl: trade.pnl,
            bars: trade.bars,
            open: trade.open,
        })
    }

    pub fn to_trade(self) -> Trade {
        Trade {
            entry_t: self.entry_t as usize,
            exit_t: self.exit_t as usize,
            direction: self.direction,
            pnl: self.pnl,
            bars: self.bars,
            open: self.open,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct TradeRecords {
    pub col: Vec<u32>,
    pub entry_t: Vec<u32>,
    pub exit_t: Vec<u32>,
    pub direction: Vec<i32>,
    pub pnl: Vec<f64>,
    pub bars: Vec<u32>,
    pub open: Vec<bool>,
}

impl TradeRecords {
    #[inline]
    fn debug_assert_consistent_lengths(&self) {
        debug_assert_eq!(
            self.entry_t.len(),
            self.col.len(),
            "entry_t length mismatch"
        );
        debug_assert_eq!(self.exit_t.len(), self.col.len(), "exit_t length mismatch");
        debug_assert_eq!(
            self.direction.len(),
            self.col.len(),
            "direction length mismatch"
        );
        debug_assert_eq!(self.pnl.len(), self.col.len(), "pnl length mismatch");
        debug_assert_eq!(self.bars.len(), self.col.len(), "bars length mismatch");
        debug_assert_eq!(self.open.len(), self.col.len(), "open length mismatch");
    }

    pub fn try_from_trades(col: u32, trades: &[Trade]) -> Result<Self, String> {
        let mut out = Self::default();
        out.try_extend_trades(col, trades)?;
        Ok(out)
    }

    pub fn len(&self) -> usize {
        self.col.len()
    }

    pub fn is_empty(&self) -> bool {
        self.col.is_empty()
    }

    pub fn validate(&self) -> Result<(), String> {
        let n = self.col.len();
        let ok = self.entry_t.len() == n
            && self.exit_t.len() == n
            && self.direction.len() == n
            && self.pnl.len() == n
            && self.bars.len() == n
            && self.open.len() == n;
        if ok {
            Ok(())
        } else {
            Err("TradeRecords length mismatch".to_string())
        }
    }

    pub fn get(&self, i: usize) -> Option<TradeRecord> {
        Some(TradeRecord {
            col: *self.col.get(i)?,
            entry_t: *self.entry_t.get(i)?,
            exit_t: *self.exit_t.get(i)?,
            direction: *self.direction.get(i)?,
            pnl: *self.pnl.get(i)?,
            bars: *self.bars.get(i)?,
            open: *self.open.get(i)?,
        })
    }

    pub fn push_record(&mut self, record: TradeRecord) {
        self.debug_assert_consistent_lengths();
        self.col.push(record.col);
        self.entry_t.push(record.entry_t);
        self.exit_t.push(record.exit_t);
        self.direction.push(record.direction);
        self.pnl.push(record.pnl);
        self.bars.push(record.bars);
        self.open.push(record.open);
        self.debug_assert_consistent_lengths();
    }

    pub fn try_push_trade(&mut self, col: u32, trade: &Trade) -> Result<(), String> {
        self.validate()?;
        let rec = TradeRecord::try_from_trade(col, trade)?;
        self.push_record(rec);
        Ok(())
    }

    pub fn try_extend_trades(&mut self, col: u32, trades: &[Trade]) -> Result<(), String> {
        self.validate()?;
        let records = trades
            .iter()
            .map(|trade| TradeRecord::try_from_trade(col, trade))
            .collect::<Result<Vec<_>, _>>()?;

        self.col.reserve(records.len());
        self.entry_t.reserve(records.len());
        self.exit_t.reserve(records.len());
        self.direction.reserve(records.len());
        self.pnl.reserve(records.len());
        self.bars.reserve(records.len());
        self.open.reserve(records.len());
        for record in records {
            self.push_record(record);
        }
        self.debug_assert_consistent_lengths();
        Ok(())
    }

    pub fn to_trades_for_col(&self, col: u32) -> Vec<Trade> {
        let mut out: Vec<Trade> = Vec::new();
        for i in 0..self.col.len() {
            if let Some(record) = self.get(i) {
                if record.col == col {
                    out.push(record.to_trade());
                }
            }
        }
        out
    }

    pub fn trade_stats_for_col(&self, col: u32) -> Option<TradeStats> {
        let mut open_trades: usize = 0;
        let mut closed_trades: usize = 0;
        let mut wins: usize = 0;
        let mut losses: usize = 0;
        let mut sum_win: f64 = 0.0;
        let mut sum_loss_abs: f64 = 0.0;
        let mut sum_win_count: usize = 0;
        let mut sum_loss_count: usize = 0;
        let mut sum_all: f64 = 0.0;

        for i in 0..self.col.len() {
            let Some(record) = self.get(i) else {
                continue;
            };
            if record.col != col {
                continue;
            }
            if record.open {
                open_trades += 1;
                continue;
            }
            closed_trades += 1;
            let r = record.pnl;
            sum_all += r;
            if r > 0.0 {
                wins += 1;
                sum_win += r;
                sum_win_count += 1;
            } else if r < 0.0 {
                losses += 1;
                sum_loss_abs += -r;
                sum_loss_count += 1;
            }
        }

        if closed_trades == 0 && open_trades == 0 {
            return None;
        }

        let denom = wins + losses;
        let win_rate = if denom > 0 {
            wins as f64 / denom as f64
        } else {
            0.0
        };
        let avg_win = if sum_win_count > 0 {
            sum_win / sum_win_count as f64
        } else {
            0.0
        };
        let avg_loss = if sum_loss_count > 0 {
            -(sum_loss_abs / sum_loss_count as f64)
        } else {
            0.0
        };
        let profit_factor = if sum_loss_abs > 0.0 {
            sum_win / sum_loss_abs
        } else if sum_win > 0.0 {
            f64::INFINITY
        } else {
            0.0
        };
        let expectancy = if closed_trades > 0 {
            sum_all / closed_trades as f64
        } else {
            0.0
        };

        Some(TradeStats {
            closed_trades,
            open_trades,
            wins,
            losses,
            win_rate,
            avg_win,
            avg_loss,
            profit_factor,
            expectancy,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trade_records_validate_and_stats_work() {
        let trades = vec![
            Trade {
                entry_t: 0,
                exit_t: 5,
                direction: 1,
                pnl: 0.10,
                bars: 5,
                open: false,
            },
            Trade {
                entry_t: 10,
                exit_t: 12,
                direction: 1,
                pnl: -0.05,
                bars: 2,
                open: false,
            },
            Trade {
                entry_t: 20,
                exit_t: 20,
                direction: 1,
                pnl: 0.0,
                bars: 0,
                open: true,
            },
        ];

        let mut recs = TradeRecords::default();
        recs.try_extend_trades(7, &trades).unwrap();
        recs.validate().unwrap();
        assert_eq!(recs.len(), 3);

        let stats = recs.trade_stats_for_col(7).unwrap();
        assert_eq!(stats.closed_trades, 2);
        assert_eq!(stats.open_trades, 1);
        assert_eq!(stats.wins, 1);
        assert_eq!(stats.losses, 1);
        assert!((stats.win_rate - 0.5).abs() < 1e-9);

        let roundtrip = recs.to_trades_for_col(7);
        assert_eq!(roundtrip.len(), 3);
        assert_eq!(roundtrip[0].entry_t, 0);
        assert_eq!(roundtrip[1].pnl, -0.05);
    }

    #[test]
    fn trade_records_validate_detects_mismatched_lengths() {
        let mut recs = TradeRecords::default();
        recs.col.push(1);
        recs.entry_t.push(0);

        assert!(recs.validate().is_err());
    }

    #[test]
    fn trade_records_stats_and_roundtrip_are_scoped_per_col() {
        let mut recs = TradeRecords::default();
        recs.try_push_trade(
            7,
            &Trade {
                entry_t: 0,
                exit_t: 4,
                direction: 1,
                pnl: 0.20,
                bars: 4,
                open: false,
            },
        )
        .unwrap();
        recs.try_push_trade(
            8,
            &Trade {
                entry_t: 1,
                exit_t: 5,
                direction: -1,
                pnl: -0.10,
                bars: 4,
                open: false,
            },
        )
        .unwrap();

        let col7 = recs.to_trades_for_col(7);
        let col8 = recs.to_trades_for_col(8);
        assert_eq!(col7.len(), 1);
        assert_eq!(col8.len(), 1);
        assert_eq!(col7[0].pnl, 0.20);
        assert_eq!(col8[0].pnl, -0.10);

        let stats7 = recs.trade_stats_for_col(7).expect("col 7 stats");
        let stats8 = recs.trade_stats_for_col(8).expect("col 8 stats");
        assert_eq!(stats7.closed_trades, 1);
        assert_eq!(stats7.wins, 1);
        assert_eq!(stats8.closed_trades, 1);
        assert_eq!(stats8.losses, 1);
    }

    #[test]
    fn trade_record_try_from_trade_rejects_timestamp_overflow() {
        let trade = Trade {
            entry_t: (u32::MAX as usize) + 1,
            exit_t: 7,
            direction: 1,
            pnl: 0.0,
            bars: 0,
            open: false,
        };

        let err = TradeRecord::try_from_trade(0, &trade).unwrap_err();
        assert!(err.contains("overflow"));
    }

    #[test]
    fn trade_records_stats_treat_flat_closed_trades_as_neutral() {
        let mut recs = TradeRecords::default();
        recs.try_push_trade(
            3,
            &Trade {
                entry_t: 0,
                exit_t: 2,
                direction: 1,
                pnl: 0.0,
                bars: 2,
                open: false,
            },
        )
        .unwrap();

        let stats = recs.trade_stats_for_col(3).expect("col 3 stats");
        assert_eq!(stats.closed_trades, 1);
        assert_eq!(stats.open_trades, 0);
        assert_eq!(stats.wins, 0);
        assert_eq!(stats.losses, 0);
        assert_eq!(stats.win_rate, 0.0);
        assert_eq!(stats.avg_win, 0.0);
        assert_eq!(stats.avg_loss, 0.0);
        assert_eq!(stats.profit_factor, 0.0);
        assert_eq!(stats.expectancy, 0.0);
    }

    #[test]
    fn trade_records_stats_report_infinite_profit_factor_when_all_closed_trades_win() {
        let mut recs = TradeRecords::default();
        for pnl in [0.15, 0.05] {
            recs.try_push_trade(
                9,
                &Trade {
                    entry_t: 0,
                    exit_t: 1,
                    direction: 1,
                    pnl,
                    bars: 1,
                    open: false,
                },
            )
            .unwrap();
        }

        let stats = recs.trade_stats_for_col(9).expect("col 9 stats");
        assert_eq!(stats.closed_trades, 2);
        assert_eq!(stats.wins, 2);
        assert_eq!(stats.losses, 0);
        assert!(stats.profit_factor.is_infinite());
        assert!(stats.expectancy > 0.0);
    }

    #[test]
    fn trade_records_get_returns_none_when_col_outpaces_sibling_columns() {
        let mut recs = TradeRecords::default();
        recs.col.push(42);

        assert_eq!(recs.get(0), None);
        assert!(recs.to_trades_for_col(42).is_empty());
        assert!(recs.trade_stats_for_col(42).is_none());
    }

    #[test]
    fn trade_records_extend_rejects_invalid_existing_state_without_appending() {
        let mut recs = TradeRecords::default();
        recs.col.push(42);

        let err = recs
            .try_push_trade(
                42,
                &Trade {
                    entry_t: 0,
                    exit_t: 1,
                    direction: 1,
                    pnl: 0.1,
                    bars: 1,
                    open: false,
                },
            )
            .expect_err("invalid existing state should be rejected");

        assert!(err.contains("length mismatch"), "{err}");
        assert_eq!(recs.col.len(), 1);
        assert!(recs.entry_t.is_empty());
    }
}
