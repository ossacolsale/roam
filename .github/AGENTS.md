# GitHub workflow instructions

Read only the workflow files relevant to the change. For release-pipeline work,
inspect all workflows and preserve these release guarantees:

- Support tag pushes and manual runs with an explicit existing tag. A manual
  run must validate the tag's SemVer and workspace version, check out that tag,
  and rebuild artifacts from its commit. A corrected default-branch workflow
  must be able to rerun the same tag without moving it or relying on old
  artifacts.
- Treat SemVer tags as immutable release identities. Serialize publishes per
  tag, reuse an existing GitHub Release on retry, and upload only artifacts
  built and validated in the current run.
- Separate target validation, build/test, archive packaging, artifact
  validation, and publishing into dependent jobs. Grant `contents: write` only
  to the publishing job.
- Validate the actual archive format, integrity, required contents, and absence
  of incomplete, prohibited local, or sensitive files; ensure corrupt or
  truncated archives fail. Produce versioned artifacts and checksums, and mark
  prereleases clearly in GitHub and user documentation.
- Document how to start a release and rerun an existing tag. Before finishing,
  check workflow triggers, permissions, dependencies, artifact paths and
  validation, and manual tag checkout. Run applicable checks and inspect the
  final diff. Report whether validation ran locally; do not imply publication
  or GitHub verification unless it happened.

Do not publish, deploy, push a tag, or create a GitHub Release without explicit
authorization.
