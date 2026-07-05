use super::*;

/// A memory-mappable precise-ephemeris interpolant artifact reader. Opaque to C.
/// Create with sidereon_precise_interpolant_artifact_open_owned or
/// sidereon_precise_interpolant_artifact_open_borrowed and release with
/// sidereon_precise_interpolant_artifact_free.
pub struct SidereonPreciseInterpolantArtifact {
    pub(crate) inner: MmapPreciseEphemerisInterpolant<'static>,
}

/// Precise-interpolant artifact open error category returned through out_error.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonPreciseInterpolantArtifactErrorKind {
    /// No artifact error occurred.
    None = 0,
    /// The byte span ended before the declared artifact length.
    Truncated = 1,
    /// A file-level or satellite-level checksum did not match.
    Corrupt = 2,
    /// Artifact bytes could not be parsed for another reason.
    Parse = 3,
    /// The artifact version is not supported.
    UnsupportedVersion = 4,
    /// The artifact time-scale tag is not supported.
    UnsupportedTimeScale = 5,
    /// A satellite-system tag is not supported.
    UnsupportedSatelliteSystem = 6,
    /// A satellite appears more than once in the index.
    DuplicateSatellite = 7,
    /// File I/O failed in the core artifact reader.
    Io = 8,
}

/// Build memory-mappable precise-interpolant artifact bytes from a loaded SP3
/// product. Output uses the variable-length contract documented in the header.
///
/// Safety: sp3 must be a live handle; out_error, out_written, and out_required
/// must point to writable storage; out must point to len bytes or be NULL when
/// len is 0.
#[no_mangle]
pub unsafe extern "C" fn sidereon_sp3_precise_interpolant_artifact_bytes(
    sp3: *const SidereonSp3,
    out_error: *mut SidereonPreciseInterpolantArtifactErrorKind,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_sp3_precise_interpolant_artifact_bytes",
        SidereonStatus::Panic,
        || {
            c_try!(init_artifact_error(
                out_error,
                SidereonPreciseInterpolantArtifactErrorKind::None
            ));
            c_try!(init_copy_counts(
                "sidereon_sp3_precise_interpolant_artifact_bytes",
                out_written,
                out_required
            ));
            let sp3 = c_try!(require_ref(
                sp3,
                "sidereon_sp3_precise_interpolant_artifact_bytes",
                "sp3"
            ));
            let bytes = match sp3.inner.precise_interpolant_store_bytes() {
                Ok(bytes) => bytes,
                Err(err) => {
                    return map_artifact_error(
                        "sidereon_sp3_precise_interpolant_artifact_bytes",
                        err,
                        out_error,
                    )
                }
            };
            c_try!(copy_prefix_to_c(
                "sidereon_sp3_precise_interpolant_artifact_bytes",
                "out",
                &bytes,
                out,
                len,
                out_written,
                out_required,
            ));
            SidereonStatus::Ok
        },
    )
}

/// Compute the artifact checksum for a byte span.
///
/// Safety: data must point to len readable bytes or be NULL when len is 0;
/// out_checksum must point to a uint64_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_precise_interpolant_artifact_checksum64(
    data: *const u8,
    len: usize,
    out_checksum: *mut u64,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_precise_interpolant_artifact_checksum64",
        SidereonStatus::Panic,
        || {
            let out_checksum = c_try!(require_out(
                out_checksum,
                "sidereon_precise_interpolant_artifact_checksum64",
                "out_checksum"
            ));
            *out_checksum = 0;
            let bytes = c_try!(require_slice(
                data,
                len,
                "sidereon_precise_interpolant_artifact_checksum64",
                "data"
            ));
            *out_checksum = precise_interpolant_store_checksum64(bytes);
            SidereonStatus::Ok
        },
    )
}

