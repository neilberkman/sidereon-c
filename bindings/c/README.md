# sidereon (C)

A C-ABI binding over the Sidereon GNSS positioning engine. It is a thin interface
in the C idiom: opaque handles, integer status codes, and caller-allocated output
buffers. It adds no modeling of its own, so a solve returns exactly the numbers
the `sidereon-core` engine produces.

## Build

Build the shared library and generate the header. The crate produces both a
`cdylib` and a `staticlib` named `sidereon`.

    # from bindings/c:
    cargo build --release
    # -> <workspace target>/release/libsidereon.{dylib,so}  and  libsidereon.a
    # This crate is the workspace member, so the library lands in the workspace
    # root's target/ directory. `cargo metadata --format-version 1` reports the
    # exact target_directory; tests/run_smoke.sh resolves it automatically.

    cargo install --locked cbindgen --version 0.29.4   # if not already installed
    cbindgen --config cbindgen.toml --crate sidereon-c --output include/sidereon.h

A committed `include/sidereon.h` is already present; regenerate it only after
changing the C surface.

## Example

Parse an SP3 byte buffer, run a single-point positioning solve, and read the
position into a caller buffer. Every fallible call returns `SIDEREON_STATUS_OK` on
success; on any other value, `sidereon_last_error_message` gives the reason.

```c
#include <stdio.h>
#include "sidereon.h"

/* sp3_bytes / sp3_len: the contents of an SP3 file you have read into memory. */
SidereonSp3 *sp3 = NULL;
if (sidereon_sp3_load(sp3_bytes, sp3_len, &sp3) != SIDEREON_STATUS_OK) {
    char msg[256];
    sidereon_last_error_message(msg, sizeof(msg));
    fprintf(stderr, "load failed: %s\n", msg);
    return 1;
}

SidereonObservation obs[] = {
    { "G01", 21000123.4 },
    { "G08", 22517889.1 },
    /* ...more satellites... */
};

SidereonSppInputs in = {
    .observations = obs,
    .observation_count = sizeof(obs) / sizeof(obs[0]),
    .t_rx_j2000_s = /* receiver time, seconds past J2000 */ 0.0,
    .t_rx_second_of_day_s = /* second of day */ 0.0,
    .day_of_year = /* 1-based, fractional allowed */ 1.0,
    .initial_guess = { 0.0, 0.0, 0.0, 0.0 },  /* [x_m, y_m, z_m, clock_state] */
    .ionosphere = false,
    .troposphere = false,
    .with_geodetic = true,
};

SidereonSppSolution *sol = NULL;
if (sidereon_solve_spp(sp3, &in, &sol) != SIDEREON_STATUS_OK) {
    sidereon_sp3_free(sp3);
    return 1;
}

double xyz[3];
sidereon_spp_solution_position(sol, xyz, 3);
printf("position = [%.6f, %.6f, %.6f] m\n", xyz[0], xyz[1], xyz[2]);
double rx_clock_s = 0.0;
sidereon_spp_solution_rx_clock_s(sol, &rx_clock_s);
printf("rx_clock_s = %.9e\n", rx_clock_s);

sidereon_spp_solution_free(sol);
sidereon_sp3_free(sp3);
```

Compile and link against the header and shared library (`$LIBDIR` is the
workspace `target/release` directory reported by `cargo metadata`):

    cc -std=c11 -I include my_program.c \
        -L "$LIBDIR" -lsidereon \
        -Wl,-rpath,"$LIBDIR" -lm -o my_program

Reader functions copy into memory the caller owns: `sidereon_sp3_epoch_count`
and `sidereon_spp_solution_rx_clock_s` write scalars, `sidereon_spp_solution_position`
writes >= 3 doubles, `_residuals` supports `(NULL, 0)` size queries with
`out_required` and copies only when the buffer is large enough, and `_dop` writes
a `SidereonDop` of geometry scalars. Free every handle with its `_free`
function. See `include/sidereon.h` for the full surface and per-function safety
notes.

## Public parity routes

The generated header exposes the fixed-value and policy helpers directly:
`sidereon_covariance6_*` covers construction, validation, unit conversion, PSD
interpolation, and ECI/RTN transforms; `sidereon_second_of_day`,
`sidereon_day_of_year`, and `sidereon_data_day_of_year` cover the calendar
conventions; `sidereon_rinex_band_*`, `sidereon_rinex_observation_*`, and
`sidereon_default_iono_free_pair` apply the signal policy; and
`sidereon_lnav_tow`, `sidereon_lnav_subframe_id`,
`sidereon_lnav_parity`, and `sidereon_lnav_parity_valid` expose the LNAV bit
helpers.

RINEX NAV and broadcast routes include full records, representable GLONASS state
vectors and frequency channels, ionosphere and leap-second header values,
lenient skipped block diagnostics, raw record lists, and NAV encoding. The
standalone GLONASS parser reads every slot token `R01`..`R99`, the extended
slots `R28` and up included, and separately exposes the raw satellite tokens of
records whose token names no satellite (such as `R00`) through its handle's
skipped-record count/item accessors; a successful parse therefore does not
imply that every input GLONASS record was read. RINEX clock routes
read, edit and write the lossless clock model described below; SBAS EMS and RTKLIB
text-log routes expose timestamped blocks and their payload bytes. RINEX RTK builders
provide single- and dual-frequency arcs with epoch metadata and observation,
position, wavelength, offset, and sort-key query/fill accessors. DTED tile-list
routes convert caller-supplied tiles to deterministic memory-mappable bytes or
write the same bytes to a path.

RTK arc solvers also provide additive `*_v2` input rows for exact prediction
epochs; dual-frequency rows additionally accept exact gap epochs. Each exact
epoch is supplied by a live `SidereonExactEpoch` handle for the duration of the
call. A set presence flag with a NULL handle is refused; a cleared flag means
the exact value is explicitly absent. Legacy rows and solver symbols retain
their original layouts and f64-only timing behavior.

Numerical state and covariance propagation provide additive
`*_with_tide_system` entrypoints. Their `gravity_tide_system` argument is a
checked `uint32_t`: `SIDEREON_GRAVITY_TIDE_SYSTEM_TIDE_FREE` (0),
`SIDEREON_GRAVITY_TIDE_SYSTEM_ZERO_TIDE` (1), or
`SIDEREON_GRAVITY_TIDE_SYSTEM_MEAN_TIDE` (2). Unknown values return
`SIDEREON_STATUS_INVALID_ARGUMENT`; the original propagation functions retain
their TideFree behavior. The selected system is applied to zonal gravity
coefficients and solid-Earth tide gravity in composite models. The
constants-aware station displacement function is exposed as
`sidereon_solid_earth_tide_with_constants`; the original
`sidereon_solid_earth_tide`, `sidereon_solid_earth_pole_tide`, and
`sidereon_ocean_tide_loading` names remain available.

Handles returned by these routes are owned by the caller. Release
`SidereonBroadcastEphemeris` with `sidereon_broadcast_ephemeris_free`,
`SidereonRinexNavParse` with `sidereon_nav_parse_free`,
`SidereonRinexNavRecords` with `sidereon_rinex_nav_records_free`,
`SidereonRinexGlonassRecords` with `sidereon_rinex_glonass_records_free`,
`SidereonRinexClock` with `sidereon_rinex_clock_free`, `SidereonClockSeries`
with `sidereon_rinex_clock_series_free`, `SidereonClockHeaderRecords` with
`sidereon_clock_header_records_free`, `SidereonClockRecords` with
`sidereon_clock_records_free`, `SidereonRinexClockResult` with
`sidereon_rinex_clock_result_free`, `SidereonBlqBlocks` with
`sidereon_blq_blocks_free`, `SidereonBlqResult` with
`sidereon_blq_result_free`, `SidereonSbasLogBlocks` with
`sidereon_sbas_log_blocks_free`, `SidereonRtkRinexArc` with
`sidereon_rtk_rinex_arc_free`, and `SidereonRtkRinexDualFrequencyArc` with
`sidereon_rtk_rinex_dual_frequency_arc_free`. Passing NULL to a free function
is a no-op. The handles and borrowed pointers must not be used after their
matching handle is freed.

Variable-length query/fill accessors use the shared caller-buffer convention:
call with a NULL buffer and length 0 to receive the required element count in
`out_required`, allocate caller-owned storage, and call again. A short buffer
returns `SIDEREON_STATUS_INVALID_ARGUMENT`, writes zero elements, and reports
the required count. The generated header documents the exact contract for each
route, including byte-oriented accessors.

Text that belongs to a typed record, such as the label or reason of a bias
notice or the path and message of a terrain error, is read with its own byte
accessor under the same convention: the call takes the record's index or
family plus the text part, and reports the byte count in `out_written` and
`out_required`. `sidereon_bias_set_notice` fills the typed fields of one
notice and `sidereon_bias_set_notice_text` copies its keyword, label, block
name, version, reason or other text part (`SidereonBiasNoticeText`). After a
DTED or terrain failure, `sidereon_last_terrain_error_text` copies the path,
message, reason or remediation of the last error of the family named
(`SidereonTerrainErrorFamily`, `SidereonTerrainErrorText`).

