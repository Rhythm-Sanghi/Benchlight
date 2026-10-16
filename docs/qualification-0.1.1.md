# Benchlight 0.1.1 local release-candidate qualification — 2026-10-16

This is a reviewable local Windows candidate, not release approval. All scan,
cleanup and upgrade fixtures were disposable. Nothing was pushed, tagged,
published, merged or remotely deleted. `VERIFIED` describes only the evidence
and environment below; `NOT VERIFIED` marks an unexercised boundary.

## Source review and corrections

- VERIFIED: Original baseline `b51c95b513ae9665bf5dfb0fea334bde4cf4f895`
  matched `origin/main`. Hardening commits `6e527f21cf8b4a7ac0e12aad215fc40213356e84`
  and `8df4f2106415c099f0ff7b0c22bae665897f7eed` exist. The prior
  [engineering handoff](qualification-2026-10-15.md) records the reproduced
  omission of a newly added file from a supposedly complete measurement,
  non-atomic cleanup plan/log writes, and source packaging of untracked files.
  I reviewed the branch diff against that baseline and reproduced or inspected
  the relevant boundaries, rather than relying on the passing test count.
- VERIFIED: The membership recheck addresses the original add-file defect,
  including unchanged directory timestamps. Final review found that this
  second enumeration did not check cancellation. It now checks cancellation
  during and after rechecking; a 100,000-name cap bounds each retained name
  set, and reaching it marks the measurement partial. The limit is per open
  directory frame, so deeply nested large directories can still use substantial
  memory; the existing 128-frame depth limit also applies.
- VERIFIED: A failed Windows Shell recycle could have lost its recorded staged
  path when that path no longer existed. The path is now retained in the plan,
  and the desktop directs the user to check both that path and the Recycle Bin.
  This is recovery evidence, not a claim that recycling succeeded.
- VERIFIED: The release version is coherently `0.1.1` in Cargo, JavaScript,
  Tauri and packaging checks. These corrections are local commits
  `efa1a16f01b77ce8a865ab902982e60a8e5ffa15`,
  `e687a30f08ccf29892bf8a046228381566b0baec`,
  `9ed58d834efe9ad7f486bbd518189f7a676e2a12` and
  `bdcc7d11e8829d76eed95e8ac89fbfa3a36c8bf2` after the original two.
  No new dependency, cleaning category or broad UI redesign was introduced.

## Scan and cleanup safety

- VERIFIED: Focused tests cover additions, deletion, rename, missing directory,
  unchanged-timestamp membership change, mutation during enumeration,
  cancellation, and the name budget. Recheck errors and a too-large name set
  prevent a complete measurement. Junctions and reparse points are skipped;
  a disposable junction inside a candidate makes that candidate incomplete,
  and an ordinary cleanup plan is refused while an external fixture remains
  untouched. Cleanup separately rechecks file identity, size, timestamps and
  project evidence; it does not hash every candidate file's content.
- NOT VERIFIED: Filesystem observations are not an atomic snapshot. A mutation
  after the final recheck can escape that scan; a same-size file rewrite with
  restored timestamps may escape the cleanup fingerprint. Permission errors
  during the recheck and an exact mutation at the final check were not
  fault-injected on this host. The code treats read errors as partial and
  cleanup revalidates, but these results do not prove all filesystem races
  are closed.
- VERIFIED: Plan creation stores the assigned id and plan detail in one SQLite
  transaction. Each staging/result transition stores plan detail and operation
  event together; an injected log-write failure rolls both back. Tests cover
  a reopened interrupted staging state, one-shot plan application, exact
  `RECYCLE <id>` confirmation, changed evidence, replacement directories,
  locks, new junctions, and a fixture sent to the Recycle Bin. A failed item
  leaves the plan partial; a recovered directory is never overwritten
  automatically. No permanent-delete fallback exists.
- NOT VERIFIED: A real process crash at every instruction boundary, a forced
  Windows Shell failure after a successful recycle, and concurrent hostile
  filesystem mutation were not reproduced. SQLite and Windows Shell recycling
  do not share one atomic transaction. A saved staging path can identify an
  ambiguous state, and users must inspect both the original path and Recycle
  Bin before manual recovery.

## Snapshots and tool execution

