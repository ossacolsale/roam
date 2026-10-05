# Repository instructions

Apply these rules throughout the repository. A nearer `AGENTS.md` may add
scope-specific rules but must not weaken an explicit global requirement. Start
with this guide and inspect the files relevant to the task; broaden the search
when repository evidence requires it.

## Repository map

- `Cargo.toml` is the workspace manifest and version source; `Cargo.lock` records the resolved application dependencies.
- `crates/` contains the Rust workspace members: core policy, NetworkManager integration, daemon, and GTK UI.
- `packaging/`, `scripts/`, `systemd/`, and `assets/` contain package definitions, installers, service files, and application assets.
- `README.md` documents installation, operation, development, and GitHub releases; `SECURITY.md` covers security reporting.
- `CHANGELOG.md` records meaningful project changes.
- `.github/workflows/ci.yml` defines CI checks; `.github/workflows/release.yml` builds and publishes Linux x86_64 prereleases.
- `docs/` contains user-facing web documentation. Keep this map aligned with the repository when its structure changes.

## Change rules

- Classify the change first and make the smallest complete, relevant edit.
- Keep code clear, modular, and explicit. Prefer existing capabilities; avoid needless dependencies, abstractions, duplication, and comments that restate code. Comment intent or non-obvious constraints.
- Validate external input, handle errors explicitly, and never expose or commit secrets or sensitive data.
- Edit sources of truth and regenerate derived files when needed; document that relationship.
- Keep user and AI documentation accurate. Update this file when repository structure, ownership, or workflows change. Add a concise changelog entry for every meaningful change.

## Verification after changes

- After each consolidated code change, run the relevant checks before considering the change complete or reporting it as done.
- For changes that can affect the project build or behavior, run the checks defined in `.github/workflows/ci.yml`: `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo test --workspace`, and `cargo build --workspace`.
- For code changes, run the complete applicable suite, including relevant tests and project-defined formatting, lint, static, build, packaging, configuration, dependency, security, and workflow checks. Choose checks that match the changed areas and their risk.
- For narrowly scoped changes, run at least the checks directly affected by the change; run the full CI checks when the impact is broad or uncertain.
- Documentation-only edits need consistency and repository checks, not irrelevant code checks.
- If a check fails, investigate and fix the cause, then rerun the relevant checks. If the environment prevents a check from running, state that clearly and do not report verification as successful.
- Do not claim checks that were not run. Add or update regression coverage for bug fixes.

## Versioning and releases

- Use Semantic Versioning for the workspace version in `Cargo.toml`: breaking change = major, compatible capability = minor, compatible fix = patch. Keep all crate versions aligned through `version.workspace = true` and update `Cargo.lock` when required.
- Update user-visible version metadata and the changelog for code changes. Documentation-only edits do not require a version bump. Use prerelease identifiers (for example `0.2.0-rc.1`) while validating a release candidate; a release candidate is not a stable release.
- Roam distributes a Linux x86_64 archive through GitHub Releases. Keep the release pipeline usable when work concerns building, packaging, versioning, distribution, or release. Do not add external registry publishing without explicit authorization.
- Keep release readiness separate from actually publishing packages, deploying, pushing a release tag, or creating a GitHub Release. These external actions need explicit authorization.
- Separate release target validation, test/build, archive packaging, artifact validation, and GitHub Release publishing into dependent jobs. Grant `contents: write` only to the publish job.
- Support tag pushes and `workflow_dispatch` with an explicit tag input. For manual runs, verify the tag exists, validate its SemVer and workspace version, check out that tag, and rebuild all artifacts from that commit rather than the default branch. A corrected workflow on the default branch must be able to rerun the same tag without moving or recreating the tag or relying on artifacts from an earlier run.
- Use immutable SemVer tags as release identities. Serialize concurrent publishes for the same tag and make retries safe: reuse an existing GitHub Release instead of creating a duplicate, and upload only artifacts built and validated during the current run.
- Validate the packaged archive using its actual format. Check its integrity, required contents, and absence of incomplete, prohibited local, or sensitive files; ensure corrupt or truncated archives fail validation. Produce versioned artifacts and checksums. Identify prereleases clearly in GitHub and in user documentation.
- Document how to start a release and how to rerun an existing tag. Before completing release-pipeline work, inspect all workflows and verify their triggers, permissions, job dependencies, artifact paths and checks, and manual tag checkout behavior. Run applicable checks and inspect the final diff.
- Report whether the pipeline was validated locally. Do not imply that a release was published or verified on GitHub unless that actually happened.

## Completion

Finish only when the requested change, relevant checks, version/changelog updates when applicable, affected documentation, generated artifacts, and workflow consistency are complete. Report any verification that could not run and why.
