# LXMF-rs v0.6.0 Release Notes

Date: 2026-06-30
Release ref: `v0.6.0`

This is the RNS/LXMF software parity milestone release. It promotes the large
parity advancement from PR #393 into the public release train while keeping the
project's scope boundary explicit: this is not a claim of complete drop-in
Python Reticulum/LXMF parity, and hardware/HIL, broad production soak, and
external-client compatibility claims remain separate evidence tracks.

The maintained parity source of truth remains `docs/status/current-roadmap.md`,
with row-level detail in `docs/status/reticulum-parity-matrix.md` and
`docs/status/lxmf-parity-matrix.md`.

## Scope

- Substantially expanded Reticulum transport, announce/path, resolver, runtime,
  interface, and utility software parity evidence.
- Broader LXMF propagation, direct-delivery, receipt, drop, duplicate,
  signature, router, handler, and SDK event observability.
- Pinned Python-reference and local software evidence gates that make the
  remaining parity gaps more explicit and more actionable.
- Continued crate and GitHub bundle release-train alignment for `0.6.0`.

## Highlights Since v0.5.2

- Added a maintained software parity ledger that maps partial Reticulum and LXMF
  rows into implementation-ready work packets while explicitly deferring
  hardware, HIL, external-client, and broad production-soak evidence.
- Expanded Reticulum path/announce policy coverage, including scoped path
  requests, known-path response ordering, roaming response behavior,
  unknown-announce ingress handling, rebroadcast policy, routed-link
  rediscovery, and intermediate-hop `LINKREQUEST` MTU signalling evidence.
- Hardened resolver and path-cache restore behavior, including Python-format
  path-table restore visibility through daemon RPC, cached announce identity
  persistence, malformed/missing/mismatched cache handling, tunnel restore
  behavior, and runtime restore status reporting.
- Broadened daemon/runtime/interface coverage with hot-applied TCP/UDP
  mutation policy, runtime status snapshots, AutoInterface software evidence,
  LocalInterface/shared-instance improvements, and richer operator-facing
  status data.
- Added `rnpath-rs` path discovery and scoped-refresh support alongside
  `rnstatus-rs`, `rnsd`, and `rnx` parity slices.
- Expanded LXMF propagation and handler observability with duplicate/drop
  events, direct packet/resource SDK inbound events, receipt event metadata,
  signature-state metadata, delivery announce wakeups, identity-miss deferral,
  and paper encode/decode access through the CLI/SDK path.
- Added and refreshed Python compatibility harness cases, local evidence
  dispatchers, parity matrices, and interop artifact baselines.

## Current Version Train

GitHub release version: `v0.6.0`

Public package versions are aligned to `0.6.0` for the automated release
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

- PR #393 merged into `main` at `225b4c43fb93dbaafbf1454135be181449c7e1af`
  after passing the hosted `quality`, `build-matrix (stable)`,
  `test-nextest-unit`, `peer-lifecycle-essential`, `contracts`, and pinned
  Python reference interop jobs.
- Release preparation passed `cargo xtask release-check` and
  `cargo run -p rns-tools --bin rnx -- e2e --timeout-secs 20` locally before
  tagging `v0.6.0`; GitHub release workflows publish bundles and aligned crates
  from the tag.

## Known Limits

- Full Python Reticulum/LXMF surface parity is not achieved.
- Remaining software gaps are tracked in the Reticulum and LXMF parity
  matrices, especially broader runtime mutation behavior, announce/path
  edge-policy evidence, discovery/resolver breadth, utility breadth, and some
  LXMF router/handler side effects.
- Hardware/HIL evidence remains required for broader RNode, RNodeMulti, Weave,
  VR-N76, BLE, serial-radio, and prepared-host interface claims.
- External-client compatibility claims for REM, RCH, Sideband, MeshChatX,
  Columba, or other clients require the corresponding external-client interop
  gates for that deployment.
