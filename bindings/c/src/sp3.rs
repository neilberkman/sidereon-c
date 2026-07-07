use super::*;

const SP3_FRAME_LABEL_MAX_BYTES: usize = 64;

/// A parsed SP3 precise-ephemeris product. Opaque to C. Create with
/// sidereon_sp3_load or sidereon_sp3_merge and release with sidereon_sp3_free.
pub struct SidereonSp3 {
    pub(crate) inner: Sp3,
}

/// An SP3 merge audit report. Opaque to C. Create with sidereon_sp3_merge and
/// release with sidereon_sp3_merge_report_free.
pub struct SidereonSp3MergeReport {
    pub(crate) inner: MergeReport,
    /// Per-epoch agreement aggregate, computed once at merge time so the C
    /// accessors are O(1) lookups rather than recomputing the rollup per call.
    pub(crate) epoch_agreement: Vec<EpochAgreement>,
}

/// Exact parsed state of one satellite at one SP3 epoch.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonSp3State {
    /// ECEF position in meters.
    pub position_m: [f64; 3],
    /// Whether clock_s is present.
    pub has_clock_s: bool,
    /// Clock offset in seconds when has_clock_s is true.
    pub clock_s: f64,
    /// Whether velocity_m_s is present.
    pub has_velocity_m_s: bool,
    /// ECEF velocity in meters per second when has_velocity_m_s is true.
    pub velocity_m_s: [f64; 3],
    /// Whether clock_rate_s_s is present.
    pub has_clock_rate_s_s: bool,
    /// Clock rate in seconds per second when has_clock_rate_s_s is true.
    pub clock_rate_s_s: f64,
    /// Clock discontinuity flag.
    pub clock_event: bool,
    /// Clock prediction flag.
    pub clock_predicted: bool,
    /// Satellite maneuver flag.
    pub maneuver: bool,
    /// Orbit prediction flag.
    pub orbit_predicted: bool,
}

/// How agreeing SP3 sources are combined during merge.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonSp3MergeCombine {
    /// Arithmetic mean of agreeing sources.
    Mean = 0,
    /// Component-wise median of agreeing sources.
    Median = 1,
    /// Highest-precedence agreeing source, using input order.
    Precedence = 2,
}

/// Controls for merging SP3 products. Initialize with
/// sidereon_sp3_merge_options_init before overriding fields.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonSp3MergeOptions {
    /// Maximum agreeing-source 3D position difference, meters.
    pub position_tolerance_m: f64,
    /// Maximum agreeing-source clock difference after datum alignment, seconds.
    pub clock_tolerance_s: f64,
    /// Minimum agreeing sources required when several sources cover one cell.
    pub min_agree: usize,
    /// Minimum common clocked satellites for clock-datum alignment.
    pub clock_min_common: usize,
    /// One of SidereonSp3MergeCombine_*.
    pub combine: u32,
    /// Whether target_epoch_interval_s is supplied.
    pub target_epoch_interval_s_enabled: bool,
    /// Output epoch spacing in seconds when enabled.
    pub target_epoch_interval_s: f64,
    /// Optional array of SidereonGnssSystem_* values encoded as uint32_t.
    pub systems: *const u32,
    /// Number of entries in systems. Zero means no system filter.
    pub system_count: usize,
    /// Optional array of asserted coordinate-label sets.
    pub asserted_frame_label_sets: *const SidereonSp3FrameLabelSet,
    /// Number of entries in asserted_frame_label_sets.
    pub asserted_frame_label_set_count: usize,
    /// Enable catalog Helmert reconciliation between known ITRF/IGS labels.
    pub helmert_frame_reconciliation: bool,
}

/// One caller-asserted set of SP3 coordinate labels.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonSp3FrameLabelSet {
    /// UTF-8 label pointers.
    pub labels: *const *const c_char,
    /// Number of labels. Must be at least two.
    pub label_count: usize,
}

/// Method used to reconcile one SP3 source coordinate label.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonSp3FrameReconciliationMethod {
    /// Caller asserted the labels are equivalent; no math was applied.
    AssertedEquivalence = 0,
    /// Catalog Helmert reconciliation, or exact identity for the same realization.
    Helmert = 1,
}

/// One SP3 coordinate-label reconciliation report row.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonSp3FrameReconciliation {
    /// Source index in the sidereon_sp3_merge input array.
    pub source_index: usize,
    /// Source label byte length, copied separately.
    pub source_label_len: usize,
    /// Target label byte length, copied separately.
    pub target_label_len: usize,
    /// Reconciliation method.
    pub method: SidereonSp3FrameReconciliationMethod,
    /// Number of labels in the caller assertion set.
    pub asserted_label_count: usize,
    /// Whether source_frame is present.
    pub source_frame_present: bool,
    /// Resolved source frame as SidereonTerrestrialFrame.
    pub source_frame: u32,
    /// Whether target_frame is present.
    pub target_frame_present: bool,
    /// Resolved target frame as SidereonTerrestrialFrame.
    pub target_frame: u32,
    /// Whether catalog_source_frame and catalog_target_frame are present.
    pub catalog_frame_present: bool,
    /// Published catalog row source as SidereonTerrestrialFrame.
    pub catalog_source_frame: u32,
    /// Published catalog row target as SidereonTerrestrialFrame.
    pub catalog_target_frame: u32,
    /// Whether the published catalog row was applied in reverse.
    pub catalog_inverse: bool,
    /// Whether reference_epoch_year is present.
    pub reference_epoch_year_present: bool,
    /// Published transform reference epoch.
    pub reference_epoch_year: f64,
    /// Whether parameters are present.
    pub parameters_present: bool,
    /// Published translation parameters in millimetres.
    pub translation_mm: [f64; 3],
    /// Published scale parameter in parts per billion.
    pub scale_ppb: f64,
    /// Published rotation parameters in milliarcseconds.
    pub rotation_mas: [f64; 3],
    /// Whether rates are present.
    pub rates_present: bool,
    /// Published translation rates in millimetres per year.
    pub translation_mm_per_year: [f64; 3],
    /// Published scale rate in parts per billion per year.
    pub scale_ppb_per_year: f64,
    /// Published rotation rates in milliarcseconds per year.
    pub rotation_mas_per_year: [f64; 3],
    /// Provenance byte length, copied separately.
    pub provenance_len: usize,
    /// Whether epoch_year_start and epoch_year_end are present.
    pub epoch_year_span_present: bool,
    /// First affected decimal year.
    pub epoch_year_start: f64,
    /// Last affected decimal year.
    pub epoch_year_end: f64,
    /// Number of satellite position records covered by the reconciliation.
    pub records_affected: usize,
    /// True when both labels resolved to the same terrestrial realization.
    pub identity: bool,
}

/// Which merge report flag list to query.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonSp3MergeFlagKind {
    /// Cells omitted because sources disagreed beyond tolerance.
    Quarantined = 0,
    /// Cells carried from one source because no cross-check was possible.
    SingleSource = 1,
    /// Cells where an accepted consensus rejected source outliers.
    PositionOutlier = 2,
}

/// One SP3 merge audit flag. Source indices are copied with
/// sidereon_sp3_merge_report_flag_sources.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonSp3MergeFlag {
    /// Flagged epoch as seconds since J2000 in the product time scale.
    pub epoch_j2000_seconds: f64,
    /// Satellite token.
    pub sat_id: SidereonSatelliteToken,
    /// Number of source indices attached to this flag.
    pub source_count: usize,
}

/// Per-epoch aggregate of the merge agreement metric: how tightly the consensus
/// sources clustered about the combined value written to the merged product,
/// pooled over the multi-source satellites at one output epoch. Mirrors
/// sidereon_core::ephemeris::EpochAgreement. Copied with
/// sidereon_sp3_merge_report_epoch_agreement; the entries are in output-epoch
/// order (count from sidereon_sp3_merge_report_epoch_agreement_count). The core
/// groups by integer (whole-second) epoch, so output epochs that fall in the same
/// integer second are pooled into one entry; for real SP3 products (epochs are
/// whole seconds apart) this is one entry per output epoch.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonSp3EpochAgreement {
    /// Output epoch as seconds since J2000 in the product time scale.
    pub epoch_j2000_seconds: f64,
    /// Satellites at this epoch with a multi-source position consensus. Zero when
    /// every cell at the epoch was single-source (the spread fields are then 0 and
    /// the clock present-flags false).
    pub satellites: usize,
    /// Member-count-weighted pooled RMS of the per-cell position dispersion over
    /// the multi-source satellites at this epoch, meters.
    pub position_rms_m: f64,
    /// Worst per-cell position dispersion at this epoch, meters.
    pub position_max_m: f64,
    /// True when clock_rms_s carries a value (a multi-source clock consensus
    /// existed at this epoch).
    pub clock_rms_present: bool,
    /// Pooled RMS of the per-cell clock dispersion at this epoch, seconds. Valid
    /// only when clock_rms_present is true.
    pub clock_rms_s: f64,
    /// True when clock_max_s carries a value.
    pub clock_max_present: bool,
    /// Worst per-cell clock dispersion at this epoch, seconds. Valid only when
    /// clock_max_present is true.
    pub clock_max_s: f64,
}

/// Per-epoch clock-reference offset of one SP3 product relative to another.
/// Copied by sidereon_sp3_clock_reference_offsets.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonSp3ClockReferenceOffset {
    /// Matched epoch as seconds since J2000 in the product time scale.
    pub epoch_j2000_seconds: f64,
    /// Other minus reference clock datum, seconds.
    pub offset_s: f64,
    /// Number of common clocked satellites used by the median estimate.
    pub satellites: usize,
}

