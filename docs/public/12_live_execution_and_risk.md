# Live Execution And Risk

VectorGrid is research software. Broker-connected workflows, if enabled, require additional caution and user supervision.

## Execution Modes

Execution-related surfaces can include:

- dry-run requests;
- local paper execution;
- Alpaca paper;
- Alpaca live;
- live runner workflows tied to a Polygon live stock stream.

Availability depends on the release configuration and documented support boundary.

## Dry Run

Dry run does not require broker credentials. It is useful for checking request construction, strategy support, and UI flow without submitting broker orders.

Dry run is not proof that live execution will succeed.

## Local Paper

Local paper execution simulates order state locally. It can help review:

- intended positions;
- bracket parent/child behavior;
- stop-loss and take-profit logic;
- local session evidence.

Local paper is not a broker and does not reproduce live fills, slippage, outages, market rules, or account restrictions.

## Alpaca Workflows

Alpaca workflows require user-supplied credentials for non-dry-run broker calls.

The app should not persist raw credentials in saved run artifacts or public reports. Credential storage and status should be reviewed through the app settings and privacy documentation.

Before using broker-connected workflows:

- understand Alpaca account type and environment;
- confirm paper vs live mode;
- confirm symbol and market status;
- confirm current position and open orders;
- confirm risk policy;
- confirm order size;
- confirm bracket settings;
- confirm whether the source data is delayed or live.

## Live Runner Dependency

Live runner workflows depend on an active Polygon live stock stream. The runner should enforce that the stream symbol matches the requested execution symbol.

Delayed market data is not equivalent to real-time execution input. Do not use delayed data as if it were live broker-grade input.

## Risk Controls

Release notes should clearly state the available risk controls. Examples can include:

- dry-run mode;
- local paper mode;
- max-notional policy;
- shorting policy;
- bracket requirements;
- emergency halt;
- cancel-open-orders action;
- flatten action;
- confirmation dialogs for live actions.

Risk controls reduce mistakes but do not make the app a production trading system.

## Broker-Connected Checklist

Before any non-dry-run broker action:

1. Verify the app version and release variant.
2. Confirm paper vs live.
3. Confirm credentials are for the intended account.
4. Confirm market status.
5. Confirm data source and freshness.
6. Confirm symbol and quantity.
7. Confirm stop-loss/take-profit behavior.
8. Confirm risk-policy limits.
9. Confirm current account position and open orders.
10. Be ready to intervene outside the app.

## Public Disclaimer

VectorGrid is not financial advice, investment advice, tax advice, legal advice, or a broker compliance tool. Users are solely responsible for trading decisions and for complying with all applicable rules, laws, broker terms, and risk controls.

