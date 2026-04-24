use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

const PAPER_BROKER_VERSION: u32 = 1;
const DEFAULT_STARTING_CASH: f64 = 100_000.0;

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PaperOrderSide {
    Buy,
    Sell,
}

impl PaperOrderSide {
    pub fn as_str(self) -> &'static str {
        match self {
            PaperOrderSide::Buy => "buy",
            PaperOrderSide::Sell => "sell",
        }
    }

    fn signed_qty(self, qty: f64) -> f64 {
        match self {
            PaperOrderSide::Buy => qty,
            PaperOrderSide::Sell => -qty,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PaperBrokerAccount {
    pub account_id: String,
    pub currency: String,
    pub starting_cash: f64,
    pub cash: f64,
    pub buying_power: f64,
    pub portfolio_value: f64,
    pub realized_pnl: f64,
    pub updated_at_unix_ms: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PaperBrokerPosition {
    pub symbol: String,
    pub qty: f64,
    pub avg_entry_price: f64,
    pub market_price: f64,
    pub market_value: f64,
    pub unrealized_pnl: f64,
    pub last_fill_unix_ms: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PaperBrokerOrder {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub client_order_id: Option<String>,
    pub symbol: String,
    pub side: PaperOrderSide,
    pub qty: f64,
    pub order_type: String,
    pub time_in_force: String,
    pub status: String,
    pub filled_qty: f64,
    pub avg_fill_price: f64,
    pub submitted_at_unix_ms: i64,
    pub filled_at_unix_ms: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PaperBrokerFill {
    pub id: String,
    pub order_id: String,
    pub symbol: String,
    pub side: PaperOrderSide,
    pub qty: f64,
    pub price: f64,
    pub fee: f64,
    pub slippage: f64,
    pub at_unix_ms: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PaperBrokerStateDoc {
    pub version: u32,
    pub account: PaperBrokerAccount,
    #[serde(default)]
    pub positions: BTreeMap<String, PaperBrokerPosition>,
    #[serde(default)]
    pub orders: Vec<PaperBrokerOrder>,
    #[serde(default)]
    pub fills: Vec<PaperBrokerFill>,
    pub next_order_seq: u64,
    pub next_fill_seq: u64,
}

#[derive(Clone, Debug)]
pub struct PaperBrokerMarketOrder {
    pub symbol: String,
    pub side: PaperOrderSide,
    pub qty: f64,
    pub price: f64,
    pub at_unix_ms: i64,
    pub fee_bps: f64,
    pub slippage_bps: f64,
    pub client_order_id: Option<String>,
}

#[derive(Clone, Debug)]
pub struct PaperBrokerExecution {
    pub order: PaperBrokerOrder,
    pub fill: PaperBrokerFill,
    pub account: PaperBrokerAccount,
    pub position: Option<PaperBrokerPosition>,
}

pub fn paper_broker_state_path(app_local_data_dir: &Path) -> PathBuf {
    app_local_data_dir
        .join("execution")
        .join("paper_broker")
        .join("state.json")
}

pub fn load_or_default(
    app_local_data_dir: &Path,
    now_unix_ms: i64,
) -> Result<PaperBrokerStateDoc, String> {
    let path = paper_broker_state_path(app_local_data_dir);
    if !path.exists() {
        return Ok(PaperBrokerStateDoc::new(now_unix_ms));
    }
    let bytes = fs::read(&path).map_err(|e| format!("failed to read paper broker state: {e}"))?;
    let mut doc: PaperBrokerStateDoc = serde_json::from_slice(&bytes)
        .map_err(|e| format!("failed to decode paper broker state: {e}"))?;
    if doc.version != PAPER_BROKER_VERSION {
        return Err(format!(
            "unsupported paper broker state version {}",
            doc.version
        ));
    }
    doc.recompute_account(now_unix_ms);
    Ok(doc)
}

pub fn save(app_local_data_dir: &Path, doc: &PaperBrokerStateDoc) -> Result<(), String> {
    let path = paper_broker_state_path(app_local_data_dir);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("failed to create paper broker state directory: {e}"))?;
    }
    let bytes = serde_json::to_vec_pretty(doc)
        .map_err(|e| format!("failed to encode paper broker state: {e}"))?;
    fs::write(&path, bytes).map_err(|e| format!("failed to write paper broker state: {e}"))
}

pub fn reset(app_local_data_dir: &Path, now_unix_ms: i64) -> Result<PaperBrokerStateDoc, String> {
    let doc = PaperBrokerStateDoc::new(now_unix_ms);
    save(app_local_data_dir, &doc)?;
    Ok(doc)
}

pub fn submit_market_order(
    app_local_data_dir: &Path,
    order: PaperBrokerMarketOrder,
) -> Result<PaperBrokerExecution, String> {
    validate_market_order(&order)?;
    let mut doc = load_or_default(app_local_data_dir, order.at_unix_ms)?;
    let order_id = format!("paper_ord_{:012}", doc.next_order_seq);
    let fill_id = format!("paper_fill_{:012}", doc.next_fill_seq);
    doc.next_order_seq = doc.next_order_seq.saturating_add(1);
    doc.next_fill_seq = doc.next_fill_seq.saturating_add(1);

    let slippage = order.price * (order.slippage_bps.max(0.0) / 10_000.0);
    let fill_price = match order.side {
        PaperOrderSide::Buy => order.price + slippage,
        PaperOrderSide::Sell => (order.price - slippage).max(0.01),
    };
    let fee = order.qty * fill_price * (order.fee_bps.max(0.0) / 10_000.0);
    let signed_qty = order.side.signed_qty(order.qty);
    doc.account.cash -= signed_qty * fill_price;
    doc.account.cash -= fee;

    let realized_delta = apply_position_fill(
        &mut doc.positions,
        &order.symbol,
        signed_qty,
        fill_price,
        order.at_unix_ms,
    );
    doc.account.realized_pnl += realized_delta - fee;

    let stored_order = PaperBrokerOrder {
        id: order_id.clone(),
        client_order_id: order.client_order_id,
        symbol: order.symbol.clone(),
        side: order.side,
        qty: order.qty,
        order_type: "market".to_string(),
        time_in_force: "day".to_string(),
        status: "filled".to_string(),
        filled_qty: order.qty,
        avg_fill_price: fill_price,
        submitted_at_unix_ms: order.at_unix_ms,
        filled_at_unix_ms: order.at_unix_ms,
    };
    let fill = PaperBrokerFill {
        id: fill_id,
        order_id,
        symbol: order.symbol.clone(),
        side: order.side,
        qty: order.qty,
        price: fill_price,
        fee,
        slippage,
        at_unix_ms: order.at_unix_ms,
    };
    doc.orders.push(stored_order.clone());
    doc.fills.push(fill.clone());
    doc.recompute_account(order.at_unix_ms);
    save(app_local_data_dir, &doc)?;

    Ok(PaperBrokerExecution {
        order: stored_order,
        fill,
        account: doc.account.clone(),
        position: doc.positions.get(&order.symbol).cloned(),
    })
}

impl PaperBrokerStateDoc {
    fn new(now_unix_ms: i64) -> Self {
        Self {
            version: PAPER_BROKER_VERSION,
            account: PaperBrokerAccount {
                account_id: "local-paper".to_string(),
                currency: "USD".to_string(),
                starting_cash: DEFAULT_STARTING_CASH,
                cash: DEFAULT_STARTING_CASH,
                buying_power: DEFAULT_STARTING_CASH,
                portfolio_value: DEFAULT_STARTING_CASH,
                realized_pnl: 0.0,
                updated_at_unix_ms: now_unix_ms,
            },
            positions: BTreeMap::new(),
            orders: Vec::new(),
            fills: Vec::new(),
            next_order_seq: 1,
            next_fill_seq: 1,
        }
    }

    pub fn recompute_account(&mut self, now_unix_ms: i64) {
        let position_value = self
            .positions
            .values_mut()
            .map(|position| {
                update_position_marks(position, position.market_price);
                position.market_value
            })
            .sum::<f64>();
        self.account.portfolio_value = self.account.cash + position_value;
        self.account.buying_power = self.account.cash.max(0.0);
        self.account.updated_at_unix_ms = now_unix_ms;
    }
}

fn validate_market_order(order: &PaperBrokerMarketOrder) -> Result<(), String> {
    if order.symbol.trim().is_empty() {
        return Err("symbol is required".to_string());
    }
    if !order.qty.is_finite() || order.qty <= 0.0 {
        return Err("qty must be finite and > 0".to_string());
    }
    if !order.price.is_finite() || order.price <= 0.0 {
        return Err("paper broker fill price must be finite and > 0".to_string());
    }
    Ok(())
}

fn apply_position_fill(
    positions: &mut BTreeMap<String, PaperBrokerPosition>,
    symbol: &str,
    signed_fill_qty: f64,
    fill_price: f64,
    at_unix_ms: i64,
) -> f64 {
    let prior = positions.get(symbol).cloned();
    let Some(prior_position) = prior else {
        let mut position = PaperBrokerPosition {
            symbol: symbol.to_string(),
            qty: signed_fill_qty,
            avg_entry_price: fill_price,
            market_price: fill_price,
            market_value: 0.0,
            unrealized_pnl: 0.0,
            last_fill_unix_ms: at_unix_ms,
        };
        update_position_marks(&mut position, fill_price);
        positions.insert(symbol.to_string(), position);
        return 0.0;
    };

    let prior_qty = prior_position.qty;
    let new_qty = prior_qty + signed_fill_qty;
    let mut realized = 0.0;
    let same_direction = prior_qty.signum() == signed_fill_qty.signum();
    let mut avg_entry_price = prior_position.avg_entry_price;

    if prior_qty.abs() <= f64::EPSILON || same_direction {
        let total_abs = prior_qty.abs() + signed_fill_qty.abs();
        if total_abs > f64::EPSILON {
            avg_entry_price = ((prior_position.avg_entry_price * prior_qty.abs())
                + (fill_price * signed_fill_qty.abs()))
                / total_abs;
        }
    } else {
        let closed_qty = prior_qty.abs().min(signed_fill_qty.abs());
        realized = if prior_qty > 0.0 {
            (fill_price - prior_position.avg_entry_price) * closed_qty
        } else {
            (prior_position.avg_entry_price - fill_price) * closed_qty
        };
        if new_qty.signum() != prior_qty.signum() && new_qty.abs() > f64::EPSILON {
            avg_entry_price = fill_price;
        }
    }

    if new_qty.abs() <= f64::EPSILON {
        positions.remove(symbol);
    } else {
        let mut position = PaperBrokerPosition {
            symbol: symbol.to_string(),
            qty: new_qty,
            avg_entry_price,
            market_price: fill_price,
            market_value: 0.0,
            unrealized_pnl: 0.0,
            last_fill_unix_ms: at_unix_ms,
        };
        update_position_marks(&mut position, fill_price);
        positions.insert(symbol.to_string(), position);
    }
    realized
}

fn update_position_marks(position: &mut PaperBrokerPosition, market_price: f64) {
    position.market_price = market_price;
    position.market_value = position.qty * market_price;
    position.unrealized_pnl = if position.qty >= 0.0 {
        (market_price - position.avg_entry_price) * position.qty.abs()
    } else {
        (position.avg_entry_price - market_price) * position.qty.abs()
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_app_local_data_dir(test_name: &str) -> PathBuf {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir()
            .join("vectorgrid_paper_broker_tests")
            .join(format!("{test_name}_{unique}"))
    }

    #[test]
    fn market_order_persists_filled_order_and_position() {
        let dir = temp_app_local_data_dir("market_order");
        let exec = submit_market_order(
            &dir,
            PaperBrokerMarketOrder {
                symbol: "AAPL".to_string(),
                side: PaperOrderSide::Buy,
                qty: 2.0,
                price: 100.0,
                at_unix_ms: 1_000,
                fee_bps: 0.0,
                slippage_bps: 0.0,
                client_order_id: None,
            },
        )
        .expect("paper fill");

        assert_eq!(exec.order.id, "paper_ord_000000000001");
        assert_eq!(exec.order.status, "filled");
        assert_eq!(exec.fill.price, 100.0);
        assert_eq!(exec.position.as_ref().map(|p| p.qty), Some(2.0));

        let doc = load_or_default(&dir, 2_000).expect("reload");
        assert_eq!(doc.orders.len(), 1);
        assert_eq!(doc.fills.len(), 1);
        assert_eq!(doc.positions.get("AAPL").map(|p| p.qty), Some(2.0));
        assert_eq!(doc.account.cash, 99_800.0);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn closing_position_records_realized_pnl() {
        let dir = temp_app_local_data_dir("realized_pnl");
        submit_market_order(
            &dir,
            PaperBrokerMarketOrder {
                symbol: "AAPL".to_string(),
                side: PaperOrderSide::Buy,
                qty: 2.0,
                price: 100.0,
                at_unix_ms: 1_000,
                fee_bps: 0.0,
                slippage_bps: 0.0,
                client_order_id: None,
            },
        )
        .expect("buy");
        submit_market_order(
            &dir,
            PaperBrokerMarketOrder {
                symbol: "AAPL".to_string(),
                side: PaperOrderSide::Sell,
                qty: 1.5,
                price: 110.0,
                at_unix_ms: 2_000,
                fee_bps: 0.0,
                slippage_bps: 0.0,
                client_order_id: None,
            },
        )
        .expect("sell");

        let doc = load_or_default(&dir, 3_000).expect("reload");
        assert_eq!(doc.positions.get("AAPL").map(|p| p.qty), Some(0.5));
        assert_eq!(doc.account.realized_pnl, 15.0);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn reset_reinitializes_account_state() {
        let dir = temp_app_local_data_dir("reset");
        submit_market_order(
            &dir,
            PaperBrokerMarketOrder {
                symbol: "AAPL".to_string(),
                side: PaperOrderSide::Buy,
                qty: 1.0,
                price: 100.0,
                at_unix_ms: 1_000,
                fee_bps: 0.0,
                slippage_bps: 0.0,
                client_order_id: None,
            },
        )
        .expect("fill");

        let doc = reset(&dir, 2_000).expect("reset");
        assert!(doc.positions.is_empty());
        assert!(doc.orders.is_empty());
        assert_eq!(doc.account.cash, DEFAULT_STARTING_CASH);
        let _ = fs::remove_dir_all(dir);
    }
}
