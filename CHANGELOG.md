# Changelog

## Unreleased

## 3.0.2 - 2026-10-05

### Fixed

- Validate NMEA epoch summary instants through the canonical UTC conversion, so a fractional leap second keeps its calendar fields without reporting an invalid instant.
- Return `SIDEREON_STATUS_UT1_OUTSIDE_COVERAGE` from PPP correction builders when strict UT1 coverage refuses the requested epoch, while preserving the existing typed error detail and permissive behavior.

## 3.0.1 - 2026-10-04

### Changed

- Coordinate C with sidereon-core 3.0.1 at 592ca2bd293177bcd901286080baa6350522020f; the engine numerical algorithms are unchanged in this patch release.
- Add public UT1 ABI regressions for table-backed, before-coverage and after-coverage flags, including typed frame-transform refusal.

## 3.0.0 - 2026-10-04

### Changed

- Engine update: sidereon-core 3.0.0 at e2fb3df (core revision `e2fb3dfdc392d23087ed8aa1ee028a0056b4021b`). Every breaking engine change below reaches the C surface.
- **Breaking.** `SidereonStatus` gains `SIDEREON_STATUS_UT1_OUTSIDE_COVERAGE`, returned where an entry point reads UT1 outside the UT1 table and the UT1 policy refuses it: pass searches, frame transforms, SPP, DGNSS, the static solve, ARAIM and reliability, SBAS protection levels, fusion, orbit fits, scenario synthesis and the RTK RINEX arc. `SidereonSbasPlError` gains `UT1_OUTSIDE_COVERAGE` (4) and `SidereonStaticPositionErrorKind` gains `UT1_OUTSIDE_COVERAGE` (9). An orbit fit whose orientation or propagation provider states another UT1 policy fails with `SIDEREON_STATUS_INVALID_ARGUMENT`.
- **Breaking.** `SidereonTimeScales` gains `ut1_degraded` (`SidereonUt1Degradation`), set when UT1 lies outside the UT1 table and was taken from the long-term delta-T curve; the frame transforms refuse time scales so marked. `SidereonSppMetadata` and `SidereonStaticPositionMetadata` gain `ut1_degraded`, the departure a permissive source accepted.
- **Breaking.** `SidereonSppInputs` gains `pseudorange_code` (`SidereonPseudorangeCode`, zero for single-frequency code). The broadcast group delay applies to single-frequency code only, as RTKLIB `prange` applies it; an ionosphere-free code takes none. Any other value is refused. Zero-initialized inputs keep single-frequency behaviour; callers that fill the struct field by field must set it.
- **Breaking.** `SidereonBroadcastRecordInfo` and `SidereonBroadcastRecord` gain `has_issue` and `has_sv_accuracy_m`: a GPS/QZSS CNAV-family record carries no issue of data, and CNAV URA_ED indices 15 and -16 carry no accuracy. `sidereon_broadcast_ephemeris_select_by_issue` finds no CNAV-family record. `SidereonBroadcastRecord` gains `stated` (`SidereonStatedNavFields`), the record fields the orbit and clock models do not read, carried back through `sidereon_encode_rinex_nav`. `SidereonNavMessage` gains `GALILEO_UNCLASSIFIED` (10) and `NAVIC_LNAV` (11).
- **Breaking.** `SidereonGlonassRecord` gains the stated epoch, the frequency channel as stated, the message frame time, the age of operation, the status flags, the L1/L2 group delay field, the URAI and the health flags, each optional field with its `has_*` flag. `SidereonIonoCorrections` gains the QZSS and NavIC Klobuchar sets, the Galileo disturbance flags and the BeiDou BDGIM coefficients.
- **Breaking.** `sidereon_encode_rinex_nav`, `sidereon_rinex_encode_nav` and `sidereon_rinex_repair_nav` fail with `SIDEREON_STATUS_INVALID_ARGUMENT` naming the record the NAV writer refuses.
- **Breaking.** Bias-SINEX parsing is strict: `sidereon_bias_sinex_parse`, `_parse_lossy`, `_load` and `_load_lossy` refuse a file that departs from Bias-SINEX 1.00. `sidereon_bias_sinex_parse_with_policy`, `sidereon_bias_sinex_load_with_policy`, `sidereon_code_dcb_parse_with_policy` and `sidereon_code_dcb_load_with_policy` take a `SidereonBiasReadPolicy`; a lenient read records each departure, read with `sidereon_bias_set_notice_count` and `sidereon_bias_set_notice`.
- **Breaking.** `sidereon_bias_set_code_osb_seconds`, `sidereon_bias_set_phase_osb_cycles` and `sidereon_bias_set_code_dsb_seconds` write a `SidereonBiasLookup` in place of a presence flag and value: the outcome (`SidereonBiasLookupStatus`), the value, the records it comes from and the records a later start overrides, into optional caller buffers. `sidereon_bias_set_phase_osb_cycles` takes an optional carrier frequency, which converts a phase bias stated in nanoseconds. `sidereon_bias_set_mode` gains `out_has_time_scale`, false for a product without a usable time scale. `SidereonBiasRecord` gains `family`, `unit` and the source line; a phase bias stated in nanoseconds reads as a phase bias.
- **Breaking.** `sidereon_oem_to_kvn`, `sidereon_oem_to_xml`, `sidereon_opm_to_kvn`, `sidereon_opm_to_xml`, `sidereon_omm_to_kvn`, `sidereon_omm_to_xml` and `sidereon_omm_to_json` fail with `SIDEREON_STATUS_INVALID_ARGUMENT` when the CCSDS writer refuses the message. The OMM, OPM, OEM and CDM readers are stricter as sidereon-core states. `SidereonSkippedOmm` gains `norad_id_present`.
- **Breaking.** `sidereon_rtcm_decode_messages` reads a stream in full or fails with `SIDEREON_STATUS_SP3_PARSE` naming what it did not read. `sidereon_rtcm_decode_stream_with_policy` takes a `SidereonRtcmPolicy`; the diagnostics gain `sidereon_rtcm_stream_diagnostics_crc_failures`, `_departure_count` and `_departure`, and `SidereonRtcmFrameSkipReason` gains `DEPARTURE`. `SidereonRtcmMsmInfo` gains `signal_mask`, the DF395 mask as transmitted (zero builds it from the cells), and `SidereonRtcmGlonassEphemeris` gains `negative_zero`. Every RTCM encoder refuses a value its field cannot hold.
- **Breaking.** `sidereon_ssr_store_from_rtcm` refuses input it cannot read and apply in full; `sidereon_ssr_store_from_rtcm_reading` reads every readable frame and reports the stream diagnostics, a trailing partial frame and the refused messages (`SidereonSsrIngestRefusals`). `sidereon_ssr_store_code_bias_m` and `sidereon_ssr_store_phase_bias_m` take the bias source (0 RTCM SSR, 1 Galileo HAS) with the raw signal index, which names the physical signal the source's table assigns. `SidereonSsrOrbitCorrection` and `SidereonSsrClockCorrection` gain the HAS navigation-message index.
- **Breaking.** `sidereon_sbas_block_encode` refuses a block whose field values its wire form cannot hold. The SBAS log parsers refuse record lines whose fields cannot be read at known positions. `SidereonSbasLogBlock` gains the message type the log line declared and the type its payload carries.
- **Breaking.** TLE reading is strict: `sidereon_tle_load` and `sidereon_parse_tle_file` refuse a column-69 checksum that disagrees or is not a digit. `sidereon_tle_load_with_policy` and `sidereon_parse_tle_file_with_policy` take a `SidereonTlePolicy`. `SidereonTleChecksumWarning` replaces `expected` with `kind` and `found`. `SidereonTleMetadata` gains `has_ephemeris_type`, `has_elset_number` and `has_rev_number`: a blank field reads as absent. A TLE file lists every rejected record (`sidereon_tle_file_rejected`, `_rejected_name`, `_rejected_error`) and each record's line (`sidereon_tle_file_line_number`); `sidereon_tle_file_skipped` counts the rejected records.
- **Breaking.** `SidereonSpkState` loses `has_velocity_km_s`: every supported segment type yields velocity, type 2 from its Chebyshev derivative. Segments are chosen as CSPICE `SPKSFS` and `SPKGEO` choose them. `sidereon_spk_state_in_frame` returns the state in a named NAIF inertial frame.
- **Breaking.** `SidereonSpaceWeatherPolicy` gains `allow_not_observed`, and `SidereonSpaceWeatherObservationClass` gains `NOT_OBSERVED`: a row whose flux qualifier states no observation is refused by default. `sidereon_space_weather_policy_default` and `sidereon_space_weather_policy_lenient` write the engine policies.
- **Breaking.** `SidereonPppObservation` gains the four signal codes of the observation, all NULL or all set, and `has_glonass_channel` with `glonass_channel`, the GLONASS FDMA channel of the observation. The C PPP routes take no SSR or HAS product, so no SSR or HAS bias is applied to their observations. `sidereon_solve_ppp_fixed` leaves the float-solution check to the engine, which accepts a float solution that left input epochs unsolved. `SidereonPppFloatMetadata` gains the solved-epoch count, the SSR/HAS bias exclusion count, the count of observations left unplaced, the residual screen and its removal count, and the solve options; `SidereonPppFixedMetadata` gains the solved-epoch, exclusion and unplaced counts. New accessors: `sidereon_ppp_float_solution_solved_epochs`, `_epoch_clocks`, `_ssr_bias_exclusions`, `_unplaced_observations`, `_residual_screen_removals`, and `sidereon_ppp_fixed_solution_solved_epochs`, `_epoch_clocks`, `_ssr_bias_exclusions`, `_unplaced_observations`. Each `SidereonPppSsrBiasExclusion` carries the transmission-epoch failure in full (`SidereonPppTransmitTimeFailure`: the `SidereonPppTransmitTimeFailureKind` and the values the failure states) and the application report (`SidereonPppSsrApplication`: the solution identity, the code and phase bias status of each signal, and the ionosphere-free combination status).
- **Breaking.** SSR: the RTCM SSR clock correction is applied with the sign RTKLIB applies it with; the previous sign put the satellite clock off by twice the correction. The corrected clock is formed as RTKLIB `satpos_ssr` forms it, a correction older than 90 s is not applied, and the correction epoch is placed in the week nearest the query as RTKLIB `adjweek` places it. These reach `sidereon_ssr_corrected_state`, `sidereon_ssr_solve_broadcast` and `sidereon_ssr_ephemeris_sample`.
- **Breaking.** Broadcast ephemeris: a satellite's record is selected within the validity windows RTKLIB `seleph` uses; unhealthy records are kept in the store and a selected record RTKLIB `satexclude` excludes yields no state; the broadcast clock's relativistic term takes the RTKLIB `eph2pos` form. `sidereon_broadcast_ephemeris_parse_nav` reads past a NAV block it cannot read, leaving it out, where it refused the whole file; only an unreadable header fails.
- **Breaking.** SPP and the static solve select, mask and weight the satellites at every iterate, as RTKLIB `estpos` re-runs `rescode`, where they selected once at the initial guess. Every SPP, static, DGNSS and fusion solution whose initial guess was not already its solution moves; a cold start from the geocentre and the coarse search now land on the warm-start solution. `SidereonSppSolveStatus` gains `SELECTION_SETTLED` (4), the status of a settled solve, and `OUTER_BUDGET_EXHAUSTED` (5), a robust solve whose outer budget ran out first. In `SidereonSppMetadata` and `SidereonStaticPositionMetadata`, `status` and `converged` describe how the whole solve ended, and `iterations` counts the trust-region iterations of every solve plus one per least-squares step. A solve whose selection has not settled after 10 passes fails with `SIDEREON_STATUS_SOLVE`; the static solve reports it as `SIDEREON_STATIC_POSITION_ERROR_KIND_SELECTION_UNSETTLED` (10). The carrier of a satellite is required only when the ionosphere correction is applied, and the PPP auto-init seed passes each GLONASS observation's channel.
- **Breaking.** `sidereon_tle_to_lines` and the fitted-TLE lines spell B\* and the second mean-motion derivative as python-sgp4's `export_tle` does: a zero B\* is ` 00000+0`, a zero second derivative ` 00000-0`, and values near a tie take python-sgp4's digits. OMM readers bridge B\* and the second derivative as stated where python-sgp4 reads the OMM, and an OMM epoch of whole microseconds takes python-sgp4's split, which moves the NAVSTAR 43 and GALAXY 15 epochs by about 0.3 microseconds. Passes, look angles and ground tracks propagate SGP4 at the split Skyfield uses for the same UTC instant.
- **Breaking.** The 1997 and 1999 leap seconds are dated 1997-07-01 and 1999-01-01 (IERS Bulletin C); every UTC conversion from 1997-01-01 to 1997-06-30 and from 1998-01-01 to 1998-12-31 moves by one second. RINEX clock GPS seconds are the correctly rounded value of the civil tag, and NMEA seconds are read as stated.
- **Breaking.** Solutions move through two engine changes to where a satellite is and what its clock reads. Placement: SPP, PPP and RTK place each satellite at its transmission epoch from the measured pseudorange, `t_rx - P / c - dts`, as RTKLIB `satposs` does, which moves solutions at the millimetre to decimetre level. Precise clock: the SPP, DGNSS and tightly coupled fusion code models add the relativistic term RTKLIB `peph2pos` adds to an SP3 or RINEX clock, `-2 r·v / c²`, which moves solutions from an SP3 or other precise source by metres to tens of metres. The broadcast satellite clock also no longer subtracts the single-frequency group delay; `pseudorange_code` states which code the group delay applies to.
- **Breaking.** A PPP observation whose code is zero or negative places no transmission epoch; it is left out and reported as a `SidereonPppUnplacedObservation` (`SIDEREON_PPP_UNPLACED_OBSERVATION_REASON_CODE_NOT_POSITIVE`).
- **Breaking.** Where an engine value has no C code yet, the C value reads `UNKNOWN` and a text slot of `SIDEREON_UNKNOWN_VARIANT_C_BYTES` bytes names it, one slot per enum field: `unknown_variant` in `SidereonBiasLookup`, `SidereonBiasNotice`, `SidereonPppUnplacedObservation`, `SidereonPppTransmitTimeFailure`, `SidereonPppSsrApplication`, `SidereonPppSsrSignalReport` and `SidereonClockWriteDeparture`; `time_system_unknown_variant` and `time_system_status_unknown_variant` in `SidereonRinexClockInfo`; `reading_unknown_variant` and `continuation_reading_unknown_variant` in `SidereonClockRecord`; `reading_unknown_variant` and `field_kind_unknown_variant` in `SidereonClockHeaderRecord`; `kind_unknown_variant` and `time_system_unknown_variant` in `SidereonClockNotice`. `SidereonBiasLookupStatus` and `SidereonPppUnplacedObservationReason` gain `UNKNOWN` (999). An engine error the C error kinds do not name keeps its variant name in the error text.
- **Breaking.** Every route that reports an engine UT1 refusal returns `SIDEREON_STATUS_UT1_OUTSIDE_COVERAGE`, including SPP (both input forms), SBAS and SSR SPP, the RINEX SPP routes, PPP float and fixed, the almanac, the event finder, covariance propagation, the reduced orbit, predicted observables and FDE. `SidereonFallbackStatus` gains `UT1_OUTSIDE_COVERAGE` (7). Under sidereon-core's default UT1 policy an instant outside the UT1 table is refused rather than read from the long-term delta-T curve.
- **Breaking.** Enum-typed input fields become `uint32_t` codes validated on entry, so a value outside the enum is refused with `SIDEREON_STATUS_INVALID_ARGUMENT` instead of being read as a Rust enum: `SidereonTimeScales.ut1_degraded`, `SidereonRtkArcReferenceEntry.system`, and `SidereonRtcmMsmInfo.system` and `kind`.
- **Breaking.** `SidereonBiasNotice` states each notice as typed fields: the `SidereonBiasNoticeKind`, the `SidereonBiasDepartureKind` of a departure, the line and the numbers the notice carries. `sidereon_bias_set_notice_text` copies a notice's text parts (`SidereonBiasNoticeText`) under the caller-buffer convention.
- **Breaking.** `SidereonDtedTileError`, `SidereonTerrainStoreError` and `SidereonTerrainDatumError` lose their fixed 512-byte `path`, `message`, `reason` and `remediation` fields and `SIDEREON_TERRAIN_ERROR_TEXT_C_BYTES` is removed; `sidereon_last_terrain_error_text` copies each text of the last error of a `SidereonTerrainErrorFamily` under the caller-buffer convention. `SidereonDtedHorizontalDatum` has `UNKNOWN` (999) for a datum the engine names that the binding has no code for; `SidereonDtedHorizontalDatumValue.text` carries the engine's value.

