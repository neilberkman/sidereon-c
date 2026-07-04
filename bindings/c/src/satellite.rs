use super::*;

/// A built-once fleet of already-initialized SGP4 satellites for repeated batch
/// operations. Opaque to C. Build it once with
/// sidereon_satellite_constellation_build from parsed TLE handles, then call
/// sidereon_satellite_constellation_propagate / _visible / _look_angle_arcs /
/// _ground_tracks / _passes as often as you like: it owns its satellites and
/// borrows them on each call, so the same constellation drives a live scene
/// across frames with no re-parse. Release with
/// sidereon_satellite_constellation_free. This is the C form of the WASM
/// Constellation and Elixir Sidereon.Constellation. It does no parsing or I/O:
/// TLE text becomes satellites at the sidereon_tle_load / sidereon_parse_tle_file
/// boundary; the constellation only batches the core geometry over the satellites
/// it was handed. The input order is the fleet order (the leading axis of every
/// batch result and the satellite_index of every pass).
pub struct SidereonSatelliteConstellation {
    pub(crate) satellites: Vec<Satellite>,
    pub(crate) ids: Vec<String>,
}

/// Per-satellite topocentric look-angle arcs over a fleet. Opaque to C. Create
/// with sidereon_satellite_constellation_look_angle_arcs and release with
/// sidereon_satellite_constellation_look_angles_free. Element i is satellite i's
/// arc, in fleet order; a satellite that fails to propagate yields an empty arc
/// so the result stays index-aligned with the constellation.
pub struct SidereonSatelliteConstellationLookAngles {
    pub(crate) inner: Vec<Vec<LookAngle>>,
}

/// Per-satellite sub-satellite (ground-track) arcs over a fleet. Opaque to C.
/// Create with sidereon_satellite_constellation_ground_tracks and release with
/// sidereon_satellite_constellation_ground_tracks_free. Element i is satellite
/// i's track, in fleet order; a satellite that fails yields an empty track so the
/// result stays index-aligned with the constellation.
pub struct SidereonSatelliteConstellationGroundTracks {
    pub(crate) inner: Vec<Vec<Wgs84Geodetic>>,
}

/// Flattened satellite passes over a fleet, each tagged with the fleet-order
/// satellite_index it belongs to. Opaque to C. Create with
/// sidereon_satellite_constellation_passes and release with
/// sidereon_satellite_constellation_passes_free.
pub struct SidereonSatelliteConstellationPasses {
    pub(crate) inner: Vec<SidereonFleetPass>,
}

/// One pass in a sidereon_satellite_constellation_passes result: the pass
/// geometry plus the fleet-order satellite_index of the satellite it belongs to
/// (map that index to your own per-satellite metadata).
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonFleetPass {
    /// Fleet-order index of the satellite this pass belongs to.
    pub satellite_index: usize,
    /// The pass geometry.
    pub pass: SidereonSatellitePass,
}

/// Build a satellite constellation from already-parsed TLE handles for repeated
/// batch operations. tles is an array of count live TLE handle pointers (each
/// carrying its own opsmode from sidereon_tle_load); the handles are borrowed,
/// not consumed, so the caller still owns and must free each one. Each
/// satellite's TLE-recorded NORAD catalog number is captured as its id (the id
/// reported by sidereon_satellite_constellation_visible), and the input order is
/// preserved as the fleet order. On success writes a newly owned handle to
/// *out_constellation. Release it with sidereon_satellite_constellation_free.
///
/// Safety: tles must point to count live TLE handle pointers; out_constellation
/// must point to handle storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_satellite_constellation_build(
    tles: *const *const SidereonTle,
    count: usize,
    out_constellation: *mut *mut SidereonSatelliteConstellation,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_satellite_constellation_build",
        SidereonStatus::Panic,
        || {
            let out_constellation = c_try!(require_out(
                out_constellation,
                "sidereon_satellite_constellation_build",
                "out_constellation"
            ));
            *out_constellation = ptr::null_mut();
            let tle_ptrs = c_try!(require_slice(
                tles,
                count,
                "sidereon_satellite_constellation_build",
                "tles"
            ));
            let mut satellites = Vec::with_capacity(tle_ptrs.len());
            let mut ids = Vec::with_capacity(tle_ptrs.len());
            for (idx, tle_ptr) in tle_ptrs.iter().enumerate() {
                let tle = c_try!(require_ref(
                    *tle_ptr,
                    "sidereon_satellite_constellation_build",
                    &format!("tles[{idx}]")
                ));
                satellites.push(tle.satellite.clone());
                ids.push(tle.elements.catalog_number.clone());
            }
            write_boxed_handle(
                out_constellation,
                SidereonSatelliteConstellation { satellites, ids },
            );
            SidereonStatus::Ok
        },
    )
}

