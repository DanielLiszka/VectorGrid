use std::collections::VecDeque;

#[derive(Clone, Debug, PartialEq)]
pub struct MockBrokerOrder {
    pub id: String,
    pub symbol: String,
    pub side: String,
    pub qty: f64,
    pub filled_qty: f64,
    pub status: String,
}

#[derive(Clone, Debug, PartialEq)]
pub enum MockBrokerSubmitResult {
    Filled,
    Partial { filled_qty: f64 },
    Rejected { reason: String },
    Timeout,
}

#[derive(Clone, Debug, PartialEq)]
pub enum MockBrokerCancelResult {
    Cancelled,
    Failed { reason: String },
}

#[derive(Clone, Debug, Default)]
pub struct MockBroker {
    orders: Vec<MockBrokerOrder>,
    next_submit_results: VecDeque<MockBrokerSubmitResult>,
    next_cancel_results: VecDeque<MockBrokerCancelResult>,
    next_id: u64,
}

impl MockBroker {
    pub fn new() -> Self {
        Self {
            next_id: 1,
            ..Self::default()
        }
    }

    pub fn queue_submit_result(&mut self, result: MockBrokerSubmitResult) {
        self.next_submit_results.push_back(result);
    }

    pub fn queue_cancel_result(&mut self, result: MockBrokerCancelResult) {
        self.next_cancel_results.push_back(result);
    }

    pub fn submit_market_order(
        &mut self,
        symbol: impl Into<String>,
        side: impl Into<String>,
        qty: f64,
    ) -> Result<MockBrokerOrder, String> {
        if !qty.is_finite() || qty <= 0.0 {
            return Err("mock broker quantity must be finite and > 0".to_string());
        }
        let id = format!("mock_ord_{}", self.next_id);
        self.next_id = self.next_id.saturating_add(1);
        let result = self
            .next_submit_results
            .pop_front()
            .unwrap_or(MockBrokerSubmitResult::Filled);
        let (status, filled_qty) = match result {
            MockBrokerSubmitResult::Filled => ("filled".to_string(), qty),
            MockBrokerSubmitResult::Partial { filled_qty } => {
                ("partially_filled".to_string(), filled_qty.clamp(0.0, qty))
            }
            MockBrokerSubmitResult::Rejected { reason: _ } => ("rejected".to_string(), 0.0),
            MockBrokerSubmitResult::Timeout => ("timeout".to_string(), 0.0),
        };
        let order = MockBrokerOrder {
            id,
            symbol: symbol.into(),
            side: side.into(),
            qty,
            filled_qty,
            status,
        };
        self.orders.push(order.clone());
        Ok(order)
    }

    pub fn cancel_open_orders(&mut self, symbol: &str) -> Result<Vec<String>, String> {
        match self
            .next_cancel_results
            .pop_front()
            .unwrap_or(MockBrokerCancelResult::Cancelled)
        {
            MockBrokerCancelResult::Cancelled => {
                let mut cancelled = Vec::new();
                for order in &mut self.orders {
                    if order.symbol == symbol
                        && matches!(
                            order.status.as_str(),
                            "new" | "partially_filled" | "timeout"
                        )
                    {
                        order.status = "canceled".to_string();
                        cancelled.push(order.id.clone());
                    }
                }
                Ok(cancelled)
            }
            MockBrokerCancelResult::Failed { reason } => Err(reason),
        }
    }

