#!/usr/bin/env python3
import argparse
import atexit
import json
import signal
import sys
import threading
import time
from pathlib import Path


class _EventSink:
    def __init__(self):
        self._lock = threading.Lock()
        self._events = []

    def onEvent(self, payload):
        with self._lock:
            self._events.append(payload)

    def drain(self):
        with self._lock:
            drained = list(self._events)
            self._events.clear()
        return drained


class _NoopSink:
    def onEvent(self, _payload):
        return None


class _ColumbaEventBridgeRuntime:
    def __init__(self, columba_root: Path, storage_dir: Path):
        self.columba_root = columba_root
        self.storage_dir = storage_dir
        self.rns_dir = storage_dir / "reticulum"
        self.lxmf_dir = storage_dir / "lxmf"
        self.reticulum = None
        self.router = None
        self.identity = None
        self.delivery_destination = None
        self.event_bridge = None
        self.RNS = None
        self.LXMF = None
        self.delivery_sink = _EventSink()

    def _load_modules(self):
        python_dir = self.columba_root / "rns-backend-py" / "src" / "main" / "python"
        if not (python_dir / "event_bridge.py").is_file():
            raise RuntimeError(
                "Columba checkout does not contain rns-backend-py/src/main/python/event_bridge.py"
            )
        sys.path.insert(0, str(python_dir))
        try:
            import event_bridge  # type: ignore
            import LXMF  # type: ignore
            import RNS  # type: ignore
        except ModuleNotFoundError as exc:
            raise RuntimeError(
                "Columba Python backend requires RNS/LXMF modules. "
                "Run with COLUMBA_PYTHON pointing at a Python environment that "
                "has Columba's pinned Reticulum and LXMF dependencies installed."
            ) from exc

        self.event_bridge = event_bridge
        self.RNS = RNS
        self.LXMF = LXMF

    def _write_rns_config(self, config):
        interfaces = config.get("enabledInterfaces") or []
        tcp_clients = [item for item in interfaces if item.get("type") == "TCPClient"]
        if not tcp_clients:
            raise RuntimeError("Columba interop config requires one TCPClient interface")
        interface = tcp_clients[0]
        host = interface.get("target_host")
        port = interface.get("target_port")
        if not host or port is None:
            raise RuntimeError("Columba TCPClient interface requires target_host and target_port")

        self.rns_dir.mkdir(parents=True, exist_ok=True)
        loglevel = _log_level_number(config.get("logLevel", "INFO"))
        config_text = f"""[reticulum]
  enable_transport = no
  share_instance = no
  panic_on_interface_error = no

[logging]
  loglevel = {loglevel}

[interfaces]
  [[LXMF RS]]
    type = TCPClientInterface
    enabled = yes
    target_host = {host}
    target_port = {int(port)}
"""
        (self.rns_dir / "config").write_text(config_text, encoding="utf-8")

    def initialize(self, config_json):
        config = json.loads(config_json)
        self.storage_dir.mkdir(parents=True, exist_ok=True)
        self.lxmf_dir.mkdir(parents=True, exist_ok=True)
        self._load_modules()
        self._write_rns_config(config)

        self.event_bridge.apply_android_env_patches()
        self.reticulum = self.RNS.Reticulum(
            configdir=str(self.rns_dir),
            loglevel=_log_level_number(config.get("logLevel", "INFO")),
        )
        self.identity = self.RNS.Identity()
        self.router = self.LXMF.LXMRouter(
            identity=self.identity,
            storagepath=str(self.lxmf_dir),
            enforce_stamps=False,
        )
        self.delivery_destination = self.router.register_delivery_identity(
            self.identity,
            display_name=config.get("display_name") or "Columba Interop",
        )
        self.event_bridge.register_callbacks(
            self.RNS.Transport,
            self.router,
            _NoopSink(),
            _NoopSink(),
            _NoopSink(),
            self.delivery_sink,
            _NoopSink(),
        )
        self.router.announce(self.delivery_destination.hash)
        return {"success": True}

    def get_lxmf_destination(self):
        if self.delivery_destination is None:
            return {"error": "delivery destination not ready"}
        return {"hex_hash": self.delivery_destination.hash.hex()}

    def get_lxmf_identity(self):
        if self.delivery_destination is None:
            return {"error": "delivery identity not ready"}
        identity = self.delivery_destination.identity
        private_key = identity.get_private_key() if hasattr(identity, "get_private_key") else b""
        return {"hash": identity.hash, "private_key": private_key}

    def poll_received_messages(self):
        messages = []
        for payload in self.delivery_sink.drain():
            fields = payload.get("fields_json")
            if isinstance(fields, str):
                try:
                    fields = json.loads(fields)
                except json.JSONDecodeError:
                    fields = {}
            messages.append(
                {
                    "message_hash": payload.get("hash"),
                    "content": payload.get("content"),
                    "source_hash": payload.get("source_hash"),
                    "destination_hash": payload.get("destination_hash"),
                    "timestamp": payload.get("timestamp"),
                    "hops": payload.get("receiving_hops"),
                    "receiving_interface": payload.get("receiving_interface"),
                    "public_key": None,
                    "fields": fields or {},
                }
            )
        return messages

    def send_lxmf_message(self, destination_hash, content, _private_key=None):
        destination_hex = destination_hash.hex()
        deadline = time.time() + 60
        remote_identity = None
        while time.time() < deadline:
            if self.RNS.Transport.has_path(destination_hash):
                remote_identity = self.RNS.Identity.recall(destination_hash)
                if remote_identity is not None:
                    break
            self.RNS.Transport.request_path(destination_hash)
            time.sleep(0.2)

        if remote_identity is None:
            return {"success": False, "error": f"timed out waiting for path/identity to {destination_hex}"}

        destination = self.RNS.Destination(
            remote_identity,
            self.RNS.Destination.OUT,
            self.RNS.Destination.SINGLE,
            self.LXMF.APP_NAME,
            "delivery",
        )
        message = self.LXMF.LXMessage(
            destination,
            self.delivery_destination,
            content,
            desired_method=self.LXMF.LXMessage.DIRECT,
            include_ticket=True,
        )
        self.router.handle_outbound(message)
        return {"success": True, "message_hash": message.hash.hex()}

    def shutdown(self):
        if self.event_bridge is not None:
            self.event_bridge.deregister_callbacks()
        if self.RNS is not None:
            try:
                self.RNS.Reticulum.exit_handler()
            except Exception:
                pass


