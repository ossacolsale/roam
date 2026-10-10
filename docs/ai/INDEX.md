# AI documentation index

Use this index to find the authoritative source relevant to a task. It is a
navigation aid, not a requirement to read every linked document.

## Product requirements

- [`proj/requirements analysis.md`](../../proj/requirements%20analysis.md) is the
  entry point to the focused requirements below. These describe intended
  behavior; compare them with current code and tests, and report discrepancies.
- [`proj/requirements/product-and-privacy.md`](../../proj/requirements/product-and-privacy.md)
  covers product goals, supported platform, high-level architecture, settings,
  persisted data, and privacy.
- [`proj/requirements/network-and-roaming-policy.md`](../../proj/requirements/network-and-roaming-policy.md)
  covers NetworkManager integration, candidate selection, signal policy,
  activation, and failure handling.
- [`proj/requirements/application-and-validation.md`](../../proj/requirements/application-and-validation.md)
  covers UI, security constraints, tests, packaging, development order, and
  acceptance criteria.

## User and operational documentation

- [`README.md`](../../README.md) is authoritative for installation, use,
  development, and release instructions.
- [`SECURITY.md`](../../SECURITY.md) is authoritative for vulnerability
  reporting and security design.
- [`docs/index.html`](../index.html) is the English project landing page; read
  it when changing published web copy.
- [`CHANGELOG.md`](../../CHANGELOG.md) records project changes; consult recent
  entries when they matter to the task.

For workflow changes, follow [`.github/AGENTS.md`](../../.github/AGENTS.md) and
the affected workflow files. Implementation and current tests live under
`crates/`; inspect the relevant crate rather than treating requirements as a
description of current behavior.