/// Write the number of satellites in a constellation to *out_count (the leading
/// axis of every batch result).
///
/// Safety: constellation must be a live handle; out_count must point to a size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_satellite_constellation_satellite_count(
    constellation: *const SidereonSatelliteConstellation,
    out_count: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_satellite_constellation_satellite_count",
        SidereonStatus::Panic,
        || {
            let out_count = c_try!(require_out(
                out_count,
                "sidereon_satellite_constellation_satellite_count",
                "out_count"
            ));
            *out_count = 0;
            let constellation = c_try!(require_ref(
                constellation,
                "sidereon_satellite_constellation_satellite_count",
                "constellation"
            ));
            *out_count = constellation.satellites.len();
            SidereonStatus::Ok
        },
    )
}

/// Copy the NORAD catalog number of the satellite at index into buf as a
/// null-terminated string. Always writes the required buffer size (including the
/// terminator) to *out_required; pass buf NULL with len 0 to query it. If len is
/// nonzero but smaller than out_required, returns InvalidArgument and leaves buf
/// null-terminated (empty) when len is positive. An out-of-range index returns
/// InvalidArgument.
///
/// Safety: constellation must be a live handle; buf must point to at least len
/// writable bytes or be NULL when len is 0; out_required must point to a size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_satellite_constellation_catalog_number(
    constellation: *const SidereonSatelliteConstellation,
    index: usize,
    buf: *mut c_char,
    len: usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_satellite_constellation_catalog_number",
        SidereonStatus::Panic,
        || {
            let out_required = c_try!(require_out(
                out_required,
                "sidereon_satellite_constellation_catalog_number",
                "out_required"
            ));
            *out_required = 0;
            if !buf.is_null() && len > 0 {
                *buf = 0;
            }
            let constellation = c_try!(require_ref(
                constellation,
                "sidereon_satellite_constellation_catalog_number",
                "constellation"
            ));
            let id = match constellation.ids.get(index) {
                Some(id) => id.as_bytes(),
                None => {
                    set_last_error(format!(
                        "sidereon_satellite_constellation_catalog_number: index {index} out of range ({} satellites)",
                        constellation.ids.len()
                    ));
                    return SidereonStatus::InvalidArgument;
                }
            };
            let required = id.len() + 1;
            *out_required = required;
            if buf.is_null() {
                if len == 0 {
                    return SidereonStatus::Ok;
                }
                set_last_error(
                    "sidereon_satellite_constellation_catalog_number: null buf".to_string(),
                );
                return SidereonStatus::NullPointer;
            }
            if len < required {
                set_last_error(format!(
                    "sidereon_satellite_constellation_catalog_number: buf needs room for {required} bytes"
                ));
                return SidereonStatus::InvalidArgument;
            }
            ptr::copy_nonoverlapping(id.as_ptr().cast::<c_char>(), buf, id.len());
            *buf.add(id.len()) = 0;
            SidereonStatus::Ok
        },
    )
}