def _load_wrapper(columba_root: Path):
    python_dir = columba_root / "python"
    if (python_dir / "reticulum_wrapper.py").is_file():
        sys.path.insert(0, str(python_dir))
        from reticulum_wrapper import ReticulumWrapper  # type: ignore

        return ReticulumWrapper
    return None


def _log_level_number(value):
    if isinstance(value, int):
        return value
    levels = {
        "CRITICAL": 0,
        "ERROR": 1,
        "WARNING": 3,
        "WARN": 3,
        "INFO": 4,
        "DEBUG": 7,
        "VERBOSE": 7,
    }
    return levels.get(str(value).upper(), 4)


def _json_default(value):
    if isinstance(value, bytes):
        return value.hex()
    raise TypeError(f"unsupported json value: {type(value)!r}")


def _write_json(path: Path, payload):
    path.parent.mkdir(parents=True, exist_ok=True)
    tmp = path.with_suffix(path.suffix + ".tmp")
    tmp.write_text(json.dumps(payload, indent=2, sort_keys=True, default=_json_default))
    tmp.replace(path)


def _normalise_text(value):
    if isinstance(value, bytes):
        return value.decode("utf-8", errors="replace")
    return value


def _normalise_message(message):
    fields = message.get("fields") or {}
    return {
        "message_hash": _normalise_text(message.get("message_hash")),
        "content": _normalise_text(message.get("content")),
        "source_hash": _json_default(message["source_hash"]) if isinstance(message.get("source_hash"), bytes) else message.get("source_hash"),
        "destination_hash": _json_default(message["destination_hash"]) if isinstance(message.get("destination_hash"), bytes) else message.get("destination_hash"),
        "timestamp": message.get("timestamp"),
        "hops": message.get("hops"),
        "receiving_interface": message.get("receiving_interface"),
        "public_key": _json_default(message["public_key"]) if isinstance(message.get("public_key"), bytes) else message.get("public_key"),
        "fields": {key: _normalise_text(value) for key, value in fields.items()},
    }


def _find_message(messages, context_hash: str, content: str, direction: str):
    for message in reversed(messages):
        if message["content"] != content:
            continue
        if context_hash and context_hash not in (message["source_hash"], message["destination_hash"]):
            continue
        if direction == "inbound" and message["source_hash"] == context_hash:
            return message
        if direction == "outbound" and message["destination_hash"] == context_hash:
            return message
        if direction == "any":
            return message
    return None