- **Breaking.** Writers refuse output that would not read back as the product: `sidereon_sp3_to_sp3_text`, `sidereon_rinex_obs_to_rinex_text`, `sidereon_antex_encode`, `sidereon_ionex_to_ionex_text`, the RINEX clock writer and the BLQ writers return `SIDEREON_STATUS_INVALID_ARGUMENT` with a required length of zero instead of writing a value that reads back as another number, an overflowed field or an epoch no record restates. Each has a `_result` route returning the refusal typed in an owned result: `SidereonSp3WriteResult`, `SidereonRinexObsWriteResult`, `SidereonAntexResult`, `SidereonRinexClockResult`, `SidereonBlqResult`.
- **Breaking.** SP3: a clock carried with a missing-orbit record is retained. `SidereonSp3EpochAgreement` gains `position_rms_present` and `position_max_present`; an absent spread is NaN, never 0.
- **Breaking.** RINEX observations: `SidereonRinexObsEpoch` gains `has_epoch`, false for an event record with blank epoch fields. `SidereonRinexObsCarrierPhase` gains `phase_shift_status` (`SidereonRinexCorrectionStatus`) and `phase_shift_conflict_count`; each epoch is read under the header in effect at it, and `sidereon_rinex_obs_carrier_phase_conflicts` copies the corrections of an ambiguous row. A version 2 downgrade refuses a change of carrier frequency. `sidereon_rinex_repair_obs` keeps its repair when the repaired product cannot be written.
- **Breaking.** RINEX clock: a `SidereonRinexClock` keeps every line it reads and restates an unedited product byte for byte. `SidereonClockPoint` gains every declared value after the bias. New routes read the header records, data records, skipped records, diagnostics, notices and source lines, edit records and the time system in place, build a product from samples, and write under a `SidereonClockWritePolicy` with its departures. `GLO` reads as UTC in every version, `IRN` has no time scale, and civil query seconds are read with every digit kept rather than rounded to the microsecond.
- **Breaking.** ANTEX: every ANTEX 1.4 record is retained, absent records stay absent, and validity seconds are held exactly (`SidereonAntexDateTime`). Frequency sections keep file order and repeats; a label whose sections differ is refused as `SIDEREON_ANTEX_ERROR_KIND_AMBIGUOUS_FREQUENCY`. Millimetre fields convert as `mm * 1e-3`, as RTKLIB `readantex`. Every ANTEX failure is typed (`sidereon_last_antex_error`).
- **Breaking.** Terrain: a DTED null posting is an unknown elevation and is refused (`SIDEREON_DTED_TILE_ERROR_KIND_NULL_POSTING`), and a tile whose DSI states a datum other than WGS84 is refused. `SidereonDtedHeightResult`, `SidereonTerrainHeightResult` and `SidereonTerrainDatumError` carry a `SidereonTerrainLookupError`; `SidereonTerrainStoreError` gains the field and the horizontal datum, and `SidereonTerrainStoreErrorKind` gains `TILE_ID_OUT_OF_RANGE`, `TILE_BOUNDS_MISMATCH` and `NON_WGS84_TILE`.
- **Breaking.** BLQ: `sidereon_blq_parse` reads every station block with its comments and column-order headers in place; the BLQ writers refuse a block the parser would not read back unchanged, with a typed error.
- **Breaking.** IONEX: `sidereon_ionex_slant_delay_with_policy` takes a `SidereonIonexSlantPolicy` and an optional typed `SidereonIonexSlantError`, and `SidereonIonexSlantDelayEvaluation.status` becomes a `SidereonIonexSlantDelayStatus` stating held, degraded and assumed mapping separately. A value a file gives as `9999` is absent: `SidereonTecSample` gains `has_vtec_tecu`, whole-second epochs and a height offset, and `SidereonTecGridSamples` gains per-map presence arrays, height maps and a header. Header records, warnings, skipped records, standalone TEC grids and batch slant evaluation have new routes. `SidereonEmissionMediaOptions` gains an IONEX slant policy. Layout: `SidereonIonexSlantDelayStatus` changes from an enum to a struct (`is_valid`, `has_held` with `coverage_error`, `has_degraded` with the node `gap`, `has_assumed_mapping` with `assumed_mapping`) and its `SIDEREON_IONEX_SLANT_DELAY_STATUS_*` constants are removed; `coverage_error` moves from `SidereonIonexSlantDelayEvaluation` into that status; and `SidereonTecGridSamplesInfo` grows by `has_height_maps` and `height_map_value_count`.
- **Breaking.** An SPP or static solve with the ionosphere correction leaves out a satellite whose carrier does not resolve and reports `SIDEREON_SPP_REJECTION_REASON_IONOSPHERE_CARRIER_UNRESOLVED`; `SIDEREON_STATIC_POSITION_ERROR_KIND_IONOSPHERE_UNSUPPORTED` is removed. The RINEX RTK arcs report unresolved carriers (`sidereon_rtk_rinex_arc_unresolved_carriers`).
- **Breaking.** Satellite tokens read the shared range `01`..`99`, so GLONASS slots `R28` and up are read rather than skipped. `sidereon_satellite_id_to_sbas_prn` converts only `S20`..`S58`, and `sidereon_sbas_store_unassigned_mask_corrections` counts corrections addressed to unassigned mask bits.
- **Breaking.** `sidereon_gnss_seconds_of_week_from_calendar` and `sidereon_gnss_week_from_calendar` refuse fields that name no calendar date and time. The TDM reader and writer follow CCSDS 503.0-B-2 strictly, refusing what the standard does not define and comments the writer cannot restate in place.
- **Breaking.** The IONEX slant-delay routes (`sidereon_ionex_slant_delay`, `_with_policy`, `_with_coverage_policy`, the batch routes and `SidereonIonexSlantRequest.epoch_j2000_s`) and `sidereon_select_ionex_over_range` read the query epoch as whole UTC seconds since J2000, the axis IONEX labels its maps on. The engine carries the query onto UTC before it meets the maps, so a query on a map label evaluates that map exactly.
- **Breaking.** Terrain: `SidereonTerrainStoreErrorKind` gains `TILE` (a DTED input the store builders could not read), with the typed tile failure in the new `SidereonTerrainStoreError.tile_error` and its path and message texts. `SidereonTerrainLookupErrorKind` gains `TILE` and `TILE_ORIGIN` with the tile, the stated origin (`origin_latitude_deg`, `origin_longitude_deg`) and the typed tile failure; single-point lookups record their texts in the new `SIDEREON_TERRAIN_ERROR_FAMILY_TERRAIN_LOOKUP` family and batch rows through `sidereon_last_terrain_batch_error_text`.
- **Breaking.** The RTCM encoder, RTCM conversion and SBAS encoder refusals return `SIDEREON_STATUS_INVALID_ARGUMENT` from the RTCM and SSR routes, with their typed detail in `sidereon_rtcm_last_error_info` and its payload.
- **Breaking.** `sidereon_sp3_merge_report_continuity_verdict_json` takes the report and the window alone, as the engine verdict does; the unused `merged` argument is removed. The verdict JSON carries every field of each defect and of each splice, including the cells a finding rests on with their roles and selections, the sources, and a hold-out residual's `node_epochs_j2000_s`.
- **Breaking.** The inertial helpers, `sidereon_solid_earth_tide_with_constants` and the station-tide routes take `const double *` and `double *` for their vectors and matrices, as the other routes do, instead of pointers to fixed arrays; each documents how many doubles it reads or writes.
- **Breaking.** `SidereonSp3MergeOptions` gains `provenance_mode` (`SidereonSp3ProvenanceMode`) and `verify_continuity_enabled` with `verify_continuity` (`SidereonSp3ContinuityOptions`, filled by `sidereon_sp3_continuity_options_for_orbit_class`, with an explicit speed bound available). `target_epoch_interval_s` is the engine's to validate: a whole number of SP3's 10-nanosecond ticks, where the binding required whole seconds. `SidereonSp3MergeFlag` and `SidereonSp3EpochAgreement` gain the exact `epoch`; `SidereonSp3AgreementSummary` gains `single_source_fraction`.
- **Breaking.** FDE and RAIM follow the engine's RTKLIB demo5 `raim_fde` exclusion rule. `SidereonFdeOptions.max_iterations` becomes `max_exclusions` (default 1, RTKLIB's single exclusion) with `max_exclusion_rms_m` (default 100 m), and `unit_weights` becomes `weights_mode` (`SidereonRaimWeightsMode`, default `SOLUTION`: each residual over the pseudorange variance the solve weighted it by). `sidereon_raim` and `sidereon_raim_normalized_residuals` take `variances_m2`, `sidereon_raim_for_solution` takes a `weights_mode`, and `SidereonRangeFdeOptions` gains `max_exclusion_rms_m`. A fault the loop leaves unresolved still returns `SIDEREON_STATUS_SOLVE`, and its last solution, exclusions, reason and detection test are read with `sidereon_last_fde_unresolved`, `_solution` and `_excluded_sats`. `sidereon_fde_solution_raim` and `_raim_normalized_residuals` read the accepted solution's detection test. `sidereon_spp_solution_pseudorange_variances` and `sidereon_spp_solution_weights` copy what the solve weighted each used satellite by. `SidereonSppSolveStatus` gains `OUTER_OSCILLATION`, and the engine's default robust `max_outer` is 100.
- SP3 merge audit trail: `SIDEREON_SP3_MERGE_FLAG_KIND_ARC_WITHHELD`, `sidereon_sp3_merge_report_omitted_epochs`, `_clock_omissions`, `_dropped_input_epochs`, `_agreement_metrics`, `_continuity_json`, `_continuity_selected_nodes`, and the per-epoch provenance (`sidereon_sp3_merge_report_provenance`, `_provenance_cells`, `_provenance_cell_members`, `_provenance_transitions`, `_provenance_coverage`). Every record carries its epoch exactly (`SidereonClockEpoch`) and as seconds since J2000.
- SP3 product coverage and node selection: `sidereon_sp3_satellite_coverage` with the grid, per-satellite position and clock spans and gaps, and `sidereon_sp3_selected_nodes`.
- SP3 continuity with a `SidereonSp3ContinuityOptions` record, which can state an explicit speed bound: `sidereon_sp3_continuity_report_json` copies the whole report (every defect, `pairs_checked`, `residuals_checked`, `residuals_skipped`), and `sidereon_sp3_continuity_verdict_json_with_options` the window verdict.
- The header now declares `SidereonSsrCorrectionSizePolicy` and the other selector enums the routes take as `uint32_t` codes (NTRIP, SGP4 fit, covariance, CNAV, broadcast group delay, observation QC, RINEX QC severity, SP3 accuracy, space weather).
- Generic owned engine error ABI: `sidereon_last_engine_error_info` reads the typed error summary (`SidereonEngineErrorInfo`: `family` and UTF-8 JSON `payload_len`), `sidereon_last_engine_error_payload` copies the Schema 1 JSON payload, and `sidereon_clear_engine_error` explicitly resets the thread-local generic error slot. The Schema 1 root object contains `schema_version` (1), `family` (`SidereonEngineErrorFamily` name), `operation`, and `error` (`kind` and `fields`); floating-point values are preserved losslessly as `{"decimal": "<string>", "bits_hex": "<16-hex-chars>"}`. Producing calls clear the generic slot before argument validation; readers, copy accessors, and `_free` functions retain it; each thread has its own TLS; and clearing the generic slot does not clear legacy diagnostic text (`sidereon_last_error_message`) or family-specific error slots. Standard two-pass `(NULL, 0)` queries write required byte counts; short buffers reject with `SIDEREON_STATUS_INVALID_ARGUMENT`, write 0 to `*out_written`, and retain the payload.
- SPP batch row-owned error ABI: `sidereon_spp_batch_error_info` and `sidereon_spp_batch_error_payload` inspect individual epochs by index. An epoch that solved reports family `SIDEREON_ENGINE_ERROR_FAMILY_NONE` (`None`) and payload length 0. A failed epoch provides structured family and Schema 1 payload owned by the `SidereonSppBatch` handle itself, independent of global thread-local storage and subsequent operations on the thread; copied bytes survive `sidereon_spp_batch_free`. Retrieving a failed solution via `sidereon_spp_batch_solution` returns `SIDEREON_STATUS_SOLVE`, retains generic thread-local storage, updates legacy diagnostic text (`sidereon_last_error_message`) with the row cause, and keeps the batch row detail intact. Legacy statuses and `sidereon_spp_batch_error` remain compatibility APIs.