/// Propagate the whole constellation over a shared UTC unix-microsecond epoch
/// grid, borrowing it (not consumed, so the same constellation drives every
/// frame). When parallel is true the engine's rayon batch path is used. Element
/// (i, j) of the result is satellite i propagated to epoch j, bit-for-bit
/// identical to the per-satellite sidereon_tle_propagate path. On success writes
/// a newly owned batch handle to *out_batch. Release it with
/// sidereon_tle_batch_propagation_free, and read it back with the
/// sidereon_tle_batch_propagation_* accessors. Fails (naming the satellite index)
/// if any satellite fails to propagate.
///
/// Safety: constellation must be a live handle; epochs_unix_us must point to
/// epoch_count int64_t values; out_batch must point to handle storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_satellite_constellation_propagate(
    constellation: *const SidereonSatelliteConstellation,
    epochs_unix_us: *const i64,
    epoch_count: usize,
    parallel: bool,
    out_batch: *mut *mut SidereonTleBatchPropagation,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_satellite_constellation_propagate",
        SidereonStatus::Panic,
        || {
            let out_batch = c_try!(require_out(
                out_batch,
                "sidereon_satellite_constellation_propagate",
                "out_batch"
            ));
            *out_batch = ptr::null_mut();
            let constellation = c_try!(require_ref(
                constellation,
                "sidereon_satellite_constellation_propagate",
                "constellation"
            ));
            let instants = c_try!(unix_instants_from_c(
                "sidereon_satellite_constellation_propagate",
                epochs_unix_us,
                epoch_count,
            ));
            let epoch_count = instants.len();
            let results = if parallel {
                propagate_teme_batch_parallel(&constellation.satellites, &instants)
            } else {
                propagate_teme_batch_serial(&constellation.satellites, &instants)
            };
            let inner = c_try!(unwrap_prediction_batch(
                "sidereon_satellite_constellation_propagate",
                results
            ));
            write_boxed_handle(
                out_batch,
                SidereonTleBatchPropagation { epoch_count, inner },
            );
            SidereonStatus::Ok
        },
    )
}

/// Find the constellation satellites visible above min_elevation_deg from a
/// ground station at one UTC unix-microsecond instant, each with its catalog
/// number and topocentric az/el/range, sorted by elevation descending. The
/// constellation form of sidereon_visible_from_satellites. Per-satellite
/// propagation or frame failures are skipped. On success writes a newly owned
/// handle to *out_visible. Release it with sidereon_visible_list_free, and read
/// it back with the sidereon_visible_list_* accessors.
///
/// Safety: constellation must be a live handle; station must point to a
/// SidereonGroundStation; out_visible must point to handle storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_satellite_constellation_visible(
    constellation: *const SidereonSatelliteConstellation,
    station: *const SidereonGroundStation,
    epoch_unix_us: i64,
    min_elevation_deg: f64,
    out_visible: *mut *mut SidereonVisibleList,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_satellite_constellation_visible",
        SidereonStatus::Panic,
        || {
            let out_visible = c_try!(require_out(
                out_visible,
                "sidereon_satellite_constellation_visible",
                "out_visible"
            ));
            *out_visible = ptr::null_mut();
            let constellation = c_try!(require_ref(
                constellation,
                "sidereon_satellite_constellation_visible",
                "constellation"
            ));
            let station = c_try!(require_ref(
                station,
                "sidereon_satellite_constellation_visible",
                "station"
            ));
            let inner = c_try!(visible_from_satellites(
                &constellation.satellites,
                &constellation.ids,
                ground_station_from_c(station),
                UtcInstant::from_unix_microseconds(epoch_unix_us),
                min_elevation_deg,
            )
            .map_err(|err| map_pass_error("sidereon_satellite_constellation_visible", err)));
            write_boxed_handle(out_visible, SidereonVisibleList { inner });
            SidereonStatus::Ok
        },
    )
}

