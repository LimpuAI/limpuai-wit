#!/usr/bin/env python3
"""Post-process cargo-about output to deduplicate license sections.

cargo-about may emit the same license (e.g. Apache-2.0) multiple times
because different crates have slightly different license text. This script
merges sections with the same license name, combining their crate lists.
"""

import re
import sys
from collections import OrderedDict


def dedup_licenses(content: str) -> str:
    # Split into header (before "## License Details") and license sections
    split_marker = "## License Details\n"
    parts = content.split(split_marker)
    if len(parts) != 2:
        print("ERROR: Could not find '## License Details' marker", file=sys.stderr)
        return content

    header = parts[0] + split_marker
    body = parts[1]

    # Split body into sections by "### " delimiter
    sections_raw = re.split(r'\n(?=### )', body)

    # Group by license name
    groups: OrderedDict[str, dict] = OrderedDict()
    for section in sections_raw:
        if not section.strip():
            continue
        name_match = re.match(r'### (.+)', section)
        if not name_match:
            continue
        name = name_match.group(1).strip()

        # Extract crate names from "Used by:" block
        used_by_match = re.search(r'Used by:\n((?:- .+\n)+)', section)
        if not used_by_match:
            continue
        crates = [
            line.strip('- ').strip()
            for line in used_by_match.group(1).strip().split('\n')
            if line.strip()
        ]

        # Extract license text from code block
        text_match = re.search(r'```\n(.*?)\n```', section, re.DOTALL)
        text = text_match.group(1) if text_match else ""

        if name not in groups:
            groups[name] = {"crates": [], "text": text}
        groups[name]["crates"].extend(crates)

    # Update overview section counts
    def update_overview_count(match):
        lic_name = match.group(1)
        if lic_name in groups:
            actual = len(groups[lic_name]["crates"])
            return f'- {lic_name} ({actual} crates)'
        return match.group(0)

    header = re.sub(
        r'- (.+?) \(\d+ crates\)',
        update_overview_count,
        header,
    )

    # Rebuild sections
    result_sections = []
    for name, data in groups.items():
        sorted_crates = sorted(data["crates"], key=lambda c: c.lower())
        crate_lines = '\n'.join(f'- {c}' for c in sorted_crates)
        section = f'### {name}\n\nUsed by:\n{crate_lines}\n\n```\n{data["text"]}\n```\n'
        result_sections.append(section)

    return header + '\n\n'.join(result_sections) + '\n'


if __name__ == '__main__':
    input_file = sys.argv[1] if len(sys.argv) > 1 else 'THIRD-PARTY-LICENSES.md'
    with open(input_file, 'r') as f:
        content = f.read()

    result = dedup_licenses(content)

    with open(input_file, 'w') as f:
        f.write(result)

    # Report
    names = [m.group(1) for m in re.finditer(r'### (.+)', result)]
    print(f"Result: {len(names)} unique license sections")
    for name in names:
        print(f"  {name}")
