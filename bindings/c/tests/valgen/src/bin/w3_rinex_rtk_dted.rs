// Print tests/w3_rinex_rtk_dted_pins.h: sidereon-core's results for the inputs
// rinex_rtk_dted_smoke.c reads (the WTZR/WTZZ RINEX RTK arcs, the DTED fixture
// tiles and their edited copies, and the inline RINEX observation texts).

use sidereon_core::ephemeris::{Sp3, Sp3InterpolationOptions};
use sidereon_core::rinex::observations::{
    carrier_phase_rows, CorrectionUnavailable, ObservationFilter, RinexObs, RinexObsWriteError,
};
use sidereon_core::rtk_filter::{
    build_dual_frequency_rinex_rtk_arc, build_rinex_rtk_arc, RtkRinexArcOptions,
    RtkRinexDualArcOptions,
};
use sidereon_core::terrain::{DtedLookupOptions, DtedTerrain, DtedTile, DtedTileError};
use sidereon_core::terrain_store::{
    dted_tile_list_to_mmap_store, dted_tree_to_mmap_store, DtedTileListEntry, MmapTerrain,
    TerrainStoreError, TerrainTileId,
};
use valgen::{
    bits, c_string, c_string_literals, header_end, header_start, read, read_bytes, tests_path,
};

const P: &str = "W3RD";
const SOURCE: &str = "rinex_rtk_dted_smoke.c";

fn def(name: &str, value: impl std::fmt::Display) {
    println!("#define {P}_{name} {value}");
}

fn def_bits(name: &str, value: f64) {
    def(name, bits(value));
}

fn def_str(name: &str, value: &str) {
    def(name, c_string(value));
}

fn def_bool(name: &str, value: bool) {
    def(name, if value { "true" } else { "false" });
}

fn def_count(name: &str, value: usize) {
    def(name, format!("((size_t)UINT64_C({value}))"));
}

fn variant(debug: &str) -> String {
    let cut = debug.find(['{', ' ', '(']).unwrap_or(debug.len());
    debug[..cut].trim().to_string()
}