/// Compute topocentric az/el/range arcs from a ground station for every satellite
/// in the constellation over a shared UTC unix-microsecond epoch grid, in fleet
/// order. When parallel is true the engine's rayon batch path is used. A
/// satellite that fails to propagate yields an empty arc so the result stays
/// index-aligned with the constellation. On success writes a newly owned handle
/// to *out_arcs. Release it with
/// sidereon_satellite_constellation_look_angles_free.
///
/// Safety: constellation must be a live handle; station must point to a
/// SidereonGroundStation; epochs_unix_us must point to epoch_count int64_t
/// values; out_arcs must point to handle storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_satellite_constellation_look_angle_arcs(
    constellation: *const SidereonSatelliteConstellation,
    station: *const SidereonGroundStation,
    epochs_unix_us: *const i64,
    epoch_count: usize,
    parallel: bool,
    out_arcs: *mut *mut SidereonSatelliteConstellationLookAngles,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_satellite_constellation_look_angle_arcs",
        SidereonStatus::Panic,
        || {
            let out_arcs = c_try!(require_out(
                out_arcs,
                "sidereon_satellite_constellation_look_angle_arcs",
                "out_arcs"
            ));
            *out_arcs = ptr::null_mut();
            let constellation = c_try!(require_ref(
                constellation,
                "sidereon_satellite_constellation_look_angle_arcs",
                "constellation"
            ));
            let station = c_try!(require_ref(
                station,
                "sidereon_satellite_constellation_look_angle_arcs",
                "station"
            ));
            let instants = c_try!(unix_instants_from_c(
                "sidereon_satellite_constellation_look_angle_arcs",
                epochs_unix_us,
                epoch_count,
            ));
            let ground_station = ground_station_from_c(station);
            let results = if parallel {
                look_angle_batch_parallel(&constellation.satellites, ground_station, &instants)
            } else {
                look_angle_batch_serial(&constellation.satellites, ground_station, &instants)
            };
            // A per-satellite failure becomes an empty arc, keeping the result
            // index-aligned with the fleet (the WASM/Elixir contract).
            let inner = results
                .into_iter()
                .map(|arc| arc.unwrap_or_default())
                .collect();
            write_boxed_handle(out_arcs, SidereonSatelliteConstellationLookAngles { inner });
            SidereonStatus::Ok
        },
    )
}

/// Write the number of satellites (arcs) in a constellation look-angle result to
/// *out_count.
///
/// Safety: arcs must be a live handle; out_count must point to a size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_satellite_constellation_look_angles_satellite_count(
    arcs: *const SidereonSatelliteConstellationLookAngles,
    out_count: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_satellite_constellation_look_angles_satellite_count",
        SidereonStatus::Panic,
        || {
            let out_count = c_try!(require_out(
                out_count,
                "sidereon_satellite_constellation_look_angles_satellite_count",
                "out_count"
            ));
            *out_count = 0;
            let arcs = c_try!(require_ref(
                arcs,
                "sidereon_satellite_constellation_look_angles_satellite_count",
                "arcs"
            ));
            *out_count = arcs.inner.len();
            SidereonStatus::Ok
        },
    )
}

/// Write the number of look angles in the arc of the satellite at index to
/// *out_len. An out-of-range index returns InvalidArgument. Use this to split the
/// flattened sidereon_satellite_constellation_look_angles_values output back into
/// per-satellite arcs (which may be ragged when a satellite failed).
///
/// Safety: arcs must be a live handle; out_len must point to a size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_satellite_constellation_look_angles_arc_len(
    arcs: *const SidereonSatelliteConstellationLookAngles,
    index: usize,
    out_len: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_satellite_constellation_look_angles_arc_len",
        SidereonStatus::Panic,
        || {
            let out_len = c_try!(require_out(
                out_len,
                "sidereon_satellite_constellation_look_angles_arc_len",
                "out_len"
            ));
            *out_len = 0;
            let arcs = c_try!(require_ref(
                arcs,
                "sidereon_satellite_constellation_look_angles_arc_len",
                "arcs"
            ));
            match arcs.inner.get(index) {
                Some(arc) => {
                    *out_len = arc.len();
                    SidereonStatus::Ok
                }
                None => {
                    set_last_error(format!(
                        "sidereon_satellite_constellation_look_angles_arc_len: index {index} out of range ({} satellites)",
                        arcs.inner.len()
                    ));
                    SidereonStatus::InvalidArgument
                }
            }
        },
    )
}