/// Open an artifact from bytes by copying them into the handle.
///
/// Safety: data must point to len readable bytes; out_error and out_artifact
/// must point to writable storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_precise_interpolant_artifact_open_owned(
    data: *const u8,
    len: usize,
    out_error: *mut SidereonPreciseInterpolantArtifactErrorKind,
    out_artifact: *mut *mut SidereonPreciseInterpolantArtifact,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_precise_interpolant_artifact_open_owned",
        SidereonStatus::Panic,
        || {
            c_try!(init_artifact_error(
                out_error,
                SidereonPreciseInterpolantArtifactErrorKind::None
            ));
            let out_artifact = c_try!(require_out(
                out_artifact,
                "sidereon_precise_interpolant_artifact_open_owned",
                "out_artifact"
            ));
            *out_artifact = ptr::null_mut();
            let bytes = c_try!(require_slice(
                data,
                len,
                "sidereon_precise_interpolant_artifact_open_owned",
                "data"
            ));
            if let Some(kind) = artifact_truncation_kind(bytes) {
                return artifact_preclassified_error(
                    "sidereon_precise_interpolant_artifact_open_owned",
                    kind,
                    out_error,
                );
            }
            let inner = match MmapPreciseEphemerisInterpolant::from_vec(bytes.to_vec()) {
                Ok(inner) => inner,
                Err(err) => {
                    return map_artifact_error(
                        "sidereon_precise_interpolant_artifact_open_owned",
                        err,
                        out_error,
                    )
                }
            };
            write_boxed_handle(out_artifact, SidereonPreciseInterpolantArtifact { inner });
            SidereonStatus::Ok
        },
    )
}

/// Open an artifact from caller-owned bytes without copying the payload arrays.
///
/// Safety: data must point to len readable bytes aligned as required by the core
/// reader. The bytes must remain alive, fixed in memory, and unmodified until
/// sidereon_precise_interpolant_artifact_free is called on the returned handle.
/// out_error and out_artifact must point to writable storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_precise_interpolant_artifact_open_borrowed(
    data: *const u8,
    len: usize,
    out_error: *mut SidereonPreciseInterpolantArtifactErrorKind,
    out_artifact: *mut *mut SidereonPreciseInterpolantArtifact,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_precise_interpolant_artifact_open_borrowed",
        SidereonStatus::Panic,
        || {
            c_try!(init_artifact_error(
                out_error,
                SidereonPreciseInterpolantArtifactErrorKind::None
            ));
            let out_artifact = c_try!(require_out(
                out_artifact,
                "sidereon_precise_interpolant_artifact_open_borrowed",
                "out_artifact"
            ));
            *out_artifact = ptr::null_mut();
            let bytes = c_try!(require_slice(
                data,
                len,
                "sidereon_precise_interpolant_artifact_open_borrowed",
                "data"
            ));
            if let Some(kind) = artifact_truncation_kind(bytes) {
                return artifact_preclassified_error(
                    "sidereon_precise_interpolant_artifact_open_borrowed",
                    kind,
                    out_error,
                );
            }
            let inner = match MmapPreciseEphemerisInterpolant::from_bytes(bytes) {
                Ok(inner) => inner,
                Err(err) => {
                    return map_artifact_error(
                        "sidereon_precise_interpolant_artifact_open_borrowed",
                        err,
                        out_error,
                    )
                }
            };
            let inner: MmapPreciseEphemerisInterpolant<'static> = std::mem::transmute(inner);
            write_boxed_handle(out_artifact, SidereonPreciseInterpolantArtifact { inner });
            SidereonStatus::Ok
        },
    )
}

/// Write the checksum of an opened artifact to *out_checksum.
///
/// Safety: artifact must be a live handle; out_checksum must point to a uint64_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_precise_interpolant_artifact_handle_checksum64(
    artifact: *const SidereonPreciseInterpolantArtifact,
    out_checksum: *mut u64,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_precise_interpolant_artifact_handle_checksum64",
        SidereonStatus::Panic,
        || {
            let out_checksum = c_try!(require_out(
                out_checksum,
                "sidereon_precise_interpolant_artifact_handle_checksum64",
                "out_checksum"
            ));
            *out_checksum = 0;
            let artifact = c_try!(require_ref(
                artifact,
                "sidereon_precise_interpolant_artifact_handle_checksum64",
                "artifact"
            ));
            *out_checksum = artifact.inner.checksum64();
            SidereonStatus::Ok
        },
    )
}

