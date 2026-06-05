# Windows Rust Build Troubleshooting

Current local Windows `cargo check` is blocked before project compilation by:

- missing MSVC linker `link.exe`
- incomplete Cargo registry cache for `cc-1.2.62` and other crates

This is not caused by the WebSocket, Docker, or Dashboard changes. Linux CI should be used as the authoritative compile/test gate until the local toolchain is repaired.

## Option A: Install MSVC Build Tools

Install Visual Studio Build Tools and select C++ build tools.

Then open Developer PowerShell and run:

```powershell
cargo clean
cargo check --all-targets
```

Confirm `link.exe` is available:

```powershell
where.exe link
```

## Option B: Use WSL2 or Linux

Use this path for CI parity:

```bash
cargo check --all-targets
cargo test
```

## Option C: Clear Corrupted Cargo Cache

PowerShell:

```powershell
cargo clean
Remove-Item -Recurse -Force "$env:USERPROFILE\.cargo\registry\src\*\cc-1.2.62"
Remove-Item -Recurse -Force "$env:USERPROFILE\.cargo\registry\src\*\rustversion-1.0.22"
cargo fetch
cargo check --all-targets
```

If other crate source directories report missing files, remove only the broken crate directories and re-fetch.