/// Whole-product rollup of the merge agreement metric: the pooled position/clock
/// dispersion of the consensus members about the combined values. Each scalar
/// mirrors a sidereon_core::ephemeris::MergeReport agreement method and carries a
/// present flag, but the present condition differs by field (see each below): the
/// pooled RMS fields are present only when some accepted cell had a multi-source
/// consensus on that channel, whereas the max fields are present whenever there
/// was any accepted cell (a single-source cell has zero dispersion, not an absent
/// max). Written by sidereon_sp3_merge_report_agreement_summary.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonSp3AgreementSummary {
    /// True when position_rms_m carries a value (some cell had >= 2 position
    /// consensus members).
    pub position_rms_present: bool,
    /// Member-count-weighted pooled RMS of the per-cell position dispersion over
    /// the whole product, meters. Valid only when position_rms_present is true.
    pub position_rms_m: f64,
    /// True when position_max_m carries a value (there was at least one accepted
    /// cell).
    pub position_max_present: bool,
    /// Largest single-cell position dispersion over the whole product, meters.
    /// Valid only when position_max_present is true.
    pub position_max_m: f64,
    /// True when clock_rms_s carries a value (some accepted cell had >= 2 clock
    /// consensus members).
    pub clock_rms_present: bool,
    /// Member-count-weighted pooled RMS of the per-cell clock dispersion over the
    /// whole product, seconds. Valid only when clock_rms_present is true.
    pub clock_rms_s: f64,
    /// True when clock_max_s carries a value (there was at least one accepted cell
    /// carrying a clock).
    pub clock_max_present: bool,
    /// Largest single-cell clock dispersion over the whole product, seconds. Valid
    /// only when clock_max_present is true.
    pub clock_max_s: f64,
}

/// Parse an SP3-c or SP3-d byte buffer into a precise-ephemeris product. On
/// success writes a newly owned handle to *out_sp3. Release it with
/// sidereon_sp3_free.
///
/// Safety: data must point to len readable bytes; out_sp3 must point to storage
/// for a SidereonSp3*.
#[no_mangle]
pub unsafe extern "C" fn sidereon_sp3_load(
    data: *const u8,
    len: usize,
    out_sp3: *mut *mut SidereonSp3,
) -> SidereonStatus {
    ffi_boundary("sidereon_sp3_load", SidereonStatus::Panic, || {
        let out_sp3 = c_try!(require_out(out_sp3, "sidereon_sp3_load", "out_sp3"));
        *out_sp3 = ptr::null_mut();
        let bytes = c_try!(require_slice(data, len, "sidereon_sp3_load", "data"));
        let inner = c_try!(guard(SidereonStatus::Sp3Parse, || {
            sidereon::load_sp3(bytes)
        }));
        write_boxed_handle(out_sp3, SidereonSp3 { inner });
        SidereonStatus::Ok
    })
}

/// Write the number of epochs in the product to *out_count.
///
/// Safety: sp3 must be a handle from sidereon_sp3_load or sidereon_sp3_merge
/// that has not been freed; out_count must point to a size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_sp3_epoch_count(
    sp3: *const SidereonSp3,
    out_count: *mut usize,
) -> SidereonStatus {
    ffi_boundary("sidereon_sp3_epoch_count", SidereonStatus::Panic, || {
        let out_count = c_try!(require_out(
            out_count,
            "sidereon_sp3_epoch_count",
            "out_count"
        ));
        *out_count = 0;
        let sp3 = c_try!(require_ref(sp3, "sidereon_sp3_epoch_count", "sp3"));
        *out_count = sp3.inner.epoch_count();
        SidereonStatus::Ok
    })
}

/// Copy satellite tokens present in the product. Uses the variable-length
/// output contract documented at the top of the header.
///
/// Safety: sp3 must be a live handle; out must point to at least len writable
/// entries or be NULL when len is 0; out_written and out_required must point to
/// size_t values.
#[no_mangle]
pub unsafe extern "C" fn sidereon_sp3_satellites(
    sp3: *const SidereonSp3,
    out: *mut SidereonSatelliteToken,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary("sidereon_sp3_satellites", SidereonStatus::Panic, || {
        c_try!(init_copy_counts(
            "sidereon_sp3_satellites",
            out_written,
            out_required
        ));
        let sp3 = c_try!(require_ref(sp3, "sidereon_sp3_satellites", "sp3"));
        let values: Vec<SidereonSatelliteToken> = sp3
            .inner
            .satellites()
            .iter()
            .copied()
            .map(satellite_token)
            .collect();
        c_try!(copy_prefix_to_c(
            "sidereon_sp3_satellites",
            "out",
            &values,
            out,
            len,
            out_written,
            out_required,
        ));
        SidereonStatus::Ok
    })
}

/// Copy parsed SP3 epoch nodes as seconds since J2000, in the product time
/// scale. Uses the variable-length output contract documented at the top of the
/// header.
///
/// Safety: sp3 must be a live handle; out must point to at least len writable
/// doubles or be NULL when len is 0; out_written and out_required must point to
/// size_t values.
#[no_mangle]
pub unsafe extern "C" fn sidereon_sp3_epochs_j2000_seconds(
    sp3: *const SidereonSp3,
    out: *mut f64,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_sp3_epochs_j2000_seconds",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_sp3_epochs_j2000_seconds",
                out_written,
                out_required
            ));
            let sp3 = c_try!(require_ref(sp3, "sidereon_sp3_epochs_j2000_seconds", "sp3"));
            let epochs = sp3.inner.epochs_j2000_seconds();
            c_try!(copy_prefix_to_c(
                "sidereon_sp3_epochs_j2000_seconds",
                "out",
                &epochs,
                out,
                len,
                out_written,
                out_required,
            ));
            SidereonStatus::Ok
        },
    )
}

/// Copy the exact parsed state of satellite sat_id at epoch_index into
/// *out_state.
///
/// Safety: sp3 must be a live handle; sat_id must be a null-terminated
/// satellite token whose terminator appears within 16 bytes; out_state must
/// point to a SidereonSp3State.
#[no_mangle]
pub unsafe extern "C" fn sidereon_sp3_state(
    sp3: *const SidereonSp3,
    sat_id: *const c_char,
    epoch_index: usize,
    out_state: *mut SidereonSp3State,
) -> SidereonStatus {
    ffi_boundary("sidereon_sp3_state", SidereonStatus::Panic, || {
        let out_state = c_try!(require_out(out_state, "sidereon_sp3_state", "out_state"));
        *out_state = empty_sp3_state();
        let sp3 = c_try!(require_ref(sp3, "sidereon_sp3_state", "sp3"));
        let sat = c_try!(parse_satellite_token("sidereon_sp3_state", sat_id));
        let state = c_try!(guard_core(
            || sp3.inner.state(sat, epoch_index),
            |err| map_sp3_argument_error("sidereon_sp3_state", err),
        ));
        *out_state = sp3_state_to_c(state);
        SidereonStatus::Ok
    })
}

/// Interpolate a satellite at each query epoch. out_position_m receives
/// epoch_count rows of 3 ECEF meters, and out_clock_s receives epoch_count
/// clock offsets in seconds, using NaN when the engine reports no clock.
///
/// Safety: sp3 must be a live handle; sat_id must be a bounded null-terminated
/// token; j2000_seconds must point to epoch_count readable doubles; output
/// buffers must have room for epoch_count*3 and epoch_count doubles.
#[no_mangle]
pub unsafe extern "C" fn sidereon_sp3_interpolate(
    sp3: *const SidereonSp3,
    sat_id: *const c_char,
    j2000_seconds: *const f64,
    epoch_count: usize,
    out_position_m: *mut f64,
    position_len: usize,
    out_clock_s: *mut f64,
    clock_len: usize,
    out_written: *mut usize,
) -> SidereonStatus {
    ffi_boundary("sidereon_sp3_interpolate", SidereonStatus::Panic, || {
        let out_written = c_try!(require_out(
            out_written,
            "sidereon_sp3_interpolate",
            "out_written"
        ));
        *out_written = 0;
        let sp3 = c_try!(require_ref(sp3, "sidereon_sp3_interpolate", "sp3"));
        let sat = c_try!(parse_satellite_token("sidereon_sp3_interpolate", sat_id));
        let queries = c_try!(require_slice(
            j2000_seconds,
            epoch_count,
            "sidereon_sp3_interpolate",
            "j2000_seconds"
        ));
        if queries.is_empty() {
            set_last_error("sidereon_sp3_interpolate: j2000_seconds array is empty");
            return SidereonStatus::InvalidArgument;
        }
        c_try!(validate_element_count::<[f64; 3]>(
            "sidereon_sp3_interpolate",
            "epoch_count",
            queries.len(),
        ));
        c_try!(validate_element_count::<f64>(
            "sidereon_sp3_interpolate",
            "position_len",
            position_len,
        ));
        c_try!(validate_element_count::<f64>(
            "sidereon_sp3_interpolate",
            "clock_len",
            clock_len,
        ));
        let required_position_len = queries.len() * 3;
        if position_len < required_position_len {
            set_last_error(format!(
                "sidereon_sp3_interpolate: out_position_m needs room for {required_position_len} doubles"
            ));
            return SidereonStatus::InvalidArgument;
        }
        if clock_len < queries.len() {
            set_last_error(format!(
                "sidereon_sp3_interpolate: out_clock_s needs room for {} doubles",
                queries.len()
            ));
            return SidereonStatus::InvalidArgument;
        }
        if out_position_m.is_null() {
            set_last_error("sidereon_sp3_interpolate: null out_position_m");
            return SidereonStatus::NullPointer;
        }
        if out_clock_s.is_null() {
            set_last_error("sidereon_sp3_interpolate: null out_clock_s");
            return SidereonStatus::NullPointer;
        }

        let mut positions = Vec::with_capacity(queries.len());
        let mut clocks = Vec::with_capacity(queries.len());
        for &query in queries {
            let state = c_try!(guard_core(
                || sp3.inner.position_at_j2000_seconds(sat, query),
                |err| map_sp3_interpolation_error("sidereon_sp3_interpolate", query, err),
            ));
            positions.push(state.position.as_array());
            clocks.push(state.clock_s.unwrap_or(f64::NAN));
        }
        for (idx, position) in positions.iter().enumerate() {
            ptr::copy_nonoverlapping(position.as_ptr(), out_position_m.add(idx * 3), 3);
        }
        ptr::copy_nonoverlapping(clocks.as_ptr(), out_clock_s, clocks.len());
        *out_written = queries.len();
        SidereonStatus::Ok
    })
}