## 2.1.1 - 2026-09-22

### Fixed

- CODE predicted ionosphere maps resolve to the archive AIUB now serves them from. `cod_prd1` is `CODE/IONO/PRD/COD0OPSP0D_<date>0000_01D_01H_GIM.INX.gz` and `cod_prd2` is `CODE/IONO/PRD/COD0OPSP1D_<date>0000_01D_01H_GIM.INX.gz` in `sidereon_data_distribution_location`, `sidereon_data_predicted_ionex_line_candidates_json` and the publication-status entry points. The `CODE/IONO/P1/<year>` and `CODE/IONO/P2/<year>` `COD0OPSPRD` trees they were read from stopped receiving issues after 2026-09-21 and are now empty, so every predicted-IONEX request returned not-published. For the dates both layouts carried the objects decompress to the same bytes. The two lines now carry distinct official filenames, and publication status counts only objects under `CODE/IONO/PRD/`, not the rolling copies CODE keeps at the tree root.

### Changed

- Engine update: sidereon 2.1.1 / sidereon-core 2.1.1.

## 2.1.0 - 2026-09-05

### Added

- Exposed the sidereon-core SP3 interpolation policy across the C surface:
  - SP3 loading and exact loading with an explicit gap threshold factor: `sidereon_sp3_load_with_gap_threshold_factor` and `sidereon_sp3_load_exact_with_gap_threshold_factor` (default unset `<= 0.0` uses core default 1.5).
  - SP3 interpolation gap threshold factor reader: `sidereon_sp3_gap_threshold_factor`.
  - Product-wide continuity and window continuity verdict exports with an explicit gap threshold factor: `sidereon_sp3_check_continuity_with_gap_threshold_factor` and `sidereon_sp3_continuity_verdict_json_with_gap_threshold_factor`.
  - Sample-backed sources and cached interpolants from canonical samples with an explicit gap threshold factor and metadata readers: `sidereon_precise_ephemeris_samples_from_samples_with_gap_threshold_factor`, `sidereon_precise_ephemeris_samples_gap_threshold_factor`, `sidereon_precise_ephemeris_interpolant_from_samples_with_gap_threshold_factor`, and `sidereon_precise_ephemeris_interpolant_gap_threshold_factor`.
  - Precise-interpolant memory-mappable artifact header reader: `sidereon_precise_interpolant_artifact_gap_threshold_factor`.

