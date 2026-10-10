# Changelog

## Unreleased

- Reorganized agent guidance around selective document consultation, split the monolithic requirements into focused documents, and moved release-workflow requirements to scoped GitHub instructions.
- Improved weak-signal roaming: use a time-spaced rolling median, align Medium and High thresholds with the UI's Weak range, require sustained recovery before restarting the degradation timer, and check/request scans more often while seeking.
- Show “Candidate ready” while a qualified alternative waits for the current connection's dwell time, and display signal percentages for diagnosis.
- Expanded repository guidance with project-specific change, verification, versioning, and release requirements.
- Prepared the GitHub Linux prerelease workflow for tag pushes and manual retries, with separated validation/build/package/publish jobs and archive checks.
- Updated checkout and artifact actions to Node 24, set the GitHub CLI repository explicitly in the publish job, and made missing manual-run tags fail with instructions for creating and pushing the tag.
- Documented the release procedure in the README.
