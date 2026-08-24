# Changelog

## 1.0.1 - 2026-08-22

### Changed

- engine update: sidereon-core 1.0.1 with trust-region-least-squares 0.10.0 (unified fail-closed HostNumerics backend seam; host power dispatch reproduces NumPy's stride-0 scalar-exponent fast paths bit-for-bit). No interface API changes.

## 1.0.0 - 2026-08-21

Sidereon 1.0.0 across every interface; additions arrive without breaking
existing callers from here.

### Added

- Exact-cache single-flight opens over the C ABI: options struct, open
  discriminant, opaque owner heartbeat/publish/release, typed timeout.
- Window-scoped continuity verdicts (`sidereon_sp3_stencil_extent`,
  verdict JSON queries) and `sidereon_data_next_issue_due_json`.

### Changed

- Engine pinned to `sidereon-core` 1.0.0.

## 0.39.1 - 2026-08-11

### Fixed

- DTED terrain lookups compute the grid cell and intra-cell fraction in
  exact integer arithmetic (engine fix): the binary64 scaling product
  rounded away up to 4096 ULP of the fraction and could flip a
  coordinate strictly below a posting into the next cell's stencil.
  No API change; heights at dyadic-exact coordinates are byte-identical.

### Changed

- Engine pinned to `sidereon-core` 0.39.1.

## 0.39.0 - 2026-08-10

### Added

- `sidereon_mmap_terrain_from_path_attested`,
  `sidereon_precise_interpolant_artifact_from_path_attested`, digest
  provenance accessors, and `_verify` escalation across both mapped
  artifact readers, mirroring the engine's attested-open contract. The
  interpolant's claim/header mismatch maps to a typed status. The ABI
  smoke gate now compiles and runs a C program exercising the new
  surface.

### Changed

- Engine pinned to `sidereon-core` 0.39.0.

## 0.38.0 - 2026-08-09

### Changed

- `sidereon_terrain_store_open_path` and the precise-interpolant path
  opener now memory-map the file read-only instead of reading it into
  memory, so opening a 30+ GB terrain store no longer costs its size in
  process memory. No ABI change: existing callers get this by upgrading.

- Engine pinned to `sidereon-core` 0.38.0 with its `mmap` feature enabled.

## 0.37.0 - 2026-08-09

### Added

- `sidereon_sp3_check_continuity` attests that a parsed or merged product
  is physically continuous, writing the number of violations plus the
  counts of what was examined so a caller can tell "checked and clean"
  from "not checked". Two checks with different jobs: a physical
  earth-fixed speed gate whose bound is a true upper bound for the orbit
  class, so it cannot false-positive and catches gross corruption; and a
  hold-out interpolation residual, which supplies the sensitivity a speed
  gate structurally cannot - adjacent GNSS MEO epochs are hundreds of
  kilometres apart, so a metre-scale splice moves the implied speed by a
  fraction of a percent. Reports rather than refuses.

### Changed

- Engine pinned to `sidereon-core` 0.37.0.

## Unreleased

### Added

- `sidereon_locate_source_with` can skip per-sensor leave-one-out influence
  solves while preserving every other source-solution output bit-for-bit; the
  existing `sidereon_locate_source` continues to include influence diagnostics.
- `sidereon_closed_form_initial_guess` names the source-localization seed
  accurately. `sidereon_chan_ho_initial_guess` remains exported as a deprecated
  alias.

### Changed

- Source-sensor influence scores are
  `max(abs(residual_s), abs(leave_one_out_residual_s)) / timing_sigma_s`;
  robust downweighting is represented only by `loss_weight`.

## 0.36.3 - 2026-08-04

- Builds against `sidereon` and `sidereon-core` 0.36.3:
  `sidereon_data_newest_published_product_json` accepts AIUB whole-tree CSV
  listings whose unrelated object paths contain spaces instead of rejecting
  the entire live listing over one such row. Found by downstream 0.36.1
  verification. The C ABI is unchanged.

## 0.36.0 - 2026-08-04

- Adds the publication-lag resilience surface over core 0.36.0:
  `sidereon_data_predicted_ionex_line_candidates_json` (the opt-in CODE
  `P1`/`P2` cross-line walk for one map date - never a neighboring day's
  map, each candidate keeping its own line identity),
  `sidereon_data_newest_published_product_json` (closed listing-dialect
  detection: an unrecognizable body is an error status, never an empty
  result; `observed_at` is the archive-reported modification text,
  verbatim), and `sidereon_data_publication_listing_urls_json` (bounded, at
  most two URLs, newest directory first).
- Adds `SIDEREON_PRODUCT_PUBLISHER_WHU` and
  `SIDEREON_SOLUTION_CLASS_NEAR_REAL_TIME` for the Wuhan MGEX
  near-real-time orbit line (`wum_nrt`, hourly `WUM0MGXNRT` 02D/05M over
  anonymous FTP, archive-verified from 2024-07-03), which flows through the
  existing catalog surface.
- Builds against `sidereon` and `sidereon-core` 0.36.0. The positioning and
  orbit numerical kernels are unchanged.

## 0.35.0 - 2026-07-24

- RINEX observation QC now treats a source `INTERVAL` of zero as
  standards-compatible unavailable metadata. The default QC path reports the
  informational `OBS-H19`, infers cadence from regular epochs when possible,
  and otherwise reports an unresolved interval; an explicit zero, negative,
  or non-finite caller override remains an error.
- Negative parsed source cadence metadata is reported separately as `OBS-H20`
  and is likewise excluded from QC calculations. Non-finite RINEX text remains
  a parse error; programmatically constructed non-finite headers receive
  `OBS-H20` in the core.
- `sidereon_observation_qc_to_json` carries the compact core lint findings,
  including `OBS-H19` and `OBS-H20`.
- When interval repair is requested, it replaces an unavailable source
  `INTERVAL` with an inferred cadence, or removes the record when cadence
  cannot be resolved.
- Builds against `sidereon` and `sidereon-core` 0.35.0. The C ABI and
  positioning/orbit numerical kernels are unchanged.

## 0.34.0 - 2026-07-21

- Adds `sidereon_data_supported_samples`, exposing the core's complete date-
  and issue-aware cadence set through the standard caller-buffer/count
  contract. Product constructors enforce the same set, including the GFZ
  ultra-rapid overlap and ESA ultra-rapid issue transition.
- Adds `sidereon_data_sp3_content_start_convention`, returning a typed
  filename/content epoch relationship and signed offset with strict issue
  validation. Historical GFZ ultra-rapid identity-derived exact requests now
  inherit the cataloged one-day content-start offset, including across a GPS
  week boundary.
- Exact SP3 loading now inherits the core's complete-record terminal
  validation: standards-compatible ASCII-space padding and LF/CRLF endings are
  accepted, while malformed, missing, premature, or followed-by-data `EOF`
  records still fail closed. The generated C fixture drives the shared
  cross-interface corpus through `sidereon_sp3_load_exact`; the ABI and
  numerical behavior are unchanged.
- Caller-built exact identities now reject a span that is syntactically valid
  but not cataloged for that product family. This is an integrity-policy change
  only; the C ABI and numerical calculations are unchanged.
- Builds against `sidereon` and `sidereon-core` 0.34.0.

## 0.33.1 - 2026-07-20

- CI now regenerates and compares the public header, then compiles, links, and
  runs the focused 0.33 data-distribution and exact-SP3 C ABI programs on Linux
  and macOS.
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
  unsupported center/product combinations from the 0.33.1 core.
- Builds against `sidereon` and `sidereon-core` 0.33.1 and
  `trust-region-least-squares` 0.9.2.

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