For exact acquired products, construct an owned request with
`sidereon_sp3_exact_request_new` or
`sidereon_sp3_exact_request_from_identity`, then call
`sidereon_sp3_load_exact`. Success reports whether the regular epoch grid uses
the half-open or inclusive boundary representation. Header/identity, cadence,
span, structure, and grid mismatches fail without returning an SP3 handle. The
legacy `sidereon_sp3_load` remains the permissive general parser; use
`sidereon_sp3_declared_epoch_count` and
`sidereon_sp3_declared_start_j2000_seconds` to inspect its line-1 evidence.

SP3 products and precise sources carry a validated interpolation policy: consecutive node intervals exceeding `gap_threshold_factor` times nominal spacing are treated as coverage gaps that position interpolation will not span. The default is 1.5 (`DEFAULT_GAP_THRESHOLD_FACTOR`). Values `<= 0.0` select this default, while values `<= 1.0`, `NaN`, or infinity fail with `SIDEREON_STATUS_INVALID_ARGUMENT`. Use `sidereon_sp3_load_with_gap_threshold_factor` or `sidereon_sp3_load_exact_with_gap_threshold_factor` to configure products, `sidereon_sp3_gap_threshold_factor` to read the current factor, `sidereon_sp3_check_continuity_with_gap_threshold_factor` and `sidereon_sp3_continuity_verdict_json_with_gap_threshold_factor` for continuity checks with an explicit policy, `sidereon_precise_ephemeris_samples_from_samples_with_gap_threshold_factor` and `sidereon_precise_ephemeris_interpolant_from_samples_with_gap_threshold_factor` (with their corresponding `_gap_threshold_factor` getters) for sample-backed sources, and `sidereon_precise_interpolant_artifact_gap_threshold_factor` to inspect an artifact's header.

The data catalog also exposes `sidereon_data_product_solution_class`,
`sidereon_data_default_sample_for_date`, `sidereon_data_supported_samples`, and
`sidereon_data_sp3_content_start_convention`. The content-start query returns a
typed convention plus the signed seconds added to the filename epoch, validates
ultra-rapid issues strictly, and is the same catalog fact inherited by exact
requests built from identities. The supported-samples query uses the standard
`(NULL, 0)` size query followed by a caller-allocated array of
`SidereonProductSample` records; its exact count and null-terminated tokens
report all cataloged cadences for the selected date and issue. Product
constructors enforce that same set. Historical IGS final CDDIS locations
report Unix `.Z` compression through the appended
`SIDEREON_ARCHIVE_COMPRESSION_UNIX_COMPRESS` value; prior enum values are
unchanged. Historical direct-BKG layout is not modeled and is rejected rather
than guessed. Catalog derivation fails before the evidenced ESA-final
SP3/clock, GFZ-rapid SP3/clock, IGS-ultra, CODE-ultra, ESA-ultra, and GFZ-ultra
starts. It preserves the historical GFZ-ultra cadence change and ESA-ultra
issue-level transition, and rejects unmodeled pre-week-2238 CDDIS long-name
SP3/IONEX locations while retaining the separately modeled legacy IGS final
`.sp3.Z` family. ESA `ESA0MGNFIN` final SP3 remains direct-only rather than
being substituted with another CDDIS product.

## Writers that refuse lossy output

`sidereon_sp3_to_sp3_text`, `sidereon_rinex_obs_to_rinex_text` and
`sidereon_antex_encode` return text only when reading it back gives the product
itself. A product holding something the format cannot state exactly is refused
with `SIDEREON_STATUS_INVALID_ARGUMENT` and the engine's text in the
thread-local message; a refusal reports a required length of zero and writes
nothing, so it never reads as a successful empty file. For SP3 that covers a
value its column would read back as a different number or as an absence
sentinel, a field wider than its columns, an epoch no record restates exactly,
and a header that disagrees with the records. For RINEX observations it covers
an epoch flag wider than its field, an observation or cycle-slip epoch with no
time, picoseconds below version 4.02, and a version 2 product holding what
version 2 cannot say, such as `SYS / SCALE FACTOR` records. For ANTEX it covers
field overflow, precision loss, validity seconds no `F13.7` form with a decimal
point states, sample coordinates the grid cannot reconstruct, and public
antenna fields that disagree with the retained validity intervals. For SP3 it
also covers a header satellite with no `01`..`99` token that reads back as
itself, which only an identifier built without the constructor can hold
(`SIDEREON_SP3_WRITE_ERROR_KIND_SATELLITE_NOT_REPRESENTABLE`, with the satellite
in `sat_id`).

Each writer also has a route that returns the refusal typed, in an owned result
the caller releases:

| Writer | Result route | Result type, release |
| --- | --- | --- |
| SP3 | `sidereon_sp3_to_sp3_text_result` | `SidereonSp3WriteResult`, `sidereon_sp3_write_result_free` |
| RINEX observation | `sidereon_rinex_obs_to_rinex_text_result` | `SidereonRinexObsWriteResult`, `sidereon_rinex_obs_write_result_free` |
| ANTEX | `sidereon_antex_encode_result` | `SidereonAntexResult`, `sidereon_antex_result_free` |
| RINEX clock | `sidereon_rinex_clock_to_text_result` | `SidereonRinexClockResult`, `sidereon_rinex_clock_result_free` |
| BLQ | `sidereon_blq_blocks_to_text_result`, `sidereon_blq_block_to_text_result` | `SidereonBlqResult`, `sidereon_blq_result_free` |

The route returns `SIDEREON_STATUS_OK` for any well-formed call. `_get_outcome`
copies the fixed-width outcome: `is_ok`, the status the refusal maps to, and the
typed error. `SidereonSp3WriteError` names every `Sp3WriteError` variant through
`SidereonSp3WriteErrorKind` and carries the variant's satellite, epoch index,
columns, decimals, counts, time scales and numbers, each behind a present flag
with NaN for an absent number; the field name and the text value are copied
with `sidereon_sp3_write_result_get_field` and `_get_text_value`.
`SidereonRinexObsWriteError` does the same for `RinexObsWriteError`, with the
observation code and the detail text (the unreadable event record, the
`LEAP SECONDS` time system, or the field that would read back changed) copied
with `sidereon_rinex_obs_write_result_get_code` and `_get_detail`.
`SidereonAntexError` names every `AntexError` variant through
`SidereonAntexErrorKind`, with its section count behind `has_sections`; the
antenna id, record, field, value, frequency and reason are copied with
`sidereon_antex_result_error_text`. The RINEX clock and BLQ results are
described in their sections below. `_get_text` copies the text of a written
result and refuses a refused one the way the plain writer does; the message the
result owns is not overwritten by later calls.

`sidereon_rinex_repair_obs` keeps its repair handle when the repaired product
cannot be written: the actions and remaining lint stay readable,
`sidereon_rinex_repair_text` refuses with the writer's text, and
`sidereon_rinex_repair_text_result` returns the refusal as a
`SidereonRinexObsWriteResult`. `sidereon_rinex_repair_crinex_text` names why a
repair has no CRINEX output instead of reporting that none is available.

## SP3 merge agreement

`SidereonSp3EpochAgreement` and `SidereonSp3AgreementSummary` carry a present
flag beside every spread, and an absent spread is NaN, never 0. A zero spread
is a measured agreement of two or more sources. An epoch whose cells were all
single-source has no position spread, and a cell that carries a clock with no
orbit (an SP3 record whose coordinates are the missing-orbit sentinel) has no
position at all, so a merge whose accepted cells are all clock-only reports both
position fields absent while its clock fields can still be present.

## RINEX observation epochs and carrier phase

`SidereonRinexObsEpoch.has_epoch` is false for an event record whose epoch
fields are blank, which RINEX 2.11 and 3.05 allow for an event without a
significant epoch; its calendar fields are then 0 with a NaN second. Such an
event is written back with blank epoch fields.

`sidereon_rinex_obs_carrier_phase` reads each epoch under the header in effect
at it, so a phase shift or GLONASS channel an event declares applies to the
epochs after the event. Every row is copied whatever the header says of its
`SYS / PHASE SHIFT` correction. `phase_shift_status` is
`SIDEREON_RINEX_CORRECTION_STATUS_AVAILABLE` with the correction in
`phase_shift_cycles`, `_UNKNOWN` where the only covering record names just the
constellation, or `_AMBIGUOUS` where records in one header block give the
signal different corrections; the cycles are NaN for the last two.
`phase_shift_conflict_count` counts an ambiguous row's corrections, and
`sidereon_rinex_obs_carrier_phase_conflicts` copies them in record order as
`SidereonRinexPhaseShiftCorrection` values, with `has_cycles` false for a blank
correction.

`SidereonArcEpochV2` adds an optional exact gap epoch without changing the
layout or behavior of `SidereonArcEpoch`. Use the `_v2` cycle-slip and smoothing
routes when exact epoch differences matter; set `has_gap_epoch` and pass a live
`SidereonExactEpoch` handle, or leave the flag false to use the legacy floating
gap coordinate. The V2 calls borrow those handles only for the call.

Bias parser failures preserve their engine variant and payload in
`sidereon_last_bias_error` and `sidereon_last_bias_error_text`. Read both on the
same thread as the failed parse; another ordinary bias operation clears the
thread-local failure, while reading either accessor does not.

