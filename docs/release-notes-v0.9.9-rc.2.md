# LXMF-rs v0.9.9-rc.2

> Superseded before publication by `v0.9.9-rc.3` because its Intel macOS job
> used the retired `macos-13` runner label.

This candidate supersedes `v0.9.9-rc.1` for the same RNS 1.4.2 software
parity scope. The first tag was not publishable because its Linux x86_64 musl
release job let `libdbus-sys` probe the host glibc installation. This candidate
adds the vendored Linux dbus feature to the BLE-consuming application packages
so native musl and cross-musl release builds use a target-compatible dbus
library.

See [`docs/release-notes-v0.9.9-rc.1.md`](release-notes-v0.9.9-rc.1.md) for
the complete parity scope, pinned references, artifact policy, and deferred
evidence boundaries. The workspace and publishable crate version remains
`0.9.9`; the prerelease suffix is carried by this GitHub tag.
