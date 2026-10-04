//! smoke.c exercise_staleness_surface: sidereon-core's product selection for
//! each epoch and policy the test asks about, with the staleness metadata it
//! reports.

#[path = "../smoke_b_shared.rs"]
mod shared;

use shared::*;
use sidereon_core::astro::time::{
    split_julian_date_from_j2000_seconds, Instant, JulianDateSplit, TimeScale,
};
use sidereon_core::atmosphere::ionosphere::Ionex;
use sidereon_core::ephemeris::Sp3;
use sidereon_core::staleness::{
    select_ionex_over_range, select_sp3_over_range, SelectionError, StalenessMetadata,
    StalenessPolicy,
};
use valgen::{bits, header_end, header_start, read, read_bytes, tests_path};

const BIN: &str = "smoke_b_staleness";
const GUARD: &str = "SIDEREON_SMOKE_B_STALENESS_PINS_H";
const P: &str = "SMOKE_B_STALENESS";

/// src/lib.rs selection_error_to_status.
fn selection_status_c(err: &SelectionError) -> String {
    c_enum("SIDEREON_SELECTION_STATUS_", err)
}

fn pin(result: Result<StalenessMetadata, SelectionError>) -> String {
    match result {
        Ok(m) => format!(
            "{{ SIDEREON_SELECTION_STATUS_OK, {}, {}, {}, {}, {} }}",
            c_enum("SIDEREON_DEGRADATION_KIND_", &m.kind),
            bits(m.requested_epoch_j2000_s),
            bits(m.source_epoch_j2000_s),
            bits(m.staleness_s),
            bits(m.staleness_days)
        ),
        Err(err) => format!(
            "{{ {}, SIDEREON_DEGRADATION_KIND_EXACT, UINT64_C(0), UINT64_C(0), UINT64_C(0), \
             UINT64_C(0) }}",
            selection_status_c(&err)
        ),
    }
}

fn main() {
    let sp3 = Sp3::parse(&read_bytes(&tests_path(
        "fixtures/sp3/GRG0MGXFIN_20201760000_01D_15M_ORB.SP3",
    )))
    .expect("SP3 fixture");
    let epochs = sp3.epochs_j2000_seconds();
    let covered = epochs[1];
    let last = *epochs.last().expect("last SP3 epoch");
    let default = StalenessPolicy::default();
    let set = [sp3];
    let select = |epoch: f64, policy: StalenessPolicy| {
        select_sp3_over_range(&set, epoch, epoch, policy).map(|s| s.metadata())
    };

    header_start(BIN, GUARD);
    println!("typedef struct {{");
    println!("    SidereonSelectionStatus status;");
    println!("    SidereonDegradationKind kind;");
    println!("    uint64_t requested_bits;");
    println!("    uint64_t source_bits;");
    println!("    uint64_t staleness_s_bits;");
    println!("    uint64_t staleness_days_bits;");
    println!("}} SmokeBStalenessPin;");
    println!();
    define_bits(&format!("{P}_DEFAULT_CAP_S_BITS"), default.max_staleness_s);
    define_bits(&format!("{P}_COVERED_EPOCH_BITS"), covered);
    define_bits(&format!("{P}_LAST_EPOCH_BITS"), last);
    let stale = last + 100.0;
    let rows = [
        ("SP3_EXACT", select(covered, default)),
        ("SP3_PRIOR", select(stale, default)),
        ("SP3_CAP", select(stale, StalenessPolicy::seconds(1.0))),
        (
            "SP3_EMPTY",
            select_sp3_over_range(&[], covered, covered, default).map(|s| s.metadata()),
        ),
        ("SP3_NO_PRIOR", select(covered - 7.0 * 86400.0, default)),
    ];
    for (name, result) in rows {
        println!(
            "static const SmokeBStalenessPin {P}_{name} = {};",
            pin(result)
        );
    }

    let ionex = Ionex::parse(&read_bytes(&tests_path(
        "fixtures/ionex/synthetic_2map_7x7.20i",
    )))
    .expect("IONEX fixture");
    // The first golden IONEX case's epoch, inside the product's coverage.
    let golden: serde_json::Value = serde_json::from_str(&read(&format!(
        "{}/ionex_golden.json",
        valgen::core_fixtures()
    )))
    .expect("ionex golden JSON");
    let covered_ionex = golden["cases"][0]["inputs"]["epoch_s"]
        .as_i64()
        .expect("epoch_s");
    let ionex_set = [ionex];
    let select_ionex = |epoch: i64, policy: StalenessPolicy| {
        let (jd_whole, fraction) = split_julian_date_from_j2000_seconds(epoch);
        let split = JulianDateSplit::new(jd_whole, fraction).expect("IONEX fixture epoch");
        let instant = Instant::from_julian_date(TimeScale::Utc, split);
        select_ionex_over_range(&ionex_set, instant, instant, policy).map(|s| s.metadata())
    };
    let rows = [
        ("IONEX_EXACT", select_ionex(covered_ionex, default)),
        ("IONEX_SHIFT", select_ionex(covered_ionex + 86400, default)),
        (
            "IONEX_NAN_POLICY",
            select_ionex(
                covered_ionex,
                StalenessPolicy {
                    max_staleness_s: f64::NAN,
                },
            ),
        ),
    ];
    for (name, result) in rows {
        println!(
            "static const SmokeBStalenessPin {P}_{name} = {};",
            pin(result)
        );
    }
    header_end(GUARD);
}