- VERIFIED: Database and reopened-core tests cover record counts, approximate
  stored JSON bytes, exact typed confirmation, stale-count rejection and
  retention of the latest baseline. Removal affects SQLite snapshot metadata,
  not project files. Approximate JSON bytes do not promise SQLite file shrinkage.
  The published 0.1.0 schema-6 database upgraded in place without a migration;
  its two snapshots and one historical plan remained readable.
- NOT VERIFIED: Simultaneous desktop/CLI history-removal contention and a
  process crash mid-removal were not stress-tested. SQLite transactions and
  stale-count checks address those paths, but the tests do not model every
  interleaving.
- VERIFIED: Tool refresh is explicit. The UI says version commands are bounded
  native processes, not sandboxed. The process test invokes timeout, oversized
  output and version fixtures via the bounded runner. The three ignored tests
  `blocked_child_fixture`, `large_output_fixture`, and `version_fixture` are
  internal child entry points; running the blocked child directly would hang.
  They are Windows process fixtures and are safely exercised by
  `version_process_timeout_and_output_limit_are_enforced`. No separate
  arbitrary third-party executable behavior is certified.

## Installation and native desktop

- VERIFIED: The downloaded, published unsigned `v0.1.0` NSIS installer matched
  its published SHA256SUMS entry, SHA-256
  `eedabb9ac4acb5134a59895f133811adc70d1e6e14a5d156e29014ebb946e588`.
  `scripts/test-upgrade.ps1` installed it in a space-containing disposable
  directory with an isolated `--data-dir`, scanned a fixture, saved two working
  snapshots and a cleanup preview, set an exclusion and appearance value in
  the isolated schema-6 database, then installed the local 0.1.1 candidate at
  the same path. Roots, exclusions, theme, plan and snapshots persisted;
  changing a fixture lockfile produced `Changed`. A fresh scan passed.
  Uninstall and reinstall preserved the isolated database and settings.
  Appearance was seeded through SQLite for this upgrade test, so the old
  desktop appearance control itself was not exercised in that sequence. After
  the final installer build, the upgraded profile was also opened in the
  installed 0.1.1 desktop: Overview showed its root and scan, Settings showed
  the dark theme and exclusion, Changes showed two working records, and Space
  showed the historical preview plan. That extra install was uninstalled and
  the isolated database remained.
- VERIFIED: A separate fresh 0.1.1 installation, same-version reinstall and
  uninstall passed `scripts/test-installer.ps1`; both isolated and co-located
  local metadata survived uninstall. The installed binaries matched release
  outputs, accounting for Tauri's documented NSIS bundle marker patch, and
  installed legal notices were present. Immediately chaining the fresh
  uninstall to another upgrade attempt briefly met the harness guard for an
  existing per-user registration; the guard refused to replace it. Once
  Windows cleared that registration, the same final-installer upgrade passed.
- VERIFIED (native UI automation): Installed 0.1.1 launched without a dev
  server into an empty profile, added a disposable root, scanned one project,
  navigated Overview/Space/Projects/Tools/Changes/Settings, refreshed tools,
  saved a working baseline, detected a changed lockfile, selected a rebuildable
  fixture for cleanup, created and revalidated a plan without applying it,
  added an exclusion, changed to light theme, restarted and reopened the same
  profile. Root, scan, exclusion and theme persisted. The accessibility tree
  exposed labeled navigation, input and action controls. The fixture was not
  recycled. These were automated UI observations, not human manual testing.
- NOT VERIFIED: Candidate 0.1.1 at exactly 800 px, 200% zoom, Windows high
  contrast, screen-reader speech and keyboard-only focus restoration were not
  established in this session. The attempted native resize/zoom controls did
  not yield a reliable changed layout. Offline operation was not tested with
  networking disabled. Missing WebView2, interrupted installer, live removable
  drives and cloud placeholders lacked a safe isolated environment. The
  historical 0.1.0 native review remains recorded separately.

## Performance and complete checks

- VERIFIED: Ten serial scan/snapshot/comparison cycles reopened the isolated
  database through new CLI processes; two tool refreshes completed. Total
  elapsed time was 42,281 ms, ending with 11 scans, 11 snapshots and a
  147,456-byte database. This was a bounded fixture run, not a long-duration
  soak test. Native UI navigation was exercised once per main screen.
