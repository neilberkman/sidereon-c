//! sp3_interpolation_smoke.c: sidereon-core's gap threshold factors,
//! refusals, continuity defect counts, verdict JSON, artifact header and
//! observable-state element statuses on the gapped G01 product.

#[path = "../w6_shared.rs"]
mod shared;

use shared::*;
use sidereon_core::ephemeris::{
    check_continuity, ContinuityOptions, EpochWindow, MmapPreciseEphemerisInterpolant,
    PreciseEphemerisInterpolant, PreciseEphemerisSamples, Sp3, Sp3InterpolationOptions,
    StencilExtent,
};
use sidereon_core::observables::{ObservableEphemerisSource, ObservableStateElementStatus};
use sidereon_core::GnssSatelliteId;
use valgen::{read_bytes, tests_path};

const HOLE_MIDPOINT_J2000_S: f64 = 646260300.0;

/// bindings/c/src/sp3.rs interpolation_options_from_c: zero keeps the default.
fn options(factor: f64) -> Result<Sp3InterpolationOptions, sidereon_core::Error> {
    if !factor.is_nan() && factor <= 0.0 {
        Ok(Sp3InterpolationOptions::default())
    } else {
        Sp3InterpolationOptions::new(factor)
    }
}

fn element_status(status: Option<ObservableStateElementStatus>) -> String {
    let status = status.unwrap_or(ObservableStateElementStatus::Error);
    c_enum("SIDEREON_OBSERVABLE_STATE_ELEMENT_STATUS", &status)
}

