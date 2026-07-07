# LXMF-rs v0.7.1 Release Notes

Date: 2026-07-07
Release ref: `v0.7.1`

This patch release follows the v0.7.0 SDK-first release train with focused
Reticulum interface additions and operator-facing documentation. It keeps the
public package and GitHub bundle versions aligned at `0.7.1`.

The maintained parity source of truth remains `docs/status/current-roadmap.md`,
with row-level detail in `docs/status/reticulum-parity-matrix.md` and
`docs/status/lxmf-parity-matrix.md`.

## Scope

- Adds a Columba-compatible `reticulum_ble` daemon interface scaffold for
  BLE-to-BLE Reticulum peer communication.
- Adds the transport-side RNode Bluetooth Classic/SPP KISS runtime scaffold.
- Adds maintained documentation for the new RNode SPP transport boundary.
- Adds a disabled-by-default `reticulumd` interface reference configuration
  covering supported interface kinds, settings, and compatibility aliases.

## Highlights Since v0.7.0

- `reticulumd` now accepts `reticulum_ble` plus the `AndroidBLE`,
  `AndroidBLEInterface`, and `BLEInterface` aliases, with Columba Protocol
  v2.2 UUID defaults and runtime status metadata for the scaffolded backend.
- The daemon-side Reticulum BLE protocol layer includes identity tracking,
  Columba-compatible fragment/reassembly logic, role-deduplication policy, and
  focused protocol tests.
- `reticulum-rs-transport` now exposes `rnode_spp`, a backend-neutral
  Bluetooth Classic/RFCOMM SPP runtime for RNode KISS byte streams.
- RNode SPP startup enforces the configured connect timeout so a stalled
  Bluetooth discovery or RFCOMM open returns a typed timeout instead of hanging
  the interface task.
- The new `docs/interfaces/rnode-spp.md` page documents the SPP runtime
  contract, Bluetooth boundary, KISS flow-control behavior, verification, and
  current daemon limitations.
- `crates/apps/reticulumd/examples/interfaces-reference.toml` provides a
  safe, disabled-by-default catalog of supported `reticulumd` interface types,
  parameters, and compatibility aliases.

## Current Version Train

GitHub release version: `v0.7.1`

Public package versions are aligned to `0.7.1` for the automated release
workflow:

- `lxmf`
- `reticulum-rs`
- `lxmf-reference`
- `lxmf-wire`
- `lxmf-sdk`
- `reticulum-rs-core`
- `reticulum-rs-transport`
- `reticulum-rs-rpc`
- `lxmf-embedded-mini`
- `rns-embedded-core`
- `rns-embedded-runtime`
- `rns-embedded-ffi`
- `rns-embedded-mininode`
- `lxmf-cli`
- `reticulumd`
- `rns-tools`

## Included GitHub Bundle Tools

- `lxmd`
- `lxmf`
- `lxmf-cli`
- `reticulumd`
- `lxm-interchange`
- `rnsd`
- `rnstatus-rs`
- `rnpath-rs`
- `rnodeconf-rs`
- `weaveconf-rs`
- `rnx`

## Validation Record

- Release metadata preparation updated the workspace `VERSION`, public package
  versions, path dependency versions, release notes, and current README release
  references for `v0.7.1`.
- Focused Reticulum BLE validation covers config normalization, Columba UUID
  defaults, fragment/reassembly behavior, role policy, peer identity tracking,
  and startup runtime status.
- Focused RNode SPP validation covers startup writes, stream-byte packet
  decoding, READY-driven flow-control flushing, and connect-timeout
  enforcement.
- Focused daemon config validation covers the maintained
  `interfaces-reference.toml` sample through the real `DaemonConfig` parser.

## Known Limits

- `reticulum_ble` currently exposes daemon protocol/status scaffolding; native
  OS dual-role BLE central/peripheral backends remain pending.
- `rnode_spp` currently exists as a transport-side runtime scaffold, not a
  `reticulumd` interface kind.
- Prepared-host hardware evidence remains required before making production
  claims for broader BLE, RNode BLE, RNode SPP, RNodeMulti, Weave, VR-N76, and
  serial-radio deployments.
- Full Python Reticulum/LXMF surface parity is not achieved.
