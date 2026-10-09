# Workshop Grid UI review

Benchlight's existing application now uses a compact desktop workbench layout.
The redesign preserves observed data, explicit scans and cleanup safeguards.

## Design system and screens

Warm neutral light and true dark themes use semantic surface, text, selection,
status and brass accent tokens. Ruled rows, native tables and monospace technical
values replace ornamental cards. The fixed sidebar, compact page header and
live status footer frame an independently scrolling workspace. Overview groups
folders and scan measurements; Space combines candidates with cleanup plans;
Projects and Tools use sortable lists with evidence inspectors; Changes uses
explicit baseline comparisons; Settings groups labels and actions in sections.
Unknown, incomplete and protected states remain visible and use plain language.

SectionHeading, StateLabel, SortHeader and SplitPane standardize headings,
state treatment, sorting, keyboard resizing and list/detail layouts. Columns
resize with Left/Right or pointer drag. Inspector widths range from 280 to 560 px.
Below 1050 effective pixels inspectors become independently scrolling drawers.
Escape dismisses inspectors and restores row focus. Non-Escape keys no longer
close project or storage inspection.

## Width persistence and accessibility follow-up

Inspector and column widths persist independently for Projects, Space and Tools
in versioned local presentation preferences. Preferences contain only UI keys
and numbers. Invalid values are clamped or reset; unavailable storage falls
back to usable session state. Tests cover remounts, isolation, malformed values
and storage failures.

Row buttons expose expanded state and their inspector relationship. Opening a
narrow drawer focuses its close control; wide-screen row focus stays intact.
Form fields retain associated labels, navigation and tables retain native
semantics, and status/error messages retain live regions. The shell retains
visible keyboard focus, a skip link, reduced-motion and forced-color support.

Light-theme muted, secondary and warning text tokens were corrected. Forty
foreground/background pairs per theme, including selected rows, meet 4.5:1
(minimum light 4.59, dark 4.62). The check runs with the frontend test command.

Below 600 effective pixels settings sections stack, forms and measurements wrap,
and sidebar navigation scrolls at short heights. Dense tables intentionally
retain horizontal scrolling. The supported minimum native window is 800 px.
WebView zoom hotkeys are enabled: Ctrl+plus, Ctrl+minus and Ctrl0.

## Preserved behavior

No Rust business logic, IPC API, schema, scanner, classification rule, tool probe,
snapshot capture, export implementation or cleanup validation was changed.
Canonical root/path payloads are preserved; displayPath is presentation only.
Scans remain explicit. Cleanup still requires an eligible complete candidate,
revalidation, the exact plan phrase and backend validation before Shell recycling.
No permanent deletion, automatic scripts, accounts, telemetry or runtime assets
were introduced. Production UI uses observations from the existing core.

## Verification

Frontend formatting, ESLint, TypeScript, Vite production assets and all 20 tests
pass. The test command also checks 80 text contrast pairs. Tests cover root/error
flows, keyboard resizing and Escape, protected/incomplete artifacts, exact
cleanup confirmation, Unknown comparisons, provenance, context menu navigation,
settings, scan errors, width persistence and narrow-drawer focus.

The original redesign verification passed Rust formatting, Clippy with warnings
denied and 36 Rust tests; three internal subprocess fixtures are invoked by
parent tests. Rust behavior was unchanged in this follow-up, so this remains the
previous result rather than a claimed new run.

The original native review covered all six pages, both themes, an 800 px window,
pointer resizing, Escape focus return, an explicit repository scan (14 projects,
zero errors) and comparison with an existing baseline. A missing favicon request
was corrected by reusing the application icon; its local endpoint returned 200.
A separate 36-byte fixture checked persisted cleanup preview, revalidation,
recovery instructions and disabled exact-phrase execution. No project data was
recycled. Stale dependency evidence was correctly rejected by the core.

The follow-up rebuilt production app was checked through five zoom increments
to 200%. Overview and Settings retain reachable controls, wrapping and scrolling.
Projects exposes a named inspector region; Tab proceeds from its close control
to Copy path and scrolls the action into view. Escape restores visible row focus.
Ctrl0 restores normal zoom. Windows UI Automation exposes navigation, headings,
tables, resize controls, associated input labels, disabled states and live status.
The screenshot projects-zoom-200.jpg records the enlarged drawer.

The NSIS bundle, matching source archive and four release hashes are refreshed.
Installer lifecycle was previously tested on 2026-10-07; this UI revision does
not claim repeated install/reinstall/uninstall testing. See release.md and the
screenshots provenance record.

## Remaining compatibility verification

A full spoken-output test with an actual screen reader and global Windows text
scaling remain separate manual compatibility checks. No screen-reader
certification is claimed. Horizontal scrolling for dense tables is intentional.
