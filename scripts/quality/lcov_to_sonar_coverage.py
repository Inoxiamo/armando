#!/usr/bin/env python3
import os
import sys
import xml.etree.ElementTree as ET


def main() -> int:
    if len(sys.argv) != 3:
        print(
            "usage: lcov_to_sonar_coverage.py <input-lcov> <output-xml>",
            file=sys.stderr,
        )
        return 1

    input_path = sys.argv[1]
    output_path = sys.argv[2]
    root_dir = os.getcwd()

    coverage: dict[str, dict[int, bool]] = {}
    current_file: str | None = None

    with open(input_path, "r", encoding="utf-8") as handle:
        for raw_line in handle:
            line = raw_line.strip()
            if line.startswith("SF:"):
                source_file = line[3:]
                abs_path = os.path.abspath(source_file)
                current_file = os.path.relpath(abs_path, root_dir)
                coverage.setdefault(current_file, {})
            elif line.startswith("DA:") and current_file:
                line_no_raw, hits_raw = line[3:].split(",", 1)
                line_no = int(line_no_raw)
                hits = int(hits_raw)
                coverage[current_file][line_no] = coverage[current_file].get(line_no, False) or hits > 0
            elif line == "end_of_record":
                current_file = None

    root = ET.Element("coverage", version="1")
    for file_path in sorted(coverage):
        file_node = ET.SubElement(root, "file", path=file_path)
        for line_no in sorted(coverage[file_path]):
            ET.SubElement(
                file_node,
                "lineToCover",
                lineNumber=str(line_no),
                covered=str(coverage[file_path][line_no]).lower(),
            )

    os.makedirs(os.path.dirname(output_path), exist_ok=True)
    tree = ET.ElementTree(root)
    tree.write(output_path, encoding="utf-8", xml_declaration=True)
    print(f"Wrote Sonar coverage XML to {output_path}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