/// Copy the flattened satellite-major look-angle rows from a constellation
/// look-angle result. Rows are concatenated in fleet order; use
/// sidereon_satellite_constellation_look_angles_arc_len to recover per-satellite
/// boundaries. Uses the variable-length output contract documented at the top of
/// the header.
///
/// Safety: arcs must be a live handle; out must point to at least len writable
/// entries or be NULL when len is 0; out_written and out_required must point to
/// size_t values.
#[no_mangle]
pub unsafe extern "C" fn sidereon_satellite_constellation_look_angles_values(
    arcs: *const SidereonSatelliteConstellationLookAngles,
    out: *mut SidereonLookAngle,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_satellite_constellation_look_angles_values",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_satellite_constellation_look_angles_values",
                out_written,
                out_required
            ));
            let arcs = c_try!(require_ref(
                arcs,
                "sidereon_satellite_constellation_look_angles_values",
                "arcs"
            ));
            c_try!(copy_flattened_rows_to_c(
                "sidereon_satellite_constellation_look_angles_values",
                &arcs.inner,
                look_angle_to_c,
                out,
                len,
                out_written,
                out_required,
            ));
            SidereonStatus::Ok
        },
    )
}

/// Compute the per-satellite sub-satellite (ground-track) geodetic arcs for every
/// satellite in the constellation over a shared UTC unix-microsecond epoch grid,
/// in fleet order. A satellite that fails yields an empty track so the result
/// stays index-aligned with the constellation. On success writes a newly owned
/// handle to *out_tracks. Release it with
/// sidereon_satellite_constellation_ground_tracks_free.
///
/// Safety: constellation must be a live handle; epochs_unix_us must point to
/// epoch_count int64_t values; out_tracks must point to handle storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_satellite_constellation_ground_tracks(
    constellation: *const SidereonSatelliteConstellation,
    epochs_unix_us: *const i64,
    epoch_count: usize,
    out_tracks: *mut *mut SidereonSatelliteConstellationGroundTracks,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_satellite_constellation_ground_tracks",
        SidereonStatus::Panic,
        || {
            let out_tracks = c_try!(require_out(
                out_tracks,
                "sidereon_satellite_constellation_ground_tracks",
                "out_tracks"
            ));
            *out_tracks = ptr::null_mut();
            let constellation = c_try!(require_ref(
                constellation,
                "sidereon_satellite_constellation_ground_tracks",
                "constellation"
            ));
            let instants = c_try!(unix_instants_from_c(
                "sidereon_satellite_constellation_ground_tracks",
                epochs_unix_us,
                epoch_count,
            ));
            // Per-satellite loop over the core ground_track kernel; a failure
            // becomes an empty track, keeping the result index-aligned with the
            // fleet (the WASM/Elixir contract).
            let inner = constellation
                .satellites
                .iter()
                .map(|satellite| ground_track(satellite, &instants).unwrap_or_default())
                .collect();
            write_boxed_handle(
                out_tracks,
                SidereonSatelliteConstellationGroundTracks { inner },
            );
            SidereonStatus::Ok
        },
    )
}

/// Write the number of satellites (tracks) in a constellation ground-track result
/// to *out_count.
///
/// Safety: tracks must be a live handle; out_count must point to a size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_satellite_constellation_ground_tracks_satellite_count(
    tracks: *const SidereonSatelliteConstellationGroundTracks,
    out_count: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_satellite_constellation_ground_tracks_satellite_count",
        SidereonStatus::Panic,
        || {
            let out_count = c_try!(require_out(
                out_count,
                "sidereon_satellite_constellation_ground_tracks_satellite_count",
                "out_count"
            ));
            *out_count = 0;
            let tracks = c_try!(require_ref(
                tracks,
                "sidereon_satellite_constellation_ground_tracks_satellite_count",
                "tracks"
            ));
            *out_count = tracks.inner.len();
            SidereonStatus::Ok
        },
    )
}

