# Repository instructions

Keep these rules concise and applicable across the repository. Read the files
relevant to the task and any nearer `AGENTS.md`; broaden the search when the
change, risk, or uncertainty calls for it. Do not read project documents by
default when they are unrelated.

## Working rules

- Make the smallest complete change. Preserve existing user changes and avoid
  unrelated behavior changes, refactors, dependencies, or documentation.
- Treat external data as untrusted. Validate inputs, handle errors, and do not
  expose or commit secrets or sensitive information.
- Do not perform destructive or external actions (including commit, push,
  publishing, deployment, or release creation) without explicit authorization.
- Keep durable information in one authoritative source. Update derived files
  when their source changes; avoid copying specifications into summaries.
- Update applicable documentation when behavior, architecture, interfaces,
  workflows, or validation requirements change. Use plans only when staged work
  materially helps.
- Add a concise `CHANGELOG.md` entry for meaningful project changes. Docs-only
  changes do not require a version bump.
- Use SemVer in `Cargo.toml`, keep workspace crate versions and `Cargo.lock`
  aligned when versions change, and update user-visible version metadata. Use
  prerelease identifiers while validating a release candidate.

## Choosing project documentation

- For user-visible behavior or policy changes, use
  [`proj/requirements analysis.md`](proj/requirements%20analysis.md) to select
  the relevant focused requirements document; compare it with implementation
  and tests, and surface contradictions instead of silently rewriting either.
- For privacy or security changes, consult [`SECURITY.md`](SECURITY.md) and
  the relevant functional requirements.
- Use [`README.md`](README.md) for current installation, operation,
  development, and release instructions. Consult `CHANGELOG.md` for recent
  recorded changes only when relevant to the task.
- For workflow or release-pipeline changes, follow [`.github/AGENTS.md`](.github/AGENTS.md)
  and inspect the affected workflow files.
- For cross-cutting, architectural, ambiguous, high-risk, or resumed work,
  consult [`docs/ai/INDEX.md`](docs/ai/INDEX.md), then read only relevant
  sources. Small local changes do not require an index review.
- Consult `docs/ai/SESSION-STATE.md` only when work may continue an unfinished
  task. Create or update it only for a useful handoff; keep it factual and
  within about 150 words, verify claims against Git and the working tree, and
  clear it when the work is complete.

## Verification

- Run checks that cover the changed area and match its risk. For code or
  behavior changes, use the applicable checks from `.github/workflows/ci.yml`:
  formatting, Clippy with warnings denied, workspace tests, and workspace
  build. Documentation-only changes need relevant consistency and link checks,
  not unrelated code checks.
- Add or update regression coverage for bug fixes. Investigate failures and
  report checks that could not run; never claim checks that were not run.
- Before finishing, inspect the final diff and report the change, verification,
  and any remaining issue or incomplete work.
