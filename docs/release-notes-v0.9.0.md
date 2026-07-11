# LXMF-rs v0.9.0

`v0.9.0` is the full software-implementation parity milestone for the pinned
Python Reticulum and LXMF references.

## Highlights

- The generated public-surface inventory contains 1,665 entries: 1,664 are
  implementation-complete, none are partial, and the single not-applicable
  provenance entry records that `CRNS` is absent from the pinned references.
- Reticulum runtime, transport, discovery, path/rate management, packet signal
  caches, segmented resources, persistence, and shared-instance management are
  available through typed Rust and additive daemon RPC surfaces.
- TCP/Backbone, Local, Pipe, UDP, Serial, KISS/AX.25, AutoInterface, I2P,
  Meshtastic, RNode, RNodeMulti, Weave, native BLE, Android BLE/serial, and
  VR-N76 are implementation-complete with deterministic software evidence.
- `LXMRouter` statistics and storage policy are typed across `SdkBackend`, RPC,
  ZeroMQ, and `lxmf-runtime::InProcessBackend`; router, handler, ticket, stamp,
  queue, peer, delivery, and propagation callables are fully mapped.
- Canonical utility names include `rnx`, `rnsd`, `rnstatus`, `rnpath`,
  `rnodeconf`, `rncp`, `rnid`, `rnir`, `rnpkg`, `rnprobe`, `rnsh`, `rngit`, and
  `lxmd`. Existing `*-rs` aliases remain available.
- Release and documentation CI enforce zero partial or stale manifest entries.

## Compatibility

The `0.8.x` wire formats and public Rust items remain supported. New runtime,
router, segmented-resource, discovery, and utility behavior is additive.

## Evidence boundary

This release proves software implementation against pinned references and
deterministic simulation. It does **not** claim physical RNode/RNodeMulti,
Weave, VR-N76, BLE, LoRa/radio, real serial-device, public-I2P, public-network,
or third-party-client validation. Hardware-capable rows retain the explicit
`hardware-unverified` evidence label.

## Validation

The release SHA must pass formatting, all-feature Clippy with warnings denied,
workspace tests, dependency boundaries, architecture and module-size checks,
security/API/interop drift checks, reproducible builds, package dry-runs, the
strict pinned-Python inventory, and `cargo xtask release-check`.