/// Write the number of points in the track of the satellite at index to
/// *out_len. An out-of-range index returns InvalidArgument. Use this to split the
/// flattened sidereon_satellite_constellation_ground_tracks_values output back
/// into per-satellite tracks (which may be ragged when a satellite failed).
///
/// Safety: tracks must be a live handle; out_len must point to a size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_satellite_constellation_ground_tracks_track_len(
    tracks: *const SidereonSatelliteConstellationGroundTracks,
    index: usize,
    out_len: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_satellite_constellation_ground_tracks_track_len",
        SidereonStatus::Panic,
        || {
            let out_len = c_try!(require_out(
                out_len,
                "sidereon_satellite_constellation_ground_tracks_track_len",
                "out_len"
            ));
            *out_len = 0;
            let tracks = c_try!(require_ref(
                tracks,
                "sidereon_satellite_constellation_ground_tracks_track_len",
                "tracks"
            ));
            match tracks.inner.get(index) {
                Some(track) => {
                    *out_len = track.len();
                    SidereonStatus::Ok
                }
                None => {
                    set_last_error(format!(
                        "sidereon_satellite_constellation_ground_tracks_track_len: index {index} out of range ({} satellites)",
                        tracks.inner.len()
                    ));
                    SidereonStatus::InvalidArgument
                }
            }
        },
    )
}

/// Copy the flattened satellite-major sub-satellite geodetic points from a
/// constellation ground-track result. Rows are concatenated in fleet order; use
/// sidereon_satellite_constellation_ground_tracks_track_len to recover
/// per-satellite boundaries. Uses the variable-length output contract documented
/// at the top of the header.
///
/// Safety: tracks must be a live handle; out must point to at least len writable
/// entries or be NULL when len is 0; out_written and out_required must point to
/// size_t values.
#[no_mangle]
pub unsafe extern "C" fn sidereon_satellite_constellation_ground_tracks_values(
    tracks: *const SidereonSatelliteConstellationGroundTracks,
    out: *mut SidereonGeodetic,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_satellite_constellation_ground_tracks_values",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_satellite_constellation_ground_tracks_values",
                out_written,
                out_required
            ));
            let tracks = c_try!(require_ref(
                tracks,
                "sidereon_satellite_constellation_ground_tracks_values",
                "tracks"
            ));
            c_try!(copy_flattened_rows_to_c(
                "sidereon_satellite_constellation_ground_tracks_values",
                &tracks.inner,
                geodetic_to_c,
                out,
                len,
                out_written,
                out_required,
            ));
            SidereonStatus::Ok
        },
    )
}

/// Find dense passes over a ground station within [start_unix_us, end_unix_us)
/// for every satellite in the constellation, flattened across the fleet: each
/// SidereonFleetPass carries the fleet-order satellite_index it belongs to.
/// options may be NULL for defaults. A satellite that fails to scan contributes
/// no passes. On success writes a newly owned handle to *out_passes. Release it
/// with sidereon_satellite_constellation_passes_free.
///
/// Safety: constellation must be a live handle; station must point to a
/// SidereonGroundStation; out_passes must point to handle storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_satellite_constellation_passes(
    constellation: *const SidereonSatelliteConstellation,
    station: *const SidereonGroundStation,
    start_unix_us: i64,
    end_unix_us: i64,
    options: *const SidereonPassFinderOptions,
    out_passes: *mut *mut SidereonSatelliteConstellationPasses,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_satellite_constellation_passes",
        SidereonStatus::Panic,
        || {
            let out_passes = c_try!(require_out(
                out_passes,
                "sidereon_satellite_constellation_passes",
                "out_passes"
            ));
            *out_passes = ptr::null_mut();
            if end_unix_us <= start_unix_us {
                set_last_error(
                    "sidereon_satellite_constellation_passes: end_unix_us must be after start_unix_us",
                );
                return SidereonStatus::InvalidArgument;
            }
            let constellation = c_try!(require_ref(
                constellation,
                "sidereon_satellite_constellation_passes",
                "constellation"
            ));
            let station = c_try!(require_ref(
                station,
                "sidereon_satellite_constellation_passes",
                "station"
            ));
            let options = c_try!(pass_finder_options_from_c(
                "sidereon_satellite_constellation_passes",
                options
            ));
            let ground_station = ground_station_from_c(station);
            let start = UtcInstant::from_unix_microseconds(start_unix_us);
            let end = UtcInstant::from_unix_microseconds(end_unix_us);
            let mut inner = Vec::new();
            for (index, satellite) in constellation.satellites.iter().enumerate() {
                let passes =
                    match find_passes_for_satellite(satellite, ground_station, start, end, options)
                    {
                        Ok(passes) => passes,
                        Err(_) => continue,
                    };
                for pass in &passes {
                    inner.push(SidereonFleetPass {
                        satellite_index: index,
                        pass: satellite_pass_to_c(pass),
                    });
                }
            }
            write_boxed_handle(out_passes, SidereonSatelliteConstellationPasses { inner });
            SidereonStatus::Ok
        },
    )
}

