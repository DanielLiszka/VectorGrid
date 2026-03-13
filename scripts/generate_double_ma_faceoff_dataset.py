#!/usr/bin/env python3
import argparse
import csv
import hashlib
import json
import math
from pathlib import Path


def parse_args() -> argparse.Namespace:
    p = argparse.ArgumentParser(
        description="Generate the frozen synthetic dataset used by the double-MA faceoff plan."
    )
    p.add_argument("--synth-len", type=int, default=200_000)
    p.add_argument("--output-csv", required=True)
    p.add_argument("--output-json", default="")
    return p.parse_args()


def generate_rows(length: int):
    px = 100.0
    for i in range(length):
        drift = 0.00005
        noise = math.sin(i * 0.017) * 0.001
        px *= 1.0 + drift + noise
        yield (
            i,
            px,
            px * 1.001,
            px * 0.999,
            px,
            1000.0,
        )


def sha256_file(path: Path) -> str:
    h = hashlib.sha256()
    with path.open("rb") as f:
        while True:
            chunk = f.read(1024 * 1024)
            if not chunk:
                break
            h.update(chunk)
    return h.hexdigest()


def main() -> int:
    args = parse_args()
    out_csv = Path(args.output_csv)
    out_csv.parent.mkdir(parents=True, exist_ok=True)

    with out_csv.open("w", newline="", encoding="utf-8") as f:
        writer = csv.writer(f, lineterminator="\n")
        writer.writerow(["timestamp", "open", "high", "low", "close", "volume"])
        for row in generate_rows(args.synth_len):
            writer.writerow(row)

    metadata = {
        "dataset_id": f"double_ma_faceoff_synth_{args.synth_len}_v1",
        "kind": "synthetic_benchmark_harness",
        "bars": args.synth_len,
        "csv_path": str(out_csv.resolve()),
        "sha256": sha256_file(out_csv),
        "columns": ["timestamp", "open", "high", "low", "close", "volume"],
        "generator": {
            "source": "ta_cli::make_synth_candles parity port",
            "initial_price": 100.0,
            "drift": 0.00005,
            "noise_formula": "sin(i * 0.017) * 0.001",
            "high_multiplier": 1.001,
            "low_multiplier": 0.999,
            "volume": 1000.0,
        },
    }

    out_json = json.dumps(metadata, indent=2)
    if args.output_json:
        out_path = Path(args.output_json)
        out_path.parent.mkdir(parents=True, exist_ok=True)
        out_path.write_text(out_json, encoding="utf-8")
    print(out_json)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
