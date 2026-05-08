# Public Docs Coverage Matrix

Use this matrix before publishing a website or release page. It maps public documentation to the major app surfaces.

| Surface | Covered by | Status |
|---|---|---|
| Product positioning | [Product Overview](01_product_overview.md) | Covered |
| Requirements | [System Requirements](02_system_requirements.md) | Covered |
| Install/update/uninstall | [Windows Install Guide](03_windows_install_guide.md) | Covered |
| Simple Mode | [First Run And Simple Mode](04_first_run_simple_mode.md) | Draft; blocked until the shipped UI implements and verifies Simple Mode |
| CSV data | [Data And Polygon.io Setup](05_data_and_polygon_setup.md) | Covered |
| Polygon historical/live | [Data And Polygon.io Setup](05_data_and_polygon_setup.md) | Covered |
| Optimization | [Optimization Workflows](06_optimization_workflows.md) | Covered |
| Candidate detail/local stability | [Optimization Workflows](06_optimization_workflows.md), [Advanced Mode Reference](10_advanced_mode_reference.md) | Covered |
| Validation | [Validation And Evidence](07_validation_and_evidence.md) | Covered |
| Reports/history/artifacts | [Reports, History, And Exports](08_reports_history_exports.md) | Covered |
| GPU/performance | [GPU And Performance](09_gpu_and_performance.md) | Covered |
| Advanced workbench | [Advanced Mode Reference](10_advanced_mode_reference.md) | Covered |
| ML Signal | [Options, ML, And Experimental Workflows](11_options_ml_experimental.md) | Covered |
| Options expression | [Options, ML, And Experimental Workflows](11_options_ml_experimental.md) | Covered |
| Live execution | [Live Execution And Risk](12_live_execution_and_risk.md) | Covered |
| Privacy/security/local data | [Privacy, Security, And Local Data](13_privacy_security_local_data.md) | Covered |
| Troubleshooting | [Troubleshooting](14_troubleshooting.md) | Covered |
| FAQ | [FAQ](15_faq.md) | Covered |
| Website copy | [Website Content Guide](16_website_content_guide.md) | Covered |
| Release page | [Release Page Template](17_release_page_template.md) | Covered |
| Known limitations | [Known Limitations And Release Status](18_known_limitations_release_status.md) | Covered |
| Support/bug reports | [Support And Bug Reporting](19_support_bug_reporting.md) | Covered |
| Legal/license/disclaimer | [Legal, License, And Disclaimers](20_legal_license_disclaimers.md) | Covered |
| Terminology | [Glossary](22_glossary.md) | Covered |
| Future scope | [Roadmap And Future Scope](23_roadmap_future_scope.md) | Covered |

## Pre-Publish Questions

Before publishing, confirm:

- Does the website clearly say whether a signed desktop artifact exists?
- Does every download link identify CPU-only vs GPU variant?
- Are hashes and publisher identity visible?
- Does the first-run guide match the released UI?
- If the guide says Simple Mode, does the shipped app actually open into Simple Mode or expose it immediately?
- Do screenshots avoid secrets, local paths, private datasets, and unpublished evidence?
- Are GPU claims variant-specific?
- Are live-execution claims either absent or strongly bounded?
- Are ML and options workflows labeled research-oriented unless promoted by release notes?
- Are support and security reporting paths visible?
- Are privacy, license, and disclaimer links present in the footer and release page?
- Does future-scope copy avoid implying an unshipped feature is already supported?