/// Export the product as SP3 text bytes. The output is not null-terminated.
/// Uses the variable-length output contract documented at the top of the
/// header.
///
/// Safety: sp3 must be a live handle; out must point to at least len writable
/// bytes or be NULL when len is 0; out_written and out_required must point to
/// size_t values.
#[no_mangle]
pub unsafe extern "C" fn sidereon_sp3_to_sp3_text(
    sp3: *const SidereonSp3,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary("sidereon_sp3_to_sp3_text", SidereonStatus::Panic, || {
        c_try!(init_copy_counts(
            "sidereon_sp3_to_sp3_text",
            out_written,
            out_required
        ));
        let sp3 = c_try!(require_ref(sp3, "sidereon_sp3_to_sp3_text", "sp3"));
        let text = sp3.inner.to_sp3_string();
        c_try!(copy_prefix_to_c(
            "sidereon_sp3_to_sp3_text",
            "out",
            text.as_bytes(),
            out,
            len,
            out_written,
            out_required,
        ));
        SidereonStatus::Ok
    })
}

/// Estimate the per-epoch clock-reference offset of `other` relative to
/// `reference`. Delegates to sidereon_core::ephemeris::clock_reference_offset.
/// Uses the variable-length output contract documented at the top of the
/// header.
///
/// Safety: reference and other must be live SP3 handles; out must point to at
/// least len writable SidereonSp3ClockReferenceOffset entries or be NULL when
/// len is 0; out_written and out_required must point to size_t values.
#[no_mangle]
pub unsafe extern "C" fn sidereon_sp3_clock_reference_offsets(
    reference: *const SidereonSp3,
    other: *const SidereonSp3,
    min_common: usize,
    out: *mut SidereonSp3ClockReferenceOffset,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_sp3_clock_reference_offsets",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_sp3_clock_reference_offsets",
                out_written,
                out_required
            ));
            let reference = c_try!(require_ref(
                reference,
                "sidereon_sp3_clock_reference_offsets",
                "reference"
            ));
            let other = c_try!(require_ref(
                other,
                "sidereon_sp3_clock_reference_offsets",
                "other"
            ));
            let values: Vec<SidereonSp3ClockReferenceOffset> =
                clock_reference_offset(&reference.inner, &other.inner, min_common)
                    .into_iter()
                    .map(|offset| SidereonSp3ClockReferenceOffset {
                        epoch_j2000_seconds: instant_to_j2000_seconds(&offset.epoch)
                            .unwrap_or(f64::NAN),
                        offset_s: offset.offset_s,
                        satellites: offset.satellites,
                    })
                    .collect();
            c_try!(copy_prefix_to_c(
                "sidereon_sp3_clock_reference_offsets",
                "out",
                &values,
                out,
                len,
                out_written,
                out_required,
            ));
            SidereonStatus::Ok
        },
    )
}

/// Return a copy of `other` with its clocks shifted onto `reference`'s clock
/// datum. Delegates to sidereon_core::ephemeris::align_clock_reference.
///
/// Safety: reference and other must be live SP3 handles; out_sp3 must point to
/// storage for a SidereonSp3*.
#[no_mangle]
pub unsafe extern "C" fn sidereon_sp3_align_clock_reference(
    reference: *const SidereonSp3,
    other: *const SidereonSp3,
    min_common: usize,
    out_sp3: *mut *mut SidereonSp3,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_sp3_align_clock_reference",
        SidereonStatus::Panic,
        || {
            let out_sp3 = c_try!(require_out(
                out_sp3,
                "sidereon_sp3_align_clock_reference",
                "out_sp3"
            ));
            *out_sp3 = ptr::null_mut();
            let reference = c_try!(require_ref(
                reference,
                "sidereon_sp3_align_clock_reference",
                "reference"
            ));
            let other = c_try!(require_ref(
                other,
                "sidereon_sp3_align_clock_reference",
                "other"
            ));
            let inner = align_clock_reference(&reference.inner, &other.inner, min_common);
            write_boxed_handle(out_sp3, SidereonSp3 { inner });
            SidereonStatus::Ok
        },
    )
}

/// Initialize SP3 merge options with engine defaults.
///
/// Safety: out_options must point to a SidereonSp3MergeOptions.
#[no_mangle]
pub unsafe extern "C" fn sidereon_sp3_merge_options_init(
    out_options: *mut SidereonSp3MergeOptions,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_sp3_merge_options_init",
        SidereonStatus::Panic,
        || {
            let out_options = c_try!(require_out(
                out_options,
                "sidereon_sp3_merge_options_init",
                "out_options"
            ));
            *out_options = default_sp3_merge_options();
            SidereonStatus::Ok
        },
    )
}

/// Merge SP3 products using the engine consensus merge path. On success writes
/// newly owned handles to *out_sp3 and *out_report. Release them with
/// sidereon_sp3_free and sidereon_sp3_merge_report_free.
///
/// Safety: sources must point to source_count live SidereonSp3* handles when
/// source_count is nonzero; options may be NULL for defaults; out_sp3 and
/// out_report must point to writable handle storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_sp3_merge(
    sources: *const *const SidereonSp3,
    source_count: usize,
    options: *const SidereonSp3MergeOptions,
    out_sp3: *mut *mut SidereonSp3,
    out_report: *mut *mut SidereonSp3MergeReport,
) -> SidereonStatus {
    ffi_boundary("sidereon_sp3_merge", SidereonStatus::Panic, || {
        let out_sp3 = c_try!(require_out(out_sp3, "sidereon_sp3_merge", "out_sp3"));
        *out_sp3 = ptr::null_mut();
        let out_report = c_try!(require_out(out_report, "sidereon_sp3_merge", "out_report"));
        *out_report = ptr::null_mut();
        c_try!(validate_element_count::<Sp3>(
            "sidereon_sp3_merge",
            "source_count",
            source_count
        ));
        let source_handles = c_try!(require_slice(
            sources,
            source_count,
            "sidereon_sp3_merge",
            "sources"
        ));
        let mut core_sources = Vec::with_capacity(source_handles.len());
        for (idx, source) in source_handles.iter().copied().enumerate() {
            let source = c_try!(require_ref(
                source,
                "sidereon_sp3_merge",
                &format!("sources[{idx}]")
            ));
            core_sources.push(source.inner.clone());
        }
        let options = c_try!(sp3_merge_options_from_c("sidereon_sp3_merge", options));
        let (inner, report) = c_try!(guard_core(
            || merge(&core_sources, &options),
            |err| map_sp3_argument_error("sidereon_sp3_merge", err),
        ));
        let sp3_handle = Box::new(SidereonSp3 { inner });
        let epoch_agreement = report.per_epoch_agreement();
        let report_handle = Box::new(SidereonSp3MergeReport {
            inner: report,
            epoch_agreement,
        });
        *out_sp3 = Box::into_raw(sp3_handle);
        *out_report = Box::into_raw(report_handle);
        SidereonStatus::Ok
    })
}

/// Write the number of coordinate-label reconciliation rows in a merge report.
///
/// Safety: report must be a live merge report handle; out_count must point to a
/// size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_sp3_merge_report_frame_reconciliation_count(
    report: *const SidereonSp3MergeReport,
    out_count: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_sp3_merge_report_frame_reconciliation_count",
        SidereonStatus::Panic,
        || {
            let out_count = c_try!(require_out(
                out_count,
                "sidereon_sp3_merge_report_frame_reconciliation_count",
                "out_count"
            ));
            *out_count = 0;
            let report = c_try!(require_ref(
                report,
                "sidereon_sp3_merge_report_frame_reconciliation_count",
                "report"
            ));
            *out_count = report.inner.frame_reconciliations.len();
            SidereonStatus::Ok
        },
    )
}

/// Copy one coordinate-label reconciliation row by index.
///
/// Safety: report must be a live merge report handle; out_reconciliation must
/// point to a SidereonSp3FrameReconciliation.
#[no_mangle]
pub unsafe extern "C" fn sidereon_sp3_merge_report_frame_reconciliation(
    report: *const SidereonSp3MergeReport,
    index: usize,
    out_reconciliation: *mut SidereonSp3FrameReconciliation,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_sp3_merge_report_frame_reconciliation",
        SidereonStatus::Panic,
        || {
            let out_reconciliation = c_try!(require_out(
                out_reconciliation,
                "sidereon_sp3_merge_report_frame_reconciliation",
                "out_reconciliation"
            ));
            *out_reconciliation = zero_sp3_frame_reconciliation();
            let report = c_try!(require_ref(
                report,
                "sidereon_sp3_merge_report_frame_reconciliation",
                "report"
            ));
            let Some(reconciliation) = report.inner.frame_reconciliations.get(index) else {
                set_last_error(format!(
                    "sidereon_sp3_merge_report_frame_reconciliation: index {index} out of range"
                ));
                return SidereonStatus::InvalidArgument;
            };
            *out_reconciliation = sp3_frame_reconciliation_to_c(reconciliation);
            SidereonStatus::Ok
        },
    )
}

