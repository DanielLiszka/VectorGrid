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

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PaperBracketLeg {
    Parent,
    StopLoss,
    TakeProfit,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PaperBracketTieBreak {
    StopLossFirst,
}

impl PaperBracketTieBreak {
    pub fn as_str(self) -> &'static str {
        match self {
            PaperBracketTieBreak::StopLossFirst => "stop_loss_first",
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_order_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bracket_group_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bracket_leg: Option<PaperBracketLeg>,
    pub symbol: String,
    pub side: PaperOrderSide,
    pub qty: f64,
    pub order_type: String,
    pub time_in_force: String,
    pub status: String,
    pub filled_qty: f64,
    pub avg_fill_price: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit_price: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stop_price: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tie_break: Option<PaperBracketTieBreak>,
    pub submitted_at_unix_ms: i64,
    pub filled_at_unix_ms: i64,
    #[serde(default)]
    pub updated_at_unix_ms: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub canceled_at_unix_ms: Option<i64>,
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
    pub bracket: Option<PaperBrokerBracketSpec>,
}

#[derive(Clone, Debug)]
pub struct PaperBrokerBracketSpec {
    pub stop_price: f64,
    pub take_profit_limit_price: f64,
    pub time_in_force: String,
    pub tie_break: PaperBracketTieBreak,
}

#[derive(Clone, Debug)]
pub struct PaperBrokerExecution {
    pub order: PaperBrokerOrder,
    pub fill: PaperBrokerFill,
    pub child_orders: Vec<PaperBrokerOrder>,
    pub account: PaperBrokerAccount,
    pub position: Option<PaperBrokerPosition>,
}

#[derive(Clone, Debug)]
pub struct PaperBrokerBracketAdvanceBar {
    pub symbol: String,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub at_unix_ms: i64,
}

#[derive(Clone, Debug)]
pub struct PaperBrokerBracketAdvanceResult {
    pub open_order_count_before: usize,
    pub filled_orders: Vec<PaperBrokerOrder>,
    pub cancelled_orders: Vec<PaperBrokerOrder>,
    pub tie_break_events: Vec<String>,
    pub account: PaperBrokerAccount,
    pub position: Option<PaperBrokerPosition>,
}

#[derive(Clone, Debug)]
pub struct PaperBrokerCancelOpenOrdersResult {
    pub open_order_count_before: usize,
    pub cancelled_orders: Vec<PaperBrokerOrder>,
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
    if let Some(bracket) = order.bracket.as_ref() {
        validate_bracket_spec(&order, bracket)?;
    }
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

    let bracket_group_id = order
        .bracket
        .as_ref()
        .map(|_| format!("paper_bracket_{order_id}"));
    let stored_order = PaperBrokerOrder {
        id: order_id.clone(),
        client_order_id: order.client_order_id,
        parent_order_id: None,
        bracket_group_id: bracket_group_id.clone(),
        bracket_leg: order.bracket.as_ref().map(|_| PaperBracketLeg::Parent),
        symbol: order.symbol.clone(),
        side: order.side,
        qty: order.qty,
        order_type: "market".to_string(),
        time_in_force: "day".to_string(),
        status: "filled".to_string(),
        filled_qty: order.qty,
        avg_fill_price: fill_price,
        limit_price: None,
        stop_price: None,
        tie_break: order.bracket.as_ref().map(|bracket| bracket.tie_break),
        submitted_at_unix_ms: order.at_unix_ms,
        filled_at_unix_ms: order.at_unix_ms,
        updated_at_unix_ms: order.at_unix_ms,
        canceled_at_unix_ms: None,
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
    let child_orders = if let (Some(bracket), Some(group_id)) =
        (order.bracket.as_ref(), bracket_group_id.as_ref())
    {
        create_bracket_child_orders(&mut doc, &stored_order, group_id, bracket)?
    } else {
        Vec::new()
    };
    doc.recompute_account(order.at_unix_ms);
    save(app_local_data_dir, &doc)?;

    Ok(PaperBrokerExecution {
        order: stored_order,
        fill,
        child_orders,
        account: doc.account.clone(),
        position: doc.positions.get(&order.symbol).cloned(),
    })
}

pub fn advance_brackets_for_bar(
    app_local_data_dir: &Path,
    bar: PaperBrokerBracketAdvanceBar,
) -> Result<PaperBrokerBracketAdvanceResult, String> {
    validate_advance_bar(&bar)?;
    let mut doc = load_or_default(app_local_data_dir, bar.at_unix_ms)?;
    let symbol = canonical_symbol(&bar.symbol);
    let open_order_count_before = doc
        .orders
        .iter()
        .filter(|order| is_open_order(order) && symbols_match(&order.symbol, &symbol))
        .count();

    #[derive(Default)]
    struct OpenBracketGroup {
        stop_idx: Option<usize>,
        take_idx: Option<usize>,
    }

    let mut groups: BTreeMap<String, OpenBracketGroup> = BTreeMap::new();
    for (idx, order) in doc.orders.iter().enumerate() {
        if !is_open_order(order)
            || !symbols_match(&order.symbol, &symbol)
            || order.submitted_at_unix_ms >= bar.at_unix_ms
        {
            continue;
        }
        let Some(group_id) = order.bracket_group_id.clone() else {
            continue;
        };
        let group = groups.entry(group_id).or_default();
        match order.bracket_leg {
            Some(PaperBracketLeg::StopLoss) => group.stop_idx = Some(idx),
            Some(PaperBracketLeg::TakeProfit) => group.take_idx = Some(idx),
            _ => {}
        }
    }

    let mut filled_orders = Vec::new();
    let mut cancelled_orders = Vec::new();
    let mut tie_break_events = Vec::new();
    for (group_id, group) in groups {
        let stop_hit = group
            .stop_idx
            .and_then(|idx| doc.orders.get(idx))
            .is_some_and(|order| bracket_child_hit(order, &bar));
        let take_hit = group
            .take_idx
            .and_then(|idx| doc.orders.get(idx))
            .is_some_and(|order| bracket_child_hit(order, &bar));
        let fill_idx = if stop_hit {
            if take_hit {
                tie_break_events.push(format!(
                    "{group_id}: both stop-loss and take-profit touched; tie_break=stop_loss_first"
                ));
            }
            group.stop_idx
        } else if take_hit {
            group.take_idx
        } else {
            None
        };
        let Some(fill_idx) = fill_idx else {
            continue;
        };
        let fill_price = bracket_child_trigger_price(&doc.orders[fill_idx]).ok_or_else(|| {
            format!(
                "open bracket order {} is missing trigger price",
                doc.orders[fill_idx].id
            )
        })?;
        let filled = fill_open_order_at_price(&mut doc, fill_idx, fill_price, bar.at_unix_ms)?;
        filled_orders.push(filled);
        let sibling_idx = if Some(fill_idx) == group.stop_idx {
            group.take_idx
        } else {
            group.stop_idx
        };
        if let Some(sibling_idx) = sibling_idx {
            if doc.orders.get(sibling_idx).is_some_and(is_open_order) {
                let cancelled = cancel_open_order_at_index(&mut doc, sibling_idx, bar.at_unix_ms)?;
                cancelled_orders.push(cancelled);
            }
        }
    }

    doc.recompute_account(bar.at_unix_ms);
    save(app_local_data_dir, &doc)?;
    Ok(PaperBrokerBracketAdvanceResult {
        open_order_count_before,
        filled_orders,
        cancelled_orders,
        tie_break_events,
        account: doc.account.clone(),
        position: doc.positions.get(&symbol).cloned(),
    })
}

pub fn cancel_open_orders_for_symbol(
    app_local_data_dir: &Path,
    symbol: &str,
    now_unix_ms: i64,
) -> Result<PaperBrokerCancelOpenOrdersResult, String> {
    let mut doc = load_or_default(app_local_data_dir, now_unix_ms)?;
    let symbol = canonical_symbol(symbol);
    let indices = doc
        .orders
        .iter()
        .enumerate()
        .filter_map(|(idx, order)| {
            (is_open_order(order) && symbols_match(&order.symbol, &symbol)).then_some(idx)
        })
        .collect::<Vec<_>>();
    let open_order_count_before = indices.len();
    let mut cancelled_orders = Vec::with_capacity(indices.len());
    for idx in indices {
        cancelled_orders.push(cancel_open_order_at_index(&mut doc, idx, now_unix_ms)?);
    }
    doc.recompute_account(now_unix_ms);
    save(app_local_data_dir, &doc)?;
    Ok(PaperBrokerCancelOpenOrdersResult {
        open_order_count_before,
        cancelled_orders,
        account: doc.account.clone(),
        position: doc.positions.get(&symbol).cloned(),
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

fn validate_bracket_spec(
    order: &PaperBrokerMarketOrder,
    bracket: &PaperBrokerBracketSpec,
) -> Result<(), String> {
    if !bracket.stop_price.is_finite() || bracket.stop_price <= 0.0 {
        return Err("paper bracket stop price must be finite and > 0".to_string());
    }
    if !bracket.take_profit_limit_price.is_finite() || bracket.take_profit_limit_price <= 0.0 {
        return Err("paper bracket take-profit limit price must be finite and > 0".to_string());
    }
    let tif = bracket.time_in_force.trim().to_ascii_lowercase();
    if tif != "day" && tif != "gtc" {
        return Err("paper bracket time_in_force must be day or gtc".to_string());
    }
    match order.side {
        PaperOrderSide::Buy => {
            if !(bracket.stop_price < order.price && order.price < bracket.take_profit_limit_price)
            {
                return Err(
                    "paper buy bracket requires stop below entry and take-profit above entry"
                        .to_string(),
                );
            }
        }
        PaperOrderSide::Sell => {
            if !(bracket.take_profit_limit_price < order.price && order.price < bracket.stop_price)
            {
                return Err(
                    "paper sell bracket requires take-profit below entry and stop above entry"
                        .to_string(),
                );
            }
        }
    }
    Ok(())
}

fn validate_advance_bar(bar: &PaperBrokerBracketAdvanceBar) -> Result<(), String> {
    if canonical_symbol(&bar.symbol).is_empty() {
        return Err("symbol is required".to_string());
    }
    for (name, value) in [
        ("open", bar.open),
        ("high", bar.high),
        ("low", bar.low),
        ("close", bar.close),
    ] {
        if !value.is_finite() || value <= 0.0 {
            return Err(format!("paper bracket bar {name} must be finite and > 0"));
        }
    }
    if bar.high < bar.low {
        return Err("paper bracket bar high must be >= low".to_string());
    }
    Ok(())
}

fn create_bracket_child_orders(
    doc: &mut PaperBrokerStateDoc,
    parent: &PaperBrokerOrder,
    group_id: &str,
    bracket: &PaperBrokerBracketSpec,
) -> Result<Vec<PaperBrokerOrder>, String> {
    let child_side = match parent.side {
        PaperOrderSide::Buy => PaperOrderSide::Sell,
        PaperOrderSide::Sell => PaperOrderSide::Buy,
    };
    let stop_id = next_order_id(doc);
    let take_id = next_order_id(doc);
    let common_tif = bracket.time_in_force.trim().to_ascii_lowercase();
    let stop = PaperBrokerOrder {
        id: stop_id,
        client_order_id: None,
        parent_order_id: Some(parent.id.clone()),
        bracket_group_id: Some(group_id.to_string()),
        bracket_leg: Some(PaperBracketLeg::StopLoss),
        symbol: parent.symbol.clone(),
        side: child_side,
        qty: parent.qty,
        order_type: "stop".to_string(),
        time_in_force: common_tif.clone(),
        status: "new".to_string(),
        filled_qty: 0.0,
        avg_fill_price: 0.0,
        limit_price: None,
        stop_price: Some(bracket.stop_price),
        tie_break: Some(bracket.tie_break),
        submitted_at_unix_ms: parent.filled_at_unix_ms,
        filled_at_unix_ms: 0,
        updated_at_unix_ms: parent.filled_at_unix_ms,
        canceled_at_unix_ms: None,
    };
    let take = PaperBrokerOrder {
        id: take_id,
        client_order_id: None,
        parent_order_id: Some(parent.id.clone()),
        bracket_group_id: Some(group_id.to_string()),
        bracket_leg: Some(PaperBracketLeg::TakeProfit),
        symbol: parent.symbol.clone(),
        side: child_side,
        qty: parent.qty,
        order_type: "limit".to_string(),
        time_in_force: common_tif,
        status: "new".to_string(),
        filled_qty: 0.0,
        avg_fill_price: 0.0,
        limit_price: Some(bracket.take_profit_limit_price),
        stop_price: None,
        tie_break: Some(bracket.tie_break),
        submitted_at_unix_ms: parent.filled_at_unix_ms,
        filled_at_unix_ms: 0,
        updated_at_unix_ms: parent.filled_at_unix_ms,
        canceled_at_unix_ms: None,
    };
    doc.orders.push(stop.clone());
    doc.orders.push(take.clone());
    Ok(vec![stop, take])
}

fn next_order_id(doc: &mut PaperBrokerStateDoc) -> String {
    let order_id = format!("paper_ord_{:012}", doc.next_order_seq);
    doc.next_order_seq = doc.next_order_seq.saturating_add(1);
    order_id
}

fn next_fill_id(doc: &mut PaperBrokerStateDoc) -> String {
    let fill_id = format!("paper_fill_{:012}", doc.next_fill_seq);
    doc.next_fill_seq = doc.next_fill_seq.saturating_add(1);
    fill_id
}

fn canonical_symbol(raw: &str) -> String {
    raw.trim().to_ascii_uppercase()
}

fn symbols_match(left: &str, right: &str) -> bool {
    canonical_symbol(left) == canonical_symbol(right)
}

fn is_open_order(order: &PaperBrokerOrder) -> bool {
    matches!(
        order.status.trim().to_ascii_lowercase().as_str(),
        "new"
            | "accepted"
            | "pending_new"
            | "accepted_for_bidding"
            | "partially_filled"
            | "pending_replace"
            | "pending_cancel"
            | "held"
            | "stopped"
            | "suspended"
            | "calculated"
    )
}

fn bracket_child_hit(order: &PaperBrokerOrder, bar: &PaperBrokerBracketAdvanceBar) -> bool {
    match (order.bracket_leg, order.side) {
        (Some(PaperBracketLeg::StopLoss), PaperOrderSide::Sell) => {
            order.stop_price.is_some_and(|price| bar.low <= price)
        }
        (Some(PaperBracketLeg::StopLoss), PaperOrderSide::Buy) => {
            order.stop_price.is_some_and(|price| bar.high >= price)
        }
        (Some(PaperBracketLeg::TakeProfit), PaperOrderSide::Sell) => {
            order.limit_price.is_some_and(|price| bar.high >= price)
        }
        (Some(PaperBracketLeg::TakeProfit), PaperOrderSide::Buy) => {
            order.limit_price.is_some_and(|price| bar.low <= price)
        }
        _ => false,
    }
}

fn bracket_child_trigger_price(order: &PaperBrokerOrder) -> Option<f64> {
    match order.bracket_leg {
        Some(PaperBracketLeg::StopLoss) => order.stop_price,
        Some(PaperBracketLeg::TakeProfit) => order.limit_price,
        _ => None,
    }
}

fn fill_open_order_at_price(
    doc: &mut PaperBrokerStateDoc,
    order_idx: usize,
    price: f64,
    at_unix_ms: i64,
) -> Result<PaperBrokerOrder, String> {
    let (order_id, symbol, side, qty) = {
        let order = doc
            .orders
            .get(order_idx)
            .ok_or_else(|| "paper order index out of range".to_string())?;
        (
            order.id.clone(),
            order.symbol.clone(),
            order.side,
            order.qty,
        )
    };
    let signed_qty = side.signed_qty(qty);
    doc.account.cash -= signed_qty * price;
    let realized_delta =
        apply_position_fill(&mut doc.positions, &symbol, signed_qty, price, at_unix_ms);
    doc.account.realized_pnl += realized_delta;
    let fill = PaperBrokerFill {
        id: next_fill_id(doc),
        order_id,
        symbol: symbol.clone(),
        side,
        qty,
        price,
        fee: 0.0,
        slippage: 0.0,
        at_unix_ms,
    };
    doc.fills.push(fill);
    let order = doc
        .orders
        .get_mut(order_idx)
        .ok_or_else(|| "paper order index out of range".to_string())?;
    order.status = "filled".to_string();
    order.filled_qty = qty;
    order.avg_fill_price = price;
    order.filled_at_unix_ms = at_unix_ms;
    order.updated_at_unix_ms = at_unix_ms;
    Ok(order.clone())
}

fn cancel_open_order_at_index(
    doc: &mut PaperBrokerStateDoc,
    order_idx: usize,
    at_unix_ms: i64,
) -> Result<PaperBrokerOrder, String> {
    let order = doc
        .orders
        .get_mut(order_idx)
        .ok_or_else(|| "paper order index out of range".to_string())?;
    order.status = "canceled".to_string();
    order.updated_at_unix_ms = at_unix_ms;
    order.canceled_at_unix_ms = Some(at_unix_ms);
    Ok(order.clone())
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
                bracket: None,
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
                bracket: None,
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
                bracket: None,
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
                bracket: None,
            },
        )
        .expect("fill");

        let doc = reset(&dir, 2_000).expect("reset");
        assert!(doc.positions.is_empty());
        assert!(doc.orders.is_empty());
        assert_eq!(doc.account.cash, DEFAULT_STARTING_CASH);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn market_order_with_bracket_creates_linked_child_orders() {
        let dir = temp_app_local_data_dir("bracket_children");
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
                bracket: Some(PaperBrokerBracketSpec {
                    stop_price: 95.0,
                    take_profit_limit_price: 110.0,
                    time_in_force: "gtc".to_string(),
                    tie_break: PaperBracketTieBreak::StopLossFirst,
                }),
            },
        )
        .expect("paper bracket fill");

        assert_eq!(exec.child_orders.len(), 2);
        assert_eq!(
            exec.child_orders[0].parent_order_id.as_deref(),
            Some(exec.order.id.as_str())
        );
        assert_eq!(exec.child_orders[0].status, "new");
        assert_eq!(exec.child_orders[1].status, "new");

        let doc = load_or_default(&dir, 2_000).expect("reload");
        assert_eq!(doc.orders.len(), 3);
        assert_eq!(
            doc.orders
                .iter()
                .filter(|order| is_open_order(order))
                .count(),
            2
        );
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn bracket_stop_fill_cancels_take_profit_sibling() {
        let dir = temp_app_local_data_dir("bracket_stop");
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
                bracket: Some(PaperBrokerBracketSpec {
                    stop_price: 95.0,
                    take_profit_limit_price: 110.0,
                    time_in_force: "gtc".to_string(),
                    tie_break: PaperBracketTieBreak::StopLossFirst,
                }),
            },
        )
        .expect("paper bracket fill");

        let advanced = advance_brackets_for_bar(
            &dir,
            PaperBrokerBracketAdvanceBar {
                symbol: "AAPL".to_string(),
                open: 99.0,
                high: 101.0,
                low: 94.0,
                close: 96.0,
                at_unix_ms: 2_000,
            },
        )
        .expect("advance");

        assert_eq!(advanced.filled_orders.len(), 1);
        assert_eq!(
            advanced.filled_orders[0].bracket_leg,
            Some(PaperBracketLeg::StopLoss)
        );
        assert_eq!(advanced.cancelled_orders.len(), 1);
        let doc = load_or_default(&dir, 3_000).expect("reload");
        assert!(!doc.positions.contains_key("AAPL"));
        assert_eq!(doc.account.realized_pnl, -10.0);
        assert_eq!(
            doc.orders
                .iter()
                .filter(|order| is_open_order(order))
                .count(),
            0
        );
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn bracket_same_bar_stop_and_take_uses_stop_loss_first() {
        let dir = temp_app_local_data_dir("bracket_tie_break");
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
                bracket: Some(PaperBrokerBracketSpec {
                    stop_price: 95.0,
                    take_profit_limit_price: 110.0,
                    time_in_force: "gtc".to_string(),
                    tie_break: PaperBracketTieBreak::StopLossFirst,
                }),
            },
        )
        .expect("paper bracket fill");

        let advanced = advance_brackets_for_bar(
            &dir,
            PaperBrokerBracketAdvanceBar {
                symbol: "AAPL".to_string(),
                open: 100.0,
                high: 111.0,
                low: 94.0,
                close: 101.0,
                at_unix_ms: 2_000,
            },
        )
        .expect("advance");

        assert_eq!(
            advanced.filled_orders[0].bracket_leg,
            Some(PaperBracketLeg::StopLoss)
        );
        assert_eq!(advanced.tie_break_events.len(), 1);
        assert!(advanced.tie_break_events[0].contains("stop_loss_first"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn cancel_open_orders_for_symbol_cancels_child_orders_only() {
        let dir = temp_app_local_data_dir("cancel_children");
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
                bracket: Some(PaperBrokerBracketSpec {
                    stop_price: 95.0,
                    take_profit_limit_price: 110.0,
                    time_in_force: "gtc".to_string(),
                    tie_break: PaperBracketTieBreak::StopLossFirst,
                }),
            },
        )
        .expect("paper bracket fill");

        let cancelled = cancel_open_orders_for_symbol(&dir, "AAPL", 2_000).expect("cancel");
        assert_eq!(cancelled.open_order_count_before, 2);
        assert_eq!(cancelled.cancelled_orders.len(), 2);
        let doc = load_or_default(&dir, 3_000).expect("reload");
        assert_eq!(
            doc.orders
                .iter()
                .filter(|order| is_open_order(order))
                .count(),
            0
        );
        assert_eq!(doc.positions.get("AAPL").map(|p| p.qty), Some(1.0));
        let _ = fs::remove_dir_all(dir);
    }
}