`sidereon_bias_sinex_to_text` and `sidereon_code_dcb_to_text` write UTF-8 text
through the variable-length buffer contract. Their `_to_bytes` counterparts
preserve source bytes exactly, including invalid UTF-8; the text routes refuse
such a source with `SIDEREON_BIAS_ERROR_KIND_INVALID_UTF8_LINE` and its line
number. Other exact-representation refusals preserve the core `BiasError` kind,
numeric payload, scale and text through the same thread-local accessors. The
four routes leave the old reader and status ABI unchanged.

## GNSS calendar fields

`sidereon_gnss_seconds_of_week_from_calendar` refuses fields that name no
calendar date and time (a month outside 1..=12, a day outside the month, an
hour outside 0..=23, a minute outside 0..=59, a second outside 0..=60) with
`SIDEREON_STATUS_INVALID_ARGUMENT`, leaving NaN in `*out_sow_s`.
`sidereon_gnss_week_from_calendar` reports such a date absent, as it does a
date before the week epoch or a scale without GNSS weeks.

## RINEX clock products

A `SidereonRinexClock` keeps the text it reads as its authority: every header
line with its exact label and payload, and every body line in order, including
blank lines, `AR`, `AS`, `CR`, `DR` and `MS` records, continuation lines, and
after `sidereon_rinex_clock_parse_lossy` the lines that do not read as a record.
`sidereon_rinex_clock_to_text` restates an unedited product byte for byte, line
terminators included. `sidereon_rinex_clock_parse` fails on the first line that
does not read; `sidereon_rinex_clock_parse_result` returns that failure typed.

`sidereon_rinex_clock_info` reports the version, column layout, satellite
system, time system (`GPS`, `GLO`, `GAL`, `QZS`, `BDS`, `IRN`, `UTC`, `TAI`),
how it was established (declared, defaulted to the 3.00 default, unrecognized,
conflicting, or built), the time scale when it resolves to one, and the counts
of every view. `GLO` is read in UTC in every version; `IRN` has no core time
scale, so its records keep their civil epochs and have no instant, and
`has_time_scale` is false. `sidereon_rinex_clock_time_system_label` copies the
label of an unrecognized or conflicting `TIME SYSTEM ID`.

The views are:

- `sidereon_rinex_clock_header_records`: an owned snapshot of every header line,
  read with `sidereon_clock_header_records_get` (line number, label column, how
  the fields were read and the typed field's numbers), `_text` (the line, label
  or payload) and `_field_text` (the typed field's text parts, in the order
  `SidereonClockHeaderFieldKind` lists for each kind).
- `sidereon_rinex_clock_records`: an owned snapshot of every data record in file
  order, including duplicate records for one name and epoch. Each
  `SidereonClockRecord` carries its type, canonical satellite (for `AS`), civil
  epoch, instant, declared values (bias first, up to six), values present beyond
  the declared count with their positions, source line and span, and how its
  lines were read. `sidereon_clock_records_name` copies its name.
- `sidereon_rinex_clock_series` and `_series_for`: the per-satellite samples of
  the `AS` records whose epoch resolves to an instant. `SidereonClockPoint`
  carries the epoch, the bias and every declared value after it (bias sigma,
  rate, rate sigma, acceleration, acceleration sigma), with unused entries NaN.
- `sidereon_rinex_clock_skipped_records`: the records outside the series.
- `sidereon_rinex_clock_diagnostics`: every line a lossy read kept without
  reading it, and header time-system errors, each with a typed
  `SidereonRinexClockError`; `sidereon_rinex_clock_diagnostic_text` copies its
  text parts.
- `sidereon_rinex_clock_notices`: non-fatal findings, such as a defaulted time
  system, a 3.04 file without `TIME SYSTEM ID`, nonconforming header records,
  and records carrying surplus values or read at the other layout's columns or
  as whitespace-separated values.
- `sidereon_rinex_clock_source_line`: one source line by line number, which
  holds a record's seconds field exactly as written.

`sidereon_rinex_clock_set_time_system`, `_set_record_values`, `_insert_record`,
`_remove_record`, `_remove_records` and `_set_records_values` edit a product in
place after checking the whole change: an edit the writer would refuse,
including one that would drop a record's surplus values, changes nothing. Each
returns its status and, when its `out_result` is not NULL, an owned
`SidereonRinexClockResult`; a removal's result carries the removed record.
`sidereon_rinex_clock_from_points` builds a product from per-satellite samples
in a stated time scale, every declared value kept.

`sidereon_rinex_clock_to_text_result` writes under a `SidereonClockWritePolicy`
(`sidereon_clock_write_policy_init` allows no departure). An epoch no
microsecond text states exactly is refused, or with
`nearest_microsecond_epochs` allowed written at the nearest microsecond and
reported as a `SidereonClockWriteDeparture` naming the record, its epoch and,
through `sidereon_rinex_clock_result_departure_text`, its name and the epoch
text written. Values are never approximated. A product built in GLONASS system
time is refused as `SIDEREON_RINEX_CLOCK_ERROR_KIND_UNSUPPORTED_TIME_SCALE`.

`sidereon_rinex_clock_bias_at_civil` interpolates at a civil epoch in the
product's own time scale, so a UTC product answers a `23:59:60` query on a
leap-second day and interpolates across it by elapsed time;
`sidereon_rinex_clock_bias_at_epoch` takes a scale-tagged instant and
`sidereon_rinex_clock_bias_at_gps_seconds` GPS seconds. An unavailable bias is
reported with `*out_available` false and NaN. `sidereon_civil_to_clock_epoch`
converts civil fields in a time scale to the instant a record at that epoch
holds. The civil routes, `sidereon_civil_to_gps_seconds` included, read the
second as the shortest decimal of the double given, every digit kept, so
`59.9999996` names that epoch rather than rounding into the next minute.

## ANTEX products

A `SidereonAntex` retains every record ANTEX 1.4 defines and keeps absent
records absent. `sidereon_antex_header` reports `ANTEX VERSION / SYST`,
`PCV TYPE / REFANT` (so relative values can be told apart from absolute ones;
`sidereon_antex_header_text` copies the reference antenna), the number of header
comments and whether `END OF HEADER` is present. `sidereon_antex_outer_comment`
copies the comments between and after the blocks with the number of blocks
before each. `sidereon_antex_block` returns every antenna block in file order,
and `sidereon_antex_antenna_at` and `sidereon_antex_satellite_antenna` the block
valid at an epoch. `sidereon_antex_skipped_records` counts the inconsistent
records a forgiving read passed over.

`sidereon_antenna_info` reports a block's kind, `DAZI` and
`ZEN1 / ZEN2 / DZEN` (each behind a present flag), whether it carries
`# OF FREQUENCIES` and `SINEX CODE`, its validity bounds and the counts of its
method records, comments and frequency sections. `SidereonAntexDateTime` holds
a validity bound exactly: the fraction of the second is
`fraction_digits / 10^fraction_scale`, every digit the `F13.7` field states.
Frequency sections are kept in file order, and a repeated label keeps every
section: `sidereon_antenna_pco` and `sidereon_antenna_pcv` refuse a label whose
sections differ as `SIDEREON_ANTEX_ERROR_KIND_AMBIGUOUS_FREQUENCY`.
`sidereon_antenna_frequency`, `_frequency_label` and `_frequency_pcv_samples`
read each section and its `START OF FREQ RMS` section.

Every ANTEX failure is typed: `sidereon_last_antex_error` and
`sidereon_last_antex_error_text` report the most recent one on the thread, and
`sidereon_antex_parse_result` and `sidereon_antex_encode_result` return it owned.

## Terrain

A DTED posting holding the null value (all bits set) is an unknown elevation.
`sidereon_dted_tile_get_elevation` refuses it, and
`sidereon_last_dted_tile_error` reports `SIDEREON_DTED_TILE_ERROR_KIND_NULL_POSTING`
with the posting; every `DtedTileError` variant is typed the same way. A terrain
lookup that weights a null posting no neighbouring tile answers is refused, and
so is a lookup served by a tile whose DSI states a horizontal datum other than
WGS84: `sidereon_last_terrain_lookup_error` reports the tile, the posting or the
datum, and each batch result carries the same `SidereonTerrainLookupError`.
`sidereon_dted_tile_horizontal_datum` reads a tile's datum. The terrain-store
converter refuses a non-WGS84 tile, and the store parser refuses a tile id
outside the coordinate domain or bounds that are not its cell's edges, each as
its own `SidereonTerrainStoreErrorKind`.

## BLQ ocean-loading blocks

`sidereon_blq_parse` reads every station block of a BLQ file into a
`SidereonBlqBlocks` list. Each block keeps its station, its coefficients in the
supported constituent order, and every comment and column-order header line in
input order with its placement (`sidereon_blq_blocks_comment`).
`sidereon_blq_blocks_new`, `_push` and `_push_comment` build a list.
`sidereon_blq_blocks_to_text_result` writes the whole list as one file, carrying
a column-order header across blocks as the parser does, and
`sidereon_blq_block_to_text_result` writes one block; a block the parser would
not read back unchanged is refused with its index and a
`SidereonBlqWriteErrorKind`. A parse refusal reports its line and a
`SidereonBlqParseErrorKind`, with the station, token or label it names copied
by `sidereon_blq_result_error_text`.

## Positioning and correction streams