/// Copy a reconciliation source label as UTF-8 bytes.
///
/// Safety: report must be a live merge report handle; out may be NULL only when
/// len is zero; out_written and out_required must point to size_t values.
#[no_mangle]
pub unsafe extern "C" fn sidereon_sp3_merge_report_frame_reconciliation_source_label(
    report: *const SidereonSp3MergeReport,
    index: usize,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    sp3_merge_report_reconciliation_bytes(
        "sidereon_sp3_merge_report_frame_reconciliation_source_label",
        report,
        index,
        |row| row.source_label.as_bytes(),
        out,
        len,
        out_written,
        out_required,
    )
}

/// Copy a reconciliation target label as UTF-8 bytes.
///
/// Safety: report must be a live merge report handle; out may be NULL only when
/// len is zero; out_written and out_required must point to size_t values.
#[no_mangle]
pub unsafe extern "C" fn sidereon_sp3_merge_report_frame_reconciliation_target_label(
    report: *const SidereonSp3MergeReport,
    index: usize,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    sp3_merge_report_reconciliation_bytes(
        "sidereon_sp3_merge_report_frame_reconciliation_target_label",
        report,
        index,
        |row| row.target_label.as_bytes(),
        out,
        len,
        out_written,
        out_required,
    )
}

/// Copy one asserted-label-set item as UTF-8 bytes.
///
/// Safety: report must be a live merge report handle; out may be NULL only when
/// len is zero; out_written and out_required must point to size_t values.
#[no_mangle]
pub unsafe extern "C" fn sidereon_sp3_merge_report_frame_reconciliation_asserted_label(
    report: *const SidereonSp3MergeReport,
    index: usize,
    label_index: usize,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    sp3_merge_report_reconciliation_bytes(
        "sidereon_sp3_merge_report_frame_reconciliation_asserted_label",
        report,
        index,
        |row| {
            row.asserted_label_set
                .as_ref()
                .and_then(|labels| labels.get(label_index))
                .map(String::as_bytes)
                .unwrap_or(&[])
        },
        out,
        len,
        out_written,
        out_required,
    )
}

/// Copy the published-table provenance as UTF-8 bytes.
///
/// Safety: report must be a live merge report handle; out may be NULL only when
/// len is zero; out_written and out_required must point to size_t values.
#[no_mangle]
pub unsafe extern "C" fn sidereon_sp3_merge_report_frame_reconciliation_provenance(
    report: *const SidereonSp3MergeReport,
    index: usize,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    sp3_merge_report_reconciliation_bytes(
        "sidereon_sp3_merge_report_frame_reconciliation_provenance",
        report,
        index,
        |row| row.provenance.as_deref().unwrap_or("").as_bytes(),
        out,
        len,
        out_written,
        out_required,
    )
}

/// Write the number of flags in a merge report flag list to *out_count. kind is
/// one of SidereonSp3MergeFlagKind_*.
///
/// Safety: report must be a live merge report handle; out_count must point to a
/// size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_sp3_merge_report_flag_count(
    report: *const SidereonSp3MergeReport,
    kind: u32,
    out_count: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_sp3_merge_report_flag_count",
        SidereonStatus::Panic,
        || {
            let out_count = c_try!(require_out(
                out_count,
                "sidereon_sp3_merge_report_flag_count",
                "out_count"
            ));
            *out_count = 0;
            let report = c_try!(require_ref(
                report,
                "sidereon_sp3_merge_report_flag_count",
                "report"
            ));
            let flags = c_try!(sp3_merge_flag_slice(
                "sidereon_sp3_merge_report_flag_count",
                &report.inner,
                kind,
            ));
            *out_count = flags.len();
            SidereonStatus::Ok
        },
    )
}

/// Copy one merge report flag by index. kind is one of
/// SidereonSp3MergeFlagKind_*.
///
/// Safety: report must be a live merge report handle; out_flag must point to a
/// SidereonSp3MergeFlag.
#[no_mangle]
pub unsafe extern "C" fn sidereon_sp3_merge_report_flag(
    report: *const SidereonSp3MergeReport,
    kind: u32,
    index: usize,
    out_flag: *mut SidereonSp3MergeFlag,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_sp3_merge_report_flag",
        SidereonStatus::Panic,
        || {
            let out_flag = c_try!(require_out(
                out_flag,
                "sidereon_sp3_merge_report_flag",
                "out_flag"
            ));
            *out_flag = SidereonSp3MergeFlag {
                epoch_j2000_seconds: 0.0,
                sat_id: SidereonSatelliteToken {
                    bytes: [0; SATELLITE_TOKEN_C_BYTES],
                },
                source_count: 0,
            };
            let report = c_try!(require_ref(
                report,
                "sidereon_sp3_merge_report_flag",
                "report"
            ));
            let flags = c_try!(sp3_merge_flag_slice(
                "sidereon_sp3_merge_report_flag",
                &report.inner,
                kind,
            ));
            let Some(flag) = flags.get(index) else {
                set_last_error(format!(
                    "sidereon_sp3_merge_report_flag: flag index {index} out of range"
                ));
                return SidereonStatus::InvalidArgument;
            };
            *out_flag = sp3_merge_flag_to_c(flag);
            SidereonStatus::Ok
        },
    )
}

/// Copy the source indices for one merge report flag. kind is one of
/// SidereonSp3MergeFlagKind_* and the output uses the variable-length contract
/// documented at the top of the header.
///
/// Safety: report must be a live merge report handle; out must point to at
/// least len writable size_t values or be NULL when len is 0; out_written and
/// out_required must point to size_t values.
#[no_mangle]
pub unsafe extern "C" fn sidereon_sp3_merge_report_flag_sources(
    report: *const SidereonSp3MergeReport,
    kind: u32,
    index: usize,
    out: *mut usize,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_sp3_merge_report_flag_sources",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_sp3_merge_report_flag_sources",
                out_written,
                out_required
            ));
            let report = c_try!(require_ref(
                report,
                "sidereon_sp3_merge_report_flag_sources",
                "report"
            ));
            let flags = c_try!(sp3_merge_flag_slice(
                "sidereon_sp3_merge_report_flag_sources",
                &report.inner,
                kind,
            ));
            let Some(flag) = flags.get(index) else {
                set_last_error(format!(
                    "sidereon_sp3_merge_report_flag_sources: flag index {index} out of range"
                ));
                return SidereonStatus::InvalidArgument;
            };
            c_try!(copy_prefix_to_c(
                "sidereon_sp3_merge_report_flag_sources",
                "out",
                &flag.sources,
                out,
                len,
                out_written,
                out_required,
            ));
            SidereonStatus::Ok
        },
    )
}

/// Write the number of per-epoch agreement entries to *out_count. This is the
/// length of the list copied element-wise by
/// sidereon_sp3_merge_report_epoch_agreement (one entry per output epoch, with
/// epochs sharing an integer second pooled, per the SidereonSp3EpochAgreement
/// note).
///
/// Safety: report must be a live merge report handle; out_count must point to a
/// size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_sp3_merge_report_epoch_agreement_count(
    report: *const SidereonSp3MergeReport,
    out_count: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_sp3_merge_report_epoch_agreement_count",
        SidereonStatus::Panic,
        || {
            let out_count = c_try!(require_out(
                out_count,
                "sidereon_sp3_merge_report_epoch_agreement_count",
                "out_count"
            ));
            *out_count = 0;
            let report = c_try!(require_ref(
                report,
                "sidereon_sp3_merge_report_epoch_agreement_count",
                "report"
            ));
            *out_count = report.epoch_agreement.len();
            SidereonStatus::Ok
        },
    )
}

/// Copy one per-epoch agreement entry (by zero-based output-epoch index) into
/// *out_agreement. Fails with SIDEREON_STATUS_INVALID_ARGUMENT if index is out of
/// range (see sidereon_sp3_merge_report_epoch_agreement_count).
///
/// Safety: report must be a live merge report handle; out_agreement must point to
/// a SidereonSp3EpochAgreement.
#[no_mangle]
pub unsafe extern "C" fn sidereon_sp3_merge_report_epoch_agreement(
    report: *const SidereonSp3MergeReport,
    index: usize,
    out_agreement: *mut SidereonSp3EpochAgreement,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_sp3_merge_report_epoch_agreement",
        SidereonStatus::Panic,
        || {
            let out_agreement = c_try!(require_out(
                out_agreement,
                "sidereon_sp3_merge_report_epoch_agreement",
                "out_agreement"
            ));
            *out_agreement = SidereonSp3EpochAgreement {
                epoch_j2000_seconds: 0.0,
                satellites: 0,
                position_rms_m: 0.0,
                position_max_m: 0.0,
                clock_rms_present: false,
                clock_rms_s: 0.0,
                clock_max_present: false,
                clock_max_s: 0.0,
            };
            let report = c_try!(require_ref(
                report,
                "sidereon_sp3_merge_report_epoch_agreement",
                "report"
            ));
            let Some(agg) = report.epoch_agreement.get(index) else {
                set_last_error(format!(
                    "sidereon_sp3_merge_report_epoch_agreement: index {index} out of range ({} epochs)",
                    report.epoch_agreement.len()
                ));
                return SidereonStatus::InvalidArgument;
            };
            *out_agreement = sp3_epoch_agreement_to_c(agg);
            SidereonStatus::Ok
        },
    )
}

