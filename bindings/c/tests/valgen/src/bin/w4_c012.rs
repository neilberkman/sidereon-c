//! Engine values core012_smoke.c checks, written as tests/w4_c012_pins.h.
//! Each section builds the inputs the C test passes and calls the
//! sidereon-core function the C route calls (src/dted.rs, src/mmap.rs,
//! src/geoid.rs, src/sbas.rs, src/araim.rs, src/almanac.rs).

use sidereon_core::araim::{
    araim, AraimGeometry, AraimRow, ConstellationIsm, IntegrityAllocation, Ism, SatelliteIsm,
    SatelliteIsmModel,
};
use sidereon_core::positioning::LineOfSight;
use sidereon_core::terrain::{DtedLookupOptions, DtedTerrain};
use sidereon_core::terrain_store::{
    dted_tree_to_mmap_store, Egm96FifteenMinuteGeoid, MmapTerrain, TerrainDatumError,
    TerrainGeoidModel,
};
use sidereon_core::GnssSystem;
use valgen::{bits, c_string, core_fixtures, header_end, header_start};

const P: &str = "W4_C012_";
const GUARD: &str = "SIDEREON_W4_C012_PINS_H";

fn f(name: &str, value: f64) {
    println!("static const uint64_t {P}{name} = {};", bits(value));
}

fn int(name: &str, value: impl std::fmt::Display) {
    println!("#define {P}{name} ({value})");
}

fn flag(name: &str, value: bool) {
    println!("#define {P}{name} {value}");
}

/// The C enum constant for a core variant: `prefix` plus the variant's Debug
/// name in upper snake case.
fn variant(prefix: &str, value: impl std::fmt::Debug) -> String {
    let debug = format!("{value:?}");
    let name = debug
        .split(|c: char| c == '(' || c == '{' || c == ' ')
        .next()
        .expect("variant name");
    let mut out = String::from(prefix);
    let mut previous: Option<char> = None;
    for c in name.chars() {
        if c.is_ascii_uppercase()
            && previous.is_some_and(|p| p.is_ascii_lowercase() || p.is_ascii_digit())
        {
            out.push('_');
        }
        out.push(c.to_ascii_uppercase());
        previous = Some(c);
    }
    out
}

fn terrain(root: &str) {
    println!("/* test_terrain_batch and test_mmap_terrain_store: sidereon_core::terrain and");
    println!(" * sidereon_core::terrain_store over the core fixture dted/tiles. */");
    let points = [(-106.5, 36.5), (-106.25, 36.75), (-106.5, f64::NAN)];
    let options = DtedLookupOptions::default();
    let mut dted = DtedTerrain::new(root);
    let results = dted.height_batch(&points, options);
    for (i, result) in results.iter().enumerate() {
        flag(&format!("DTED_BATCH{i}_OK"), result.is_ok());
        match result {
            Ok(height) => f(&format!("DTED_BATCH{i}_HEIGHT_BITS"), *height),
            Err(err) => println!(
                "#define {P}DTED_BATCH{i}_ERROR_KIND {}",
                variant("SIDEREON_TERRAIN_LOOKUP_ERROR_KIND_", err)
            ),
        }
    }

    let store = dted_tree_to_mmap_store(root).expect("terrain store");
    let mmap = MmapTerrain::from_vec(store).expect("mmap terrain");
    let index = mmap.tile_index();
    int("TILE_INDEX_COUNT", index.len());
    int("TILE0_LON_COUNT", index[0].lon_count);
    int("TILE0_LAT_COUNT", index[0].lat_count);
    let ortho = mmap.orthometric_height_batch(&points, options);
    for (i, result) in ortho.iter().enumerate() {
        flag(&format!("MMAP_BATCH{i}_OK"), result.is_ok());
        match result {
            Ok(height) => f(&format!("MMAP_BATCH{i}_HEIGHT_BITS"), height.metres()),
            Err(err) => println!(
                "#define {P}MMAP_BATCH{i}_ERROR_KIND {}",
                variant("SIDEREON_TERRAIN_LOOKUP_ERROR_KIND_", err)
            ),
        }
    }
    let ellipsoidal = mmap
        .ellipsoidal_height_m_with_model(-106.5, 36.5, options, TerrainGeoidModel::Egm96OneDegree)
        .expect("ellipsoidal height");
    f("ELLIPSOIDAL_ONE_DEGREE_BITS", ellipsoidal.metres());

    let missing = format!("{root}/WW15MGH.DAC");
    match Egm96FifteenMinuteGeoid::from_ww15mgh_dac_path(std::path::Path::new(&missing)) {
        Err(TerrainDatumError::MissingEgm96Dac { remediation, .. }) => {
            flag("MISSING_DAC_REFUSED", true);
            println!(
                "static const char {P}MISSING_DAC_REMEDIATION[] = {};",
                c_string(remediation)
            );
        }
        other => panic!("missing WW15MGH.DAC read as {other:?}"),
    }
    println!();
}