### Changed

- Engine update: sidereon 2.1.0 / sidereon-core 2.1.0. Additive upstream release: the SP3 coverage-gap threshold is now a validated, product-carried policy (`Sp3InterpolationOptions`, default 1.5 and bit-identical to before), the SP3 window-scoped continuity reach is derived from the interpolator's actual selectable node spans, and RINEX 4 CNAV week/TOW round trips are stable at the week boundary.

## 2.0.0 - 2026-09-02

### Changed

- Engine update: sidereon 2.0.0 / sidereon-core 2.0.0.
  Accommodates two breaking upstream changes:
  - Input options, configuration, and request structs in sidereon-core are now
    marked `#[non_exhaustive]`, so internal callers construct them through
    `Default::default()` or `new(...)` followed by field assignment rather
    than struct literal syntax.
  - The terrain, IONEX TEC grid, and dense-output APIs now return typed error
    enums (`DtedTileError`, `TecGridError`, and `DenseOutputError`) whose
    message text is unchanged.
  No C ABI changes.

## 1.4.1 - 2026-08-31

### Changed

- Engine update: sidereon 1.4.1 / sidereon-core 1.4.1, superseding 1.4.0.
  Solutions are unchanged; the engine's trust-region backend no longer
  overrides the solver's own reductions, so the reported first-order
  optimality and evaluation counts of fits match 1.3.3 again. No C ABI
  changes.