    pub fn orders(&self) -> &[MockBrokerOrder] {
        &self.orders
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct MockFeedBar {
    pub symbol: String,
    pub open_unix_ms: i64,
    pub close: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub enum MockFeedEvent {
    Bar(MockFeedBar),
    Duplicate(MockFeedBar),
    OutOfOrder(MockFeedBar),
    Closed,
    Error(String),
}

#[derive(Clone, Debug, Default)]
pub struct MockFeed {
    events: VecDeque<MockFeedEvent>,
    last_open_unix_ms: Option<i64>,
}

impl MockFeed {
    pub fn push_bar(&mut self, symbol: impl Into<String>, open_unix_ms: i64, close: f64) {
        self.events.push_back(MockFeedEvent::Bar(MockFeedBar {
            symbol: symbol.into(),
            open_unix_ms,
            close,
        }));
    }

    pub fn push_close(&mut self) {
        self.events.push_back(MockFeedEvent::Closed);
    }

    pub fn push_error(&mut self, message: impl Into<String>) {
        self.events.push_back(MockFeedEvent::Error(message.into()));
    }

    pub fn next_event(&mut self) -> Option<MockFeedEvent> {
        let event = self.events.pop_front()?;
        let classified = match event {
            MockFeedEvent::Bar(bar) => {
                let classified = if self.last_open_unix_ms == Some(bar.open_unix_ms) {
                    MockFeedEvent::Duplicate(bar.clone())
                } else if self
                    .last_open_unix_ms
                    .map(|last| bar.open_unix_ms < last)
                    .unwrap_or(false)
                {
                    MockFeedEvent::OutOfOrder(bar.clone())
                } else {
                    MockFeedEvent::Bar(bar.clone())
                };
                if matches!(classified, MockFeedEvent::Bar(_)) {
                    self.last_open_unix_ms = Some(bar.open_unix_ms);
                }
                classified
            }
            other => other,
        };
        Some(classified)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mock_broker_models_fills_partials_rejects_timeouts_and_cancel_failures() {
        let mut broker = MockBroker::new();
        broker.queue_submit_result(MockBrokerSubmitResult::Partial { filled_qty: 0.4 });
        broker.queue_submit_result(MockBrokerSubmitResult::Rejected {
            reason: "risk".to_string(),
        });
        broker.queue_submit_result(MockBrokerSubmitResult::Timeout);

        assert_eq!(
            broker
                .submit_market_order("AAPL", "buy", 1.0)
                .unwrap()
                .status,
            "partially_filled"
        );
        assert_eq!(
            broker
                .submit_market_order("AAPL", "buy", 1.0)
                .unwrap()
                .status,
            "rejected"
        );
        assert_eq!(
            broker
                .submit_market_order("AAPL", "buy", 1.0)
                .unwrap()
                .status,
            "timeout"
        );

        broker.queue_cancel_result(MockBrokerCancelResult::Failed {
            reason: "broker unavailable".to_string(),
        });
        assert!(broker
            .cancel_open_orders("AAPL")
            .unwrap_err()
            .contains("broker unavailable"));
        broker.queue_cancel_result(MockBrokerCancelResult::Cancelled);
        assert_eq!(
            broker.cancel_open_orders("AAPL").unwrap(),
            vec!["mock_ord_1", "mock_ord_3"]
        );
        assert_eq!(broker.orders()[0].status, "canceled");
        assert_eq!(broker.orders()[1].status, "rejected");
        assert_eq!(broker.orders()[2].status, "canceled");
    }

    #[test]
    fn mock_feed_classifies_normal_duplicate_out_of_order_close_and_error_events() {
        let mut feed = MockFeed::default();
        feed.push_bar("AAPL", 1000, 101.0);
        feed.push_bar("AAPL", 1000, 101.1);
        feed.push_bar("AAPL", 900, 100.5);
        feed.push_error("stream failed");
        feed.push_close();

        assert!(matches!(feed.next_event(), Some(MockFeedEvent::Bar(_))));
        assert!(matches!(
            feed.next_event(),
            Some(MockFeedEvent::Duplicate(_))
        ));
        assert!(matches!(
            feed.next_event(),
            Some(MockFeedEvent::OutOfOrder(_))
        ));
        assert!(
            matches!(feed.next_event(), Some(MockFeedEvent::Error(message)) if message == "stream failed")
        );
        assert_eq!(feed.next_event(), Some(MockFeedEvent::Closed));
        assert_eq!(feed.next_event(), None);
    }
}
