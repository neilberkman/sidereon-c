use super::*;

// ===========================================================================

/// One satellite visible above the elevation mask at one epoch, from
/// sidereon_sp3_geometry_visible.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonGeometryVisible {
    /// Satellite identifier (RINEX token, for example G08).
    pub satellite: SidereonSatelliteToken,
    /// Topocentric elevation, degrees.
    pub elevation_deg: f64,
    /// Topocentric azimuth, degrees in [0, 360).
    pub azimuth_deg: f64,
}

/// Visible-satellite count for one sampled epoch, from
/// sidereon_sp3_geometry_visibility_series.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonVisibilitySeriesPoint {
    /// Zero-based sample index from the window start.
    pub step_index: usize,
    /// Number of satellites visible at this sample.
    pub n_visible: usize,
}

/// One sampled visibility pass, from sidereon_sp3_geometry_passes.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonVisibilityPass {
    /// Satellite identifier (RINEX token, for example G08).
    pub satellite: SidereonSatelliteToken,
    /// Zero-based sample index of the first above-mask sample.
    pub rise_step_index: usize,
    /// Zero-based sample index of the last above-mask sample.
    pub set_step_index: usize,
    /// Maximum sampled elevation in the pass, degrees.
    pub peak_elevation_deg: f64,
    /// Zero-based sample index of the maximum sampled elevation.
    pub peak_step_index: usize,
}

/// Copy the stable lowercase label for a SidereonObservabilityTier enum value
/// into out. Mirrors Python's ObservabilityTier.label and wasm's
/// observabilityTierLabel in the C naming convention.
///
/// Safety: out points to len writable bytes or is NULL when len is 0;
/// out_written and out_required point to size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_observability_tier_label(
    tier: u32,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_observability_tier_label",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_observability_tier_label",
                out_written,
                out_required
            ));
            let label = match tier {
                value if value == SidereonObservabilityTier::RankDeficient as u32 => {
                    "rank_deficient"
                }
                value if value == SidereonObservabilityTier::ZeroRedundancy as u32 => {
                    "zero_redundancy"
                }
                value if value == SidereonObservabilityTier::Weak as u32 => "weak",
                value if value == SidereonObservabilityTier::Nominal as u32 => "nominal",
                _ => {
                    set_last_error(format!(
                        "sidereon_observability_tier_label: invalid tier {tier}"
                    ));
                    return SidereonStatus::InvalidArgument;
                }
            };
            c_try!(copy_prefix_to_c(
                "sidereon_observability_tier_label",
                "out",
                label.as_bytes(),
                out,
                len,
                out_written,
                out_required,
            ));
            SidereonStatus::Ok
        },
    )
}
