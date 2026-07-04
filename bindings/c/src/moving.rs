use super::*;

// --- Moving-baseline RTK (sidereon_core::rtk_filter::moving_baseline) ---------

/// Integer ambiguity verdict for a moving-baseline epoch, mirroring
/// sidereon_core::rtk_filter::moving_baseline::MovingBaselineStatus.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonMovingBaselineStatus {
    /// LAMBDA fixed the integers; the reported baseline is the fixed solution.
    Fixed = 0,
    /// The integers were not fixed; the reported baseline is the float solution.
    Float = 1,
}

/// One moving-baseline epoch: the base receiver's own ECEF position this epoch,
/// a single double-difference observation epoch, and the ambiguity set to
/// resolve. The ambiguity inputs mirror the static RTK fixed-solve config.
#[repr(C)]
pub struct SidereonMovingBaselineEpoch {
    /// Base receiver ECEF position (metres) at this epoch.
    pub base_position_m: [f64; 3],
    /// The double-difference observation epoch (one epoch's measurements).
    pub epoch: SidereonRtkEpoch,
    /// Pointer to ambiguity_id_count null-terminated ambiguity id strings.
    pub ambiguity_ids: *const *const c_char,
    /// Number of ambiguity id strings.
    pub ambiguity_id_count: usize,
    /// Pointer to ambiguity_satellite_count ambiguity-to-satellite map entries.
    pub ambiguity_satellites: *const SidereonRtkAmbiguitySatellite,
    /// Number of ambiguity-to-satellite map entries.
    pub ambiguity_satellite_count: usize,
    /// Pointer to wavelength_count ambiguity wavelength entries.
    pub wavelengths_m: *const SidereonRtkFloatMapEntry,
    /// Number of wavelength entries.
    pub wavelength_count: usize,
    /// Pointer to offset_count ambiguity offset entries.
    pub offsets_m: *const SidereonRtkFloatMapEntry,
    /// Number of offset entries.
    pub offset_count: usize,
    /// Optional array of SidereonGnssSystem_* values encoded as uint32_t.
    pub float_only_systems: *const u32,
    /// Number of float-only system entries.
    pub float_only_system_count: usize,
}

/// Complete typed input bundle for a moving-baseline RTK solve. Initialize model
/// and option structs with their existing RTK init functions before overriding.
#[repr(C)]
pub struct SidereonMovingBaselineConfig {
    /// Pointer to epoch_count moving-baseline epochs.
    pub epochs: *const SidereonMovingBaselineEpoch,
    /// Number of epochs.
    pub epoch_count: usize,
    /// RTK measurement model.
    pub model: SidereonRtkMeasurementModel,
    /// Float solve options used before integer fixing.
    pub float_options: SidereonRtkFloatOptions,
    /// Fixed solve options.
    pub fixed_options: SidereonRtkFixedOptions,
    /// Initial baseline guess (metres) for the first epoch's linearization.
    pub initial_baseline_m: [f64; 3],
    /// Carry each epoch's solved baseline forward as the next epoch's
    /// linearization point (RTKLIB-style continuity).
    pub warm_start: bool,
    /// Optional receiver-antenna corrections for base/rover; NULL disables them.
    pub receiver_antenna: *const SidereonRtkReceiverAntennaCorrections,
}

/// One solved moving-baseline epoch summary.
#[repr(C)]
pub struct SidereonMovingBaselineEpochSummary {
    /// Base receiver ECEF position (metres) used for this epoch.
    pub base_position_m: [f64; 3],
    /// Baseline vector rover - base (metres), ECEF. The fixed baseline when
    /// status is SIDEREON_MOVING_BASELINE_STATUS_FIXED, otherwise the float one.
    pub baseline_m: [f64; 3],
    /// Euclidean baseline length (metres).
    pub baseline_length_m: f64,
    /// Integer-fix verdict, a SidereonMovingBaselineStatus value.
    pub status: SidereonMovingBaselineStatus,
    /// Float baseline solve metadata.
    pub float: SidereonRtkFloatMetadata,
    /// Integer-fixed baseline solve metadata.
    pub fixed: SidereonRtkFixedMetadata,
}

/// A solved moving-baseline arc. Opaque to C. Create with
/// sidereon_solve_moving_baseline; release with
/// sidereon_moving_baseline_solution_free.
pub struct SidereonMovingBaselineSolution {
    pub(crate) epochs: Vec<MovingBaselineEpochSolution>,
}

/// Number of solved epochs in a moving-baseline solution.
///
/// Safety: solution is a live handle; out_count points to a size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_moving_baseline_solution_epoch_count(
    solution: *const SidereonMovingBaselineSolution,
    out_count: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_moving_baseline_solution_epoch_count",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out_count,
                "sidereon_moving_baseline_solution_epoch_count",
                "out_count"
            ));
            *out = 0;
            let solution = c_try!(require_ref(
                solution,
                "sidereon_moving_baseline_solution_epoch_count",
                "solution"
            ));
            *out = solution.epochs.len();
            SidereonStatus::Ok
        },
    )
}

/// Copy the summary of one solved moving-baseline epoch into *out.
///
/// Safety: solution is a live handle; out points to a
/// SidereonMovingBaselineEpochSummary.
#[no_mangle]
pub unsafe extern "C" fn sidereon_moving_baseline_solution_epoch(
    solution: *const SidereonMovingBaselineSolution,
    index: usize,
    out: *mut SidereonMovingBaselineEpochSummary,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_moving_baseline_solution_epoch",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out,
                "sidereon_moving_baseline_solution_epoch",
                "out"
            ));
            let solution = c_try!(require_ref(
                solution,
                "sidereon_moving_baseline_solution_epoch",
                "solution"
            ));
            let epoch = match solution.epochs.get(index) {
                Some(epoch) => epoch,
                None => {
                    set_last_error(format!(
                        "sidereon_moving_baseline_solution_epoch: index {index} out of range ({} epochs)",
                        solution.epochs.len()
                    ));
                    return SidereonStatus::InvalidArgument;
                }
            };
            *out = moving_baseline_summary(epoch);
            SidereonStatus::Ok
        },
    )
}

/// Release a moving-baseline solution handle. Passing NULL is a no-op.
///
/// Safety: solution must be a handle from sidereon_solve_moving_baseline or NULL.
#[no_mangle]
pub unsafe extern "C" fn sidereon_moving_baseline_solution_free(
    solution: *mut SidereonMovingBaselineSolution,
) {
    free_boxed(solution);
}

fn moving_baseline_summary(
    epoch: &MovingBaselineEpochSolution,
) -> SidereonMovingBaselineEpochSummary {
    SidereonMovingBaselineEpochSummary {
        base_position_m: epoch.base_position_m,
        baseline_m: epoch.baseline_m,
        baseline_length_m: epoch.baseline_length_m,
        status: match epoch.status {
            MovingBaselineStatus::Fixed => SidereonMovingBaselineStatus::Fixed,
            MovingBaselineStatus::Float => SidereonMovingBaselineStatus::Float,
        },
        float: rtk_float_metadata(&epoch.float),
        fixed: rtk_fixed_metadata_from_solution(&epoch.fixed, &epoch.float.geometry_quality),
    }
}
