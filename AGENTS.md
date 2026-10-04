# Repository instructions

## Verification after changes

- After each consolidated code change, run the relevant checks before considering the change complete or reporting it as done.
- For changes that can affect the project build or behavior, run the checks defined in `.github/workflows/ci.yml`: `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo test --workspace`, and `cargo build --workspace`.
- For narrowly scoped changes, run at least the checks directly affected by the change; run the full CI checks when the impact is broad or uncertain.
- If a check fails, investigate and fix the cause, then rerun the relevant checks. If the environment prevents a check from running, state that clearly and do not report verification as successful.
