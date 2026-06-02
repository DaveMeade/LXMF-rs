# Current Roadmap Status

Last updated: 2026-06-02

This document is the current source of truth for repository-wide delivery
status. Update this file first when parity status, release confidence, or the
active execution order changes.

Related documents:

- Execution board: `docs/plans/2026-03-19-python-compatibility-execution-board.md`
- Numbered compatibility backlog: `docs/plans/2026-03-18-rust-python-compat-issue-list.md`
- LXMF parity snapshot: `docs/plans/lxmf-parity-matrix.md`
- Reticulum parity snapshot: `docs/plans/reticulum-parity-matrix.md`

## Current Summary

- Focused build, test, and clippy checks are green for the current daemon/CLI
  slices, and the strict architecture boundary check passes.
- `cargo run -p xtask -- architecture-checks` is green for both strict boundary
  checks and the module-size gate.
- Live GitHub metadata checked on 2026-06-02 shows the `Python reference
  interop` pull-request workflow green for the current `codex/kiss-codec-parity`
  branch and for the latest `main` run. That workflow is the current
  Rust/Python checkpoint against the pinned reference revisions below.
- The repository is not blocked by known broken parity gates. The main blockers
  are remaining behavioral parity gaps and keeping docs/current CI in sync.

## What Is True Now

### Landed Baseline

The following compatibility foundation work is on `main` and should be treated
as the current baseline:

- buffer writer parity (`#110`)
- buffer callback parity (`#111`)
- resource lifecycle truth and generic-resource handling (`#112`)
- daemon receipt semantics for resource-backed sends (`#113`)
- honor LXMF delivery modes in the `reticulumd` bridge (`#114`)
- path tag lifetime parity (`#115`)

This means older planning notes that say `reticulumd` ignores requested LXMF
delivery modes are stale. Delivery-mode handling is no longer an open baseline
gap, even though deeper propagation-router parity remains open.

### Still Open

- Rust/Python live interop is now represented in `.github/workflows/python-interop.yml`
  for pinned Reticulum/LXMF references. The high-signal local gates are
  `python_channel_interop`, `python_paper_interop`, and `python_compat_matrix`.
  `crates/apps/lxmf-cli/tests/python_lxmd_remote_relay.rs` is also included for
  cross-implementation LXMD relay paths.
  The pinned checkpoint currently reports Reticulum conformance
  `0319444b20e0815f26c6b9ceeba8fa44de037c9b`, Python Reticulum
  `15320e4d2cfabb143c1db20ca887e275fd521585`, and Python LXMF
  `727830cefda83d9c6e3982b48675425f3f988f9c`.
- `reticulumd` now defaults to local Unix RPC, treats TCP as opt-in, rejects
  unauthenticated remote TCP binds, handles graceful listener shutdown, and
  documents service-manager deployment in
  `docs/runbooks/reticulumd-operational-deployment.md`.
- Propagation-router behavior is still partial relative to Python LXMF.
- Stamp, ticket, and propagation-stamp semantics are still partial.
- Peer/router/runtime parity remains partial.
- Reticulum interface breadth is still narrower than the Python reference.
- Parser-only `rns-tools` utility placeholders have been retired from the
  release surface. Utility parity remains incomplete until real equivalents for
  the retired Python-style commands are implemented.
- Migration-era legacy crates and router/runtime stubs have been removed from
  the repository surface; active code must stay in the workspace crates listed
  in `Cargo.toml`.
- The module-size gate is green after splitting `lxmd` launch/config helpers,
  `rnx` TCP/BLE/scenario helpers, RPC event/helper/status tests, LXMF wire tests,
  SDK backend/client/app-control/node tests, and transport resource/interface/
  path/tunnel helpers out of oversized modules.

## Active Execution Order

1. Keep architecture and boundary gates trustworthy.
2. Keep the pinned Rust/Python interop workflow green and extend it when new
   compatibility rows become supported.
3. Align `README.md`, `docs/runbooks/release-readiness.md`, and GitHub CI with
   the same definition of "green".

## Update Rules

- Update this file in the same PR that changes project-wide status claims.
- When a historical planning note disagrees with this file, treat the planning
  note as stale until it is refreshed.
- Do not mark parity items as complete here unless the behavior is implemented
  in active workspace code and backed by non-ignored evidence.
