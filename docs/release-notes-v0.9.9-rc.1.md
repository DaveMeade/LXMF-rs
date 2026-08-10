# LXMF-rs v0.9.9-rc.1

This is the RNS 1.4.2 software-parity prerelease for the Rust Reticulum and
LXMF implementations. The workspace and publishable crate version is `0.9.9`;
the `-rc.1` suffix is carried by the GitHub tag and release. Final stable
`v0.9.9` publication is outside this candidate.

## RNS 1.4.2 parity

The candidate is compared with these pinned Python references:

- Reticulum: `1.4.2`, commit
  `b48b96e61676504e0a4e527b33b9a0b4495c6872`.
- LXMF: commit `727830cefda83d9c6e3982b48675425f3f988f9c`.

The strict generated callable inventory is:

| Total | Applicable | Complete | Partial | Unmapped | Not applicable |
| ---: | ---: | ---: | ---: | ---: | ---: |
| 1,811 | 1,810 | 1,810 | 0 | 0 | 1 |

The one not-applicable entry is the provenance-backed absent `CRNS` package.
The seven tracked LXMF module rows are complete for their named software
scenarios. The RNS implementation closures include interface gravity and
gravity-aware route selection, dynamic path rebalancing, boundary path-request
behavior, destination request-size limits, request/response maximum-size
enforcement, the Resource collision-guard serving window, blocked-IP status,
typed runtime/lifecycle and legacy RPC-return behavior, `rnstatus`, and the
full transport-neutral `rngit` repository-service surface.

Resource behavior also includes negotiated MTU handling, Python-shaped adaptive
request/window scheduling, large-resource interoperability fixes, outbound
compression, lazy split-resource construction, and explicit cancellation and
failure reporting.

The inventory is an implementation statement, not a claim of universal live
network substitutability. The exact validation scope and any scenario-level
exceptions are recorded in
[`docs/status/v0.9.9-release-candidate.md`](status/v0.9.9-release-candidate.md).

## Release infrastructure

The tag-triggered release workflow is intended to produce:

- static Linux musl archives for x86_64, aarch64, and armv7;
- Windows x64 ZIP and MSI artifacts, with optional Azure Trusted Signing;
- a macOS universal archive, with optional codesigning;
- architecture-specific Debian and RPM packages;
- a multi-architecture OCI image in GHCR;
- CycloneDX SBOMs, SHA-256 checksums, keyless checksum signing, and GitHub
  build-provenance attestations.

The RC does not publish crates to crates.io, move the stable OCI `latest` tag,
or update the Homebrew stable tap. Missing optional signing credentials are
recorded as unsigned rather than treated as successful signatures.

## Performance dashboard

The tag-triggered performance workflow publishes
`lxmf-rs-performance.json` and `lxmf-rs-performance.html` when its comparison
completes. This document does not fabricate benchmark numbers: unavailable
measurements are represented as `N/A`, and the current repository dashboard
remains the historical baseline until the RC workflow emits its dataset.
The workflow result and asset checksums belong in the RC evidence ledger.

## Known evidence boundaries

The following are deliberately separate evidence tracks and are not counted as
implementation partiality: physical RNode and RNodeMulti devices, Weave,
VR-N76, BLE, serial-radio hardware, public I2P, public Reticulum networks,
Sideband, MeshChatX, Columba, and other third-party clients.

The repository-owned pinned-reference cases in `.github/workflows/verify.yml`
and `tests/hil/cases/interop.toml` also retain scenario-level boundaries where
the controller does not currently execute the reference-only boundary case or
the Python-to-Rust remote-relay case. Those boundaries must not be described as
failed implementation rows or as universal interop proof.

## Candidate status

The release candidate is ready for consideration only after the local gates,
tag-triggered hosted workflows, release artifacts, signatures/attestations,
OCI state, and performance dashboard have been verified. The evidence ledger
is the authoritative record of that decision and must name every failed,
skipped, unavailable, or deferred check.
