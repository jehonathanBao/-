# Cargo Cache Recovery

This note covers the local recovery path for a corrupted Cargo registry cache, especially the observed `tracing-subscriber-0.3.23` cache where `src/filter/env.rs` is missing.

## Symptoms

`cargo check`, `cargo build`, or `cargo test` fails before compiling project code with an error from the local Cargo registry cache, for example a missing file under `tracing-subscriber-0.3.23`.

## Preferred Recovery

Run these from the repository root:

```powershell
cargo clean
cargo update -p tracing-subscriber
cargo build
```

Then verify:

```powershell
cargo check
cargo test -j 1
```

## Temporary Cargo Home

If the global Cargo cache is still corrupted, use an isolated temporary Cargo home so the project can fetch a clean registry copy without mutating the broken global cache:

```powershell
$env:CARGO_HOME = "D:\Temp\cargo-home-toxic-monitor"
cargo build
cargo check
cargo test -j 1
```

After the build is healthy, either keep using that temporary Cargo home for this workspace or repair the global cache separately.

## Lockfile Check

Inspect `Cargo.lock` and confirm whether `tracing-subscriber` is pinned to an unexpected version:

```powershell
Select-String -Path Cargo.lock -Pattern 'name = "tracing-subscriber"','version = '
```

Do not downgrade dependencies only to bypass a local cache issue. Prefer cache repair, `cargo update -p tracing-subscriber`, or an isolated `CARGO_HOME` first, because arbitrary dependency downgrades can introduce production runtime risk.

## Acceptance

The local toolchain is considered recovered when these pass:

```powershell
cargo check
cargo build
cargo test -j 1
```

If tests still fail after the cache is repaired, list the remaining failures separately and verify they are not introduced by Contract Whale Monitor changes.