/// Write the whole-product agreement rollup (pooled position/clock dispersion of
/// the consensus members about the combined values) to *out_summary. Each scalar
/// carries a present flag; an absent value (no accepted multi-source cell on that
/// channel) sets the flag false and the scalar to 0.
///
/// Safety: report must be a live merge report handle; out_summary must point to a
/// SidereonSp3AgreementSummary.
#[no_mangle]
pub unsafe extern "C" fn sidereon_sp3_merge_report_agreement_summary(
    report: *const SidereonSp3MergeReport,
    out_summary: *mut SidereonSp3AgreementSummary,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_sp3_merge_report_agreement_summary",
        SidereonStatus::Panic,
        || {
            let out_summary = c_try!(require_out(
                out_summary,
                "sidereon_sp3_merge_report_agreement_summary",
                "out_summary"
            ));
            *out_summary = SidereonSp3AgreementSummary {
                position_rms_present: false,
                position_rms_m: 0.0,
                position_max_present: false,
                position_max_m: 0.0,
                clock_rms_present: false,
                clock_rms_s: 0.0,
                clock_max_present: false,
                clock_max_s: 0.0,
            };
            let report = c_try!(require_ref(
                report,
                "sidereon_sp3_merge_report_agreement_summary",
                "report"
            ));
            let pos_rms = report.inner.position_agreement_rms_m();
            let pos_max = report.inner.position_agreement_max_m();
            let clk_rms = report.inner.clock_agreement_rms_s();
            let clk_max = report.inner.clock_agreement_max_s();
            *out_summary = SidereonSp3AgreementSummary {
                position_rms_present: pos_rms.is_some(),
                position_rms_m: pos_rms.unwrap_or(0.0),
                position_max_present: pos_max.is_some(),
                position_max_m: pos_max.unwrap_or(0.0),
                clock_rms_present: clk_rms.is_some(),
                clock_rms_s: clk_rms.unwrap_or(0.0),
                clock_max_present: clk_max.is_some(),
                clock_max_s: clk_max.unwrap_or(0.0),
            };
            SidereonStatus::Ok
        },
    )
}

/// Release an SP3 merge report handle. Null is a no-op. A non-null handle must
/// come from sidereon_sp3_merge and must be freed exactly once with this
/// function.
///
/// Safety: report must be NULL or a live handle from sidereon_sp3_merge. Passing
/// a handle after it has already been freed is invalid.
#[no_mangle]
pub unsafe extern "C" fn sidereon_sp3_merge_report_free(report: *mut SidereonSp3MergeReport) {
    ffi_boundary("sidereon_sp3_merge_report_free", (), || {
        free_boxed(report);
    });
}

/// Release an SP3 handle. Null is a no-op. A non-null handle must come from
/// sidereon_sp3_load or sidereon_sp3_merge and must be freed exactly once with
/// this function.
///
/// Safety: sp3 must be NULL or a live handle from sidereon_sp3_load or
/// sidereon_sp3_merge. Passing a handle after it has already been freed is
/// invalid.
#[no_mangle]
pub unsafe extern "C" fn sidereon_sp3_free(sp3: *mut SidereonSp3) {
    ffi_boundary("sidereon_sp3_free", (), || {
        free_boxed(sp3);
    });
}

/// List satellites visible from a static receiver at one epoch, scanning the SP3
/// product's own satellites. Delegates to sidereon_core::geometry::visible. Uses
/// the variable-length output contract documented at the top of the header.
///
/// `systems` may be NULL with systems_len 0 to keep every constellation, or
/// point to systems_len SidereonGnssSystem codes (cast to uint32_t) to filter.
///
/// Safety: sp3 must be a live handle; receiver_ecef_m must point to three
/// readable doubles; systems must point to systems_len readable uint32_t (or be
/// NULL when systems_len is 0); out must point to at least len writable
/// SidereonGeometryVisible or be NULL when len is 0; out_written and out_required
/// must point to size_t values.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn sidereon_sp3_geometry_visible(
    sp3: *const SidereonSp3,
    receiver_ecef_m: *const f64,
    t_rx_j2000_s: f64,
    elevation_mask_deg: f64,
    systems: *const u32,
    systems_len: usize,
    out: *mut SidereonGeometryVisible,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_sp3_geometry_visible",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_sp3_geometry_visible",
                out_written,
                out_required
            ));
            let sp3 = c_try!(require_ref(sp3, "sidereon_sp3_geometry_visible", "sp3"));
            let receiver = c_try!(require_slice(
                receiver_ecef_m,
                3,
                "sidereon_sp3_geometry_visible",
                "receiver_ecef_m"
            ));
            let receiver_ecef_m = [receiver[0], receiver[1], receiver[2]];
            let options = c_try!(visibility_options_from_c(
                "sidereon_sp3_geometry_visible",
                elevation_mask_deg,
                systems,
                systems_len
            ));
            let rows = c_try!(guard_dop("sidereon_sp3_geometry_visible", || {
                geometry_visible(
                    &sp3.inner,
                    sp3.inner.satellites(),
                    receiver_ecef_m,
                    t_rx_j2000_s,
                    &options,
                )
            }));
            let values: Vec<SidereonGeometryVisible> =
                rows.iter().map(geometry_visible_to_c).collect();
            c_try!(copy_prefix_to_c(
                "sidereon_sp3_geometry_visible",
                "out",
                &values,
                out,
                len,
                out_written,
                out_required,
            ));
            SidereonStatus::Ok
        },
    )
}

/// Count visible satellites over an inclusive sampled window. Delegates to
/// sidereon_core::geometry::visibility_series. Uses the variable-length output
/// contract documented at the top of the header.
///
/// Safety: sp3 must be a live handle; receiver_ecef_m must point to three
/// readable doubles; systems follows sidereon_sp3_geometry_visible; out must
/// point to at least len writable SidereonVisibilitySeriesPoint or be NULL when
/// len is 0; out_written and out_required must point to size_t values.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn sidereon_sp3_geometry_visibility_series(
    sp3: *const SidereonSp3,
    receiver_ecef_m: *const f64,
    window_start_j2000_s: f64,
    window_end_j2000_s: f64,
    step_seconds: u64,
    elevation_mask_deg: f64,
    systems: *const u32,
    systems_len: usize,
    out: *mut SidereonVisibilitySeriesPoint,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_sp3_geometry_visibility_series",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_sp3_geometry_visibility_series",
                out_written,
                out_required
            ));
            let sp3 = c_try!(require_ref(
                sp3,
                "sidereon_sp3_geometry_visibility_series",
                "sp3"
            ));
            let receiver = c_try!(require_slice(
                receiver_ecef_m,
                3,
                "sidereon_sp3_geometry_visibility_series",
                "receiver_ecef_m"
            ));
            let receiver_ecef_m = [receiver[0], receiver[1], receiver[2]];
            let options = c_try!(visibility_options_from_c(
                "sidereon_sp3_geometry_visibility_series",
                elevation_mask_deg,
                systems,
                systems_len
            ));
            let points = c_try!(guard_dop("sidereon_sp3_geometry_visibility_series", || {
                geometry_visibility_series(
                    &sp3.inner,
                    sp3.inner.satellites(),
                    receiver_ecef_m,
                    (window_start_j2000_s, window_end_j2000_s),
                    step_seconds,
                    &options,
                )
            }));
            let values: Vec<SidereonVisibilitySeriesPoint> =
                points.iter().map(visibility_series_point_to_c).collect();
            c_try!(copy_prefix_to_c(
                "sidereon_sp3_geometry_visibility_series",
                "out",
                &values,
                out,
                len,
                out_written,
                out_required,
            ));
            SidereonStatus::Ok
        },
    )
}

/// Build sampled rise/set/peak visibility passes over an inclusive window.
/// Delegates to sidereon_core::geometry::passes. Uses the variable-length output
/// contract documented at the top of the header.
///
/// Safety: sp3 must be a live handle; receiver_ecef_m must point to three
/// readable doubles; systems follows sidereon_sp3_geometry_visible; out must
/// point to at least len writable SidereonVisibilityPass or be NULL when len is
/// 0; out_written and out_required must point to size_t values.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn sidereon_sp3_geometry_passes(
    sp3: *const SidereonSp3,
    receiver_ecef_m: *const f64,
    window_start_j2000_s: f64,
    window_end_j2000_s: f64,
    step_seconds: u64,
    elevation_mask_deg: f64,
    systems: *const u32,
    systems_len: usize,
    out: *mut SidereonVisibilityPass,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_sp3_geometry_passes",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_sp3_geometry_passes",
                out_written,
                out_required
            ));
            let sp3 = c_try!(require_ref(sp3, "sidereon_sp3_geometry_passes", "sp3"));
            let receiver = c_try!(require_slice(
                receiver_ecef_m,
                3,
                "sidereon_sp3_geometry_passes",
                "receiver_ecef_m"
            ));
            let receiver_ecef_m = [receiver[0], receiver[1], receiver[2]];
            let options = c_try!(visibility_options_from_c(
                "sidereon_sp3_geometry_passes",
                elevation_mask_deg,
                systems,
                systems_len
            ));
            let found = c_try!(guard_dop("sidereon_sp3_geometry_passes", || {
                geometry_passes(
                    &sp3.inner,
                    sp3.inner.satellites(),
                    receiver_ecef_m,
                    (window_start_j2000_s, window_end_j2000_s),
                    step_seconds,
                    &options,
                )
            }));
            let values: Vec<SidereonVisibilityPass> =
                found.iter().map(visibility_pass_to_c).collect();
            c_try!(copy_prefix_to_c(
                "sidereon_sp3_geometry_passes",
                "out",
                &values,
                out,
                len,
                out_written,
                out_required,
            ));
            SidereonStatus::Ok
        },
    )
}