fn sbas() {
    use sidereon_core::sbas::{SbasBlock, SbasMessage, SbasWireForm};
    println!("/* test_sbas_decode: sidereon_core::sbas::SbasBlock::decode of the test's");
    println!(" * 226-bit body. */");
    let hex = "5366819010029EE7ED83018202819BBE1A08BF8008FFA00000004066C0";
    let body: Vec<u8> = (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).expect("hex"))
        .collect();
    let block = SbasBlock::decode(&body, SbasWireForm::Body226).expect("SBAS block");
    println!(
        "#define {P}SBAS_KIND {}",
        variant("SIDEREON_SBAS_MESSAGE_KIND_", &block.message)
    );
    int("SBAS_MESSAGE_TYPE", block.message.message_type());
    let SbasMessage::LongTermCorrections(message) = &block.message else {
        panic!("SBAS vector is not long-term corrections");
    };
    let long_term_count: usize = message.halves.iter().map(|h| h.records.len()).sum();
    int("SBAS_LONG_TERM_COUNT", long_term_count);
    int("SBAS_HALF_COUNT", message.halves.len());
    for (h, half) in message.halves.iter().enumerate() {
        flag(&format!("SBAS_HALF{h}_VELOCITY_CODE"), half.velocity_code);
        int(&format!("SBAS_HALF{h}_IODP"), half.iodp);
        int(&format!("SBAS_HALF{h}_RECORD_COUNT"), half.records.len());
        let r = &half.records[0];
        let p = format!("SBAS_HALF{h}_RECORD0_");
        int(&format!("{p}MONITORED_INDEX"), r.monitored_index);
        int(&format!("{p}IODE"), r.iode);
        int(&format!("{p}DELTA_X"), r.delta_x);
        int(&format!("{p}DELTA_Y"), r.delta_y);
        int(&format!("{p}DELTA_Z"), r.delta_z);
        int(&format!("{p}DELTA_X_RATE"), r.delta_x_rate);
        int(&format!("{p}DELTA_Y_RATE"), r.delta_y_rate);
        int(&format!("{p}DELTA_Z_RATE"), r.delta_z_rate);
        int(&format!("{p}DELTA_A_F0"), r.delta_a_f0);
        int(&format!("{p}DELTA_A_F1"), r.delta_a_f1);
        flag(&format!("{p}HAS_TIME_OF_DAY"), r.time_of_day_s.is_some());
        int(&format!("{p}TIME_OF_DAY_S"), r.time_of_day_s.unwrap_or(0));
    }
    println!();
}

/// sidereon_araim_allocation_lpv_200 written to C and read back by
/// araim_allocation_from_c.
fn allocation() -> IntegrityAllocation {
    let mut value = IntegrityAllocation::lpv_200();
    if value.p_emt == 0.0 {
        value.p_emt = 1.0e-5;
    }
    value
}

fn summary(prefix: &str, result: &sidereon_core::araim::AraimResult) {
    flag(&format!("{prefix}AVAILABLE"), result.available);
    int(
        &format!("{prefix}FAULT_MODE_COUNT"),
        result.fault_modes.len(),
    );
    f(&format!("{prefix}HPL_BITS"), result.hpl_m);
    f(&format!("{prefix}VPL_BITS"), result.vpl_m);
    f(&format!("{prefix}EMT_BITS"), result.emt_m);
    f(&format!("{prefix}SIGMA_ACC_V_BITS"), result.sigma_acc_v_m);
}