/// Write the number of passes in a constellation pass result to *out_count.
///
/// Safety: passes must be a live handle; out_count must point to a size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_satellite_constellation_passes_count(
    passes: *const SidereonSatelliteConstellationPasses,
    out_count: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_satellite_constellation_passes_count",
        SidereonStatus::Panic,
        || {
            let out_count = c_try!(require_out(
                out_count,
                "sidereon_satellite_constellation_passes_count",
                "out_count"
            ));
            *out_count = 0;
            let passes = c_try!(require_ref(
                passes,
                "sidereon_satellite_constellation_passes_count",
                "passes"
            ));
            *out_count = passes.inner.len();
            SidereonStatus::Ok
        },
    )
}

/// Copy fleet pass rows. Uses the variable-length output contract documented at
/// the top of the header.
///
/// Safety: passes must be a live handle; out must point to at least len writable
/// entries or be NULL when len is 0; out_written and out_required must point to
/// size_t values.
#[no_mangle]
pub unsafe extern "C" fn sidereon_satellite_constellation_passes_values(
    passes: *const SidereonSatelliteConstellationPasses,
    out: *mut SidereonFleetPass,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_satellite_constellation_passes_values",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_satellite_constellation_passes_values",
                out_written,
                out_required
            ));
            let passes = c_try!(require_ref(
                passes,
                "sidereon_satellite_constellation_passes_values",
                "passes"
            ));
            c_try!(copy_prefix_to_c(
                "sidereon_satellite_constellation_passes_values",
                "out",
                &passes.inner,
                out,
                len,
                out_written,
                out_required,
            ));
            SidereonStatus::Ok
        },
    )
}

/// Release a satellite constellation handle. Null is a no-op. A non-null handle
/// must come from sidereon_satellite_constellation_build and must be freed
/// exactly once with this function. The TLE handles it was built from are
/// independent and unaffected.
///
/// Safety: constellation must be NULL or a live handle from
/// sidereon_satellite_constellation_build. Passing a handle after it has already
/// been freed is invalid.
#[no_mangle]
pub unsafe extern "C" fn sidereon_satellite_constellation_free(
    constellation: *mut SidereonSatelliteConstellation,
) {
    ffi_boundary("sidereon_satellite_constellation_free", (), || {
        free_boxed(constellation);
    });
}

/// Release a constellation look-angle result handle. Null is a no-op. A non-null
/// handle must come from sidereon_satellite_constellation_look_angle_arcs and
/// must be freed exactly once with this function.
///
/// Safety: arcs must be NULL or a live handle from
/// sidereon_satellite_constellation_look_angle_arcs. Passing a handle after it
/// has already been freed is invalid.
#[no_mangle]
pub unsafe extern "C" fn sidereon_satellite_constellation_look_angles_free(
    arcs: *mut SidereonSatelliteConstellationLookAngles,
) {
    ffi_boundary(
        "sidereon_satellite_constellation_look_angles_free",
        (),
        || {
            free_boxed(arcs);
        },
    );
}

/// Release a constellation ground-track result handle. Null is a no-op. A
/// non-null handle must come from sidereon_satellite_constellation_ground_tracks
/// and must be freed exactly once with this function.
///
/// Safety: tracks must be NULL or a live handle from
/// sidereon_satellite_constellation_ground_tracks. Passing a handle after it has
/// already been freed is invalid.
#[no_mangle]
pub unsafe extern "C" fn sidereon_satellite_constellation_ground_tracks_free(
    tracks: *mut SidereonSatelliteConstellationGroundTracks,
) {
    ffi_boundary(
        "sidereon_satellite_constellation_ground_tracks_free",
        (),
        || {
            free_boxed(tracks);
        },
    );
}