- VERIFIED: Three fresh optimized 50-project / 20,050-file benchmark fixtures
  measured scans at 1,105, 1,070 and 1,067 ms; 100-page queries at 12, 14 and
  16 ms; cancellation at 4, 4 and 2 ms, each reporting `cancelled`. Working-set
  samples (200 ms interval) peaked at 8,802,304, 7,593,984 and 7,557,120
  bytes. The earlier runs of 1,949, 2,183 and 1,297 ms used different system
  load. Neither set proves a speedup or worst-case memory bound.
- VERIFIED: After the last application/source change, `scripts/check.ps1`
  passed Rust formatting, strict Clippy, 48 Rust tests (three internal child
  fixtures ignored), source-archive regression, frontend formatting, lint,
  typecheck, 23 component tests, 80 contrast pairs and production frontend
  build. The release CLI and NSIS installer built. A first-party Markdown check
  of the candidate archive found no missing targets in 29 local links or 2
  local anchors.

## Source and GitHub integration

- VERIFIED: Historical archive commit
  `8df4f2106415c099f0ff7b0c22bae665897f7eed`, tree
  `b536a83bcbc7ea30871b973ec22e6609a9944c1c`, has 913 Git files; all
  913 ZIP file entries match, with legal notices. Its archive SHA-256 is
  `a7bc8213ca4520d90842a92c63923d140cc22ee18051a9f9657801fb193f82b1`.
  This is a local historical archive, distinct from the published v0.1.0
  installer hash above.
- VERIFIED: Candidate source commit
  `bdcc7d11e8829d76eed95e8ac89fbfa3a36c8bf2`, tree
  `06f72af654792ec361e96cc8cc4f139a537e2412`, has 915 Git files; all 915
  ZIP file entries match and include `LICENSE` and the third-party notice
  inventory. The archive SHA-256 is
  `2a4effe99348438f2bda4fa4354cae3e24645194cc1139d47293c2e85cc64206`.
  The archive fixture proves untracked/dirty exclusions, exact committed
  version checks, legal inclusion, repeatable hash and refusal of a committed
  sensitive path. The report-only commit following source commit `bdcc7d1`
  is intentionally outside that candidate archive.
- VERIFIED: The final local build from candidate source commit `bdcc7d1`
  produced `benchlight.exe` (3,897,344 bytes, SHA-256
  `cdde0c8aa69ada25ddb1c4a1f855242ddb7f8986d349c7c70317168136ee7633`),
  `benchlight-desktop.exe` (12,919,296 bytes, SHA-256
  `f8315f233a5a5bbc1b567613be54433ff3a70737d276667ffa6e261461ba0172`),
  and unsigned `Benchlight_0.1.1_x64-setup.exe` (3,949,314 bytes,
  SHA-256 `d8994493a15b8e09301ce9001067796c1b64fddcaa5524ab94c3f4e579e776a2`).
  The candidate manifest itself has SHA-256
  `530b9da4a32c62789dcbde99d5595569acf9bdf0f6c8fcfe2e5fc1ed47683fa3`.
- VERIFIED: Current remote draft [PR #1](https://github.com/Rhythm-Sanghi/Benchlight/pull/1)
  remains documentation-only at head
  `4bea96c7b6ea83ce14ab47cd5ce8f22bea63a7ff` and base
  `b51c95b513ae9665bf5dfb0fea334bde4cf4f895`. Its 38 Rust / 20 frontend /
  80 contrast figures are labeled as historical 0.1.0 verification; current
  counts above should not replace them. The Tools provenance wording agrees
  that installation origin is inference, and the unsigned/offline/cleanup
  caveats remain qualified. `git merge-tree --write-tree` produced a tree
  without conflicts against source commit `bdcc7d1`; this does not replace
  review of the merged wording or CI.
- NOT VERIFIED: Hosted Windows CI has not run on these local commits. No
  0.1.1 Git tag, published asset or stable-release approval exists.

For owner integration: review this local branch and candidate hashes, push a
review branch when authorized, run hosted Windows checks and repeat an
installed-app smoke test from its exact commit. Merge the documentation PR
after verifying its historical 0.1.0 claims, or integrate its documentation
changes into the hardening PR, then recheck README/STATUS wording and links.
Only after a separate release decision should a new 0.1.1 tag and assets be
prepared. Preserve the existing v0.1.0 release and assets.