// ===========================================================================
// Predicted observables (line of sight, range rate, Doppler, az/el) from an SP3
// or broadcast source. Delegates to sidereon_core::observables::predict.

/// Predict one satellite's observables from a loaded SP3 product. Delegates to
/// sidereon_core::observables::predict. options may be NULL for the engine
/// defaults.
///
/// Safety: sp3 must be a live handle; sat_id must be a null-terminated token;
/// receiver_ecef_m must point to three readable doubles; options must be NULL or
/// point to a SidereonObservablesOptions; out must point to a
/// SidereonPredictedObservables.
#[no_mangle]
pub unsafe extern "C" fn sidereon_sp3_observables(
    sp3: *const SidereonSp3,
    sat_id: *const c_char,
    receiver_ecef_m: *const f64,
    t_rx_j2000_s: f64,
    options: *const SidereonObservablesOptions,
    out: *mut SidereonPredictedObservables,
) -> SidereonStatus {
    ffi_boundary("sidereon_sp3_observables", SidereonStatus::Panic, || {
        let out = c_try!(require_out(out, "sidereon_sp3_observables", "out"));
        let sp3 = c_try!(require_ref(sp3, "sidereon_sp3_observables", "sp3"));
        let satellite = c_try!(parse_satellite_token("sidereon_sp3_observables", sat_id));
        let receiver = c_try!(require_slice(
            receiver_ecef_m,
            3,
            "sidereon_sp3_observables",
            "receiver_ecef_m"
        ));
        let receiver_ecef_m = [receiver[0], receiver[1], receiver[2]];
        let opts = c_try!(predict_options_from_c("sidereon_sp3_observables", options));
        let obs =
            match observables_predict(&sp3.inner, satellite, receiver_ecef_m, t_rx_j2000_s, opts) {
                Ok(obs) => obs,
                Err(err) => return map_observables_error("sidereon_sp3_observables", err),
            };
        *out = predicted_observables_to_c(&obs);
        SidereonStatus::Ok
    })
}

/// Evaluate an SP3 precise product at a J2000 second for one satellite via the
/// same ObservableEphemerisSource contract as the broadcast path. Delegates to
/// sidereon_core::observables::ObservableEphemerisSource::observable_state_at_j2000_s.
///
/// Safety: as sidereon_broadcast_observable_state with an SP3 handle.
#[no_mangle]
pub unsafe extern "C" fn sidereon_sp3_observable_state(
    sp3: *const SidereonSp3,
    satellite_id: *const c_char,
    t_j2000_s: f64,
    out_position_ecef_m: *mut f64,
    out_clock_s: *mut f64,
    out_has_clock: *mut bool,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_sp3_observable_state",
        SidereonStatus::Panic,
        || {
            let sp3 = c_try!(require_ref(sp3, "sidereon_sp3_observable_state", "sp3"));
            observable_state_common(
                "sidereon_sp3_observable_state",
                &sp3.inner,
                satellite_id,
                t_j2000_s,
                out_position_ecef_m,
                out_clock_s,
                out_has_clock,
            )
        },
    )
}

/// Predict observables for many `(satellite, receiver, epoch)` requests from a
/// loaded SP3 product in one call. Delegates to the core serial `predict_batch`.
/// `out` and `out_ok` are caller arrays of `count` entries: `out[i]` holds the
/// observables for `requests[i]` and `out_ok[i]` is true on success. A
/// per-request failure (e.g. no ephemeris) sets `out_ok[i]` false and zeroes
/// `out[i]`; the call still returns SIDEREON_STATUS_OK. options may be NULL for
/// the engine defaults.
///
/// Safety: sp3 must be a live handle; requests must point to count entries (each
/// with a valid sat_id); out and out_ok must each point to count writable
/// entries (or be NULL when count is 0); options must be NULL or point to a
/// SidereonObservablesOptions.
#[no_mangle]
pub unsafe extern "C" fn sidereon_sp3_observables_batch(
    sp3: *const SidereonSp3,
    requests: *const SidereonPredictRequest,
    count: usize,
    options: *const SidereonObservablesOptions,
    out: *mut SidereonPredictedObservables,
    out_ok: *mut bool,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_sp3_observables_batch",
        SidereonStatus::Panic,
        || {
            let sp3 = c_try!(require_ref(sp3, "sidereon_sp3_observables_batch", "sp3"));
            let raw = c_try!(require_slice(
                requests,
                count,
                "sidereon_sp3_observables_batch",
                "requests"
            ));
            // Guard the caller's output arrays (non-null when count > 0, no element
            // overflow) before writing them element-by-element below.
            c_try!(require_slice(
                out as *const SidereonPredictedObservables,
                count,
                "sidereon_sp3_observables_batch",
                "out"
            ));
            c_try!(require_slice(
                out_ok as *const bool,
                count,
                "sidereon_sp3_observables_batch",
                "out_ok"
            ));
            let opts = c_try!(predict_options_from_c(
                "sidereon_sp3_observables_batch",
                options
            ));
            let parsed = c_try!(predict_requests_from_c(
                "sidereon_sp3_observables_batch",
                raw
            ));
            let results = core_predict_batch(&sp3.inner, &parsed, opts);
            write_predict_batch_results(&results, out, out_ok);
            SidereonStatus::Ok
        },
    )
}

/// Extract a loaded SP3 product as its canonical precise-ephemeris samples, in
/// SI units, one per real position record in ascending epoch order. Round-tripping
/// through sidereon_precise_ephemeris_samples_from_samples rebuilds the same
/// interpolatable source. Uses the variable-length output contract documented at
/// the top of the header.
///
/// Safety: sp3 must be a live handle; out must point to at least len writable
/// SidereonPreciseEphemerisSample or be NULL when len is 0; out_written and
/// out_required must point to size_t values.
#[no_mangle]
pub unsafe extern "C" fn sidereon_sp3_precise_ephemeris_samples(
    sp3: *const SidereonSp3,
    out: *mut SidereonPreciseEphemerisSample,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_sp3_precise_ephemeris_samples",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_sp3_precise_ephemeris_samples",
                out_written,
                out_required
            ));
            let sp3 = c_try!(require_ref(
                sp3,
                "sidereon_sp3_precise_ephemeris_samples",
                "sp3"
            ));
            let samples = sp3.inner.precise_ephemeris_samples();
            let values: Vec<SidereonPreciseEphemerisSample> =
                samples.iter().map(precise_sample_to_c).collect();
            c_try!(copy_prefix_to_c(
                "sidereon_sp3_precise_ephemeris_samples",
                "out",
                &values,
                out,
                len,
                out_written,
                out_required,
            ));
            SidereonStatus::Ok
        },
    )
}

/// Sample a loaded SP3 source over a regular satellite/epoch grid.
///
/// Safety: sp3 must be a live handle; satellites points to satellite_count
/// null-terminated tokens; out points to len SidereonEphemerisSampleRow or NULL
/// when len is 0; out_written and out_required point to size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_sp3_ephemeris_sample(
    sp3: *const SidereonSp3,
    satellites: *const *const c_char,
    satellite_count: usize,
    start_j2000_s: f64,
    stop_j2000_s: f64,
    step_s: f64,
    out: *mut SidereonEphemerisSampleRow,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_sp3_ephemeris_sample",
        SidereonStatus::Panic,
        || {
            let sp3 = c_try!(require_ref(sp3, "sidereon_sp3_ephemeris_sample", "sp3"));
            ephemeris_sample_common(
                "sidereon_sp3_ephemeris_sample",
                &sp3.inner,
                satellites,
                satellite_count,
                start_j2000_s,
                stop_j2000_s,
                step_s,
                out,
                len,
                out_written,
                out_required,
            )
        },
    )
}

/// Predict geometric ranges for many (satellite, receiver, epoch) requests from a
/// loaded SP3 product in one call, writing out[i] for requests[i]. Delegates to
/// sidereon_core::observables::predict_ranges. options may be NULL for the engine
/// defaults.
///
/// Safety: sp3 must be a live handle; requests must point to count entries (each
/// with a valid sat_id); out must point to count writable entries (or be NULL
/// when count is 0); options must be NULL or point to a
/// SidereonObservablesOptions.
#[no_mangle]
pub unsafe extern "C" fn sidereon_sp3_predict_ranges(
    sp3: *const SidereonSp3,
    requests: *const SidereonRangePredictionRequest,
    count: usize,
    options: *const SidereonObservablesOptions,
    out: *mut SidereonRangePrediction,
) -> SidereonStatus {
    ffi_boundary("sidereon_sp3_predict_ranges", SidereonStatus::Panic, || {
        let sp3 = c_try!(require_ref(sp3, "sidereon_sp3_predict_ranges", "sp3"));
        predict_ranges_into(
            "sidereon_sp3_predict_ranges",
            &sp3.inner,
            requests,
            count,
            options,
            out,
        )
    })
}