/// Copy satellites present in an opened artifact. Output uses the
/// variable-length contract documented in the header.
///
/// Safety: artifact must be a live handle; out must point to len tokens or be
/// NULL when len is 0; out_written and out_required must point to size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_precise_interpolant_artifact_satellites(
    artifact: *const SidereonPreciseInterpolantArtifact,
    out: *mut SidereonSatelliteToken,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_precise_interpolant_artifact_satellites",
        SidereonStatus::Panic,
        || {
            let artifact = c_try!(require_ref(
                artifact,
                "sidereon_precise_interpolant_artifact_satellites",
                "artifact"
            ));
            let values: Vec<SidereonSatelliteToken> = artifact
                .inner
                .satellites()
                .iter()
                .copied()
                .map(satellite_token)
                .collect();
            c_try!(copy_prefix_to_c(
                "sidereon_precise_interpolant_artifact_satellites",
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

/// Evaluate one satellite state from an opened artifact at seconds since J2000.
///
/// Safety: artifact must be a live handle; sat_id must be a null-terminated
/// satellite token; out_state must point to a SidereonSp3State.
#[no_mangle]
pub unsafe extern "C" fn sidereon_precise_interpolant_artifact_state(
    artifact: *const SidereonPreciseInterpolantArtifact,
    sat_id: *const c_char,
    epoch_j2000_s: f64,
    out_state: *mut SidereonSp3State,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_precise_interpolant_artifact_state",
        SidereonStatus::Panic,
        || {
            let out_state = c_try!(require_out(
                out_state,
                "sidereon_precise_interpolant_artifact_state",
                "out_state"
            ));
            *out_state = empty_artifact_sp3_state();
            let artifact = c_try!(require_ref(
                artifact,
                "sidereon_precise_interpolant_artifact_state",
                "artifact"
            ));
            let sat = c_try!(parse_satellite_token(
                "sidereon_precise_interpolant_artifact_state",
                sat_id
            ));
            let state = c_try!(guard_core(
                || artifact.inner.position_at_j2000_seconds(sat, epoch_j2000_s),
                |err| map_artifact_eval_error("sidereon_precise_interpolant_artifact_state", err),
            ));
            *out_state = artifact_sp3_state_to_c(&state);
            SidereonStatus::Ok
        },
    )
}

/// Release a precise-interpolant artifact handle. Passing NULL is a no-op.
///
/// Safety: artifact must be NULL or a live handle from an artifact open function
/// that has not already been freed.
#[no_mangle]
pub unsafe extern "C" fn sidereon_precise_interpolant_artifact_free(
    artifact: *mut SidereonPreciseInterpolantArtifact,
) {
    ffi_boundary("sidereon_precise_interpolant_artifact_free", (), || {
        free_boxed(artifact);
    });
}

unsafe fn init_artifact_error(
    out_error: *mut SidereonPreciseInterpolantArtifactErrorKind,
    value: SidereonPreciseInterpolantArtifactErrorKind,
) -> Result<(), SidereonStatus> {
    let out_error = require_out(
        out_error,
        "sidereon_precise_interpolant_artifact",
        "out_error",
    )?;
    *out_error = value;
    Ok(())
}

fn artifact_truncation_kind(bytes: &[u8]) -> Option<SidereonPreciseInterpolantArtifactErrorKind> {
    const HEADER_LEN: usize = 64;
    const TOTAL_LEN_OFFSET: usize = 32;
    if bytes.len() < HEADER_LEN {
        return Some(SidereonPreciseInterpolantArtifactErrorKind::Truncated);
    }
    let mut total_len_bytes = [0u8; 8];
    total_len_bytes.copy_from_slice(&bytes[TOTAL_LEN_OFFSET..TOTAL_LEN_OFFSET + 8]);
    let total_len = u64::from_le_bytes(total_len_bytes) as usize;
    if total_len > bytes.len() {
        Some(SidereonPreciseInterpolantArtifactErrorKind::Truncated)
    } else {
        None
    }
}

unsafe fn artifact_preclassified_error(
    fn_name: &str,
    kind: SidereonPreciseInterpolantArtifactErrorKind,
    out_error: *mut SidereonPreciseInterpolantArtifactErrorKind,
) -> SidereonStatus {
    let _ = init_artifact_error(out_error, kind);
    set_last_error(format!("{fn_name}: precise interpolant artifact {kind:?}"));
    SidereonStatus::InvalidArgument
}

fn artifact_error_kind(
    err: &PreciseInterpolantStoreError,
) -> SidereonPreciseInterpolantArtifactErrorKind {
    match err {
        PreciseInterpolantStoreError::Io { .. } => SidereonPreciseInterpolantArtifactErrorKind::Io,
        PreciseInterpolantStoreError::Parse { reason } => {
            if reason.contains("extends past")
                || reason.contains("out of bounds")
                || reason.contains("needs at least")
                || reason.contains("total length")
            {
                SidereonPreciseInterpolantArtifactErrorKind::Truncated
            } else {
                SidereonPreciseInterpolantArtifactErrorKind::Parse
            }
        }
        PreciseInterpolantStoreError::UnsupportedVersion { .. } => {
            SidereonPreciseInterpolantArtifactErrorKind::UnsupportedVersion
        }
        PreciseInterpolantStoreError::UnsupportedTimeScale { .. } => {
            SidereonPreciseInterpolantArtifactErrorKind::UnsupportedTimeScale
        }
        PreciseInterpolantStoreError::UnsupportedSatelliteSystem { .. } => {
            SidereonPreciseInterpolantArtifactErrorKind::UnsupportedSatelliteSystem
        }
        PreciseInterpolantStoreError::DuplicateSatellite { .. } => {
            SidereonPreciseInterpolantArtifactErrorKind::DuplicateSatellite
        }
        PreciseInterpolantStoreError::Checksum { .. }
        | PreciseInterpolantStoreError::SatelliteChecksum { .. } => {
            SidereonPreciseInterpolantArtifactErrorKind::Corrupt
        }
    }
}

unsafe fn map_artifact_error(
    fn_name: &str,
    err: PreciseInterpolantStoreError,
    out_error: *mut SidereonPreciseInterpolantArtifactErrorKind,
) -> SidereonStatus {
    let kind = artifact_error_kind(&err);
    let _ = init_artifact_error(out_error, kind);
    set_last_error(format!("{fn_name}: {err}"));
    match kind {
        SidereonPreciseInterpolantArtifactErrorKind::Io => SidereonStatus::Solve,
        _ => SidereonStatus::InvalidArgument,
    }
}

fn map_artifact_eval_error(fn_name: &str, err: CoreError) -> SidereonStatus {
    set_last_error(format!("{fn_name}: {err}"));
    match err {
        CoreError::UnknownSatellite(_) | CoreError::InvalidInput(_) => {
            SidereonStatus::InvalidArgument
        }
        CoreError::EpochOutOfRange => SidereonStatus::Solve,
        _ => SidereonStatus::Solve,
    }
}

fn empty_artifact_sp3_state() -> SidereonSp3State {
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

fn artifact_sp3_state_to_c(state: &Sp3State) -> SidereonSp3State {
    SidereonSp3State {
        position_m: state.position.as_array(),
        has_clock_s: state.clock_s.is_some(),
        clock_s: state.clock_s.unwrap_or(0.0),
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;

    fn fixture_sp3() -> Sp3 {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/sp3/COD0MGXFIN_20201770000_01D_05M_ORB.SP3");
        let bytes = fs::read(path).expect("read SP3 fixture");
        Sp3::parse(&bytes).expect("parse SP3")
    }

    #[test]
    fn precise_artifact_bytes_and_state_match_core_reader() {
        let sp3 = fixture_sp3();
        let epoch = sp3.epochs_j2000_seconds()[10];
        let sat_id = sp3.satellites()[0];
        let expected = sp3
            .precise_interpolant_store_bytes()
            .expect("core artifact bytes");
        let sp3_handle = SidereonSp3 { inner: sp3 };
        let mut error = SidereonPreciseInterpolantArtifactErrorKind::None;
        let mut written = 0usize;
        let mut required = 0usize;
        let status = unsafe {
            sidereon_sp3_precise_interpolant_artifact_bytes(
                &sp3_handle,
                &mut error,
                ptr::null_mut(),
                0,
                &mut written,
                &mut required,
            )
        };
        assert_eq!(status, SidereonStatus::Ok);
        assert_eq!(required, expected.len());

        let mut bytes = vec![0u8; required];
        let status = unsafe {
            sidereon_sp3_precise_interpolant_artifact_bytes(
                &sp3_handle,
                &mut error,
                bytes.as_mut_ptr(),
                bytes.len(),
                &mut written,
                &mut required,
            )
        };
        assert_eq!(status, SidereonStatus::Ok);
        assert_eq!(written, expected.len());
        assert_eq!(bytes, expected);

        let mut checksum = 0u64;
        let status = unsafe {
            sidereon_precise_interpolant_artifact_checksum64(
                bytes.as_ptr(),
                bytes.len(),
                &mut checksum,
            )
        };
        assert_eq!(status, SidereonStatus::Ok);
        assert_eq!(checksum, precise_interpolant_store_checksum64(&bytes));

        let mut artifact = ptr::null_mut();
        let status = unsafe {
            sidereon_precise_interpolant_artifact_open_owned(
                bytes.as_ptr(),
                bytes.len(),
                &mut error,
                &mut artifact,
            )
        };
        assert_eq!(status, SidereonStatus::Ok);
        assert_eq!(error, SidereonPreciseInterpolantArtifactErrorKind::None);

        let core_reader =
            MmapPreciseEphemerisInterpolant::from_vec(bytes.clone()).expect("core reader");
        let sat = std::ffi::CString::new(sat_id.to_string()).expect("sat token");
        let mut state = empty_artifact_sp3_state();
        let status = unsafe {
            sidereon_precise_interpolant_artifact_state(artifact, sat.as_ptr(), epoch, &mut state)
        };
        assert_eq!(status, SidereonStatus::Ok);
        let expected_state = core_reader
            .position_at_j2000_seconds(sat_id, epoch)
            .expect("core state");
        assert_eq!(
            state.position_m.map(f64::to_bits),
            expected_state.position.as_array().map(f64::to_bits)
        );
        assert_eq!(
            state.clock_s.to_bits(),
            expected_state.clock_s.unwrap_or(0.0).to_bits()
        );

        unsafe { sidereon_precise_interpolant_artifact_free(artifact) };

        let mut borrowed = ptr::null_mut();
        let status = unsafe {
            sidereon_precise_interpolant_artifact_open_borrowed(
                bytes.as_ptr(),
                bytes.len(),
                &mut error,
                &mut borrowed,
            )
        };
        assert_eq!(status, SidereonStatus::Ok);
        assert_eq!(error, SidereonPreciseInterpolantArtifactErrorKind::None);

        let mut borrowed_checksum = 0u64;
        let status = unsafe {
            sidereon_precise_interpolant_artifact_handle_checksum64(
                borrowed,
                &mut borrowed_checksum,
            )
        };
        assert_eq!(status, SidereonStatus::Ok);
        assert_eq!(borrowed_checksum, checksum);

        let mut borrowed_state = empty_artifact_sp3_state();
        let status = unsafe {
            sidereon_precise_interpolant_artifact_state(
                borrowed,
                sat.as_ptr(),
                epoch,
                &mut borrowed_state,
            )
        };
        assert_eq!(status, SidereonStatus::Ok);
        assert_eq!(
            borrowed_state.position_m.map(f64::to_bits),
            expected_state.position.as_array().map(f64::to_bits)
        );
        unsafe { sidereon_precise_interpolant_artifact_free(borrowed) };

        let mut truncated = ptr::null_mut();
        let status = unsafe {
            sidereon_precise_interpolant_artifact_open_owned(
                bytes.as_ptr(),
                bytes.len() - 1,
                &mut error,
                &mut truncated,
            )
        };
        assert_eq!(status, SidereonStatus::InvalidArgument);
        assert_eq!(
            error,
            SidereonPreciseInterpolantArtifactErrorKind::Truncated
        );
    }
}
