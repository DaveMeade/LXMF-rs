# LXMF-rs v0.9.5

v0.9.5 completes the software-controlled ZeroMQ SDK access surface and adds
reproducible performance publication against pinned Python Reticulum and LXMF.

## Highlights

- Canonical single-endpoint ZeroMQ ROUTER/DEALER transport with concurrent
  clients, request correlation, timeout handling, loopback defaults, and
  fail-closed token authentication for remote endpoints.
- Native ZeroMQ async operations and cursor-replaying event streams with
  bounded backpressure, reconnect, and deduplication.
- Complete existing `SdkBackend` topic, telemetry, attachment, marker,
  remote-command, voice-signaling, and manual-tick coverage over ZeroMQ.
- Capability-gated RNS runtime, transport, interface, and data-plane controls,
  plus LXMF router and propagation extension traits shared by HTTP/Unix and
  ZeroMQ transports.
- Additive SDK contract v2.6 in schema namespace v2 and protocol version 2;
  v2.5 requests remain compatible.
- Generated SDK-access inventory covering all 1,665 pinned-Python entries and
  every registered daemon operation.
- Five-node mesh discovery now re-announces during convergence and is required
  to establish full-mesh visibility before delivery.
- Pinned, interleaved Rust/Python benchmark reporting with five timing runs,
  three isolated resource runs, variability checks, committed JSON, and
  generated README/performance documentation.

## v1.0 boundary

Physical RNode/RNodeMulti, Weave, VR-N76, BLE/serial/radio validation, public
I2P/network soak, third-party client validation, manual mobile/operator flows,
and interactive signing ceremonies remain explicit v1.0 targets. They are
hardware-unverified or human-validation evidence, not v0.9.5 software blockers.