fn snake(name: &str) -> String {
    let mut out = String::new();
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

/// The C constant the binding writes for an engine enum value.
fn c_const(prefix: &str, value: &impl std::fmt::Debug) -> String {
    format!("{prefix}_{}", snake(&variant(&format!("{value:?}"))))
}

/// The FNV-1a-64 digest the C program takes of a byte buffer.
fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

fn arc_section() {
    // sidereon_sp3_load (default interpolation options) and
    // sidereon_rinex_obs_parse of the files run_ci_smoke.sh passes.
    let sp3 = Sp3::parse(&read_bytes(&tests_path(
        "fixtures/sp3/GBM0MGXRAP_20201770000_01D_05M_ORB_120epoch.sp3",
    )))
    .expect("parse SP3")
    .with_interpolation_options(Sp3InterpolationOptions::default());
    let base = RinexObs::parse(&read(&tests_path(
        "fixtures/obs/WTZR00DEU_R_20201770000_01D_30S_MO_120epoch.rnx",
    )))
    .expect("parse WTZR");
    let rover = RinexObs::parse(&read(&tests_path(
        "fixtures/obs/WTZZ00DEU_R_20201770000_01D_30S_MO_120epoch.rnx",
    )))
    .expect("parse WTZZ");

    // sidereon_rtk_rinex_arc_options_init with max_epochs 120 and prediction
    // time on: GPS C1C/L1C, four common satellites.
    let mut options = RtkRinexArcOptions::gps_l1_c();
    options.max_epochs = Some(120);
    options.min_common_satellites = 4;
    options.include_prediction_time = true;
    let arc = build_rinex_rtk_arc(&sp3, &base, &rover, &options).expect("single arc");
    println!("/* check_single_arc: build_rinex_rtk_arc of WTZR/WTZZ, epoch 0. */");
    def_count("SINGLE_EPOCH_COUNT", arc.epochs.len());
    def_count("SINGLE_SKIPPED_COUNT", arc.skipped_epoch_count);
    def_count("SINGLE_UNRESOLVED_COUNT", arc.unresolved_carriers.len());
    let epoch = &arc.epochs[0];
    def_count("SINGLE_E0_BASE_COUNT", epoch.base.len());
    def_count("SINGLE_E0_ROVER_COUNT", epoch.rover.len());
    def_count(
        "SINGLE_E0_POSITION_COUNT",
        epoch.satellite_positions_m.len(),
    );
    def_count(
        "SINGLE_E0_BASE_POSITION_COUNT",
        epoch.base_satellite_positions_m.len(),
    );
    def_count(
        "SINGLE_E0_ROVER_POSITION_COUNT",
        epoch.rover_satellite_positions_m.len(),
    );
    def_bool("SINGLE_E0_HAS_VELOCITY", epoch.velocity_mps.is_some());
    def_bool(
        "SINGLE_E0_HAS_PREDICTION_TIME",
        epoch.prediction_time_s.is_some(),
    );
    def_bits(
        "SINGLE_E0_PREDICTION_TIME",
        epoch.prediction_time_s.unwrap_or(0.0),
    );
    for (name, obs) in [("BASE0", &epoch.base[0]), ("ROVER0", &epoch.rover[0])] {
        def_str(&format!("SINGLE_E0_{name}_SAT"), &obs.satellite_id);
        def_str(&format!("SINGLE_E0_{name}_AMBIGUITY"), &obs.ambiguity_id);
        def_bool(&format!("SINGLE_E0_{name}_HAS_LLI"), obs.lli.is_some());
        def(&format!("SINGLE_E0_{name}_LLI"), obs.lli.unwrap_or(0));
        def_bits(&format!("SINGLE_E0_{name}_CODE"), obs.code_m);
        def_bits(&format!("SINGLE_E0_{name}_PHASE"), obs.phase_m);
    }
    for (name, map) in [
        ("POSITION0", &epoch.satellite_positions_m),
        ("BASE_POSITION0", &epoch.base_satellite_positions_m),
        ("ROVER_POSITION0", &epoch.rover_satellite_positions_m),
    ] {
        let (id, pos) = map.iter().next().expect("a satellite position");
        def_str(&format!("SINGLE_E0_{name}_ID"), id);
        for (k, value) in pos.iter().enumerate() {
            def_bits(&format!("SINGLE_E0_{name}_{k}"), *value);
        }
    }
    def_count("SINGLE_WAVELENGTH_COUNT", arc.wavelengths_m.len());
    let (id, value) = arc.wavelengths_m.iter().next().expect("a wavelength");
    def_str("SINGLE_WAVELENGTH0_ID", id);
    def_bits("SINGLE_WAVELENGTH0", *value);
    def_count("SINGLE_OFFSET_COUNT", arc.offsets_m.len());
    let (id, value) = arc.offsets_m.iter().next().expect("an offset");
    def_str("SINGLE_OFFSET0_ID", id);
    def_bits("SINGLE_OFFSET0", *value);
    let mut invalid = RtkRinexArcOptions::gps_l1_c();
    invalid.max_epochs = Some(120);
    invalid.min_common_satellites = 0;
    invalid.include_prediction_time = true;
    // The C status of a refusal is the binding's (map_rtk_rinex_arc_error).
    def_bool(
        "SINGLE_MIN_ZERO_REFUSED",
        build_rinex_rtk_arc(&sp3, &base, &rover, &invalid).is_err(),
    );
    println!();

    let mut dual_options = RtkRinexDualArcOptions::gps_l1_l2_cw();
    dual_options.max_epochs = Some(120);
    dual_options.min_common_satellites = 4;
    dual_options.include_prediction_time = true;
    let dual =
        build_dual_frequency_rinex_rtk_arc(&sp3, &base, &rover, &dual_options).expect("dual arc");
    println!("/* check_dual_arc: build_dual_frequency_rinex_rtk_arc, epoch 0. */");
    def_count("DUAL_EPOCH_COUNT", dual.epochs.len());
    def_count("DUAL_SKIPPED_COUNT", dual.skipped_epoch_count);
    def_count("DUAL_UNRESOLVED_COUNT", dual.unresolved_carriers.len());
    let epoch = &dual.epochs[0];
    def_count("DUAL_E0_OBSERVATION_COUNT", epoch.observations.len());
    def_count("DUAL_E0_POSITION_COUNT", epoch.satellite_positions_m.len());
    def_count(
        "DUAL_E0_BASE_POSITION_COUNT",
        epoch.base_satellite_positions_m.len(),
    );
    def_count(
        "DUAL_E0_ROVER_POSITION_COUNT",
        epoch.rover_satellite_positions_m.len(),
    );
    def_bool("DUAL_E0_HAS_VELOCITY", epoch.velocity_mps.is_some());
    def_bool("DUAL_E0_HAS_GAP_TIME", epoch.gap_time_s.is_some());
    def_bool(
        "DUAL_E0_HAS_PREDICTION_TIME",
        epoch.prediction_time_s.is_some(),
    );
    def_bits("DUAL_E0_JD_WHOLE", epoch.jd_whole);
    def_bits("DUAL_E0_JD_FRACTION", epoch.jd_fraction);
    def_bits("DUAL_E0_GAP_TIME", epoch.gap_time_s.unwrap_or(0.0));
    def_bits(
        "DUAL_E0_PREDICTION_TIME",
        epoch.prediction_time_s.unwrap_or(0.0),
    );
    def_str(
        "DUAL_E0_SORT_KEY",
        epoch.epoch_sort_key.as_deref().unwrap_or(""),
    );
    let first = &epoch.observations[0];
    def_str("DUAL_E0_OBS0_SAT", &first.satellite_id);
    for (name, obs) in [("BASE", &first.base), ("ROVER", &first.rover)] {
        def_str(&format!("DUAL_E0_OBS0_{name}_AMBIGUITY"), &obs.ambiguity_id);
        def_bool(&format!("DUAL_E0_OBS0_{name}_HAS_LLI1"), obs.lli1.is_some());
        def(&format!("DUAL_E0_OBS0_{name}_LLI1"), obs.lli1.unwrap_or(0));
        def_bool(&format!("DUAL_E0_OBS0_{name}_HAS_LLI2"), obs.lli2.is_some());
        def(&format!("DUAL_E0_OBS0_{name}_LLI2"), obs.lli2.unwrap_or(0));
        def_bits(&format!("DUAL_E0_OBS0_{name}_P1"), obs.p1_m);
        def_bits(&format!("DUAL_E0_OBS0_{name}_P2"), obs.p2_m);
        def_bits(&format!("DUAL_E0_OBS0_{name}_PHI1"), obs.phi1_cycles);
        def_bits(&format!("DUAL_E0_OBS0_{name}_PHI2"), obs.phi2_cycles);
        def_bits(&format!("DUAL_E0_OBS0_{name}_F1"), obs.f1_hz);
        def_bits(&format!("DUAL_E0_OBS0_{name}_F2"), obs.f2_hz);
    }
    let (id, pos) = epoch
        .satellite_positions_m
        .iter()
        .next()
        .expect("a satellite position");
    def_str("DUAL_E0_POSITION0_ID", id);
    for (k, value) in pos.iter().enumerate() {
        def_bits(&format!("DUAL_E0_POSITION0_{k}"), *value);
    }
    println!();
}

/// The data-record layout of the 5 x 5 fixture tiles, as the C program states
/// it: the first data record's offset, the record length, and the DSI datum
/// field's offset.
const DTED_DATA_OFFSET: usize = 3428;
const DTED_BLOCK_LEN: usize = 12 + 2 * 5;
const DTED_DSI_DATUM_OFFSET: usize = 80 + 144;

fn set_block_checksum(tile: &mut [u8], block: usize) {
    let offset = DTED_DATA_OFFSET + block * DTED_BLOCK_LEN;
    let mut sum: i32 = 0;
    for i in 0..DTED_BLOCK_LEN - 4 {
        sum = sum.wrapping_add(i32::from(tile[offset + i]));
    }
    tile[offset + DTED_BLOCK_LEN - 4..offset + DTED_BLOCK_LEN]
        .copy_from_slice(&(sum as u32).to_be_bytes());
}

fn store_error_kind(err: &TerrainStoreError) -> String {
    c_const("SIDEREON_TERRAIN_STORE_ERROR_KIND", err)
}

/// The C kind terrain_lookup_error_to_c (src/terrain.rs) writes.
fn lookup_kind(err: &sidereon_core::Error) -> &'static str {
    use sidereon_core::Error as E;
    match err {
        E::InvalidInput(_) => "SIDEREON_TERRAIN_LOOKUP_ERROR_KIND_INVALID_INPUT",
        E::Parse(_) => "SIDEREON_TERRAIN_LOOKUP_ERROR_KIND_PARSE",
        E::MissingTerrainTile { .. } => "SIDEREON_TERRAIN_LOOKUP_ERROR_KIND_MISSING_TILE",
        E::UnknownTerrainElevation { .. } => "SIDEREON_TERRAIN_LOOKUP_ERROR_KIND_UNKNOWN_ELEVATION",
        E::NonWgs84TerrainTile { .. } => "SIDEREON_TERRAIN_LOOKUP_ERROR_KIND_NON_WGS84_TILE",
        _ => "SIDEREON_TERRAIN_LOOKUP_ERROR_KIND_OTHER",
    }
}