/// Release a constellation pass result handle. Null is a no-op. A non-null handle
/// must come from sidereon_satellite_constellation_passes and must be freed
/// exactly once with this function.
///
/// Safety: passes must be NULL or a live handle from
/// sidereon_satellite_constellation_passes. Passing a handle after it has already
/// been freed is invalid.
#[no_mangle]
pub unsafe extern "C" fn sidereon_satellite_constellation_passes_free(
    passes: *mut SidereonSatelliteConstellationPasses,
) {
    ffi_boundary("sidereon_satellite_constellation_passes_free", (), || {
        free_boxed(passes);
    });
}

/// A combined broadcast orbit + clock evaluation, mirroring
/// sidereon_core::ephemeris::SatelliteState.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonSatelliteState {
    /// The orbit evaluation (ECEF position and all intermediates).
    pub orbit: SidereonOrbitState,
    /// The clock offset evaluation.
    pub clock: SidereonClockOffset,
}

/// Frequency-dependent satellite antenna calibration, mirroring
/// sidereon_core::ppp_corrections::SatelliteAntennaFrequency.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonSatelliteAntennaFrequency {
    /// Null-terminated frequency label, for example "G01".
    pub label: *const c_char,
    /// Phase-center offset, meters.
    pub pco_m: [f64; 3],
    /// Pointer to noazi_count SidereonNoaziPcvSample.
    pub noazi_pcv: *const SidereonNoaziPcvSample,
    /// Number of PCV samples.
    pub noazi_count: usize,
}

/// Satellite antenna block selected by PRN and validity window, mirroring
/// sidereon_core::ppp_corrections::SatelliteAntenna.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonSatelliteAntenna {
    /// Null-terminated satellite token.
    pub sat_id: *const c_char,
    /// Whether valid_from is set.
    pub has_valid_from: bool,
    /// Start of the validity window (used when has_valid_from).
    pub valid_from: SidereonCivilDateTime,
    /// Whether valid_until is set.
    pub has_valid_until: bool,
    /// End of the validity window (used when has_valid_until).
    pub valid_until: SidereonCivilDateTime,
    /// Pointer to frequency_count SidereonSatelliteAntennaFrequency.
    pub frequencies: *const SidereonSatelliteAntennaFrequency,
    /// Number of frequency calibrations.
    pub frequency_count: usize,
}

/// Satellite antenna correction options, mirroring
/// sidereon_core::ppp_corrections::SatelliteAntennaOptions.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonSatelliteAntennaOptions {
    /// Null-terminated band-1 label.
    pub freq1_label: *const c_char,
    /// Band-1 carrier frequency, Hz.
    pub freq1_hz: f64,
    /// Null-terminated band-2 label.
    pub freq2_label: *const c_char,
    /// Band-2 carrier frequency, Hz.
    pub freq2_hz: f64,
    /// Pointer to antenna_count SidereonSatelliteAntenna.
    pub antennas: *const SidereonSatelliteAntenna,
    /// Number of antenna blocks.
    pub antenna_count: usize,
}

/// Apparent visual magnitude of a sunlit body from a diffuse-sphere phase law.
/// Delegates to sidereon_core::astro::observation::satellite_visual_magnitude.
///
/// Safety: out_magnitude points to a double.
#[no_mangle]
pub unsafe extern "C" fn sidereon_satellite_visual_magnitude(
    range_km: f64,
    phase_angle_deg: f64,
    standard_magnitude: f64,
    reference_range_km: f64,
    out_magnitude: *mut f64,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_satellite_visual_magnitude",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out_magnitude,
                "sidereon_satellite_visual_magnitude",
                "out_magnitude"
            ));
            *out = 0.0;
            match satellite_visual_magnitude(
                range_km,
                phase_angle_deg,
                standard_magnitude,
                reference_range_km,
            ) {
                Ok(value) => {
                    *out = value;
                    SidereonStatus::Ok
                }
                Err(err) => map_observation_error("sidereon_satellite_visual_magnitude", err),
            }
        },
    )
}

impl SidereonSatelliteState {
    pub(crate) fn from_core(s: &CoreSatelliteState) -> Self {
        Self {
            orbit: SidereonOrbitState::from_core(&s.orbit),
            clock: SidereonClockOffset::from_core(&s.clock),
        }
    }
}