## 1.4.0 - 2026-08-31

### Changed

- Engine update: sidereon 1.4.0 / sidereon-core 1.4.0 /
  trust-region-least-squares 0.11.0. Every transcendental and fused
  multiply-add now goes through portable kernels and decompositions run on a
  portable scalar, so results are bit-identical across x86_64 and arm64;
  SVD-derived covariance and geometry diagnostics take the values 1.3.3
  produced on x86_64/glibc, and frozen iterative-fit outputs move in their
  last bits. No C ABI changes.

## 1.3.3 - 2026-08-30

### Changed

- Engine update: sidereon 1.3.3 / sidereon-core 1.3.3. Archive-listing parsing
  is no longer quadratic (154 s to 0.23 s on AIUB's ~426k-row listing), and
  transcendental math is bit-identical across x86_64 and arm64. No C ABI
  changes.

## 1.3.1 - 2026-08-29

### Changed

- Engine update: sidereon 1.3.1 / sidereon-core 1.3.1. Coordination release
  keeping the shared release number across the language interfaces, which
  ships the Go interface relicense from Apache-2.0 to MIT. No C ABI changes.

## 1.3.0 - 2026-08-29

### Added

- A route that decodes a bare RTCM SSR message body into its own handle, so
  callers reach SSR data without naming RTCM types, plus the grouped raw
  code-bias and phase-bias record and signal accessors that the existing RTCM
  message path advertised through its counts but never exposed.
- An SBAS broadcast PRN to satellite-id lookup.

### Changed

- Engine update: sidereon 1.3.0 / sidereon-core 1.3.0.
- Smoke failures no longer attribute a stale ABI error to an unrelated
  assertion.

## 1.2.0 - 2026-08-28

### Added

- Public parity routes for covariance-6 construction and ECI/RTN transforms,
  lenient NAV and SBAS EMS/RTKLIB parsers, calendar helpers, raw NAV record
  lists, rich broadcast records, RINEX clock series, single- and dual-frequency
  RTK arc builders, frequency and wavelength policy, default ionosphere-free
  pairs, GPS LNAV word helpers, and explicit DTED tile-list stores.

### Changed

- Engine update: sidereon 1.2.0 / sidereon-core 1.2.0, which corrects lenient
  RINEX 4 CNAV decoding and RTKLIB SBAS wire-form preservation.

## Unreleased

### Added

- Public C parity routes for covariance/calendar and signal/LNAV policy,
  rich broadcast and RINEX NAV/clock/GLONASS/header data, SBAS logs, RINEX
  RTK arcs, and DTED tile-list stores, including diagnostics for skipped
  extended GLONASS slots, with deterministic CI smokes covering the new
  surfaces and committed fixtures.
- Added a C route that decodes a bare RTCM SSR message body into its own
  handle, so callers reach SSR data without naming RTCM types, and added the
  grouped raw code-bias and phase-bias record and signal accessors that the
  existing RTCM message path advertised through its counts but never exposed.
- Added the public C SBAS broadcast-PRN-to-satellite-id lookup route.

## 1.1.1 - 2026-08-26

### Changed

- Engine update: sidereon 1.1.1 / sidereon-core 1.1.1, the coordination
  release restoring the shared release number across the language interfaces.
  No numerical, algorithmic, or API changes.

### Fixed

- The committed header's `SIDEREON_VERSION_MAJOR/MINOR/PATCH/STRING` macros
  still said 0.35.0 while `sidereon_version` and `sidereon_version_string`
  reported the crate version. They now agree, and the CI smoke gate checks
  them on every host. No API changes.

## 1.1.0 - 2026-08-24

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
