# Website Content Guide

Use this guide when building a dedicated VectorGrid website.

## Primary Website Goal

The website should help a new user understand:

- what VectorGrid does;
- why local-first strategy research matters;
- what Simple Mode is;
- what system requirements apply;
- how to install safely;
- what data is supported;
- how to run a first workflow;
- what GPU claims actually mean;
- why results are research evidence, not trading advice.

## Recommended Site Structure

1. Home.
2. Product.
3. Download.
4. Docs.
5. Data setup.
6. GPU and performance.
7. Validation.
8. Privacy and security.
9. Support.
10. Release notes.

## Home Page Message

Suggested headline:

```text
VectorGrid
```

Suggested supporting copy:

```text
Local Windows research software for fast trading-strategy optimization, validation evidence, and exportable run artifacts.
```

Suggested primary call to action:

```text
Download for Windows
```

Suggested secondary call to action:

```text
Read the first-run guide
```

## Website Promise

Use conservative claims:

- local-first strategy research;
- streamlined Simple Mode;
- optional advanced workbench;
- CPU-only compatibility;
- GPU acceleration for supported workloads;
- validation evidence after optimization;
- exportable summaries and reports.

Avoid broad claims:

- guaranteed profits;
- production trading automation;
- universal GPU speedups;
- live execution safety guarantees;
- broker compliance;
- AI-generated trading certainty.

## Home Page Sections

Recommended sections:

| Section | Purpose |
|---|---|
| First viewport | Name, one-sentence purpose, download/docs actions, Windows release status. |
| Simple workflow | Load data, optimize, validate, export summary. |
| Local data | CSV, bundled sample, Polygon cache, local artifacts. |
| Validation evidence | Explain why validation exists and what it does not prove. |
| CPU and GPU | Clarify CPU default and GPU variant boundaries. |
| Advanced workbench | Show deeper controls without making it the default path. |
| Privacy and disclaimer | Local-first data posture and no-financial-advice language. |
| Release integrity | Signed installer, hashes, publisher identity. |

## Download Page Checklist

The download page should include:

- latest version;
- release date;
- CPU-only installer;
- optional GPU installer if available;
- SHA-256 hashes;
- publisher/signature identity;
- Windows requirements;
- WebView2 note;
- install guide link;
- release notes link;
- known limitations;
- privacy/disclaimer links.

Do not hide unsigned status. If a build is unsigned, label it as private beta or test-only.

## Docs Page Checklist

Docs navigation should include:

- First Run And Simple Mode;
- Data And Polygon.io Setup;
- Optimization Workflows;
- Validation And Evidence;
- Reports, History, And Exports;
- GPU And Performance;
- Advanced Mode Reference;
- Options, ML, And Experimental Workflows;
- Live Execution And Risk;
- Privacy, Security, And Local Data;
- Troubleshooting;
- FAQ.

Also link:

- Known Limitations And Release Status;
- Support And Bug Reporting;
- Legal, License, And Disclaimers;
- Glossary;
- Roadmap And Future Scope.

## Screenshot Guidance

Screenshots should show:

- Simple Mode first;
- a loaded dataset;
- a completed optimization;
- optional validation recommendation;
- a Markdown summary or report export;
- route labels only when relevant.

Avoid screenshots that expose:

- API keys;
- broker account identifiers;
- private local paths;
- private dataset names;
- unpublished benchmark artifacts.

## Accessibility And Usability

The website should be easier to use than the app's most advanced screens.

Use:

- readable contrast;
- keyboard-visible focus states;
- alt text for screenshots;
- descriptive link text;
- stable headings;
- tables only where they improve scanning;
- no auto-playing motion;
- mobile-friendly download and docs navigation.

The first-run guide should remain readable without screenshots because some users will be following it in a separate window.

## Benchmark Presentation

When showing performance, include:

- exact release/commit;
- CPU/GPU hardware;
- dataset size;
- candidate count;
- workflow boundary;
- warm/cold state;
- date run;
- whether the result is a stored snapshot.

Use charts only when the caption contains enough context to avoid misleading comparisons.

## Legal And Risk Language

Every download page, release page, and footer should link:

- Disclaimer;
- Privacy;
- Security;
- Support;
- License.

Add visible no-financial-advice language near performance and live-execution content.

## Launch Readiness Checklist

Before publishing the website:

- all download links point to signed artifacts or clearly labeled beta artifacts;
- all hashes match release artifacts;
- docs match the released app version;
- screenshots do not leak secrets or local paths;
- GPU claims match the exact release variant;
- live execution is either omitted or documented with strong warnings;
- privacy and disclaimer pages are linked from every major page;
- support and security reporting paths are clear.
