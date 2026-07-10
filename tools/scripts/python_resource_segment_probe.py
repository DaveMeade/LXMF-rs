#!/usr/bin/env python3
"""Probe split-resource metadata from the pinned Python Reticulum tree."""

import argparse
import json
import os
import pathlib
import sys


class ProbeLink:
    type = 0x03
    hash = bytes(16)
    mtu = 500
    mdu = None
    traffic_timeout_factor = 1.0
    rtt = 0.1
    establishment_cost = 1

    @staticmethod
    def encrypt(data):
        return data


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--python-rns-path", required=True)
    parser.add_argument("--json-out")
    args = parser.parse_args()

    root = pathlib.Path(args.python_rns_path).resolve()
    sys.path.insert(0, str(root))
    import RNS  # pylint: disable=import-outside-toplevel

    payload = os.urandom(RNS.Resource.MAX_EFFICIENT_SIZE + 257)
    first = RNS.Resource(payload, ProbeLink(), advertise=False, auto_compress=False)
    second = RNS.Resource(
        first.input_file,
        ProbeLink(),
        advertise=False,
        auto_compress=False,
        segment_index=2,
        original_hash=first.original_hash,
        sent_metadata_size=first.metadata_size,
    )
    first_adv = RNS.ResourceAdvertisement(first)
    second_adv = RNS.ResourceAdvertisement(second)
    result = {
        "max_efficient_size": RNS.Resource.MAX_EFFICIENT_SIZE,
        "first_segment_index": first_adv.i,
        "second_segment_index": second_adv.i,
        "total_segments": first_adv.l,
        "split_flags": [bool(first_adv.s), bool(second_adv.s)],
        "original_hash_preserved": first_adv.o == second_adv.o,
        "segment_payload_sizes": [
            first.size - RNS.Resource.RANDOM_HASH_SIZE,
            second.size - RNS.Resource.RANDOM_HASH_SIZE,
        ],
    }
    expected = {
        "max_efficient_size": 1024 * 1024 - 1,
        "first_segment_index": 1,
        "second_segment_index": 2,
        "total_segments": 2,
        "split_flags": [True, True],
        "original_hash_preserved": True,
        "segment_payload_sizes": [1024 * 1024 - 1, 257],
    }
    if result != expected:
        raise RuntimeError(f"unexpected pinned Python segmentation behavior: {result!r}")
    encoded = json.dumps(result, sort_keys=True)
    print(encoded)
    if args.json_out:
        output = pathlib.Path(args.json_out)
        output.parent.mkdir(parents=True, exist_ok=True)
        output.write_text(encoded + "\n", encoding="utf-8")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
