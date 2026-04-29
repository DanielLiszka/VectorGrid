# VectorGrid Privacy Notes

VectorGrid is designed as a local desktop application. Public release builds should not include maintainer credentials, private datasets, benchmark outputs, or local machine paths beyond normal application runtime paths.

## Local Data

The app may store local settings, cached market data, run artifacts, reports, logs, and provider configuration under the operating system's application data directory for the configured app identifier.

On Windows, the current identifier is:

```text
dev.vectoralpha.vectorgrid
```

## Credentials

Provider credentials should be stored through the operating system credential store where supported. Public release artifacts must not bundle API keys, broker credentials, signing keys, updater private keys, or local secret fallback files.

## Network Access

Network access is used only for workflows the user configures or invokes, such as provider data access, live/delayed market streams, updater checks when enabled, and links opened by the operating system.

The updater is disabled by default unless a signed release explicitly enables it and documents its HTTPS update endpoint.

## Reports And Artifacts

Generated reports and artifacts are local files created by the user workflow. Users are responsible for reviewing files before sharing them because they may contain ticker symbols, strategy parameters, run metadata, local timestamps, and dataset provenance.

## Trading Disclaimer

VectorGrid is research and analysis software. It is not financial advice, a recommendation to trade, or a production trading system. Broker-connected workflows, if enabled, require user supervision and separate risk controls.