`SidereonSppModelOptions` selects the receiver QZSS clock (`GPS` or `Separate`)
and troposphere model (`RTKLIB` or `SaastamoinenNiell`). Initialize it with
`sidereon_spp_model_options_init`; its fields are checked `uint32_t` tags, so an
unknown value is refused rather than interpreted as an enum. The additive
`*_with_models` SPP, broadcast-SPP, and RINEX-SPP entry points,
`sidereon_solve_spp_v2_with_models_at_exact_epoch`,
`sidereon_solve_broadcast_with_models_at_exact_epoch`,
`sidereon_sbas_solve_broadcast_v2_with_models_at_exact_epoch`,
`sidereon_ssr_solve_broadcast_v2_with_models_at_exact_epoch`, and
`sidereon_solve_spp_batch_v2_serial` accept these choices. Static callers use
`SidereonStaticPositionOptionsV2` and
`sidereon_static_position_options_v2_init`. Existing input and options layouts
retain their GPS-clock and RTKLIB-troposphere defaults.

`sidereon_ppp_corrections_build_with_validity_and_tide_constants` selects strict
or permissive validity and either `Conventions` or `IersRoutine` station-tide
constants. It returns a typed `SidereonPppCorrectionsErrorKind` on refusal and
stores a before/after-coverage degradation reason on a successful handle;
`sidereon_ppp_corrections_degraded_reason` reads that reason. The original
builder keeps its existing default behavior.

An ionosphere-corrected SPP or static solve leaves out a satellite whose carrier
does not resolve (a GLONASS satellite with no channel, or a channel outside the
`-7..=6` FDMA allocation) and reports it as a rejected satellite with
`SIDEREON_SPP_REJECTION_REASON_IONOSPHERE_CARRIER_UNRESOLVED`; the rest of the
epoch is solved. The RINEX RTK arc builders form a satellite's measurement from
the first configured pair whose carriers resolve and report every measurement
no pair resolves through `sidereon_rtk_rinex_arc_unresolved_carriers` and
`sidereon_rtk_rinex_dual_frequency_arc_unresolved_carriers`, naming the
receiver, epoch, satellite and observable.

The legacy ephemeris-source result structs retain their boolean `degraded`
fields. Immediately after a broadcast, precise, SP3, SBAS, or SSR source-state
or transmit-clock query, call `sidereon_last_degrade_reason` on the same
operating-system thread to read its typed `None`, `BeforeCoverage`, or
`AfterCoverage` reason without changing those existing struct layouts. After
validating output pointers, each listed query clears the record before source
lookup, including empty results and errors, and writes its typed result before
returning; another listed query replaces it.

`sidereon_sgp4_last_error_info` returns the typed SGP4 failure category and its
engine error code or resonance-step budget when applicable. The budget refusal
therefore remains available as an integer payload as well as in the thread-local
message.

`sidereon_rtcm_message_encode` and `sidereon_rtcm_message_to_frame` refuse a
message whose satellite or signal lists the masks or satellite fields cannot
state, instead of writing another satellite. `sidereon_satellite_id_to_sbas_prn`
converts only the SBAS slots `S20`..`S58`, and
`sidereon_sbas_store_unassigned_mask_corrections` counts, per mask number, the
corrections a GEO addressed to mask bits that name no satellite.

## Generic engine error ABI

The generic engine error ABI provides versioned, structured core diagnostic reporting across public operations without discarding engine error variants or payload fields.

### Error inspection and reset

- `sidereon_last_engine_error_info`:
  ```c
  SidereonStatus sidereon_last_engine_error_info(SidereonEngineErrorInfo *out);
  ```
  Copies the summary record for the most recent generic engine error on the current OS thread. `SidereonEngineErrorInfo` carries:
  - `family`: the `SidereonEngineErrorFamily` enum identifying the subsystem.
  - `payload_len`: length in bytes of the UTF-8 JSON payload, excluding any null terminator.
  When no engine error is retained on the thread, it returns `SIDEREON_STATUS_OK` with `family` set to `SIDEREON_ENGINE_ERROR_FAMILY_NONE` (0) and `payload_len` set to 0. Calling this function does not clear the retained record.

- `sidereon_last_engine_error_payload`:
  ```c
  SidereonStatus sidereon_last_engine_error_payload(
      uint8_t *out,
      size_t len,
      size_t *out_written,
      size_t *out_required
  );
  ```
  Copies the retained Schema 1 UTF-8 JSON payload using the standard two-pass caller-buffer convention. Calling with a NULL `out` pointer and `len` 0 writes 0 to `*out_written` and reports the required buffer size in `*out_required`. If `len` is smaller than the required payload length, the function returns `SIDEREON_STATUS_INVALID_ARGUMENT`, writes 0 to `*out_written`, reports the required length in `*out_required`, and rejects the call without copying partial data. Reading the payload or encountering a short buffer retains the payload for subsequent calls.

- `sidereon_clear_engine_error`:
  ```c
  void sidereon_clear_engine_error(void);
  ```
  Explicitly clears the generic engine error retained in thread-local storage on the calling thread. It resets only the generic engine error TLS slot; legacy diagnostic text (`sidereon_last_error_message`) and family-specific error slots remain unchanged.

### Error families and Schema 1 payload

`SidereonEngineErrorFamily` provides stable numeric discriminants across core subsystems: `NONE` (0), `RTK` (1), `STATIC_REFERENCE` (2), `TRLS` (3), `ILS` (4), `SPK` (5), `CDM` (6), `TDM` (7), `FUSION` (8), `FUSION_STATE_CODEC` (9), `ALLAN` (10), `POWER_LAW_NOISE` (11), `FRAME_CATALOG` (12), `SIDEREAL` (13), `ATMOSPHERE` (14), `SOURCE_LOCALIZATION` (15), `GEODETIC_TIME_SERIES` (16), `NORMALITY` (17), `TRACK` (18), `PRECISE_SAMPLES` (19), `PRECISE_INTERPOLANT` (20), `SPACE_WEATHER` (21), `ARAIM` (22), `REDUCED_ORBIT` (23), `REDUCED_ORBIT_SOURCE` (24), `PIECEWISE_ORBIT` (25), `ORBIT_FIT` (26), `ELEMENTS` (27), `EQUINOCTIAL` (28), `RTN_FRAME` (29), `ANOMALY` (30), `PROPAGATION` (31), `DECAY` (32), `DGNSS` (33), `SCENARIO` (34), `CATALOG` (35), `EXACT_CACHE` (36), `TCA` (37), `ALMANAC` (38), `OBSERVE` (39), `BODY_OBSERVATION` (40), `LOOK_ANGLE` (41), `PASS` (42), `EVENT_FINDER` (43), `FRAME_TRANSFORM` (44), `CONJUNCTION` (45), `FACADE` (46), `SPP` (47), `SPP_POLICY` (48), `SUN_MOON` (49), and `UNKNOWN` (999).

The payload is formatted as a versioned UTF-8 JSON tree:
- `schema_version`: integer schema revision (`1`).
- `family`: string name of the family (`"spp"`, `"rtk"`, `"tdm"`, etc.).
- `operation`: string name of the producing C API function.
- `error`: structured error object containing:
  - `kind`: string name of the specific core error variant.
  - `fields`: object containing variant-specific typed fields.

Floating-point numbers in payload fields are represented losslessly as exact float objects:
```json
{
  "decimal": "123.456",
  "bits_hex": "405edd2f1a9fbe77"
}
```
This preserves IEEE-754 binary64 bit patterns, signed zeroes, subnormals, and non-finite values (infinity and NaN) without decimal conversion distortion or floating-point rounding.

### Lifecycle, thread isolation, and coexistence

- **Producing calls**: Every producing C API operation clears the generic engine error slot on entry before validating its arguments.
- **Readers and cleanup**: Query functions (`sidereon_last_engine_error_info`, `sidereon_last_engine_error_payload`), handle inspection methods, and `*_free` functions retain the error slot intact.
- **Thread isolation**: Each operating system thread maintains its own thread-local storage slot. Concurrent calls on separate threads do not interfere with or overwrite each other's error state.
- **Coexistence with legacy and family-specific ABIs**: Clearing the generic slot via `sidereon_clear_engine_error` does not clear legacy diagnostic text (`sidereon_last_error_message`). Existing family-specific error mechanisms (such as terrain error text, bias error records, SGP4 error info, and RTCM error records) remain active and are retained according to their own documented contracts. Legacy status codes and messages remain fully supported compatibility APIs.

### Row-owned batch SPP error ABI

Batch SPP operations provide handle-owned error inspection for individual batch epochs:

```c
SidereonStatus sidereon_spp_batch_error_info(
    const SidereonSppBatch *batch,
    size_t index,
    SidereonEngineErrorInfo *out_info
);

SidereonStatus sidereon_spp_batch_error_payload(
    const SidereonSppBatch *batch,
    size_t index,
    uint8_t *out,
    size_t len,
    size_t *out_written,
    size_t *out_required
);
```