fn araim_section() {
    println!("/* test_araim: sidereon_core::araim::araim on the test's geometry and ISM. */");
    let half_pi = std::f64::consts::PI / 2.0;
    let rows_in: [(&str, [f64; 3], GnssSystem); 10] = [
        ("G01", [0.0966, -0.0225, -0.9951], GnssSystem::Gps),
        ("G02", [0.2612, -0.6750, 0.6900], GnssSystem::Gps),
        ("G03", [0.7477, -0.0723, 0.6601], GnssSystem::Gps),
        ("G04", [0.2269, 0.9398, -0.2553], GnssSystem::Gps),
        ("G05", [0.2877, 0.5907, 0.7539], GnssSystem::Gps),
        ("E01", [0.9455, 0.3236, 0.0354], GnssSystem::Galileo),
        ("E02", [0.5957, 0.6748, -0.4356], GnssSystem::Galileo),
        ("E03", [0.7075, -0.0938, 0.7004], GnssSystem::Galileo),
        ("E04", [0.7709, -0.5571, -0.3088], GnssSystem::Galileo),
        ("E05", [0.2780, -0.6622, -0.6958], GnssSystem::Galileo),
    ];
    let row = |(id, los, system): (&str, [f64; 3], GnssSystem)| AraimRow {
        id: id.parse().expect("satellite token"),
        line_of_sight: LineOfSight::new(los[0], los[1], los[2]),
        system,
        elevation_rad: half_pi,
    };
    let receiver = sidereon_core::frame::Wgs84Geodetic::new(0.0, 0.0, 0.0).expect("receiver");
    let geometry = AraimGeometry {
        rows: rows_in.iter().copied().map(row).collect(),
        receiver,
        clock_systems: vec![GnssSystem::Gps, GnssSystem::Galileo],
        ut1_degraded: None,
    };
    let model = SatelliteIsmModel::new(0.75, 0.5, 0.5, 1.0e-5);
    let effective: [(&str, f64, f64); 10] = [
        ("G01", 3.8865, 3.5740),
        ("G02", 1.4377, 1.1252),
        ("G03", 0.8604, 0.5479),
        ("G04", 1.6383, 1.3258),
        ("G05", 1.3229, 1.0104),
        ("E01", 0.8434, 0.5309),
        ("E02", 0.8963, 0.5838),
        ("E03", 0.8669, 0.5544),
        ("E04", 0.8573, 0.5448),
        ("E05", 1.3616, 1.0491),
    ];
    let satellites = effective
        .iter()
        .map(|(id, int_var, acc_var)| {
            SatelliteIsm::new_with_effective_sigmas(
                id.parse().expect("satellite token"),
                0.75,
                0.5,
                0.5,
                1.0e-5,
                int_var.sqrt(),
                acc_var.sqrt(),
            )
        })
        .collect();
    let ism = Ism::new(
        vec![
            ConstellationIsm::new(GnssSystem::Gps, 1.0e-4, model),
            ConstellationIsm::new(GnssSystem::Galileo, 1.0e-4, model),
        ],
        satellites,
    );
    let result = araim(&geometry, &ism, &allocation()).expect("ARAIM");
    summary("ARAIM_", &result);
    let mode = &result.fault_modes[0];
    flag("ARAIM_MODE0_MONITORABLE", mode.monitorable);
    int("ARAIM_MODE0_EXCLUDED_COUNT", mode.excluded.len());
    flag(
        "ARAIM_MODE0_HAS_EXCLUDED_CONSTELLATION",
        mode.excluded_constellation.is_some(),
    );

    let s = 0.5773502691896258;
    let sparse_rows = [
        ("G01", [s, s, s], GnssSystem::Gps),
        ("G02", [s, -s, -s], GnssSystem::Gps),
        ("G03", [-s, s, -s], GnssSystem::Gps),
        ("G04", [-s, -s, s], GnssSystem::Gps),
    ];
    let sparse_geometry = AraimGeometry {
        rows: sparse_rows.iter().copied().map(row).collect(),
        receiver,
        clock_systems: vec![GnssSystem::Gps],
        ut1_degraded: None,
    };
    let sparse_ism = Ism::new(
        vec![ConstellationIsm::new(
            GnssSystem::Gps,
            0.0,
            SatelliteIsmModel::new(0.75, 0.5, 0.75, 1.0e-5),
        )],
        Vec::new(),
    );
    let sparse = araim(&sparse_geometry, &sparse_ism, &allocation()).expect("sparse ARAIM");
    summary("SPARSE_", &sparse);
    println!();
}

fn angles() {
    use sidereon_core::astro::angles::{angular_separation_coords, position_angle};
    println!("/* test_astro_angles: sidereon_core::astro::angles. */");
    f(
        "SEPARATION_BITS",
        angular_separation_coords((0.0, 0.0), (90.0, 0.0)).expect("separation"),
    );
    f(
        "POSITION_ANGLE_EAST_BITS",
        position_angle((0.0, 0.0), (90.0, 0.0)).expect("position angle"),
    );
    f(
        "POSITION_ANGLE_NORTH_BITS",
        position_angle((0.0, 0.0), (0.0, 10.0)).expect("position angle"),
    );
    println!();
}

fn main() {
    let root = format!("{}/dted/tiles", core_fixtures());
    header_start("w4_c012", GUARD);
    terrain(&root);
    sbas();
    araim_section();
    angles();
    header_end(GUARD);
}
