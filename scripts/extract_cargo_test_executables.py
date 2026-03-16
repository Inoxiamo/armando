#!/usr/bin/env python3
import json
import sys


def main() -> int:
    if len(sys.argv) != 2:
        print(
            "usage: extract_cargo_test_executables.py <cargo-jsonl>",
            file=sys.stderr,
        )
        return 1

    input_path = sys.argv[1]
    seen: set[str] = set()

    with open(input_path, "r", encoding="utf-8") as handle:
        for line in handle:
            line = line.strip()
            if not line:
                continue

            try:
                event = json.loads(line)
            except json.JSONDecodeError:
                continue

            if event.get("reason") != "compiler-artifact":
                continue

            executable = event.get("executable")
            profile = event.get("profile") or {}
            if not executable or not profile.get("test"):
                continue

            if executable in seen:
                continue

            seen.add(executable)
            print(executable)

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