- **Successful epochs**: When epoch `index` solved successfully, `sidereon_spp_batch_error_info` writes `family = SIDEREON_ENGINE_ERROR_FAMILY_NONE` and `payload_len = 0`, and `sidereon_spp_batch_error_payload` sets `*out_written = 0` and `*out_required = 0`.
- **Failed epochs**: When epoch `index` failed, `out_info` reports the row's error family and payload size, and `sidereon_spp_batch_error_payload` copies the row's Schema 1 JSON payload under the standard two-pass caller-buffer protocol.
- **Ownership and lifecycle**: The row error detail belongs to the `SidereonSppBatch` handle itself. It is stored per row independently of global thread-local storage and is not overwritten by subsequent operations executed on the thread.
- **Solution extraction**: When extracting an epoch solution with `sidereon_spp_batch_solution(batch, index, &sol)`, a failed epoch returns `SIDEREON_STATUS_SOLVE`, retains generic thread-local storage, updates legacy diagnostic text (`sidereon_last_error_message`) with the row cause, and leaves the batch's row-owned error detail intact.
- **Buffer lifetime**: Bytes copied into caller-allocated buffers survive subsequent destruction of the batch via `sidereon_spp_batch_free`.
- **Compatibility**: Legacy batch error inspection (`sidereon_spp_batch_error`) and status returns continue to function as compatibility APIs.

RINEX-SPP per-epoch results expose the same row-owned family and Schema 1 JSON
contract through `sidereon_rinex_spp_solution_error_info` and
`sidereon_rinex_spp_solution_error_payload`. Successful rows report family
`SIDEREON_ENGINE_ERROR_FAMILY_NONE` and zero payload length. Failed rows retain
their `SolvePolicyError` family and structured cause in the solutions handle;
the existing `sidereon_rinex_spp_solution_error` diagnostic string remains
available for compatibility.

## IONEX vertical-TEC products

`sidereon_ionex_parse` reads an IONEX 1 product from a byte buffer and yields an
owned `SidereonIonex` handle released with `sidereon_ionex_free`.
`sidereon_ionex_parse_with_warnings` yields the same product plus an owned
`SidereonIonexWarningList` released with `sidereon_ionex_warning_list_free`; a
refused parse transfers no owned handle and leaves every output pointer it can
write NULL. Both outputs are cleared before either is validated, so passing
NULL for one of them still leaves the other NULL rather than whatever it held.
The two output slots must be disjoint from each other and from the bytes at
`data`. Internal reader temporaries are a separate matter: the parser allocates
while it reads and releases before it returns.
The warning list reports its length through
`sidereon_ionex_warning_list_count`, the typed numeric payload of one finding
through `sidereon_ionex_warning_get_info`, and two strings through
`sidereon_ionex_warning_get_label` and `sidereon_ionex_warning_get_message`,
both on the shared caller-buffer convention. An index past the end returns
`SIDEREON_STATUS_INVALID_ARGUMENT` rather than reading anything. The label is
the record label or the map kind the finding concerns, and is empty for a
finding this binding does not name; the message is always the engine's own
text, so a finding added to the engine later still reads. `SidereonIonexWarningInfo`
carries the line, map number, exponent and the line that set it, the declared
map count with the TEC and total map counts, the declared interval with the
spacing actually found, the node latitude and longitude, and the declared and
actual epochs behind a `has_epochs` flag. Both epochs are whole seconds:
`declared_epoch_j2000_whole_s` and `maps_epoch_j2000_whole_s` hold them exactly
as `int64_t`, and `declared_epoch_j2000_s` and `maps_epoch_j2000_s` are those
integers converted to a double once, which is exact up to 2^53 seconds in
magnitude. An epoch in a form other than the reader's own leaves `has_epochs`
false rather than writing a NaN or a rounded neighbour.

`sidereon_ionex_to_ionex_text` serializes a product back to IONEX 1 text on the
same caller-buffer convention. It is fallible: a value no exponent writes
exactly in `I5`, an axis or header field its IONEX field cannot hold, a map
epoch whose civil year the `I6` epoch field cannot hold, and a
`MAPPING FUNCTION` code that does not read back as itself are each refused with
a status and a thread-local message. The `MAPPING FUNCTION` field holds a code
of at most four characters without blanks, so the refused codes are a blank
code, a code containing a blank, a custom code longer than four characters, and
a custom code spelling one the spec names, such as `COSZ` set through
`SIDEREON_IONEX_MAPPING_FUNCTION_KIND_OTHER`, which would read back as the
standard `COSZ` rather than as the custom code. A refusal reports a required
length of zero and writes nothing, so an empty result never reads as a
successful empty file.

`sidereon_ionex_skipped_records` reports how many records a forgiving read
passed over: an `AUX DATA` block counts as one, as do each unrecognized header
record and each summary record that could not be read. A product built from
samples reports zero.

`sidereon_ionex_get_header` clones a product's header into an owned
`SidereonIonexHeader`, and `sidereon_ionex_header_new` and
`sidereon_ionex_header_clone` build one directly; all three are released with
`sidereon_ionex_header_free`. The handle is the only header authority: the
product keeps its own copy, and editing a handle never reaches back into a
product built from it. Typed getters and setters cover the format version, the
satellite system, program, agency and date strings, the interval, the elevation
cutoff, the observables description, the station, satellite and map counts, the
`MAPPING FUNCTION` declaration, and the description and comment lists. An
optional count is read as a value and a separate presence flag, so a file that
omits the record reads as absent rather than as zero. Every string route copies
the exact UTF-8 bytes the product carries, with no terminator added and no
trimming, on the shared caller-buffer convention.
`sidereon_ionex_header_get_mapping_declaration` reports whether the product
declares a mapping function and which code it names, and
`sidereon_ionex_header_get_mapping_function_code` copies the code's text
verbatim, including a code the spec does not name; a product with no record
reports a required length of zero.

Every exported IONEX and TEC-grid entry point states its own pointer
preconditions in the generated header: which arguments may be NULL and when,
what a length counts, whether an output buffer may overlap the handle it reads,
which call transfers ownership and which `_free` releases it, and that text is
copied with no NUL terminator appended. A NULL where the contract does not
allow one is refused with `SIDEREON_STATUS_NULL_POINTER` rather than
dereferenced; the `_free` calls return nothing and accept NULL as a no-op. No
route validates a non-null pointer, so a pointer that is not live and aligned
for the whole call is undefined behavior. Where a call has more than one
writable output -- a value and its presence flag, a count pair, an evaluation
and its error detail, a grid and its error, the four standalone-grid
dimensions -- every non-null output is cleared before any output is validated,
so a NULL in one position never leaves another output holding what the caller
left in it.

`sidereon_ionex_header_new` declares no mapping function: every record sits at
the value the spec gives for an unstated one and the handle carries no
`MAPPING FUNCTION` record. The same header is used wherever a builder takes a
`SidereonIonexHeader *` and the caller passes NULL, including
`sidereon_ionex_from_tec_samples`, `sidereon_ionex_from_tec_samples_with_header`
and the `header` field of `SidereonTecGridSamples`. A product on an undeclared
header reports `has_assumed_mapping` true with kind
`SIDEREON_IONEX_ASSUMED_MAPPING_KIND_ABSENT` on a successful slant row under the
default `SIDEREON_IONEX_MAPPING_POLICY_SINGLE_LAYER`, is refused under
`SIDEREON_IONEX_MAPPING_POLICY_DECLARED`, and writes back no `MAPPING FUNCTION`
record. Call `sidereon_ionex_header_set_mapping_function` with
`SIDEREON_IONEX_MAPPING_FUNCTION_KIND_COS_Z` to declare the factor the engine
applies; a declared `COSZ` reports no assumed mapping, satisfies the `Declared`
policy, and writes back a `MAPPING FUNCTION COSZ` record.

`sidereon_ionex_slant_delay` evaluates one slant ionospheric group delay in
positive meters under the engine default policy, which refuses a query outside
the product's coverage, one whose interpolation weights a node the product
gives as non-available, and a product whose height maps do not give every node
one height. It sets `*out_delay_m` to 0.0 before reading any other argument
and leaves 0.0 there on every refusal, so the status is the only thing that
separates a refusal from a zero-TEC product; the typed routes below return a
NaN delay on a refusal instead. `sidereon_ionex_slant_delay_with_policy` takes an explicit
`SidereonIonexSlantPolicy` and reports a `SidereonIonexSlantDelayEvaluation`,
and `sidereon_ionex_slant_delay_with_coverage_policy` is the same call with
only the coverage field chosen. Both take an optional
`SidereonIonexSlantError` output that may be NULL; when it is not, a refusal
fills it with the typed detail and the evaluation keeps a NaN delay whose
status is not valid, so a refused result cannot be read as a nominal zero. The
text of an `Other` mapping code named by a scalar refusal stays on the product:
read it with `sidereon_ionex_header_get_mapping_function_code` while the handle
lives. Neither the scalar evaluation nor its typed error owns any text of its
own; the engine's message for a scalar refusal is the thread-local one, which
the next failing call in this thread overwrites. A caller who wants an owned
complete result for a single query passes a one-row request array to
`sidereon_ionex_slant_delay_results` and reads row 0: that list owns both the
message and the mapping code.

The evaluation status reports its three conditions independently rather than
collapsing them into one enum. `has_held` with `coverage_error` names the
coverage miss an explicit hold policy held the value through; `has_degraded`
with `gap` names the map, the cell and the four indexed corners a renormalizing
policy interpolated around, with the wrapped column carried in
`lon_index_next`; and `has_assumed_mapping` with `assumed_mapping` names what a
product declares where the single-layer factor mapped a product declaring
anything but `COSZ`. `is_valid` is true when the value was neither held nor
degraded; an assumed mapping does not make it false, because most published
global products declare something other than `COSZ`.

