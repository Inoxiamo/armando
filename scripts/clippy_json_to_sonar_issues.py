#!/usr/bin/env python3
import json
import os
import sys


def usage() -> None:
    print(
        "usage: clippy_json_to_sonar_issues.py <input-jsonl> <output-json>",
        file=sys.stderr,
    )


def normalize_path(path: str, root_dir: str) -> str:
    abs_path = os.path.abspath(path)
    try:
        rel_path = os.path.relpath(abs_path, root_dir)
    except ValueError:
        return path
    return rel_path


def first_primary_span(message: dict) -> dict | None:
    spans = message.get("spans", [])
    for span in spans:
        if span.get("is_primary"):
            return span
    return spans[0] if spans else None


def issue_type_for_level(level: str) -> str:
    if level == "error":
        return "BUG"
    return "CODE_SMELL"


def severity_for_level(level: str) -> str:
    if level == "error":
        return "CRITICAL"
    if level == "warning":
        return "MAJOR"
    return "MINOR"


def main() -> int:
    if len(sys.argv) != 3:
        usage()
        return 1

    input_path = sys.argv[1]
    output_path = sys.argv[2]
    root_dir = os.getcwd()
    issues: list[dict] = []

    with open(input_path, "r", encoding="utf-8") as handle:
        for line in handle:
            line = line.strip()
            if not line:
                continue

            try:
                event = json.loads(line)
            except json.JSONDecodeError:
                continue

            if event.get("reason") != "compiler-message":
                continue

            message = event.get("message", {})
            code = message.get("code") or {}
            rule_id = code.get("code")
            if not rule_id or not rule_id.startswith("clippy::"):
                continue

            span = first_primary_span(message)
            if not span:
                continue

            file_name = span.get("file_name")
            if not file_name:
                continue

            issues.append(
                {
                    "engineId": "clippy",
                    "ruleId": rule_id,
                    "severity": severity_for_level(message.get("level", "")),
                    "type": issue_type_for_level(message.get("level", "")),
                    "primaryLocation": {
                        "message": message.get("message", rule_id),
                        "filePath": normalize_path(file_name, root_dir),
                        "textRange": {
                            "startLine": span.get("line_start", 1),
                            "endLine": span.get("line_end", span.get("line_start", 1)),
                            "startColumn": span.get("column_start", 1),
                            "endColumn": span.get(
                                "column_end", span.get("column_start", 1)
                            ),
                        },
                    },
                }
            )

    os.makedirs(os.path.dirname(output_path), exist_ok=True)
    with open(output_path, "w", encoding="utf-8") as handle:
        json.dump({"issues": issues}, handle, indent=2)
        handle.write("\n")

    print(f"Wrote {len(issues)} clippy issues to {output_path}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