fn main() {
    let guard = "SIDEREON_W6_SP3_INTERPOLATION_PINS_H";
    valgen::header_start("w6_sp3_interpolation", guard);
    let bytes = read_bytes(&tests_path("fixtures/sp3/GAP_G01_20201760000_15M.sp3"));
    let g01: GnssSatelliteId = "G01".parse().expect("G01");

    let refusal = options(1.0).expect_err("factor 1.0 is refused");
    comment("Sp3InterpolationOptions::new(1.0) is refused; the status the binding maps it to.");
    def("W6_SP3I_FACTOR_ONE_STATUS", sp3_argument_status(&refusal));

    let default_options = options(0.0).expect("default options");
    let sp3 = Sp3::parse(&bytes)
        .expect("gapped SP3")
        .with_interpolation_options(default_options);
    comment("The default gap threshold factor.");
    def_bits(
        "W6_SP3I_DEFAULT_FACTOR_BITS",
        sp3.interpolation_options().gap_threshold_factor(),
    );
    let hole = sp3.position_at_j2000_seconds(g01, HOLE_MIDPOINT_J2000_S);
    comment("G01 at the hole midpoint under the default factor: the status (OK when served).");
    def(
        "W6_SP3I_DEFAULT_HOLE_STATUS",
        match &hole {
            Ok(_) => "SIDEREON_STATUS_OK",
            Err(err) => sp3_interpolation_status(err),
        },
    );

    // sidereon_sp3_check_continuity_with_gap_threshold_factor(sp3, -1, 1.0, f).
    let defects = |factor: f64| {
        let options = ContinuityOptions::new(None, Some(1.0))
            .expect("valid continuity options")
            .with_interpolation_options(options(factor).expect("options"));
        check_continuity(&sp3.precise_ephemeris_samples(), &options)
            .expect("valid continuity options")
            .defects
            .len()
    };
    comment("Continuity defect counts with no speed bound, 1 m residual tolerance.");
    def_size("W6_SP3I_DEFAULT_DEFECTS", defects(0.0));
    def_size("W6_SP3I_WIDE_DEFECTS", defects(13.0));

    let verdict_options = ContinuityOptions::new(None, Some(1.0))
        .expect("valid continuity options")
        .with_interpolation_options(options(13.0).expect("options"));
    let report = check_continuity(&sp3.precise_ephemeris_samples(), &verdict_options)
        .expect("valid continuity options");
    let window = EpochWindow::new(HOLE_MIDPOINT_J2000_S - 100.0, HOLE_MIDPOINT_J2000_S + 100.0)
        .expect("window");
    let stencil = StencilExtent::for_sp3(&sp3).expect("stencil");
    let verdict = window_continuity_verdict_json(report.verdict_for_window(window, stencil));
    comment("Continuity verdict JSON with factor 13 over the hole midpoint +/- 100 s.");
    def_str(
        "W6_SP3I_WIDE_VERDICT_JSON",
        &serde_json::to_string(&verdict).expect("serialize verdict"),
    );

    let samples = sp3.precise_ephemeris_samples();
    comment("Canonical precise-ephemeris sample count.");
    def_size("W6_SP3I_SAMPLE_COUNT", samples.len());
    // The test rebuilds its sources from the samples it copied out.
    let samples = samples_through_c(&samples);

    let wide = Sp3::parse(&bytes)
        .expect("gapped SP3")
        .with_interpolation_options(options(13.0).expect("options"));
    comment("Factor 13 on the product.");
    def_bits(
        "W6_SP3I_WIDE_FACTOR_BITS",
        wide.interpolation_options().gap_threshold_factor(),
    );
    let served = wide.position_at_j2000_seconds(g01, HOLE_MIDPOINT_J2000_S);
    def(
        "W6_SP3I_WIDE_HOLE_STATUS",
        match &served {
            Ok(_) => "SIDEREON_STATUS_OK",
            Err(err) => sp3_interpolation_status(err),
        },
    );
    if let Ok(state) = &served {
        def_bits_array(
            "W6_SP3I_WIDE_HOLE_POSITION_BITS",
            &state.position.as_array(),
        );
    }

    let artifact = wide
        .precise_interpolant_store_bytes()
        .expect("artifact bytes");
    comment("The precise-interpolant store artifact of the factor-13 product.");
    def_size("W6_SP3I_ARTIFACT_LEN", artifact.len());
    let opened = MmapPreciseEphemerisInterpolant::from_vec(artifact).expect("artifact opens");
    def_bits(
        "W6_SP3I_ARTIFACT_FACTOR_BITS",
        opened.interpolation_options().gap_threshold_factor(),
    );

    comment("Samples and interpolant rebuilt from the samples: factor and G01 hole status.");
    for (label, factor) in [("DEFAULT", 0.0), ("WIDE", 13.0)] {
        let source = PreciseEphemerisSamples::from_samples(samples.clone())
            .expect("samples source")
            .with_interpolation_options(options(factor).expect("options"));
        def_bits(
            &format!("W6_SP3I_SAMPLES_{label}_FACTOR_BITS"),
            source.interpolation_options().gap_threshold_factor(),
        );
        let batch = source.observable_states_at_shared_j2000_s(&[g01], HOLE_MIDPOINT_J2000_S);
        def(
            &format!("W6_SP3I_SAMPLES_{label}_ELEMENT_STATUS"),
            element_status(batch.element_status(0)),
        );
        def_bits_array(
            &format!("W6_SP3I_SAMPLES_{label}_POSITION_BITS"),
            &batch.positions_ecef_m[0],
        );

        let interpolant = PreciseEphemerisInterpolant::from_samples(samples.clone())
            .expect("interpolant")
            .with_interpolation_options(options(factor).expect("options"));
        def_bits(
            &format!("W6_SP3I_INTERPOLANT_{label}_FACTOR_BITS"),
            interpolant.interpolation_options().gap_threshold_factor(),
        );
        let batch = interpolant.observable_states_at_shared_j2000_s(&[g01], HOLE_MIDPOINT_J2000_S);
        def(
            &format!("W6_SP3I_INTERPOLANT_{label}_ELEMENT_STATUS"),
            element_status(batch.element_status(0)),
        );
        def_bits_array(
            &format!("W6_SP3I_INTERPOLANT_{label}_POSITION_BITS"),
            &batch.positions_ecef_m[0],
        );
    }
    valgen::header_end(guard);
}