fn emit_lookup_error(name: &str, err: &sidereon_core::Error) {
    use sidereon_core::Error as E;
    def(&format!("{name}_KIND"), lookup_kind(err));
    match err {
        E::UnknownTerrainElevation {
            lat_index,
            lon_index,
            latitude_posting,
            longitude_posting,
        } => {
            def(&format!("{name}_LAT_INDEX"), lat_index);
            def(&format!("{name}_LON_INDEX"), lon_index);
            def(&format!("{name}_LATITUDE_POSTING"), latitude_posting);
            def(&format!("{name}_LONGITUDE_POSTING"), longitude_posting);
        }
        E::NonWgs84TerrainTile {
            lat_index,
            lon_index,
            datum,
        } => {
            def(&format!("{name}_LAT_INDEX"), lat_index);
            def(&format!("{name}_LON_INDEX"), lon_index);
            def(
                &format!("{name}_DATUM"),
                c_const("SIDEREON_DTED_HORIZONTAL_DATUM", datum),
            );
        }
        other => panic!("{name}: lookup failure {other:?}"),
    }
}

fn dted_section() {
    let root = tests_path("fixtures/dted/tiles");
    let w107 = format!("{root}/n36_w107_1arc_v3.dt2");
    let w106 = format!("{root}/n36_w106_1arc_v3.dt2");
    let entries = [
        DtedTileListEntry::new(TerrainTileId::new(36, -107), &w107),
        DtedTileListEntry::new(TerrainTileId::new(36, -106), &w106),
    ];
    let store = dted_tile_list_to_mmap_store(&entries).expect("tile-list store");
    println!("/* check_dted_store: the terrain store of the two fixture tiles. */");
    def_count("STORE_LEN", store.len());
    def(
        "STORE_FNV1A64",
        format!("UINT64_C({:#018x})", fnv1a64(&store)),
    );
    let tree = dted_tree_to_mmap_store(&root).expect("tree store");
    def_bool("TREE_EQUALS_LIST", tree == store);
    let mismatch = [DtedTileListEntry::new(TerrainTileId::new(35, -107), &w107)];
    let err = dted_tile_list_to_mmap_store(&mismatch).expect_err("tile-id mismatch");
    def("MISMATCH_KIND", store_error_kind(&err));
    let missing_parent = std::env::temp_dir().join("sidereon-w3-valgen-missing-parent/store");
    let err =
        sidereon_core::terrain_store::write_dted_tile_list_to_mmap_store(&entries, &missing_parent)
            .expect_err("write into a missing directory");
    def("WRITE_MISSING_PARENT_KIND", store_error_kind(&err));
    println!();

    let tile = read_bytes(&w106);
    println!("/* check_dted_nulls_and_datum: edited copies of the n36_w106 tile. */");
    def_count("TILE_LEN", tile.len());
    let dir = std::env::temp_dir().join(format!("sidereon-w3-valgen-dted-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("temp tile dir");
    let path = dir.join("n36_w106_1arc_v3.dt2");

    let mut null_tile = tile.clone();
    null_tile[DTED_DATA_OFFSET + 8] = 0xFF;
    null_tile[DTED_DATA_OFFSET + 9] = 0xFF;
    set_block_checksum(&mut null_tile, 0);
    std::fs::write(&path, &null_tile).expect("write null tile");
    let loaded = DtedTile::from_path(&path).expect("null tile loads");
    let datum = loaded.horizontal_datum();
    def(
        "NULL_TILE_DATUM",
        c_const("SIDEREON_DTED_HORIZONTAL_DATUM", datum),
    );
    def_bool(
        "NULL_TILE_DATUM_WGS84_COMPATIBLE",
        datum.is_wgs84_compatible(),
    );
    match loaded.get_elevation(-106.0, 36.0) {
        Err(DtedTileError::NullPosting {
            longitude_index,
            latitude_index,
        }) => {
            def(
                "NULL_TILE_ERROR_KIND",
                "SIDEREON_DTED_TILE_ERROR_KIND_NULL_POSTING",
            );
            def("NULL_TILE_LONGITUDE_INDEX", longitude_index);
            def("NULL_TILE_LATITUDE_INDEX", latitude_index);
        }
        other => panic!("null corner reads {other:?}"),
    }
    def(
        "NULL_TILE_KNOWN_ELEVATION",
        loaded.get_elevation(-105.5, 36.5).expect("known posting"),
    );
    let mut terrain = DtedTerrain::new(&dir);
    let err = terrain
        .height_m(-106.0, 36.0)
        .expect_err("terrain null corner");
    emit_lookup_error("NULL_TERRAIN_ERROR", &err);
    let batch = terrain.height_batch(
        &[(-106.0, 36.0), (-105.5, 36.5)],
        DtedLookupOptions::default(),
    );
    match &batch[0] {
        Err(err) => def("NULL_BATCH0_KIND", lookup_kind(err)),
        Ok(value) => panic!("batch null corner reads {value}"),
    }
    def_bits(
        "NULL_BATCH1_HEIGHT",
        *batch[1].as_ref().expect("batch known posting"),
    );
    let null_entry = [DtedTileListEntry::new(TerrainTileId::new(36, -106), &path)];
    let null_store = dted_tile_list_to_mmap_store(&null_entry).expect("null store");
    let mut mmap = MmapTerrain::from_vec(null_store).expect("open null store");
    let err = mmap.height_m(-106.0, 36.0).expect_err("store null corner");
    emit_lookup_error("NULL_STORE_ERROR", &err);

    let mut wgs72_tile = tile.clone();
    wgs72_tile[DTED_DSI_DATUM_OFFSET..DTED_DSI_DATUM_OFFSET + 5].copy_from_slice(b"WGS72");
    std::fs::write(&path, &wgs72_tile).expect("write WGS72 tile");
    let loaded = DtedTile::from_path(&path).expect("WGS72 tile loads");
    def(
        "WGS72_TILE_DATUM",
        c_const("SIDEREON_DTED_HORIZONTAL_DATUM", loaded.horizontal_datum()),
    );
    def_bool(
        "WGS72_TILE_DATUM_WGS84_COMPATIBLE",
        loaded.horizontal_datum().is_wgs84_compatible(),
    );
    let wgs72_entry = [DtedTileListEntry::new(TerrainTileId::new(36, -106), &path)];
    let err = dted_tile_list_to_mmap_store(&wgs72_entry).expect_err("WGS72 store refused");
    def("WGS72_STORE_KIND", store_error_kind(&err));
    match &err {
        TerrainStoreError::NonWgs84Tile { datum, .. } => def(
            "WGS72_STORE_DATUM",
            c_const("SIDEREON_DTED_HORIZONTAL_DATUM", datum),
        ),
        other => panic!("WGS72 store refusal reads {other:?}"),
    }
    let mut terrain = DtedTerrain::new(&dir);
    let err = terrain
        .height_m(-105.5, 36.5)
        .expect_err("WGS72 lookup refused");
    emit_lookup_error("WGS72_TERRAIN_ERROR", &err);
    let _ = std::fs::remove_dir_all(&dir);

    let missing = std::env::temp_dir().join("sidereon-w3-valgen-missing-tile.dt2");
    match DtedTile::from_path(&missing) {
        Err(err) => def(
            "MISSING_TILE_KIND",
            c_const("SIDEREON_DTED_TILE_ERROR_KIND", &err),
        ),
        Ok(_) => panic!("a missing tile loads"),
    }
    println!();
}

fn obs_section() {
    let source = tests_path(SOURCE);
    println!("/* The inline RINEX observation texts of the C source. */");
    let scale = c_string_literals(&source, "static const char SCALE_FACTOR_V2_OBS[] =");
    let obs = RinexObs::parse(&scale).expect("scale-factor product parses");
    match obs.to_rinex_string() {
        Err(RinexObsWriteError::ScaleFactorsInVersionTwo { count }) => {
            def(
                "SCALE_WRITE_ERROR_KIND",
                "SIDEREON_RINEX_OBS_WRITE_ERROR_KIND_SCALE_FACTORS_IN_VERSION_TWO",
            );
            def_count("SCALE_WRITE_ERROR_COUNT", count);
        }
        other => panic!("scale-factor write reads {other:?}"),
    }

    let untimed = c_string_literals(&source, "static const char UNTIMED_EVENT_OBS[] =");
    let obs = RinexObs::parse(&untimed).expect("untimed event parses");
    def_count("UNTIMED_EPOCH_COUNT", obs.epochs().len());
    for (i, epoch) in obs.epochs().iter().enumerate() {
        def_bool(&format!("UNTIMED_E{i}_HAS_EPOCH"), epoch.epoch.is_some());
        def(&format!("UNTIMED_E{i}_FLAG"), epoch.flag);
        def_count(&format!("UNTIMED_E{i}_SATELLITE_COUNT"), epoch.sats.len());
        if let Some(time) = epoch.epoch {
            def(&format!("UNTIMED_E{i}_YEAR"), time.year);
            def(&format!("UNTIMED_E{i}_MINUTE"), time.minute);
        }
    }
    let written = obs.to_rinex_string().expect("untimed event writes");
    let event_line = written
        .lines()
        .find(|line| line.starts_with('>') && line.trim_end().ends_with("4  1"))
        .expect("the event line");
    def_str("UNTIMED_EVENT_LINE", &format!("\n{event_line}\n"));
    let reread = RinexObs::parse(&written).expect("untimed event reparses");
    def_count("UNTIMED_REREAD_EPOCH_COUNT", reread.epochs().len());
    def_bool(
        "UNTIMED_REREAD_E1_HAS_EPOCH",
        reread.epochs()[1].epoch.is_some(),
    );
    def("UNTIMED_REREAD_E1_FLAG", reread.epochs()[1].flag);

    let ambiguous = c_string_literals(&source, "static const char AMBIGUOUS_PHASE_SHIFT_OBS[] =");
    let obs = RinexObs::parse(&ambiguous).expect("ambiguous phase shift parses");
    let header = obs.header_at(0).expect("header at epoch 0");
    let rows: Vec<_> = carrier_phase_rows(&header, &obs.epochs()[0], &ObservationFilter::all())
        .expect("carrier-phase rows")
        .into_iter()
        .flat_map(|(sat, rows)| rows.into_iter().map(move |row| (sat, row)))
        .collect();
    def_count("PHASE_ROW_COUNT", rows.len());
    for (i, (sat, row)) in rows.iter().enumerate() {
        def_str(&format!("PHASE_ROW{i}_SAT"), &sat.to_string());
        def_str(&format!("PHASE_ROW{i}_CODE"), &row.code);
        def_bool(
            &format!("PHASE_ROW{i}_HAS_VALUE"),
            row.value_cycles.is_some(),
        );
        def_bits(
            &format!("PHASE_ROW{i}_VALUE"),
            row.value_cycles.unwrap_or(0.0),
        );
        match &row.phase_shift_cycles {
            Ok(cycles) => {
                def(
                    &format!("PHASE_ROW{i}_STATUS"),
                    "SIDEREON_RINEX_CORRECTION_STATUS_AVAILABLE",
                );
                def_bits(&format!("PHASE_ROW{i}_SHIFT"), *cycles);
                def_count(&format!("PHASE_ROW{i}_CONFLICT_COUNT"), 0);
            }
            Err(CorrectionUnavailable::Unknown) => {
                def(
                    &format!("PHASE_ROW{i}_STATUS"),
                    "SIDEREON_RINEX_CORRECTION_STATUS_UNKNOWN",
                );
                def_count(&format!("PHASE_ROW{i}_CONFLICT_COUNT"), 0);
            }
            Err(CorrectionUnavailable::Ambiguous { corrections }) => {
                def(
                    &format!("PHASE_ROW{i}_STATUS"),
                    "SIDEREON_RINEX_CORRECTION_STATUS_AMBIGUOUS",
                );
                def_count(&format!("PHASE_ROW{i}_CONFLICT_COUNT"), corrections.len());
                for (j, correction) in corrections.iter().enumerate() {
                    def_bool(
                        &format!("PHASE_ROW{i}_CONFLICT{j}_HAS_CYCLES"),
                        correction.is_some(),
                    );
                    def_bits(
                        &format!("PHASE_ROW{i}_CONFLICT{j}_CYCLES"),
                        correction.unwrap_or(0.0),
                    );
                }
            }
        }
    }
    println!();
}

fn main() {
    let guard = "SIDEREON_W3_RINEX_RTK_DTED_PINS_H";
    header_start("w3_rinex_rtk_dted", guard);
    arc_section();
    dted_section();
    obs_section();
    header_end(guard);
}
