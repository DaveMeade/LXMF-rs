#!/usr/bin/env python3

import argparse
import json
import socketserver
import threading
import time
from pathlib import Path

import LXMF
import RNS


class EndpointState:
    def __init__(self, display_name: str, storage: Path):
        self.display_name = display_name
        self.storage = storage
        self.lock = threading.Lock()
        self.messages = []
        self.reticulum = None
        self.router = None
        self.delivery_destination = None

    def start(self, config_dir: str) -> None:
        self.reticulum = RNS.Reticulum(configdir=config_dir, loglevel=7)
        self.router = LXMF.LXMRouter(storagepath=str(self.storage), enforce_stamps=False)
        self.router.register_delivery_callback(self._on_delivery)
        identity = RNS.Identity()
        self.delivery_destination = self.router.register_delivery_identity(
            identity,
            display_name=self.display_name,
        )

    def _on_delivery(self, message) -> None:
        with self.lock:
            self.messages.append(
                {
                    "source_hash": message.source_hash.hex(),
                    "destination_hash": message.destination_hash.hex(),
                    "title": message.title_as_string(),
                    "content": message.content_as_string(),
                    "timestamp": message.timestamp,
                }
            )

    def status(self) -> dict:
        return {
            "delivery_destination_hash": self.delivery_destination.hash.hex(),
            "identity_hash": self.delivery_destination.identity.hash.hex(),
            "inbox_count": len(self.messages),
        }

    def announce(self) -> dict:
        self.router.announce(self.delivery_destination.hash)
        return {"announced": True}

    def list_messages(self) -> dict:
        with self.lock:
            return {"messages": list(self.messages)}

    def send_message(self, destination_hex: str, title: str, content: str) -> dict:
        destination_hash = bytes.fromhex(destination_hex)

        if not RNS.Transport.has_path(destination_hash):
            RNS.Transport.request_path(destination_hash)

        deadline = time.time() + 60
        recipient_identity = None
        while time.time() < deadline:
            if RNS.Transport.has_path(destination_hash):
                recipient_identity = RNS.Identity.recall(destination_hash)
                if recipient_identity is not None:
                    break
            time.sleep(0.1)

        if recipient_identity is None:
            raise RuntimeError(f"timed out waiting for path/identity to {destination_hex}")

        destination = RNS.Destination(
            recipient_identity,
            RNS.Destination.OUT,
            RNS.Destination.SINGLE,
            "lxmf",
            "delivery",
        )
        message = LXMF.LXMessage(
            destination,
            self.delivery_destination,
            content,
            title,
            desired_method=LXMF.LXMessage.DIRECT,
            include_ticket=True,
        )
        self.router.handle_outbound(message)
        return {"accepted": True, "destination": destination_hex}


class ControlServer(socketserver.ThreadingTCPServer):
    allow_reuse_address = True

    def __init__(self, server_address, handler_class, state: EndpointState):
        super().__init__(server_address, handler_class)
        self.state = state


class ControlHandler(socketserver.StreamRequestHandler):
    def handle(self) -> None:
        line = self.rfile.readline()
        if not line:
            return

        try:
            request = json.loads(line.decode("utf-8"))
            method = request.get("method")
            params = request.get("params")
            if params is None:
                params = {}

            if method == "status":
                result = self.server.state.status()
            elif method == "announce":
                result = self.server.state.announce()
            elif method == "list_messages":
                result = self.server.state.list_messages()
            elif method == "send_message":
                result = self.server.state.send_message(
                    params["destination"],
                    params.get("title", ""),
                    params.get("content", ""),
                )
            else:
                raise RuntimeError(f"unknown method: {method}")

            response = {"ok": True, "result": result}
        except Exception as exc:
            response = {"ok": False, "error": str(exc)}

        self.wfile.write(json.dumps(response).encode("utf-8"))


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--name", required=True)
    parser.add_argument("--display-name", required=True)
    parser.add_argument("--rnsconfig", required=True)
    parser.add_argument("--storage", required=True)
    parser.add_argument("--control-port", type=int, required=True)
    args = parser.parse_args()

    storage = Path(args.storage)
    storage.mkdir(parents=True, exist_ok=True)

    state = EndpointState(args.display_name, storage)
    state.start(args.rnsconfig)

    with ControlServer(("127.0.0.1", args.control_port), ControlHandler, state) as server:
        server.serve_forever(poll_interval=0.1)


if __name__ == "__main__":
    main()
