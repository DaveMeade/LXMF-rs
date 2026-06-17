# LXMF-rs v0.4.1 Release Notes

Date: 2026-06-17
Release ref: `v0.4.1`

This is a maintenance release for the `0.4.x` line. It keeps the `v0.4.0`
ZeroMQ SDK and daemon bundle scope, and adds two compatibility fixes needed by
operators and downstream Rust consumers.

This release is not a claim of complete drop-in Python Reticulum/LXMF
replacement parity. The maintained parity source of truth remains
`docs/status/current-roadmap.md`.

## Changes Since v0.4.0

- Replaced the remaining `serde_cbor` use in RPC capability parsing with the
  supported MessagePack/JSON parsing path.
- Accepted `tcp://host:port` endpoint strings in the HTTP RPC client endpoint
  normalizer, fixing event stream and RPC polling failures that previously
  attempted to resolve `tcp:` as the network address.

## Current Version Train

GitHub release version: `v0.4.1`

Crate/package versions intentionally remain per the publish plan rather than one
blanket workspace version:

- `lxmf`: `0.3.0`
- `reticulum-rs-rpc`: `0.3.0`
- `lxmf-sdk`: `0.2.1`
- `lxmf-wire`: `0.2.0`
- `reticulum-rs-core`: `0.2.0`
- `reticulum-rs-transport`: `0.2.0`
- app/tool crates remain unpublished and are distributed through GitHub bundles

## Validation Record

- Main included fixes:
  - https://github.com/FreeTAKTeam/LXMF-rs/pull/349
  - https://github.com/FreeTAKTeam/LXMF-rs/pull/350
- Focused local checks before release publication included:
  - `cargo test -p lxmf-sdk rpc_endpoint_accepts_tcp_scheme_for_http_rpc_compatibility`
  - `cargo test -p lxmf-sdk backend::rpc::transport::tests`
  - `cargo fmt --all -- --check`
  - `git diff --check`

## Known Limits

- Propagation interoperability and operational substitutability are still marked
  partial in `docs/status/current-roadmap.md`.
- Full Python surface parity is not achieved.
- Application-level REM/RCH schemas are out of scope for this release; LXMF-rs
  provides the basic LXMF fields and typed SDK transport behavior those clients
  need.
- External-client compatibility claims for Sideband, MeshChatX, and Columba
  require separate external-client interop gate evidence.
- Hardware and prepared-host evidence for all interface paths remains broader
  than the automated CI evidence.
