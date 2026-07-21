# Changelog

## Unreleased

## 0.33.0 - 2026-07-20

- Adds date-aware IGS combined-final SP3 identities and CDDIS locations across
  the legacy `.sp3.Z` and current long-filename `.SP3.gz` eras, plus current
  direct-BKG locations, while preserving IGS broadcast-navigation derivation.
  Historical direct-BKG layout remains explicitly unsupported.
- Appends `SIDEREON_ARCHIVE_COMPRESSION_UNIX_COMPRESS` without changing the
  existing archive-compression discriminants.
- Adds product-aware solution classification and date-aware default-cadence
  queries, including the published GFZ rapid and ultra-rapid cadence changes
  and the issue-sensitive ESA ultra-rapid transition.
- Rejects SP3/clock dates before each evidenced family start, including the
  CODE ultra long-name boundary, and rejects unmodeled pre-week-2238 CDDIS
  long-name SP3/IONEX locations. ESA `ESA0MGNFIN` final SP3 remains direct-only
  instead of being substituted at CDDIS.
- Adds owned exact-SP3 requests, exact parse/validation with half-open or
  inclusive coverage reporting, and accessors for the declared line-1 epoch
  count and start. The legacy `sidereon_sp3_load` remains permissive.
- Inherits product-specific CODE HTTPS routes and fail-closed rejection of
  unsupported center/product combinations from the 0.33.0 core.
- Builds against `sidereon` and `sidereon-core` 0.33.0.

## 0.32.0 - 2026-07-18

- Adds `sidereon_navcen_parse_at` with owned assessment metadata and NANU
  provenance accessors, plus `sidereon_constellation_build_at`, for explicit UTC
  NAVCEN usability evaluation. Parsed forecast intervals are half-open;
  malformed timing is reported and does not invent an outage.
- The time-aware path recognizes active `UNUSUFN` notices as immediately
  unusable while retaining the legacy entry point's historical behavior.
- Keeps `sidereon_constellation_build` ABI and clock-free behavior unchanged.
- Builds against `sidereon` and `sidereon-core` 0.32.0.

## 0.31.2 - 2026-07-16

- Returns the complete merged-SP3 identity through an owned result handle,
  including canonical contributors and ordered precedence contributors.
- Uses validated fixed-width integers for every nested identity selector and
  presence flag crossing the C ABI, rejecting invalid values without undefined
  behavior.
- Adds the shared literal provenance fixture and builds against `sidereon` and
  `sidereon-core` 0.31.2.

## 0.31.0 - 2026-07-16

- Adds `sidereon_sp3_merge_input_identity`, which validates complete exact SP3
  artifact records plus the full merge policy and returns the shared versioned
  stable identity. Incomplete, malformed, mismatched, duplicate, or non-SP3
  records fail closed.
- Builds against `sidereon` and `sidereon-core` 0.31.0.

## 0.30.0 - 2026-07-16

- Adds the complete analysis-center and parsed-format-version fields to
  `SidereonProductIdentity`, plus public canonical cache-key derivation.
- Adds native exact-cache handles with bounded cross-process lock ownership,
  locked and unlocked digest-verified reads, immutable atomic publication,
  abandoned-entry cleanup, and authenticated byte/path/entry-id accessors.
- Adds `SIDEREON_STATUS_TIMEOUT` so a bounded cache-lock wait is not reported as
  an invalid argument.
- This is an intentional C ABI version advance because
  `SidereonProductIdentity` grows to retain the complete exact identity.
- Builds against `sidereon` and `sidereon-core` 0.30.0.

## 0.29.2 - 2026-07-16

- Adds `sidereon_data_validate_exact_product_set`, a fail-closed gate for a
  declared exact identity inventory. Empty declarations, duplicates, missing
  products, and undeclared products are rejected.
- Preserves prediction-tier identity during exact-set comparison. SP3
  observed/predicted timing remains available from the parser's authoritative
  record-flag summary.
- Builds against `sidereon` and `sidereon-core` 0.29.2.

## 0.29.1 - 2026-07-15

- Derives CODE predicted IONEX P1 and P2 direct locations from their current
  official tier-specific HTTPS directories, including identity-year rollover.
- Keeps same-filename P1 and P2 exact product cache keys distinct.
- Builds against `sidereon` and `sidereon-core` 0.29.1.

## 0.29.0 - 2026-07-15

- Adds pure exact GNSS product identity and explicit distribution-location
  derivation for direct archives, NASA CDDIS/Earthdata, local files, and
  in-memory input. The C library performs no hidden network or credential IO.
- Builds against `sidereon` and `sidereon-core` 0.29.0.

## 0.28.1 - 2026-07-15

- Builds against `sidereon` and `sidereon-core` 0.28.1, inheriting the repaired
  official HTTPS source for CODE ultra-rapid products and the symmetric RTK
  candidate-selection fixes.

## 0.28.0 - 2026-07-13

- Adds per-cell SP3 precedence, optional deterministic outlier rejection,
  clock-outlier report access, and observed/predicted epoch summaries.
- Builds against `sidereon` and `sidereon-core` 0.28.0.

## 0.27.1 - 2026-07-13

- Builds against `sidereon` and `sidereon-core` 0.27.1.
- Fixes LAMBDA integer least-squares searches with finite ambiguities outside
  the `int64_t` output domain: they now return
  `SIDEREON_STATUS_INVALID_ARGUMENT` instead of a successful result containing
  saturated integers and non-finite scores.

## 0.27.0 - 2026-07-12

- Builds against `sidereon` and `sidereon-core` 0.27.0.
- Adds `sidereon_geoid_grid_from_proj_egm96_gtx` for PROJ's public EGM96
  15-arcminute GTX grid.
- Adds `sidereon_geoid_grid_undulation_proj_rad` with an explicit
  fused-versus-separately-rounded arithmetic selector and typed coordinate
  error detail. Existing geoid lookup functions retain their previous bits.

## 0.26.1 - 2026-07-12

- Builds against `sidereon` and `sidereon-core` 0.26.1.
- Fixes a process/VM denial of service when parsing malicious RINEX 2
  observation input with an oversized declared epoch satellite count. C binding
  releases 0.11.1 through 0.26.0 are affected; upgrade to 0.26.1 or later.

## 0.26.0 - 2026-07-12

- Builds against `sidereon` and `sidereon-core` 0.26.0.
- Removes the unsound sequential RTK innovation-screen interface together with
  `SidereonRtkInnovationScreen`, its epoch accessor, and the three corresponding
  fields in `SidereonRtkArcUpdateOptions`. This is an intentional breaking ABI
  change matching the core 0.26.0 removal.
- Inherits the core fix that keeps near-polar TEC coordinates finite.