/// Evaluate many SP3 observable states with per-satellite epochs.
///
/// out_positions_ecef_m receives count triples in row-major XYZ order, meters.
/// out_clocks_s and out_has_clocks_s receive count satellite-clock entries in
/// seconds. out_element_statuses receives Valid/Gap/Error. out_result_statuses
/// receives SIDEREON_STATUS_OK for valid elements, SIDEREON_STATUS_SOLVE for
/// gaps/source errors, and SIDEREON_STATUS_INVALID_ARGUMENT for invalid scalar
/// inputs. Failed elements receive the missing-position sentinel.
///
/// Safety: sp3 is a live handle; satellites and epochs_j2000_s point to count
/// entries; output arrays point to count entries except out_positions_ecef_m,
/// which points to count * 3 doubles.
#[no_mangle]
pub unsafe extern "C" fn sidereon_sp3_observable_states_at_j2000_s(
    sp3: *const SidereonSp3,
    satellites: *const *const c_char,
    epochs_j2000_s: *const f64,
    count: usize,
    out_positions_ecef_m: *mut f64,
    out_clocks_s: *mut f64,
    out_has_clocks_s: *mut bool,
    out_element_statuses: *mut SidereonObservableStateElementStatus,
    out_result_statuses: *mut SidereonStatus,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_sp3_observable_states_at_j2000_s",
        SidereonStatus::Panic,
        || {
            let sp3 = c_try!(require_ref(
                sp3,
                "sidereon_sp3_observable_states_at_j2000_s",
                "sp3"
            ));
            observable_states_at_j2000_s_common(
                "sidereon_sp3_observable_states_at_j2000_s",
                &sp3.inner,
                satellites,
                epochs_j2000_s,
                count,
                out_positions_ecef_m,
                out_clocks_s,
                out_has_clocks_s,
                out_element_statuses,
                out_result_statuses,
            )
        },
    )
}

/// Evaluate many SP3 observable states at one shared epoch.
///
/// Safety: same output-array contract as
/// sidereon_sp3_observable_states_at_j2000_s.
#[no_mangle]
pub unsafe extern "C" fn sidereon_sp3_observable_states_at_shared_j2000_s(
    sp3: *const SidereonSp3,
    satellites: *const *const c_char,
    satellite_count: usize,
    epoch_j2000_s: f64,
    out_positions_ecef_m: *mut f64,
    out_clocks_s: *mut f64,
    out_has_clocks_s: *mut bool,
    out_element_statuses: *mut SidereonObservableStateElementStatus,
    out_result_statuses: *mut SidereonStatus,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_sp3_observable_states_at_shared_j2000_s",
        SidereonStatus::Panic,
        || {
            let sp3 = c_try!(require_ref(
                sp3,
                "sidereon_sp3_observable_states_at_shared_j2000_s",
                "sp3"
            ));
            observable_states_at_shared_j2000_s_common(
                "sidereon_sp3_observable_states_at_shared_j2000_s",
                &sp3.inner,
                satellites,
                satellite_count,
                epoch_j2000_s,
                out_positions_ecef_m,
                out_clocks_s,
                out_has_clocks_s,
                out_element_statuses,
                out_result_statuses,
            )
        },
    )
}

fn map_sp3_argument_error(fn_name: &str, err: CoreError) -> SidereonStatus {
    set_last_error(format!("{fn_name}: {err}"));
    match err {
        CoreError::Parse(_) => SidereonStatus::Sp3Parse,
        CoreError::UnknownSatellite(_)
        | CoreError::EpochOutOfRange
        | CoreError::InvalidInput(_) => SidereonStatus::InvalidArgument,
        _ => SidereonStatus::Solve,
    }
}

fn map_sp3_interpolation_error(fn_name: &str, query: f64, err: CoreError) -> SidereonStatus {
    set_last_error(format!(
        "{fn_name}: interpolation at j2000 second {query}: {err}"
    ));
    match err {
        CoreError::Parse(_) => SidereonStatus::Sp3Parse,
        CoreError::UnknownSatellite(_) | CoreError::InvalidInput(_) => {
            SidereonStatus::InvalidArgument
        }
        CoreError::EpochOutOfRange => SidereonStatus::Solve,
        _ => SidereonStatus::Solve,
    }
}

fn empty_sp3_state() -> SidereonSp3State {
    SidereonSp3State {
        position_m: [0.0; 3],
        has_clock_s: false,
        clock_s: 0.0,
        has_velocity_m_s: false,
        velocity_m_s: [0.0; 3],
        has_clock_rate_s_s: false,
        clock_rate_s_s: 0.0,
        clock_event: false,
        clock_predicted: false,
        maneuver: false,
        orbit_predicted: false,
    }
}

fn sp3_state_to_c(state: Sp3State) -> SidereonSp3State {
    SidereonSp3State {
        position_m: state.position.as_array(),
        has_clock_s: state.clock_s.is_some(),
        clock_s: state.clock_s.unwrap_or(0.0),
        has_velocity_m_s: state.velocity.is_some(),
        velocity_m_s: state
            .velocity
            .map(|velocity| velocity.as_array())
            .unwrap_or([0.0; 3]),
        has_clock_rate_s_s: state.clock_rate_s_s.is_some(),
        clock_rate_s_s: state.clock_rate_s_s.unwrap_or(0.0),
        clock_event: state.flags.clock_event,
        clock_predicted: state.flags.clock_predicted,
        maneuver: state.flags.maneuver,
        orbit_predicted: state.flags.orbit_predicted,
    }
}

fn default_sp3_merge_options() -> SidereonSp3MergeOptions {
    let options = MergeOptions::default();
    SidereonSp3MergeOptions {
        position_tolerance_m: options.position_tolerance_m,
        clock_tolerance_s: options.clock_tolerance_s,
        min_agree: options.min_agree,
        clock_min_common: options.clock_min_common,
        combine: SidereonSp3MergeCombine::Mean as u32,
        target_epoch_interval_s_enabled: options.target_epoch_interval_s.is_some(),
        target_epoch_interval_s: options.target_epoch_interval_s.unwrap_or(0.0),
        systems: ptr::null(),
        system_count: 0,
        asserted_frame_label_sets: ptr::null(),
        asserted_frame_label_set_count: 0,
        helmert_frame_reconciliation: false,
    }
}

unsafe fn sp3_merge_options_from_c(
    fn_name: &str,
    options: *const SidereonSp3MergeOptions,
) -> Result<MergeOptions, SidereonStatus> {
    let Some(options) = options.as_ref() else {
        return Ok(MergeOptions::default());
    };
    require_positive_finite(
        fn_name,
        "position_tolerance_m",
        options.position_tolerance_m,
    )?;
    require_positive_finite(fn_name, "clock_tolerance_s", options.clock_tolerance_s)?;
    if options.min_agree == 0 {
        set_last_error(format!("{fn_name}: min_agree must be at least 1"));
        return Err(SidereonStatus::InvalidArgument);
    }
    if options.clock_min_common == 0 {
        set_last_error(format!("{fn_name}: clock_min_common must be at least 1"));
        return Err(SidereonStatus::InvalidArgument);
    }
    let combine = match options.combine {
        value if value == SidereonSp3MergeCombine::Mean as u32 => MergeCombine::Mean,
        value if value == SidereonSp3MergeCombine::Median as u32 => MergeCombine::Median,
        value if value == SidereonSp3MergeCombine::Precedence as u32 => MergeCombine::Precedence,
        _ => {
            set_last_error(format!("{fn_name}: invalid merge combine selector"));
            return Err(SidereonStatus::InvalidArgument);
        }
    };
    let target_epoch_interval_s = if options.target_epoch_interval_s_enabled {
        require_positive_finite(
            fn_name,
            "target_epoch_interval_s",
            options.target_epoch_interval_s,
        )?;
        Some(options.target_epoch_interval_s)
    } else {
        None
    };
    let systems = if options.system_count == 0 {
        None
    } else {
        let raw_systems = require_slice(options.systems, options.system_count, fn_name, "systems")?;
        let mut systems = BTreeSet::new();
        for (idx, system) in raw_systems.iter().copied().enumerate() {
            systems.insert(gnss_system_from_c_code(
                fn_name,
                &format!("systems[{idx}]"),
                system,
            )?);
        }
        Some(systems)
    };
    let asserted_equivalent_label_sets = if options.asserted_frame_label_set_count == 0 {
        Vec::new()
    } else {
        parse_sp3_asserted_frame_label_sets(
            fn_name,
            options.asserted_frame_label_sets,
            options.asserted_frame_label_set_count,
        )?
    };

    Ok(MergeOptions {
        position_tolerance_m: options.position_tolerance_m,
        clock_tolerance_s: options.clock_tolerance_s,
        min_agree: options.min_agree,
        clock_min_common: options.clock_min_common,
        combine,
        target_epoch_interval_s,
        systems,
        frame_reconciliation: Sp3FrameReconciliationOptions {
            asserted_equivalent_label_sets,
            helmert: options.helmert_frame_reconciliation,
        },
    })
}

unsafe fn parse_sp3_asserted_frame_label_sets(
    fn_name: &str,
    label_sets: *const SidereonSp3FrameLabelSet,
    label_set_count: usize,
) -> Result<Vec<Sp3FrameLabelSet>, SidereonStatus> {
    let raw_sets = require_slice(
        label_sets,
        label_set_count,
        fn_name,
        "asserted_frame_label_sets",
    )?;
    let mut parsed = Vec::with_capacity(raw_sets.len());
    for (set_idx, set) in raw_sets.iter().copied().enumerate() {
        if set.label_count < 2 {
            set_last_error(format!(
                "{fn_name}: asserted_frame_label_sets[{set_idx}] must contain at least two labels"
            ));
            return Err(SidereonStatus::InvalidArgument);
        }
        let raw_labels = require_slice(
            set.labels,
            set.label_count,
            fn_name,
            &format!("asserted_frame_label_sets[{set_idx}].labels"),
        )?;
        let mut labels = Vec::with_capacity(raw_labels.len());
        for (label_idx, label) in raw_labels.iter().copied().enumerate() {
            let label = parse_bounded_c_string(
                fn_name,
                &format!("asserted_frame_label_sets[{set_idx}].labels[{label_idx}]"),
                label,
                SP3_FRAME_LABEL_MAX_BYTES,
            )?;
            let label = label.trim().to_string();
            if label.is_empty() {
                set_last_error(format!(
                    "{fn_name}: asserted_frame_label_sets[{set_idx}].labels[{label_idx}] is empty"
                ));
                return Err(SidereonStatus::InvalidArgument);
            }
            labels.push(label);
        }
        parsed.push(Sp3FrameLabelSet::new(labels));
    }
    Ok(parsed)
}

