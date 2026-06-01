# Windows Rust Build Stability Runbook

## 1. Scope

This runbook is for Windows-local Rust validation in this repository.
It is environment-focused.
It does not change trading logic.
It does not change Discord bridge logic.
It does not change the AlertService sidecar chain.

The goal is to make these commands stable and repeatable on Windows:

- `cargo fmt --check`
- `cargo check`
- `cargo clippy --all-targets -- -D warnings`
- `cargo test`

## 2. Failure Signatures This Runbook Covers

This runbook is meant for these Windows-local failure patterns:

- `link.exe` pagefile / virtual memory failure
- `os error 1455`
- `rustc` panic during heavy builds
- `STATUS_STACK_BUFFER_OVERRUN`
- `can't find crate for std`
- `only metadata stub found for rlib dependency`

These errors can look like repository breakage even when the code is fine.
On this machine, they were primarily environment and build-pressure symptoms.

## 3. Baseline Environment Checks

Run these first from the repository root.

```powershell
rustc -Vv
cargo -V
rustup show
rustup component list --installed
where.exe rustc
where.exe cargo
cmd /c cl
cmd /c link
```

If `cl` and `link` are not found, do not trust later MSVC-link failures.
This shell does not always inherit the Visual Studio Build Tools environment.

## 4. Required Windows Tooling

This repository uses the MSVC toolchain:

- target: `x86_64-pc-windows-msvc`
- Rust toolchain: `stable-x86_64-pc-windows-msvc`
- Visual Studio Build Tools with `cl.exe` and `link.exe`

On this machine, the Build Tools existed here:

```text
C:\Program Files (x86)\Microsoft Visual Studio\18\BuildTools\Common7\Tools\VsDevCmd.bat
```

Do not run the full repository validation from a plain shell if `cl` and `link` are missing.
Use `VsDevCmd.bat` first.

## 5. Stable Command Environment

Preferred approach:

1. Open a Developer Command Prompt for Visual Studio.
2. `cd` into the repository.
3. Run the validation commands with `-j 1` for heavy steps.

If you are already inside PowerShell, start the commands through `VsDevCmd.bat`:

```powershell
cmd /c "\"C:\Program Files (x86)\Microsoft Visual Studio\18\BuildTools\Common7\Tools\VsDevCmd.bat\" -no_logo -arch=x64 -host_arch=x64 && cargo check -j 1"
```

Repeat the same pattern for `clippy` and `test`.

## 6. Toolchain Repair

Before retrying a broken environment, refresh the toolchain:

```powershell
rustup update
rustup component add clippy rustfmt
```

If these fail, stop and fix the toolchain before diagnosing repository code.

## 7. Cache Repair

Clear build artifacts before retrying a damaged or suspicious state:

```powershell
cargo clean
```

If there are custom temporary target directories from prior work, only remove them if they are clearly stale and not needed for active work.
Do not delete unrelated repository data.

## 8. Stable Validation Sequence

Run the repository in this order:

```powershell
cargo fmt --check
cargo check -j 1
cargo clippy -j 1 --all-targets -- -D warnings
cargo test -j 1
```

Why `-j 1`:

- reduces peak linker memory pressure
- reduces concurrent metadata corruption symptoms
- avoids turning a real test run into a Windows resource incident

## 9. Failure Classification

### 9.1 Environment Failure

Treat these as environment failures first:

- `os error 1455`
- `link.exe` pagefile failures
- `can't find crate for std`
- `only metadata stub found for rlib dependency`
- `STATUS_STACK_BUFFER_OVERRUN`
- `rustc` panic immediately after linker or toolchain instability

First response:

1. confirm `cl` and `link` exist in the active shell
2. run `rustup update`
3. run `cargo clean`
4. rerun with `-j 1`
5. check pagefile size

### 9.2 Compile Failure

Treat this as repository compile failure when:

- the toolchain is healthy
- `cl` and `link` are available
- `cargo check -j 1` or `cargo clippy -j 1 --all-targets -- -D warnings` fails with code diagnostics

Example:

- missing struct field in a test fixture
- a clippy lint in repository code or tests

### 9.3 Test Failure

Treat this as a real repository test failure when:

- build completes
- test binaries run
- a test assertion fails

Example from this machine after environment stabilization:

```text
test replay_heatmap_dashboard_template_locks_view_only_export_surface ... FAILED
web/index.html is missing token:
Read-only operator heatmap for multi-signal replay comparison, JSON summary, and Markdown summary. No live trading.
```

That is a repository failure, not an environment failure.

Minimal repro:

```powershell
cargo test -j 1 --test replay_heatmap_dashboard_tests
```

## 10. Pagefile Guidance for `os error 1455`

Observed on this machine:

- physical memory was sufficient
- pagefile allocation was about `9216 MB`
- heavy Rust/MSVC link steps still hit `os error 1455`

Interpretation:

- `os error 1455` is strongly consistent with Windows virtual memory / pagefile pressure
- large Rust test and lint builds can exhaust commit even when RAM is not full

