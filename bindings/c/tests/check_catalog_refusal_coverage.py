#!/usr/bin/env python3
"""Check that the catalog refusal table covers the declared API."""
from pathlib import Path
import re
import sys

here = Path(__file__).resolve().parent
header = (here.parent / "include" / "sidereon.h").read_text()
test = (here / "catalog_refusal_smoke.c").read_text()
# Comments may refer to functions without declaring them.
header = re.sub(r"/\*.*?\*/", "", header, flags=re.S)
declared = set(re.findall(r"\b(sidereon_\w+)\s*\(", header))
functions = set(re.findall(r"\b(sidereon_(?:celestrak_|navcen_|parse_celestrak_)\w+)\s*\(", header))
functions.update({
    "sidereon_data_allowed_hosts", "sidereon_omm_from_tle",
    "sidereon_omm_with_ccsds_header", "sidereon_omm_to_kvn_lenient",
    "sidereon_omm_to_xml_lenient", "sidereon_omm_equal",
    "sidereon_omm_array_equal", "sidereon_omm_array_span",
    "sidereon_omm_array_span_count", "sidereon_sgp4_fit_tle_with_omm_header",
})
table = test.split("#define ENTRIES(X)", 1)[1].split("struct entry", 1)[0]
entries = re.findall(r"\bX\((sidereon_\w+)\)", table)
missing_declarations = functions - declared
missing = functions - set(entries)
extra = set(entries) - functions
duplicate = {name for name in entries if entries.count(name) != 1}
if missing_declarations or missing or extra or duplicate:
    print(f"catalog refusal coverage: missing_declarations={sorted(missing_declarations)} missing={sorted(missing)} extra={sorted(extra)} duplicate={sorted(duplicate)}", file=sys.stderr)
    sys.exit(1)
print(f"catalog refusal coverage: {len(entries)} declarations")