fn sp3_merge_flag_to_c(flag: &MergeFlag) -> SidereonSp3MergeFlag {
    SidereonSp3MergeFlag {
        epoch_j2000_seconds: instant_to_j2000_seconds(&flag.epoch).unwrap_or(f64::NAN),
        sat_id: satellite_token(flag.satellite),
        source_count: flag.sources.len(),
    }
}

fn sp3_frame_reconciliation_to_c(
    value: &sidereon_core::ephemeris::Sp3FrameReconciliation,
) -> SidereonSp3FrameReconciliation {
    let parameters = value.parameters;
    let rates = value.rates;
    let epoch_year_span = value.epoch_year_span;
    SidereonSp3FrameReconciliation {
        source_index: value.source_index,
        source_label_len: value.source_label.len(),
        target_label_len: value.target_label.len(),
        method: match value.method {
            Sp3FrameReconciliationMethod::AssertedEquivalence => {
                SidereonSp3FrameReconciliationMethod::AssertedEquivalence
            }
            Sp3FrameReconciliationMethod::Helmert => SidereonSp3FrameReconciliationMethod::Helmert,
        },
        asserted_label_count: value.asserted_label_set.as_ref().map(Vec::len).unwrap_or(0),
        source_frame_present: value.source_frame.is_some(),
        source_frame: value
            .source_frame
            .map(sp3_terrestrial_frame_to_c)
            .unwrap_or(0),
        target_frame_present: value.target_frame.is_some(),
        target_frame: value
            .target_frame
            .map(sp3_terrestrial_frame_to_c)
            .unwrap_or(0),
        catalog_frame_present: value.catalog_source_frame.is_some()
            && value.catalog_target_frame.is_some(),
        catalog_source_frame: value
            .catalog_source_frame
            .map(sp3_terrestrial_frame_to_c)
            .unwrap_or(0),
        catalog_target_frame: value
            .catalog_target_frame
            .map(sp3_terrestrial_frame_to_c)
            .unwrap_or(0),
        catalog_inverse: value.catalog_inverse,
        reference_epoch_year_present: value.reference_epoch_year.is_some(),
        reference_epoch_year: value.reference_epoch_year.unwrap_or(0.0),
        parameters_present: parameters.is_some(),
        translation_mm: parameters
            .map(|parameters| parameters.translation_mm)
            .unwrap_or([0.0; 3]),
        scale_ppb: parameters
            .map(|parameters| parameters.scale_ppb)
            .unwrap_or(0.0),
        rotation_mas: parameters
            .map(|parameters| parameters.rotation_mas)
            .unwrap_or([0.0; 3]),
        rates_present: rates.is_some(),
        translation_mm_per_year: rates
            .map(|rates| rates.translation_mm_per_year)
            .unwrap_or([0.0; 3]),
        scale_ppb_per_year: rates.map(|rates| rates.scale_ppb_per_year).unwrap_or(0.0),
        rotation_mas_per_year: rates
            .map(|rates| rates.rotation_mas_per_year)
            .unwrap_or([0.0; 3]),
        provenance_len: value.provenance.as_ref().map(String::len).unwrap_or(0),
        epoch_year_span_present: epoch_year_span.is_some(),
        epoch_year_start: epoch_year_span.map(|span| span[0]).unwrap_or(0.0),
        epoch_year_end: epoch_year_span.map(|span| span[1]).unwrap_or(0.0),
        records_affected: value.records_affected,
        identity: value.identity,
    }
}

fn zero_sp3_frame_reconciliation() -> SidereonSp3FrameReconciliation {
    SidereonSp3FrameReconciliation {
        source_index: 0,
        source_label_len: 0,
        target_label_len: 0,
        method: SidereonSp3FrameReconciliationMethod::AssertedEquivalence,
        asserted_label_count: 0,
        source_frame_present: false,
        source_frame: 0,
        target_frame_present: false,
        target_frame: 0,
        catalog_frame_present: false,
        catalog_source_frame: 0,
        catalog_target_frame: 0,
        catalog_inverse: false,
        reference_epoch_year_present: false,
        reference_epoch_year: 0.0,
        parameters_present: false,
        translation_mm: [0.0; 3],
        scale_ppb: 0.0,
        rotation_mas: [0.0; 3],
        rates_present: false,
        translation_mm_per_year: [0.0; 3],
        scale_ppb_per_year: 0.0,
        rotation_mas_per_year: [0.0; 3],
        provenance_len: 0,
        epoch_year_span_present: false,
        epoch_year_start: 0.0,
        epoch_year_end: 0.0,
        records_affected: 0,
        identity: false,
    }
}

fn sp3_terrestrial_frame_to_c(value: sidereon_core::frame_catalog::TerrestrialFrame) -> u32 {
    match value {
        sidereon_core::frame_catalog::TerrestrialFrame::Itrf2020 => {
            SidereonTerrestrialFrame::Itrf2020 as u32
        }
        sidereon_core::frame_catalog::TerrestrialFrame::Itrf2014 => {
            SidereonTerrestrialFrame::Itrf2014 as u32
        }
        sidereon_core::frame_catalog::TerrestrialFrame::Itrf2008 => {
            SidereonTerrestrialFrame::Itrf2008 as u32
        }
        sidereon_core::frame_catalog::TerrestrialFrame::Etrf2020 => {
            SidereonTerrestrialFrame::Etrf2020 as u32
        }
    }
}

unsafe fn sp3_merge_report_reconciliation_bytes<'a, F>(
    fn_name: &str,
    report: *const SidereonSp3MergeReport,
    index: usize,
    select: F,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus
where
    F: FnOnce(&'a sidereon_core::ephemeris::Sp3FrameReconciliation) -> &'a [u8],
{
    ffi_boundary(fn_name, SidereonStatus::Panic, || {
        c_try!(init_copy_counts(fn_name, out_written, out_required));
        let report = c_try!(require_ref(report, fn_name, "report"));
        let Some(reconciliation) = report.inner.frame_reconciliations.get(index) else {
            set_last_error(format!("{fn_name}: index {index} out of range"));
            return SidereonStatus::InvalidArgument;
        };
        c_try!(copy_prefix_to_c(
            fn_name,
            "out",
            select(reconciliation),
            out,
            len,
            out_written,
            out_required,
        ));
        SidereonStatus::Ok
    })
}

fn sp3_epoch_agreement_to_c(agg: &EpochAgreement) -> SidereonSp3EpochAgreement {
    SidereonSp3EpochAgreement {
        epoch_j2000_seconds: instant_to_j2000_seconds(&agg.epoch).unwrap_or(f64::NAN),
        satellites: agg.satellites,
        position_rms_m: agg.position_rms_m,
        position_max_m: agg.position_max_m,
        clock_rms_present: agg.clock_rms_s.is_some(),
        clock_rms_s: agg.clock_rms_s.unwrap_or(0.0),
        clock_max_present: agg.clock_max_s.is_some(),
        clock_max_s: agg.clock_max_s.unwrap_or(0.0),
    }
}

unsafe fn visibility_options_from_c(
    fn_name: &str,
    elevation_mask_deg: f64,
    systems: *const u32,
    systems_len: usize,
) -> Result<VisibilityOptions, SidereonStatus> {
    let raw = require_slice(systems, systems_len, fn_name, "systems")?;
    let systems = if raw.is_empty() {
        None
    } else {
        let mut set = BTreeSet::new();
        for code in raw {
            set.insert(gnss_system_from_c_code(fn_name, "systems", *code)?);
        }
        Some(set)
    };
    Ok(VisibilityOptions {
        elevation_mask_deg,
        systems,
    })
}

fn geometry_visible_to_c(sat: &GeometryVisibleSatellite) -> SidereonGeometryVisible {
    SidereonGeometryVisible {
        satellite: satellite_token(sat.satellite),
        elevation_deg: sat.elevation_deg,
        azimuth_deg: sat.azimuth_deg,
    }
}

fn visibility_series_point_to_c(point: &VisibilitySeriesPoint) -> SidereonVisibilitySeriesPoint {
    SidereonVisibilitySeriesPoint {
        step_index: point.step_index,
        n_visible: point.n_visible,
    }
}

fn visibility_pass_to_c(pass: &VisibilityPass) -> SidereonVisibilityPass {
    SidereonVisibilityPass {
        satellite: satellite_token(pass.satellite),
        rise_step_index: pass.rise_step_index,
        set_step_index: pass.set_step_index,
        peak_elevation_deg: pass.peak_elevation_deg,
        peak_step_index: pass.peak_step_index,
    }
}

fn precise_sample_to_c(sample: &PreciseEphemerisSample) -> SidereonPreciseEphemerisSample {
    SidereonPreciseEphemerisSample {
        sat: satellite_token(sample.sat),
        time_scale: time_scale_to_c_code(sample.epoch.scale),
        epoch_j2000_s: instant_to_j2000_seconds(&sample.epoch).unwrap_or(f64::NAN),
        position_ecef_m: sample.position_ecef_m,
        has_clock_s: sample.clock_s.is_some(),
        clock_s: sample.clock_s.unwrap_or(0.0),
        clock_event: sample.clock_event,
    }
}

fn require_positive_finite(
    fn_name: &str,
    arg_name: &str,
    value: f64,
) -> Result<(), SidereonStatus> {
    if value.is_finite() && value > 0.0 {
        Ok(())
    } else {
        set_last_error(format!("{fn_name}: {arg_name} must be positive and finite"));
        Err(SidereonStatus::InvalidArgument)
    }
}