Manual pagefile increase steps:

1. Open `sysdm.cpl`
2. Go to `Advanced`
3. Under `Performance`, click `Settings`
4. Go to `Advanced`
5. Under `Virtual memory`, click `Change`
6. Uncheck `Automatically manage paging file size for all drives`
7. Select the system drive
8. Choose `Custom size`
9. Set a larger size

Practical starting point:

- Initial size: `16384 MB`
- Maximum size: `32768 MB`

If builds are still unstable, raise further based on free disk space.
Reboot after changing the pagefile.

## 11. Rustc Panic Triage

If `rustc` still panics after:

- verified MSVC shell
- `rustup update`
- `cargo clean`
- `-j 1`

then classify the panic with one of these likely causes:

1. cached build artifact corruption
2. toolchain corruption
3. Windows memory / pagefile exhaustion
4. actual compiler bug

Next actions:

```powershell
rustc -Vv
rustup show
cargo clean
cargo check -j 1
cargo test -j 1 --test <failing-test>
set RUST_BACKTRACE=1
```

If the panic still reproduces in a small command after a clean environment, capture the exact command and stack output before changing repository code.

## 12. Current Machine Findings

During the latest run, the environment stabilized after these actions:

- updated Rust from `1.95.0` to `1.96.0`
- confirmed `clippy` and `rustfmt` installed
- confirmed Visual Studio Build Tools existed locally
- switched to a `VsDevCmd.bat`-initialized shell
- ran heavy commands with `-j 1` or `CARGO_BUILD_JOBS=1`
- cleared build cache with `cargo clean`
- stopped the local `btc-toxic-flow-monitor-rs.exe` process before cleaning because `target\debug\deps\btc_toxic_flow_monitor_rs.exe` was file-locked
- corrected the `cmd.exe` env syntax from `set CARGO_BUILD_JOBS=1 && ...` to `set "CARGO_BUILD_JOBS=1" && ...` to avoid the malformed value `1 `

Results:

- `cargo fmt --check` passed
- `cargo check -j 1` passed
- `cargo clippy -j 1 --all-targets -- -D warnings` passed
- `cargo test --no-run` passed
- `cargo test -j 1` passed

The only repository-side issue exposed during this run was a real static test mismatch:

- `tests/replay_heatmap_dashboard_tests.rs`
- expected fixture token did not match the current `web/index.html` text because the page now includes `whale flow overlay dimensions`

That issue was resolved with a minimal fixture update in:

- `tests/fixtures/replay_heatmap_ui_spec.json`

This confirms the earlier `1455` / linker / `std` / metadata-stub symptoms were environment or shell-shape issues, not a current repository-wide blocker.

## 13. Operator Checklist

Use this checklist before declaring the repository broken:

- [ ] `rustup update` completed
- [ ] `clippy` and `rustfmt` are installed
- [ ] `cl` is available in the active shell
- [ ] `link` is available in the active shell
- [ ] `cargo clean` was run
- [ ] heavy commands were retried with `-j 1`
- [ ] pagefile size was checked if `1455` appeared
- [ ] environment failures were separated from test assertion failures

## 14. Final Verification Result

Final status for this run:

- `FULL_PASS`

Verification summary:

- `cargo fmt --check` -> `PASS`
- `cargo check` -> `PASS`
- `cargo clippy --all-targets -- -D warnings` -> `PASS`
- `cargo test --no-run` -> `PASS`
- `cargo test` -> `PASS`

Classification summary:

- environment failure:
  - reproduced historically on this machine, but not on the final stabilized run
- compile failure:
  - none on the final stabilized run
- lint failure:
  - none on the final stabilized run
- test failure:
  - one real fixture mismatch was found and fixed, then the full suite passed

Recommended stable command sequence on this machine:

```powershell
cargo fmt --check

$cmd = 'call "C:\Program Files (x86)\Microsoft Visual Studio\18\BuildTools\Common7\Tools\VsDevCmd.bat" -no_logo -arch=x64 -host_arch=x64 && set "CARGO_BUILD_JOBS=1" && cargo check'
cmd /c $cmd

$cmd = 'call "C:\Program Files (x86)\Microsoft Visual Studio\18\BuildTools\Common7\Tools\VsDevCmd.bat" -no_logo -arch=x64 -host_arch=x64 && set "CARGO_BUILD_JOBS=1" && cargo clippy --all-targets -- -D warnings'
cmd /c $cmd

$cmd = 'call "C:\Program Files (x86)\Microsoft Visual Studio\18\BuildTools\Common7\Tools\VsDevCmd.bat" -no_logo -arch=x64 -host_arch=x64 && set "CARGO_BUILD_JOBS=1" && cargo test --no-run'
cmd /c $cmd

$cmd = 'call "C:\Program Files (x86)\Microsoft Visual Studio\18\BuildTools\Common7\Tools\VsDevCmd.bat" -no_logo -arch=x64 -host_arch=x64 && set "CARGO_BUILD_JOBS=1" && cargo test'
cmd /c $cmd
```