def serve(args):
    columba_root = Path(args.columba_root).resolve()
    storage_dir = Path(args.storage_dir).resolve()
    control_dir = Path(args.control_dir).resolve()
    commands_dir = control_dir / "commands"
    results_dir = control_dir / "results"
    state_path = control_dir / "state.json"
    commands_dir.mkdir(parents=True, exist_ok=True)
    results_dir.mkdir(parents=True, exist_ok=True)
    storage_dir.mkdir(parents=True, exist_ok=True)

    ReticulumWrapper = _load_wrapper(columba_root)
    if ReticulumWrapper is not None:
        wrapper = ReticulumWrapper(str(storage_dir))
    else:
        wrapper = _ColumbaEventBridgeRuntime(columba_root, storage_dir)
    config = {
        "storagePath": str(storage_dir),
        "enabledInterfaces": [
            {
                "type": "TCPClient",
                "target_host": args.transport_host,
                "target_port": args.transport_port,
            }
        ],
        "logLevel": args.log_level,
        "allowAnonymous": True,
        "display_name": args.display_name,
        "enable_transport": False,
    }
    result = wrapper.initialize(json.dumps(config))
    if not result.get("success"):
        raise RuntimeError(f"columba initialize failed: {result}")

    deadline = time.time() + args.start_timeout
    while time.time() < deadline:
        destination = wrapper.get_lxmf_destination()
        identity = wrapper.get_lxmf_identity()
        if "error" not in destination and "error" not in identity:
            break
        time.sleep(0.1)
    else:
        raise RuntimeError("columba wrapper did not expose identity before timeout")

    destination = wrapper.get_lxmf_destination()
    identity = wrapper.get_lxmf_identity()
    _write_json(
        state_path,
        {
            "columba_root": str(columba_root),
            "storage_dir": str(storage_dir),
            "lxmf_hash": destination["hex_hash"],
            "identity_hash": identity["hash"].hex(),
        },
    )

    running = True
    seen_hashes = set()
    received_messages = []

    def stop_handler(_signum, _frame):
        nonlocal running
        running = False

    signal.signal(signal.SIGTERM, stop_handler)
    signal.signal(signal.SIGINT, stop_handler)

    try:
        while running:
            for message in wrapper.poll_received_messages():
                normalised = _normalise_message(message)
                msg_hash = normalised["message_hash"]
                if msg_hash in seen_hashes:
                    continue
                seen_hashes.add(msg_hash)
                received_messages.append(normalised)

            for command_path in sorted(commands_dir.glob("*.json")):
                try:
                    request = json.loads(command_path.read_text())
                except json.JSONDecodeError:
                    continue
                result_path = results_dir / f"{command_path.stem}.json"
                try:
                    command = request["command"]
                    if command == "send":
                        identity = wrapper.get_lxmf_identity()
                        response = wrapper.send_lxmf_message(
                            bytes.fromhex(request["destination_hash"]),
                            request["content"],
                            identity["private_key"],
                        )
                        response["ok"] = bool(response.get("success"))
                    elif command == "find_message":
                        message = _find_message(
                            received_messages,
                            request.get("context_hash", ""),
                            request["content"],
                            request.get("direction", "any"),
                        )
                        response = {
                            "ok": message is not None,
                            "command": command,
                            "message": message,
                        }
                    elif command == "shutdown":
                        running = False
                        response = {"ok": True, "command": command}
                    else:
                        raise ValueError(f"unsupported command '{command}'")
                except Exception as exc:  # pragma: no cover - harness failure path
                    response = {"ok": False, "error": str(exc)}

                _write_json(result_path, response)
                command_path.unlink(missing_ok=True)
            time.sleep(0.1)
    finally:
        try:
            wrapper.shutdown()
        finally:
            try:
                import RNS  # type: ignore

                atexit.unregister(RNS.Reticulum.exit_handler)
            except Exception:
                pass


def main():
    parser = argparse.ArgumentParser(description="Columba interop harness control shim")
    subparsers = parser.add_subparsers(dest="subcommand", required=True)

    serve_parser = subparsers.add_parser("serve")
    serve_parser.add_argument("--columba-root", required=True)
    serve_parser.add_argument("--storage-dir", required=True)
    serve_parser.add_argument("--control-dir", required=True)
    serve_parser.add_argument("--transport-host", default="127.0.0.1")
    serve_parser.add_argument("--transport-port", type=int, required=True)
    serve_parser.add_argument("--display-name", default="Columba Interop")
    serve_parser.add_argument("--log-level", default="INFO")
    serve_parser.add_argument("--start-timeout", type=float, default=30.0)

    args = parser.parse_args()

    if args.subcommand == "serve":
        serve(args)


if __name__ == "__main__":
    main()