`sidereon_ionex_slant_delays` evaluates a request array into a caller-allocated
array of doubles under the default policy. It allocates nothing, and the first
row the engine refuses fails the whole call and leaves every output element
zero. The output array is validated and zeroed before any other argument is
read, so whenever `out_delays_m` and `count` describe storage the call can
write, that storage holds `count` zeroes on a null product and on a null
request array with a nonzero count too. The output pointer itself is outside
that promise: a NULL one with a nonzero count is `SIDEREON_STATUS_NULL_POINTER`
and a count no slice can span is `SIDEREON_STATUS_INVALID_ARGUMENT`, and
neither writes anything, because neither names storage to initialize. `sidereon_ionex_slant_delay_results` is the loud form: it takes an
explicit policy and yields an owned `SidereonIonexSlantResultList` released with
`sidereon_ionex_slant_result_list_free`. Every input row appears in the list in
its input order, whether the engine gave it a value or refused it, including a
row whose receiver geometry or carrier frequency the engine rejects. The call
itself fails only on a malformed argument: a null product, a null request array
with a nonzero count, an unrecognized policy tag, or a null output pointer, and
no list is transferred then. `sidereon_ionex_slant_result_list_count` reports
the row count, `sidereon_ionex_slant_result_get_row` copies one row, and
`sidereon_ionex_slant_result_get_message` and
`sidereon_ionex_slant_result_get_mapping_code` copy the engine's own text and
the `MAPPING FUNCTION` code the row names. The list owns both strings, so a row
stays readable after the product and header handles it came from are freed and
after any number of later failing calls.

Two kinds of row name a code. A failed row whose refusal is
`SIDEREON_IONEX_SLANT_REFUSAL_KIND_MAPPING_FUNCTION` reports the code its
declaration carries. A successful row whose `assumed_mapping` is
`SIDEREON_IONEX_ASSUMED_MAPPING_KIND_OTHER` reports the custom code the product
declared while the single-layer factor was applied: that kind names the case
without carrying its text, which the engine keeps only on the header, so the
list's copy is the only one that outlives the product. Any other row reports a
required length of zero, and a successful row reports zero for the message
either way, because a mapping code is mapping context rather than a failure.

A zero required length is therefore not by itself an absent declaration; the
row's typed fields keep the cases apart. A failed row separates them with
`mapping_declaration`, where `Declared` with `has_mapping_function` names a code
and `Absent` names no record. A successful row separates them with
`assumed_mapping`, where `Other` names a custom code, `Absent` names no record,
and `NoMapping` or `QFactor` name a standard code the accessor does not repeat.

A blank custom code is not excluded. `sidereon_ionex_header_set_mapping_function`
accepts `SIDEREON_IONEX_MAPPING_FUNCTION_KIND_OTHER` with a zero-length code, a
product built from that header keeps it, and a single-layer evaluation reports
it, so a successful row can report `Other` with a required length of zero
exactly as an `Absent` row does. Only serialization refuses a blank code, as it
refuses every code that does not read back as itself; that refusal belongs to
`sidereon_ionex_to_ionex_text` and does not reach construction or evaluation. Read `assumed_mapping`, not the length, to tell an
empty custom code from no declaration at all.

A row's `is_ok` selects which payload carries meaning. A successful row reads
`evaluation`; a failed row reads `status` and `error`, whose `kind` separates a
rejected input, a coverage miss, a weighted node without a value, and a product
that gives no slant delay under the requested policy, with a further value for
a failure this binding does not name. A coverage failure carries the same
coverage kind the status uses; a node failure carries the indexed corner masks;
and a slant refusal carries the height map number with the latitude and
longitude indices of the node, or the mapping declaration and the declared
code's kind.

The sample intermediate representation moves a whole grid across the boundary
without text. `SidereonTecGridSamples` takes caller-owned buffers in
`[map][latitude][longitude]` order with longitude varying fastest, and
`sidereon_ionex_from_tec_grid_samples` builds a product from it;
`sidereon_ionex_from_tec_samples` and
`sidereon_ionex_from_tec_samples_with_header` build one from a flat stream of
`SidereonTecSample` nodes. In both directions the presence flag is the
authority and the numeric slot follows it. On input a false flag marks a node
without a value and the matching number is ignored, so a caller need not write
NaN there; a present value must be finite, and an explicit zero is a value. On
output a node without a value writes NaN with its flag false.

An IONEX map epoch is a whole second, and the engine reads every epoch as
exactly one second or refuses it; nothing is rounded. A sample epoch arrives
either as a double of seconds since J2000 or as an `int64_t` of whole seconds:
`SidereonTecGridSamples` reads `map_epochs_j2000_whole_s` when it is non-NULL
and `map_epochs_j2000_s` otherwise, and a `SidereonTecSample` reads
`epoch_j2000_whole_s` when `has_epoch_j2000_whole_s` is true and
`epoch_j2000_s` otherwise. A double must hold a whole second exactly. One that
holds a fraction of a second, is not finite, or lies outside the `int64_t`
range names no map epoch and is refused as
`SIDEREON_TEC_SAMPLES_ERROR_KIND_EPOCH_NOT_REPRESENTABLE`, naming the input and
its index; the binding converts no double by rounding. A double states every
whole second only up to 2^53 seconds in magnitude, so a caller with an epoch
past that uses the integer form, which states every second. Zero-filled structs
select the double form, so existing callers keep working.

The builders return `SIDEREON_STATUS_INVALID_ARGUMENT` on a refusal of the
samples, with text naming the input and index in the thread-local message.
`sidereon_ionex_from_tec_grid_samples_result` and
`sidereon_ionex_from_tec_samples_result` take the same inputs and return the
refusal typed: they return `SIDEREON_STATUS_OK` whenever the call itself was
well formed, transfer the product only when one was built, and always transfer
an owned `SidereonTecSamplesResult`, released with
`sidereon_tec_samples_result_free`. `sidereon_tec_samples_result_get_outcome`
copies a `SidereonTecSamplesOutcome` whose `SidereonTecSamplesError` names every
engine `TecSamplesError` kind, with the short axis's node count or the refused
axis value where the kind carries one, and for the binding's own checks the
input (`SIDEREON_TEC_SAMPLES_INPUT_*`) and index a refusal names: a map epoch
or sample epoch that is not a whole second, a present value that is not finite,
or a value count that disagrees with the axes, with both counts.
`sidereon_tec_samples_result_get_message` copies the owned text. A structural
failure -- a NULL output, a NULL buffer with a nonzero count, a count no slice
can span, an unknown time scale tag -- leaves both outputs NULL, returns a
status that is not OK and allocates nothing.

NULL means a different thing in each of `SidereonTecGridSamples`' three groups
of pointer fields. The mandatory value buffers -- `map_epochs_j2000_s`,
`lat_nodes_deg`, `lon_nodes_deg` and `tec_maps_tecu` -- are read over the count
beside each and may be NULL only when that count is 0; since
`tec_map_value_count` must equal the product of the three node counts, a NULL
`tec_maps_tecu` means a grid with no cells at all. The presence buffers --
`tec_maps_present`, `rms_maps_present` and `height_maps_present` -- are
optional at every count: a NULL one reads all of that map's values as
present. The RMS and height stacks are gated by `has_rms_maps` and
`has_height_maps`; when a flag is false its buffers and its value count are not
read at all and may hold anything. `has_rms_maps`
and `has_height_maps` distinguish a product that carries no optional map from
one that carries a map whose every node is missing, which is not the same
thing. An absent map reports its flag false and a required length of zero; a
present map whose every node is missing reports its flag true, the full
flattened length, every presence flag false and every value NaN. That
distinction is asserted separately for each optional map, so a regression in one
does not hide behind the other. The authority for either map stays in the
engine; this binding reports what the engine holds and does not duplicate it.

Neither sample axis is required to run in a particular direction. A latitude
axis may run north to south or south to north and a longitude axis either way,
as long as `dlat_deg` and `dlon_deg` carry the sign that takes the first node of
their axis to the last; the engine refuses a step whose sign contradicts its
bounds. `sidereon_ionex_lat_nodes_deg` and `sidereon_ionex_lon_nodes_deg` copy
a product's axes in the product's own order. The standalone `SidereonTecGrid` is
a separate type with its own rule: its three axes must be strictly increasing.

`sidereon_ionex_tec_grid_samples_info` reports the axis lengths, the
signed steps, the shell geometry, the exponent and the flat value counts that
size every extraction buffer, and the extraction routes are
`sidereon_ionex_tec_grid_samples_epochs_j2000_s`,
`..._tec_maps_tecu`, `..._tec_presence`, `..._rms_maps_tecu`,
`..._rms_presence`, `..._height_maps_km`, `..._height_presence` and
`sidereon_ionex_tec_samples`. An absent optional map reports a required length
of zero rather than a buffer of zeros. Vertical TEC and RMS are TECU, height
offsets are kilometers above `HGT1`, angles are degrees, and a node epoch is
seconds since J2000 in the time scale the sample names. Every epoch output comes
from the engine's exact whole-second axis: `sidereon_ionex_map_epochs_j2000_s`
copies it as `int64_t`, exact at every magnitude;
`sidereon_ionex_tec_grid_samples_epochs_j2000_s` copies the same seconds each
converted to a double once; and each `SidereonTecSample` carries both, with
`has_epoch_j2000_whole_s` true, so the samples read back through the builders
as the epochs they came from.

