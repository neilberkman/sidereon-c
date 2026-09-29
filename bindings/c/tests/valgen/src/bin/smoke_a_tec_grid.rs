// smoke.c exercise_tec_grid_latitude_clamp and
// exercise_tec_grid_nonfinite_latitude: sidereon-core's pierce-point
// evaluations of the smoke's two-epoch grids, as
// sidereon_tec_grid_vtec_at_pierce_point and its typed route make them
// (TecGrid::new, then vtec_at_pierce_point_with_policy at unix nanosecond 0,
// day 0, under the strict missing-node policy).

#[path = "smoke_a_support/mod.rs"]
mod support;

use sidereon_core::atmosphere::ionosphere::{
    IonexMissingNodePolicy, TecGrid, TecGridEpoch, TecGridError,
};
use support::*;
use valgen::{header_end, header_start};

const GUARD: &str = "SIDEREON_SMOKE_A_TEC_GRID_PINS_H";

/// The smoke's grid: epochs 0 and 1000 ns, the given latitudes, longitudes 20
/// and 60, every node present with the smoke's flat values.
fn grid(latitudes_deg: [f64; 2]) -> TecGrid {
    let values = [1.0, 3.0, 7.0, 15.0, 2.0, 6.0, 14.0, 30.0];
    TecGrid::new(
        vec![0.0, 1000.0],
        latitudes_deg.to_vec(),
        vec![20.0, 60.0],
        values.iter().map(|v| Some(*v)).collect(),
    )
    .expect("TEC grid")
}

/// One query's outcome: the value, or the engine error with the fields the
/// smoke reads.
fn query_pins(prefix: &str, grid: &TecGrid, lat_deg: f64) {
    let result = grid.vtec_at_pierce_point_with_policy(
        TecGridEpoch::new(0, 0),
        20.0,
        lat_deg,
        IonexMissingNodePolicy::Strict,
    );
    match result {
        Ok(evaluation) => {
            def_bool(&format!("{prefix}_OK"), true);
            def_bits(&format!("{prefix}_VTEC_BITS"), evaluation.value);
            def_bool(&format!("{prefix}_HAS_GAP"), evaluation.degraded.is_some());
            def_str(&format!("{prefix}_ERROR"), "");
            def_str(&format!("{prefix}_AXIS"), "");
            def_bits(&format!("{prefix}_AXIS_VALUE_BITS"), 0.0);
            def_str(&format!("{prefix}_FIELD"), "");
            def_str(&format!("{prefix}_REASON"), "");
        }
        Err(err) => {
            def_bool(&format!("{prefix}_OK"), false);
            def_bits(&format!("{prefix}_VTEC_BITS"), f64::NAN);
            def_bool(&format!("{prefix}_HAS_GAP"), false);
            def_str(&format!("{prefix}_ERROR"), &variant_name(&err));
            let (axis, axis_value) = match &err {
                TecGridError::OutOfBounds { name, value } => (name.to_string(), *value),
                _ => (String::new(), 0.0),
            };
            def_str(&format!("{prefix}_AXIS"), &axis);
            def_bits(&format!("{prefix}_AXIS_VALUE_BITS"), axis_value);
            let (field, reason) = match &err {
                TecGridError::InvalidField { field, reason } => {
                    (field.to_string(), reason.to_string())
                }
                _ => (String::new(), String::new()),
            };
            def_str(&format!("{prefix}_FIELD"), &field);
            def_str(&format!("{prefix}_REASON"), &reason);
        }
    }
}

fn main() {
    let wide = grid([80.0, 89.0]);
    let narrow = grid([80.0, 85.0]);
    header_start("smoke_a_tec_grid", GUARD);
    comment("Latitude axis [80, 89]: queries at 89, 87.5, +inf, -inf and NaN degrees.");
    query_pins("SMOKE_A_TEC_GRID_LAT_89", &wide, 89.0);
    query_pins("SMOKE_A_TEC_GRID_LAT_87_5", &wide, 87.5);
    query_pins("SMOKE_A_TEC_GRID_LAT_PLUS_INF", &wide, f64::INFINITY);
    query_pins("SMOKE_A_TEC_GRID_LAT_MINUS_INF", &wide, f64::NEG_INFINITY);
    query_pins("SMOKE_A_TEC_GRID_LAT_NAN", &wide, f64::NAN);
    comment("Latitude axis [80, 85]: a query at 89 degrees.");
    query_pins("SMOKE_A_TEC_GRID_NARROW_LAT_89", &narrow, 89.0);
    header_end(GUARD);
}
