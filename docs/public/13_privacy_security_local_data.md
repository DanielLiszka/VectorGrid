# Privacy, Security, And Local Data

VectorGrid is designed as a local desktop application. Most user data remains on the user's machine unless the user invokes a provider, broker, updater, or external link.

## Local Data Categories

VectorGrid may store:

- app settings;
- market-data root configuration;
- Polygon cache metadata and CSVs;
- binary CSV caches;
- saved run artifacts;
- validation reports;
- JSON/CSV/XLSX/Markdown exports;
- desktop logs;
- window state;
- local paper execution state;
- provider credential status metadata.

## Windows App Identifier

The current Windows app identifier is:

```text
dev.vectoralpha.vectorgrid
```

Application data, settings, caches, and reports may be stored under operating-system application data locations associated with this identifier.

## Credentials

Public release builds must not include maintainer credentials.

User credentials can include:

- Polygon.io API key;
- Alpaca API key and secret;
- updater signing trust metadata if a release enables updates.

VectorGrid should prefer operating-system credential storage where supported. Polygon can use a local JSON fallback when keyring storage is unavailable. Alpaca workflows should avoid plaintext credential persistence.

Never share API keys, broker secrets, signing keys, updater private keys, or local secret fallback files.

## Network Access

Network access may occur when the user:

- installs the app and allows the Windows installer to download or repair Microsoft Edge WebView2 Runtime if it is missing;
- downloads Polygon.io historical data;
- starts a Polygon live stock stream;
- uses Alpaca broker workflows;
- opens external documentation/support links;
- uses an updater if enabled in a signed release.

The core local CSV optimization workflow can run offline after installation.

## Reports And Artifacts

Generated artifacts and reports may include sensitive research context:

- symbols;
- time ranges;
- strategy parameters;
- metrics;
- validation warnings;
- local timestamps;
- dataset provenance;
- app version;
- route metadata.

Review before sharing. Redact local paths or account context when needed.

## Logs

Logs are useful for troubleshooting, but they may include workflow state, errors, paths, provider status, or timestamps.

Before posting logs publicly:

- remove API keys and secrets;
- remove broker account identifiers;
- remove private local paths if necessary;
- remove private dataset names if necessary;
- keep enough context for maintainers to reproduce the issue.

## Security Reporting

Do not report security vulnerabilities in public issue threads.

Use the repository's private vulnerability reporting path if available, or contact the maintainer privately through the repository owner account listed in `SECURITY.md`.

## Deleting Local Data

Users can remove old reports, artifacts, and caches when they are no longer needed. Provider caches may need to be redownloaded later.

Before deleting:

- export any research reports you want to keep;
- note provider rate limits if cache redownloads will be needed;
- make sure no running app workflow is writing to the folder.