The standalone regular TEC grid is a separate source with its own handle.
`sidereon_tec_grid_new` takes three strictly increasing axes and a flat value
buffer in epoch-latitude-longitude order with longitude varying fastest, plus
an optional presence buffer that may be NULL to mean every value is present. It
yields an owned `SidereonTecGrid` released with `sidereon_tec_grid_free`, and
an optional `SidereonTecGridError` output that may be NULL. The axis lengths
are reconciled before any buffer is read, so a count that overflows or
disagrees with the value count is refused without a slice ever being formed
over it, and no handle is transferred on any failure. Epoch coordinates are
`f64` Unix nanoseconds; because the axis is `f64`, adjacent nanoseconds are not
distinguishable at large magnitudes, and a query takes an exact `int64_t` that
the engine converts to that axis. Latitude and longitude are degrees on the
axes and TEC values are TECU. `sidereon_tec_grid_dimensions` reports the three
axis lengths and the flat value count, and
`sidereon_tec_grid_epochs_ns`, `sidereon_tec_grid_latitudes_deg`,
`sidereon_tec_grid_longitudes_deg`, `sidereon_tec_grid_values_tecu` and
`sidereon_tec_grid_value_presence` copy the stored axes and values out on the
shared caller-buffer convention. These copy from the engine's own immutable
getters and duplicate no state.

`sidereon_tec_grid_vtec_at_pierce_point` takes the pierce-point latitude and
longitude in degrees, the unit of the grid's own axes, and hands them to the
engine unchanged in the order the engine takes them, so a query on a node is
evaluated at that node exactly rather than a unit round trip away from it. It
reports the non-available nodes in both directions: the ones a renormalizing
policy interpolated around, and the ones a strict refusal names, so a strict
refusal no longer loses its indexed corners. A refusal leaves a NaN value and,
when the error output is not NULL, a typed detail naming the axis a query left
with the effective coordinate it left it at, the supplied and required value
counts of a count mismatch, or the indexed corner masks of a node failure.

The engine clamps the latitude to
`[-87.5, 87.5]` degrees, the band an IONEX grid covers, and evaluates at that
effective coordinate. The clamp applies on both pierce-point routes, the
convenience one and the complete typed one, and neither reports it: on a grid
whose latitude axis is `[80.0, 89.0]`, a query at 89 degrees returns the value
at 87.5 degrees with `SIDEREON_STATUS_OK`, `is_ok` true and no held,
degraded or gap marker, because nothing was interpolated around and no node was
missing. The longitude and the epoch are not clamped. Where the clamped
latitude still falls outside a narrower axis, the failure is an ordinary
out-of-bounds one and `axis_value` is the effective coordinate, 87.5, not the
89 the caller supplied. A caller who needs the supplied latitude back
has to keep it; the binding does not carry it through.

The clamp is a pair of comparisons, so it decides the two non-finite latitudes
differently, and both routes behave alike. A latitude of `+INFINITY` is above
the upper bound and evaluates at 87.5; `-INFINITY` is below the lower bound
and evaluates at -87.5. Either is then an ordinary success or an ordinary
out-of-bounds failure depending on the axis, and `axis_value` names the bound,
never an infinity. `NAN` compares false against both bounds, so no clamp
applies and the engine's shared validation refuses the query as an
`InvalidField` with field `latitude` and reason `not finite` -- which
`sidereon_tec_grid_vtec_at_pierce_point_result` hands back as owned text and
the convenience route leaves only in the thread-local message. The binding adds
no finite-angle check of its own in front of either case: the accepted inputs
are the engine's.

`sidereon_tec_grid_new` and `sidereon_tec_grid_vtec_at_pierce_point` are the
convenience routes. `SidereonTecGridError` is fixed width, so the two parts of
a failure that are text live in the thread-local message, which the next
failing call in this thread overwrites: the whole message, and the field label
and reason of an `InvalidField`, which that record names by kind alone.

`sidereon_tec_grid_new_result` and
`sidereon_tec_grid_vtec_at_pierce_point_result` are the complete typed routes.
They take the same inputs, keep the same angular contract, and return
`SIDEREON_STATUS_OK` whenever the call itself was well formed, handing back an
owned `SidereonTecGridResult` released with `sidereon_tec_grid_result_free`.
`sidereon_tec_grid_result_get_outcome` copies the fixed-width
`SidereonTecGridOutcome`: `is_ok`, the status the failure maps to, the vertical
TEC when there is one, the indexed corners a renormalized value was
interpolated around, and the same typed `SidereonTecGridError`.
`sidereon_tec_grid_result_get_message`, `sidereon_tec_grid_result_get_field`
and `sidereon_tec_grid_result_get_reason` copy the engine's whole text and the
exact `InvalidField` label and reason on the shared caller-buffer convention.
The result owns all three, so every detail still reads after the grid is freed
and after any number of later failing calls. A failure the engine adds after
this binding was written keeps its own text the same way, rather than being
reduced to a transient message and a broad kind.

A construction or query failure travels inside the result: the outer status is
`SIDEREON_STATUS_OK`, `*out_grid` stays NULL, and the result reports `is_ok`
false, which is how `sidereon_ionex_slant_delay_results` already carries a
refused row. A structural failure of the call itself - a null out-parameter, a
null buffer with a nonzero count, a count no slice can span, or a policy tag
this binding does not name - returns a status that is not OK, sets every
out-parameter to NULL, and allocates nothing, so no owned handle is left behind
a failing status. `sidereon_tec_grid_new_result` clears both of its outputs
before it validates either, so a null argument in either position still leaves
the other reading NULL rather than whatever the caller left in it. The grid and the result are independent allocations, each
with its own `_free`, and passing NULL to either is a no-op.

Both routes report the same typed kinds: the axes being too short, an axis that
does not increase, a dimension overflow, a value count mismatch carrying both
counts, an `InvalidField` naming an input the engine's shared validation
rejected, a node failure carrying the full indexed corner masks, a bound
failure carrying the axis and the coordinate it refused, and a kind for a
failure this binding does not yet name.
`SIDEREON_TEC_GRID_ERROR_KIND_VALUE_NOT_FINITE` is not one of the engine's: it
is this binding's own check on a value the caller marked present, made before
the grid is built, and `value_index` names the rejected entry. The engine never
saw that value, so the failure claims no engine field and no engine reason.

Policy fields are numeric tags validated by value; an unrecognized tag is
refused and no numeric tag is ever read as a Rust enum. The generated header
names every tag, so no call site has to spell a number. Coverage uses
`SIDEREON_IONEX_COVERAGE_POLICY_STRICT` and
`SIDEREON_IONEX_COVERAGE_POLICY_HOLD`; the missing-node field uses
`SIDEREON_IONEX_MISSING_NODE_POLICY_STRICT` and
`SIDEREON_IONEX_MISSING_NODE_POLICY_RENORMALIZE`, which
`sidereon_tec_grid_vtec_at_pierce_point` and its `_result` form take directly;
and the mapping field uses `SIDEREON_IONEX_MAPPING_POLICY_DECLARED` and
`SIDEREON_IONEX_MAPPING_POLICY_SINGLE_LAYER`, which is the default. Build a
policy with `sidereon_ionex_slant_policy_default`,
`sidereon_ionex_slant_policy_from_coverage` or
`sidereon_ionex_slant_policy_init`. The emission-epoch media batches take the
same policy for their IONEX correction: `SidereonEmissionMediaOptions` carries
`ionex_policy_enabled` and `ionex_policy`, and with the flag false, as
`sidereon_emission_media_options_init` and a zero-filled struct leave it, the
product is evaluated under the engine default policy.

The callback-driven ECEF entries the engine exposes as `tec_xyz` and
`iono_delay_xyz` are not bound here. They take a caller function that maps an
ECEF pierce point to longitude, latitude and altitude, and this binding has no
callback convention to hold one safely, so exposing them is separate ABI work
rather than an omission this surface closes over.

The layouts a foreign interface binds against:

```c
typedef struct {
    uint32_t coverage;      /* SIDEREON_IONEX_COVERAGE_POLICY_*     */
    uint32_t missing_nodes; /* SIDEREON_IONEX_MISSING_NODE_POLICY_* */
    uint32_t mapping;       /* SIDEREON_IONEX_MAPPING_POLICY_*      */
} SidereonIonexSlantPolicy;

typedef struct {
    bool has_missing;
    size_t map_number;      /* counts from 1 */
    size_t lat_index;
    size_t lon_index;
    size_t lon_index_next;  /* 0 on the cell that closes the circle */
    bool missing[4];
} SidereonIonexMissingNodes;

typedef struct {
    bool has_gap;
    SidereonIonexMissingNodes earlier;
    SidereonIonexMissingNodes later;
} SidereonIonexNodeGap;

typedef struct {
    bool is_valid;
    bool has_held;
    SidereonIonexCoverageErrorKind coverage_error;
    bool has_degraded;
    SidereonIonexNodeGap gap;
    bool has_assumed_mapping;
    SidereonIonexAssumedMappingKind assumed_mapping;
} SidereonIonexSlantDelayStatus;

typedef struct {
    double delay_m;         /* NaN on a refusal */
    SidereonIonexSlantDelayStatus status;
} SidereonIonexSlantDelayEvaluation;

typedef struct {
    SidereonIonexSlantErrorKind kind;
    SidereonIonexCoverageErrorKind coverage_error;
    bool has_gap;
    SidereonIonexNodeGap gap;
    SidereonIonexSlantRefusalKind refusal;
    size_t refusal_map_number;
    size_t refusal_lat_index;
    size_t refusal_lon_index;
    bool has_mapping_declaration;
    SidereonIonexMappingDeclarationKind mapping_declaration;
    bool has_mapping_function;
    SidereonIonexMappingFunctionKind mapping_function;
} SidereonIonexSlantError;

typedef struct {
    double lat_deg;
    double lon_deg;
    double azimuth_deg;
    double elevation_deg;
    int64_t epoch_j2000_s;
    double frequency_hz;
} SidereonIonexSlantRequest;

typedef struct {
    bool is_ok;
    SidereonStatus status;
    SidereonIonexSlantDelayEvaluation evaluation; /* when is_ok */
    SidereonIonexSlantError error;                /* when !is_ok */
} SidereonIonexSlantRowResult;

typedef struct {
    SidereonIonexWarningKind kind;
    size_t line;
    size_t map_number;
    size_t set_by_line;
    uint64_t declared_count;
    size_t tec_map_count;
    size_t all_map_count;
    uint32_t declared_interval_s;
    int64_t actual_spacing_s;
    int32_t exponent;
    double lat_deg;
    double lon_deg;
    bool has_epochs;
    double declared_epoch_j2000_s;       /* the whole second as a double */
    double maps_epoch_j2000_s;
    int64_t declared_epoch_j2000_whole_s; /* exact */
    int64_t maps_epoch_j2000_whole_s;
} SidereonIonexWarningInfo;

typedef struct {
    SidereonTecGridErrorKind kind;
    bool has_gap;
    SidereonIonexNodeGap gap;
    SidereonTecGridAxis axis;
    bool has_axis_value;
    double axis_value;
    size_t value_count;
    size_t expected_value_count;
    bool has_value_index;
    size_t value_index;
} SidereonTecGridError;

typedef struct {
    bool is_ok;
    SidereonStatus status;
    bool has_vtec;
    double vtec_tecu;              /* when has_vtec, else NaN */
    SidereonIonexNodeGap degraded; /* when a value was produced around a gap */
    SidereonTecGridError error;    /* when !is_ok */
} SidereonTecGridOutcome;

typedef struct {
    uint32_t time_scale;
    double epoch_j2000_s;          /* read when !has_epoch_j2000_whole_s */
    bool has_epoch_j2000_whole_s;
    int64_t epoch_j2000_whole_s;
    double lat_deg;
    double lon_deg;
    bool has_vtec_tecu;
    double vtec_tecu;
    bool has_rms_tecu;
    double rms_tecu;
    bool has_height_offset_km;
    double height_offset_km;
} SidereonTecSample;

typedef struct {
    uint32_t time_scale;
    const double *map_epochs_j2000_s;        /* read when the next is NULL */
    const int64_t *map_epochs_j2000_whole_s; /* NULL: read the doubles */
    size_t map_epoch_count;
    const double *lat_nodes_deg;
    size_t lat_node_count;
    const double *lon_nodes_deg;
    size_t lon_node_count;
    double dlat_deg;
    double dlon_deg;
    double shell_height_km;
    double base_radius_km;
    int32_t exponent;
    const double *tec_maps_tecu;
    const bool *tec_maps_present;   /* NULL means every value is present */
    size_t tec_map_value_count;
    bool has_rms_maps;
    const double *rms_maps_tecu;
    const bool *rms_maps_present;
    size_t rms_map_value_count;
    bool has_height_maps;
    const double *height_maps_km;
    const bool *height_maps_present;
    size_t height_map_value_count;
    const SidereonIonexHeader *header; /* NULL: unstated records, no MAPPING FUNCTION */
} SidereonTecGridSamples;

typedef struct {
    size_t map_epoch_count;
    size_t lat_node_count;
    size_t lon_node_count;
    double dlat_deg;
    double dlon_deg;
    double shell_height_km;
    double base_radius_km;
    int32_t exponent;
    bool has_rms_maps;
    size_t tec_map_value_count;
    size_t rms_map_value_count;
    bool has_height_maps;
    size_t height_map_value_count;
} SidereonTecGridSamplesInfo;

typedef struct {
    SidereonTecSamplesErrorKind kind;
    SidereonTecSamplesInput input; /* when has_index */
    bool has_index;
    size_t index;
    size_t node_count;             /* TooFewNodes */
    size_t value_count;            /* count mismatches the binding found */
    size_t expected_value_count;
    bool has_axis_value;
    double axis_value;             /* AxisOutOfRange */
} SidereonTecSamplesError;

typedef struct {
    bool is_ok;
    SidereonStatus status;
    SidereonTecSamplesError error; /* when !is_ok */
} SidereonTecSamplesOutcome;
```

## Integrity

The direct post-solve integrity APIs are available without ephemeris handles or
solver coupling:

- Call `sidereon_raim` with satellite tokens, post-fit residuals, their
  variances, a `SidereonRaimWeightsMode`, false-alarm probability, and optional
  GNSS clock-system count. The default mode, `SIDEREON_RAIM_WEIGHTS_MODE_SOLUTION`,
  divides each residual by the standard deviation it was weighted by, as RTKLIB
  demo5 `valsol` does; `BY_SATELLITE` takes per-satellite inverse-variance
  weights instead, and `UNIT` uses sigma = 1 m, which on metre-scale residuals
  makes `fault_detected` saturate near 100%. The output is `SidereonRaimResult`
  with `fault_detected`, `test_statistic`, `threshold`, `worst_sat`, `dof`,
  `reduced_chi_square`, `rms_m`, and the weighted residual count. Use
  `sidereon_raim_normalized_residuals` to copy the per-satellite weighted
  residual rows with the standard `(NULL, 0)` size query contract.
  `sidereon_raim_for_solution` reads the residuals, variances and clock count
  from an SPP solution. Under `BY_SATELLITE` the weights are inverse variances:

  ```c
  SidereonFdeRaimWeight weights[6];
  for (size_t i = 0; i < 6; i++) {
      double sin_el = fmax(sin(elevation_rad[i]), 0.2);
      double variance_m2 = (0.8 / sin_el) * (0.8 / sin_el);
      weights[i] = (SidereonFdeRaimWeight){sat_ids[i], 1.0 / variance_m2};
  }
  ```
- Call `sidereon_araim` with `SidereonAraimGeometry`, `SidereonAraimIsm`, and
  `SidereonAraimIntegrityAllocation`; read `hpl_m`, `vpl_m`,
  `sigma_acc_h_m`, and `sigma_acc_v_m` through
  `sidereon_araim_result_summary`, then release the result with
  `sidereon_araim_result_free`.

## Smoke test

`tests/run_smoke.sh` builds the library, regenerates the header with cbindgen
0.29.4, compiles `tests/smoke.c`, and runs it on a committed crate-side SP3
fixture, asserting the binding reproduces the engine reference position
bit-exact:

    ./tests/run_smoke.sh

CI runs `tests/run_ci_smoke.sh` on Linux and macOS. It compares regenerated and
committed headers byte-for-byte, then compiles, links, and executes the focused
programs using only repository fixtures. In addition to the existing gates, the
script runs `fixed_policy_smoke` with no arguments,
`rinex_nav_clock_smoke` with `fixtures/nav/ESBC00DNK_R_20201770000_01D_MN.rnx`
and `fixtures/clk/synthetic_rinex_clock.clk`,
`rinex_rtk_dted_smoke` with the committed SP3, WTZR/WTZZ observation fixtures,
and `fixtures/dted/tiles`, and `format_contracts_smoke` with
`fixtures/antex/igs20_wettzell_trim.atx`.

The smoke scripts compile with `-ffp-contract=off`, so C arithmetic that
forms an input the tests compare against engine output (the WTZR antenna
reference point, for one) rounds each operation separately, as Rust does.

## Generated goldens

The goldens `tests/run_generators.sh` writes come from sidereon-core at the
revision `tests/CORE_REVISION` names. (`tests/spk_fixture.h` holds CSPICE
reference states, not engine output, and no generator writes it.) The Rust
generators (`fbgen`, `velgen`, `rinexgen`, `sppgen`, `pingen`, `rtkgen`, `valgen`) depend
on sidereon-core from its git repository at that revision, with committed
`Cargo.lock` files, and each generated file records the revision. `tests/run_generators.sh` runs
sidereon-core's own fixture emitters, the Rust generators, the PPP expected
block writer and the Python transcribers in order:

    ./tests/run_generators.sh
    git diff --exit-code -- .

CI runs both commands on macOS; a committed golden the generators no longer
reproduce fails the build. The propagation and SP3 goldens are read from the
files sidereon-core's `SIDEREON_DUMP_FIXTURES` emitters write in the core
checkout the generators resolve; the former `PY_FIXTURES` variable, which
pointed the transcribers at another binding's fixture copies, is removed.
