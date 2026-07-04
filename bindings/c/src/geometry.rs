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
