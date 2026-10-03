use super::*;

/// A memory-mappable precise-ephemeris interpolant artifact reader. Opaque to C.
/// Create with sidereon_precise_interpolant_artifact_open_owned or
/// sidereon_precise_interpolant_artifact_open_borrowed, or open a file with
/// sidereon_precise_interpolant_artifact_from_path. A trusted caller can defer
/// payload hashing with sidereon_precise_interpolant_artifact_from_path_attested.
/// Release the handle with sidereon_precise_interpolant_artifact_free.
pub struct SidereonPreciseInterpolantArtifact {
    pub(crate) inner: MmapPreciseEphemerisInterpolant<'static>,
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_precise_artifact_source_state_at_epoch_queries(
    artifact: *const SidereonPreciseInterpolantArtifact,
    sat_id: *const c_char,
    state_epoch: *const SidereonExactEpochQuery,
    selection_epoch: *const SidereonExactEpochQuery,
    out: *mut SidereonEphemerisSourceState,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_precise_artifact_source_state_at_epoch_queries";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        let out = c_try!(require_out(out, FN_NAME, "out"));
        *out = SidereonEphemerisSourceState::default();
        record_degrade_reason(None);
        let artifact = c_try!(require_ref(artifact, FN_NAME, "artifact"));
        let satellite = c_try!(parse_satellite_token(FN_NAME, sat_id));
        let state_epoch = c_try!(require_ref(state_epoch, FN_NAME, "state_epoch"));
        let selection_epoch = c_try!(require_ref(selection_epoch, FN_NAME, "selection_epoch"));
        let state = c_try!(guard_core(
            || {
                sidereon_core::positioning::EphemerisSource::try_position_clock_group_delay_selected_at_epoch_query(
                &artifact.inner, satellite, &state_epoch.inner, &selection_epoch.inner,
            )
            },
            |error| crate::precise::precise_source_error_to_status(FN_NAME, error),
        ));
        if let Some(state) = state {
            record_degrade_reason(state.degraded);
            *out = SidereonEphemerisSourceState {
                has_state: true,
                position_ecef_m: state.value.0,
                clock_s: state.value.1,
                has_group_delay: state.value.2.is_some(),
                group_delay_s: state.value.2.unwrap_or_default(),
                degraded: state.degraded.is_some(),
            };
        }
        SidereonStatus::Ok
    })
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_precise_artifact_source_transmit_epoch_clock_at_epoch_queries(
    artifact: *const SidereonPreciseInterpolantArtifact,
    sat_id: *const c_char,
    transmit_epoch: *const SidereonExactEpochQuery,
    selection_epoch: *const SidereonExactEpochQuery,
    out_has_clock: *mut bool,
    out_clock_s: *mut f64,
    out_degraded: *mut bool,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_precise_artifact_source_transmit_epoch_clock_at_epoch_queries";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        let out_has_clock = c_try!(require_out(out_has_clock, FN_NAME, "out_has_clock"));
        let out_clock_s = c_try!(require_out(out_clock_s, FN_NAME, "out_clock_s"));
        let out_degraded = c_try!(require_out(out_degraded, FN_NAME, "out_degraded"));
        *out_has_clock = false;
        *out_clock_s = 0.0;
        *out_degraded = false;
        record_degrade_reason(None);
        let artifact = c_try!(require_ref(artifact, FN_NAME, "artifact"));
        let satellite = c_try!(parse_satellite_token(FN_NAME, sat_id));
        let transmit_epoch = c_try!(require_ref(transmit_epoch, FN_NAME, "transmit_epoch"));
        let selection_epoch = c_try!(require_ref(selection_epoch, FN_NAME, "selection_epoch"));
        let clock = c_try!(guard_core(
            || sidereon_core::positioning::EphemerisSource::try_transmit_epoch_clock_at_epoch_query(
                &artifact.inner,
                satellite,
                &transmit_epoch.inner,
                &selection_epoch.inner,
            ),
            |error| crate::precise::precise_source_error_to_status(FN_NAME, error),
        ));
        if let Some(clock) = clock {
            *out_has_clock = true;
            *out_clock_s = clock.value;
            record_degrade_reason(clock.degraded);
            *out_degraded = clock.degraded.is_some();
        }
        SidereonStatus::Ok
    })
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_precise_artifact_source_clock_relativity_at_epoch_query(
    artifact: *const SidereonPreciseInterpolantArtifact,
    sat_id: *const c_char,
    epoch: *const SidereonExactEpochQuery,
    position_ecef_m: *const f64,
    out_kind: *mut SidereonClockRelativityKind,
    out_term_s: *mut f64,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_precise_artifact_source_clock_relativity_at_epoch_query";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        let out_kind = c_try!(require_out(out_kind, FN_NAME, "out_kind"));
        let out_term_s = c_try!(require_out(out_term_s, FN_NAME, "out_term_s"));
        *out_kind = SidereonClockRelativityKind::NotApplicable;
        *out_term_s = 0.0;
        let artifact = c_try!(require_ref(artifact, FN_NAME, "artifact"));
        let satellite = c_try!(parse_satellite_token(FN_NAME, sat_id));
        let epoch = c_try!(require_ref(epoch, FN_NAME, "epoch"));
        let position = c_try!(require_slice(
            position_ecef_m,
            3,
            FN_NAME,
            "position_ecef_m"
        ));
        let position = [position[0], position[1], position[2]];
        match sidereon_core::positioning::EphemerisSource::clock_relativity_for_state_at_epoch_query(
            &artifact.inner,
            satellite,
            &epoch.inner,
            position,
        ) {
            sidereon_core::positioning::ClockRelativity::NotApplicable => {}
            sidereon_core::positioning::ClockRelativity::Term(term) => {
                *out_kind = SidereonClockRelativityKind::Term;
                *out_term_s = term;
            }
            sidereon_core::positioning::ClockRelativity::Unavailable => {
                *out_kind = SidereonClockRelativityKind::Unavailable;
            }
        }
        SidereonStatus::Ok
    })
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_precise_artifact_source_ephemeris_variance_at_epoch_queries(
    artifact: *const SidereonPreciseInterpolantArtifact,
    sat_id: *const c_char,
    state_epoch: *const SidereonExactEpochQuery,
    selection_epoch: *const SidereonExactEpochQuery,
    out_variance_m2: *mut f64,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_precise_artifact_source_ephemeris_variance_at_epoch_queries";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        let out_variance_m2 = c_try!(require_out(out_variance_m2, FN_NAME, "out_variance_m2"));
        *out_variance_m2 = 0.0;
        let artifact = c_try!(require_ref(artifact, FN_NAME, "artifact"));
        let satellite = c_try!(parse_satellite_token(FN_NAME, sat_id));
        let state_epoch = c_try!(require_ref(state_epoch, FN_NAME, "state_epoch"));
        let selection_epoch = c_try!(require_ref(selection_epoch, FN_NAME, "selection_epoch"));
        *out_variance_m2 =
            sidereon_core::positioning::EphemerisSource::ephemeris_variance_at_epoch_query(
                &artifact.inner,
                satellite,
                &state_epoch.inner,
                &selection_epoch.inner,
            );
        SidereonStatus::Ok
    })
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
    /// A caller-attested checksum differed from the checksum in the header.
    AttestedChecksumMismatch = 9,
    /// A satellite payload checksum did not match its index record.
    SatelliteChecksum = 10,
    /// The first eight bytes do not contain the precise-store magic.
    BadMagic = 11,
    /// The byte span ends before the fixed artifact header is complete.
    HeaderTruncated = 12,
    /// The byte span contains bytes after the declared artifact length.
    TrailingBytes = 13,
    /// An index region extends beyond the declared artifact length.
    RangeOutOfBounds = 14,
    /// A refusal added by a later engine version; its detail remains in the
    /// thread-local error message.
    Unknown = 999,
}

/// Which text field of the last precise-interpolant artifact error to copy.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonPreciseInterpolantArtifactErrorText {
    /// File path associated with an I/O error.
    Path = 0,
    /// I/O or future-variant diagnostic.
    Message = 1,
    /// Parse reason.
    Reason = 2,
    /// Region name for RangeOutOfBounds.
    Region = 3,
}

/// Lossless typed details for the latest precise-interpolant artifact failure.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonPreciseInterpolantArtifactError {
    /// Error selector as SidereonPreciseInterpolantArtifactErrorKind.
    pub kind: u32,
    /// Unsupported artifact version when applicable.
    pub version: u16,
    /// Unsupported time-scale or satellite-system tag when applicable.
    pub tag: u8,
    /// Bytes found by BadMagic.
    pub found_magic: [u8; 8],
    /// Available bytes for truncated/trailing/range errors.
    pub available: u64,
    /// Header-declared length for truncated/trailing errors.
    pub declared: u64,
    /// Offset and length for RangeOutOfBounds.
    pub offset: u64,
    /// Offset and length for RangeOutOfBounds.
    pub len: u64,
    /// Expected checksum for Checksum and SatelliteChecksum.
    pub expected_checksum64: u64,
    /// Computed checksum for Checksum and SatelliteChecksum.
    pub found_checksum64: u64,
    /// Claimed checksum for AttestedChecksumMismatch.
    pub claimed_checksum64: u64,
    /// Header-declared checksum for AttestedChecksumMismatch.
    pub declared_checksum64: u64,
    /// Whether satellite identifies the affected satellite.
    pub has_satellite: bool,
    /// Satellite for DuplicateSatellite, SatelliteChecksum or a bounded region.
    pub satellite: SidereonSatelliteToken,
}

thread_local! {
    static LAST_PRECISE_ARTIFACT_ERROR: RefCell<Option<SidereonPreciseInterpolantArtifactError>> =
        const { RefCell::new(None) };
    static LAST_PRECISE_ARTIFACT_ERROR_TEXTS: RefCell<Vec<(u32, String)>> =
        const { RefCell::new(Vec::new()) };
}

fn no_precise_artifact_error() -> SidereonPreciseInterpolantArtifactError {
    SidereonPreciseInterpolantArtifactError {
        kind: SidereonPreciseInterpolantArtifactErrorKind::None as u32,
        version: 0,
        tag: 0,
        found_magic: [0; 8],
        available: 0,
        declared: 0,
        offset: 0,
        len: 0,
        expected_checksum64: 0,
        found_checksum64: 0,
        claimed_checksum64: 0,
        declared_checksum64: 0,
        has_satellite: false,
        satellite: satellite_token_from_text(""),
    }
}

fn reset_precise_artifact_error() {
    LAST_PRECISE_ARTIFACT_ERROR.with(|slot| *slot.borrow_mut() = None);
    LAST_PRECISE_ARTIFACT_ERROR_TEXTS.with(|slot| slot.borrow_mut().clear());
}

/// Copy the retained typed details of the latest artifact producer failure.
///
/// Safety: out_error must point to a SidereonPreciseInterpolantArtifactError.
#[no_mangle]
pub unsafe extern "C" fn sidereon_last_precise_interpolant_artifact_error(
    out_error: *mut SidereonPreciseInterpolantArtifactError,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_last_precise_interpolant_artifact_error",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out_error,
                "sidereon_last_precise_interpolant_artifact_error",
                "out_error"
            ));
            *out = LAST_PRECISE_ARTIFACT_ERROR
                .with(|slot| *slot.borrow())
                .unwrap_or_else(no_precise_artifact_error);
            SidereonStatus::Ok
        },
    )
}

/// Copy one retained text field of the latest artifact producer failure.
/// Use a NULL output and zero length to learn the required size, then call
/// again with a buffer of that size. Reads do not clear the retained error.
///
/// Safety: out points to len writable bytes or is NULL when len is zero;
/// out_written and out_required point to writable size_t values.
#[no_mangle]
pub unsafe extern "C" fn sidereon_last_precise_interpolant_artifact_error_text(
    part: u32,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_last_precise_interpolant_artifact_error_text";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        c_try!(init_copy_counts(FN_NAME, out_written, out_required));
        if part > SidereonPreciseInterpolantArtifactErrorText::Region as u32 {
            set_last_error(format!("{FN_NAME}: unknown text part {part}"));
            return SidereonStatus::InvalidArgument;
        }
        let text = LAST_PRECISE_ARTIFACT_ERROR_TEXTS.with(|slot| {
            slot.borrow()
                .iter()
                .find(|(kind, _)| *kind == part)
                .map(|(_, text)| text.clone())
                .unwrap_or_default()
        });
        c_try!(copy_prefix_to_c(
            FN_NAME,
            "out",
            text.as_bytes(),
            out,
            len,
            out_written,
            out_required,
        ));
        SidereonStatus::Ok
    })
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
            reset_precise_artifact_error();
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
                    );
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

/// Open a memory-mappable precise-interpolant artifact from a filesystem path.
/// File-level and per-satellite payload checksums are verified before the
/// handle is returned.
///
/// Safety: path must be a non-empty UTF-8 C string; out_error and out_artifact
/// must point to writable storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_precise_interpolant_artifact_from_path(
    path: *const c_char,
    out_error: *mut SidereonPreciseInterpolantArtifactErrorKind,
    out_artifact: *mut *mut SidereonPreciseInterpolantArtifact,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_precise_interpolant_artifact_from_path",
        SidereonStatus::Panic,
        || {
            reset_precise_artifact_error();
            c_try!(init_artifact_error(
                out_error,
                SidereonPreciseInterpolantArtifactErrorKind::None
            ));
            let out_artifact = c_try!(require_out(
                out_artifact,
                "sidereon_precise_interpolant_artifact_from_path",
                "out_artifact"
            ));
            *out_artifact = ptr::null_mut();
            let path = c_try!(parse_c_string(
                "sidereon_precise_interpolant_artifact_from_path",
                "path",
                path
            ));
            let inner =
                match MmapPreciseEphemerisInterpolant::from_path(std::path::Path::new(&path)) {
                    Ok(inner) => inner,
                    Err(err) => {
                        return map_artifact_error(
                            "sidereon_precise_interpolant_artifact_from_path",
                            err,
                            out_error,
                        );
                    }
                };
            write_boxed_handle(out_artifact, SidereonPreciseInterpolantArtifact { inner });
            SidereonStatus::Ok
        },
    )
}

/// Open a memory-mappable precise-interpolant artifact using a
/// caller-attested checksum. Header, index, dimension, length, and payload
/// layout validation still run, but checksum hashing is deferred until
/// sidereon_precise_interpolant_artifact_verify. The claim must equal the
/// checksum declared by the artifact header; a mismatch fails closed without
/// hashing the payload.
///
/// Safety: path must be a non-empty UTF-8 C string; out_error and out_artifact
/// must point to writable storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_precise_interpolant_artifact_from_path_attested(
    path: *const c_char,
    claimed_checksum64: u64,
    out_error: *mut SidereonPreciseInterpolantArtifactErrorKind,
    out_artifact: *mut *mut SidereonPreciseInterpolantArtifact,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_precise_interpolant_artifact_from_path_attested",
        SidereonStatus::Panic,
        || {
            reset_precise_artifact_error();
            c_try!(init_artifact_error(
                out_error,
                SidereonPreciseInterpolantArtifactErrorKind::None
            ));
            let out_artifact = c_try!(require_out(
                out_artifact,
                "sidereon_precise_interpolant_artifact_from_path_attested",
                "out_artifact"
            ));
            *out_artifact = ptr::null_mut();
            let path = c_try!(parse_c_string(
                "sidereon_precise_interpolant_artifact_from_path_attested",
                "path",
                path
            ));
            let inner = match MmapPreciseEphemerisInterpolant::from_path_attested(
                std::path::Path::new(&path),
                claimed_checksum64,
            ) {
                Ok(inner) => inner,
                Err(err) => {
                    return map_artifact_error(
                        "sidereon_precise_interpolant_artifact_from_path_attested",
                        err,
                        out_error,
                    );
                }
            };
            write_boxed_handle(out_artifact, SidereonPreciseInterpolantArtifact { inner });
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
            reset_precise_artifact_error();
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
            let inner = match MmapPreciseEphemerisInterpolant::from_vec(bytes.to_vec()) {
                Ok(inner) => inner,
                Err(err) => {
                    return map_artifact_error(
                        "sidereon_precise_interpolant_artifact_open_owned",
                        err,
                        out_error,
                    );
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
            reset_precise_artifact_error();
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
            let inner = match MmapPreciseEphemerisInterpolant::from_bytes(bytes) {
                Ok(inner) => inner,
                Err(err) => {
                    return map_artifact_error(
                        "sidereon_precise_interpolant_artifact_open_borrowed",
                        err,
                        out_error,
                    );
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

/// Return whether the checksum carried by this artifact handle is verified or
/// caller-attested.
///
/// Safety: artifact must be a live handle; out_provenance must point to a
/// SidereonDigestProvenance.
#[no_mangle]
pub unsafe extern "C" fn sidereon_precise_interpolant_artifact_digest_provenance(
    artifact: *const SidereonPreciseInterpolantArtifact,
    out_provenance: *mut SidereonDigestProvenance,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_precise_interpolant_artifact_digest_provenance",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out_provenance,
                "sidereon_precise_interpolant_artifact_digest_provenance",
                "out_provenance"
            ));
            *out = SidereonDigestProvenance::Verified;
            let artifact = c_try!(require_ref(
                artifact,
                "sidereon_precise_interpolant_artifact_digest_provenance",
                "artifact"
            ));
            *out = digest_provenance_to_c(artifact.inner.digest_provenance());
            SidereonStatus::Ok
        },
    )
}

/// Hash and verify file-level and per-satellite payload checksums. Success
/// changes digest provenance to SIDEREON_DIGEST_PROVENANCE_VERIFIED.
///
/// Safety: artifact must be a live mutable handle; out_error must point to
/// writable storage.
#[no_mangle]
pub unsafe extern "C" fn sidereon_precise_interpolant_artifact_verify(
    artifact: *mut SidereonPreciseInterpolantArtifact,
    out_error: *mut SidereonPreciseInterpolantArtifactErrorKind,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_precise_interpolant_artifact_verify",
        SidereonStatus::Panic,
        || {
            reset_precise_artifact_error();
            c_try!(init_artifact_error(
                out_error,
                SidereonPreciseInterpolantArtifactErrorKind::None
            ));
            let artifact = c_try!(require_mut(
                artifact,
                "sidereon_precise_interpolant_artifact_verify",
                "artifact"
            ));
            match artifact.inner.verify() {
                Ok(()) => SidereonStatus::Ok,
                Err(err) => map_artifact_error(
                    "sidereon_precise_interpolant_artifact_verify",
                    err,
                    out_error,
                ),
            }
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

#[no_mangle]
pub unsafe extern "C" fn sidereon_precise_interpolant_artifact_state_at_epoch_query(
    artifact: *const SidereonPreciseInterpolantArtifact,
    sat_id: *const c_char,
    query: *const SidereonExactEpochQuery,
    out_state: *mut SidereonSp3State,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_precise_interpolant_artifact_state_at_epoch_query";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        let out_state = c_try!(require_out(out_state, FN_NAME, "out_state"));
        *out_state = empty_artifact_sp3_state();
        let artifact = c_try!(require_ref(artifact, FN_NAME, "artifact"));
        let sat = c_try!(parse_satellite_token(FN_NAME, sat_id));
        let query = c_try!(require_ref(query, FN_NAME, "query"));
        let state = c_try!(guard_core(
            || artifact.inner.position_at_epoch_query(sat, &query.inner),
            |error| map_artifact_eval_error(FN_NAME, error),
        ));
        *out_state = artifact_sp3_state_to_c(&state);
        SidereonStatus::Ok
    })
}

/// Write the SP3 interpolation gap threshold factor recorded in this
/// artifact's header to *out_gap_threshold_factor.
///
/// Safety: artifact must be a live handle; out_gap_threshold_factor must point
/// to a double.
#[no_mangle]
pub unsafe extern "C" fn sidereon_precise_interpolant_artifact_gap_threshold_factor(
    artifact: *const SidereonPreciseInterpolantArtifact,
    out_gap_threshold_factor: *mut f64,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_precise_interpolant_artifact_gap_threshold_factor";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        let out_gap_threshold_factor = c_try!(require_out(
            out_gap_threshold_factor,
            FN_NAME,
            "out_gap_threshold_factor"
        ));
        *out_gap_threshold_factor = 0.0;
        let artifact = c_try!(require_ref(artifact, FN_NAME, "artifact"));
        *out_gap_threshold_factor = artifact
            .inner
            .interpolation_options()
            .gap_threshold_factor();
        SidereonStatus::Ok
    })
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

fn artifact_error_to_c(
    err: &PreciseInterpolantStoreError,
) -> (SidereonPreciseInterpolantArtifactError, Vec<(u32, String)>) {
    use SidereonPreciseInterpolantArtifactErrorKind as Kind;
    let mut out = no_precise_artifact_error();
    let mut texts = Vec::new();
    match err {
        PreciseInterpolantStoreError::Io { path, message } => {
            out.kind = Kind::Io as u32;
            texts.push((
                SidereonPreciseInterpolantArtifactErrorText::Path as u32,
                path.display().to_string(),
            ));
            texts.push((
                SidereonPreciseInterpolantArtifactErrorText::Message as u32,
                message.clone(),
            ));
        }
        PreciseInterpolantStoreError::Parse { reason } => {
            out.kind = Kind::Parse as u32;
            texts.push((
                SidereonPreciseInterpolantArtifactErrorText::Reason as u32,
                reason.clone(),
            ));
        }
        PreciseInterpolantStoreError::BadMagic { found } => {
            out.kind = Kind::BadMagic as u32;
            out.found_magic = *found;
        }
        PreciseInterpolantStoreError::HeaderTruncated { available } => {
            out.kind = Kind::HeaderTruncated as u32;
            out.available = *available;
        }
        PreciseInterpolantStoreError::Truncated {
            declared,
            available,
        } => {
            out.kind = Kind::Truncated as u32;
            out.declared = *declared;
            out.available = *available;
        }
        PreciseInterpolantStoreError::TrailingBytes {
            declared,
            available,
        } => {
            out.kind = Kind::TrailingBytes as u32;
            out.declared = *declared;
            out.available = *available;
        }
        PreciseInterpolantStoreError::RangeOutOfBounds {
            region,
            sat,
            offset,
            len,
            available,
        } => {
            out.kind = Kind::RangeOutOfBounds as u32;
            out.offset = *offset;
            out.len = *len;
            out.available = *available;
            texts.push((
                SidereonPreciseInterpolantArtifactErrorText::Region as u32,
                (*region).to_owned(),
            ));
            if let Some(sat) = sat {
                out.has_satellite = true;
                out.satellite = satellite_token(*sat);
            }
        }
        PreciseInterpolantStoreError::UnsupportedVersion { version } => {
            out.kind = Kind::UnsupportedVersion as u32;
            out.version = *version;
        }
        PreciseInterpolantStoreError::UnsupportedTimeScale { tag } => {
            out.kind = Kind::UnsupportedTimeScale as u32;
            out.tag = *tag;
        }
        PreciseInterpolantStoreError::UnsupportedSatelliteSystem { tag } => {
            out.kind = Kind::UnsupportedSatelliteSystem as u32;
            out.tag = *tag;
        }
        PreciseInterpolantStoreError::DuplicateSatellite { sat } => {
            out.kind = Kind::DuplicateSatellite as u32;
            out.has_satellite = true;
            out.satellite = satellite_token(*sat);
        }
        PreciseInterpolantStoreError::Checksum { expected, found } => {
            out.kind = Kind::Corrupt as u32;
            out.expected_checksum64 = *expected;
            out.found_checksum64 = *found;
        }
        PreciseInterpolantStoreError::SatelliteChecksum {
            sat,
            expected,
            found,
        } => {
            out.kind = Kind::SatelliteChecksum as u32;
            out.expected_checksum64 = *expected;
            out.found_checksum64 = *found;
            out.has_satellite = true;
            out.satellite = satellite_token(*sat);
        }
        PreciseInterpolantStoreError::AttestedChecksumMismatch { claimed, declared } => {
            out.kind = Kind::AttestedChecksumMismatch as u32;
            out.claimed_checksum64 = *claimed;
            out.declared_checksum64 = *declared;
        }
        other => {
            out.kind = Kind::Unknown as u32;
            texts.push((
                SidereonPreciseInterpolantArtifactErrorText::Message as u32,
                format!("{other} ({other:?})"),
            ));
        }
    }
    (out, texts)
}

fn artifact_error_kind(
    err: &PreciseInterpolantStoreError,
) -> SidereonPreciseInterpolantArtifactErrorKind {
    use SidereonPreciseInterpolantArtifactErrorKind as Kind;
    match err {
        PreciseInterpolantStoreError::Io { .. } => Kind::Io,
        PreciseInterpolantStoreError::Parse { .. } => Kind::Parse,
        PreciseInterpolantStoreError::BadMagic { .. } => Kind::BadMagic,
        PreciseInterpolantStoreError::HeaderTruncated { .. } => Kind::HeaderTruncated,
        PreciseInterpolantStoreError::Truncated { .. } => Kind::Truncated,
        PreciseInterpolantStoreError::TrailingBytes { .. } => Kind::TrailingBytes,
        PreciseInterpolantStoreError::RangeOutOfBounds { .. } => Kind::RangeOutOfBounds,
        PreciseInterpolantStoreError::UnsupportedVersion { .. } => Kind::UnsupportedVersion,
        PreciseInterpolantStoreError::UnsupportedTimeScale { .. } => Kind::UnsupportedTimeScale,
        PreciseInterpolantStoreError::UnsupportedSatelliteSystem { .. } => {
            Kind::UnsupportedSatelliteSystem
        }
        PreciseInterpolantStoreError::DuplicateSatellite { .. } => Kind::DuplicateSatellite,
        PreciseInterpolantStoreError::Checksum { .. } => Kind::Corrupt,
        PreciseInterpolantStoreError::SatelliteChecksum { .. } => Kind::SatelliteChecksum,
        PreciseInterpolantStoreError::AttestedChecksumMismatch { .. } => {
            Kind::AttestedChecksumMismatch
        }
        _ => Kind::Unknown,
    }
}

unsafe fn map_artifact_error(
    fn_name: &str,
    err: PreciseInterpolantStoreError,
    out_error: *mut SidereonPreciseInterpolantArtifactErrorKind,
) -> SidereonStatus {
    let (typed, texts) = artifact_error_to_c(&err);
    let kind = artifact_error_kind(&err);
    LAST_PRECISE_ARTIFACT_ERROR.with(|slot| *slot.borrow_mut() = Some(typed));
    LAST_PRECISE_ARTIFACT_ERROR_TEXTS.with(|slot| *slot.borrow_mut() = texts);
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
        CoreError::Ut1OutsideCoverage(_) => SidereonStatus::Ut1OutsideCoverage,
        // `sidereon_core::Error` is non-exhaustive: a failure a later engine
        // adds keeps its variant name in the message.
        other => {
            set_last_error(format!("{fn_name}: {other} ({other:?})"));
            SidereonStatus::Solve
        }
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
    use std::ffi::CString;
    use std::fs;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};

    const HEADER_INDEX_OFFSET_OFFSET: usize = 16;
    const HEADER_CHECKSUM_OFFSET: usize = 40;
    const INDEX_POS_KX_OFFSET_OFFSET: usize = 24;

    struct TempArtifact(PathBuf);

    impl TempArtifact {
        fn write(name: &str, bytes: &[u8]) -> Self {
            static NEXT_ID: AtomicU64 = AtomicU64::new(0);
            let path = std::env::temp_dir().join(format!(
                "sidereon-c-{name}-{}-{}",
                std::process::id(),
                NEXT_ID.fetch_add(1, Ordering::Relaxed)
            ));
            fs::write(&path, bytes).expect("write temporary precise artifact");
            Self(path)
        }

        fn c_path(&self) -> CString {
            CString::new(self.0.to_string_lossy().as_bytes()).expect("temporary path has no NUL")
        }
    }

    impl Drop for TempArtifact {
        fn drop(&mut self) {
            let _ = fs::remove_file(&self.0);
        }
    }

    fn read_u64(bytes: &[u8], offset: usize) -> u64 {
        u64::from_le_bytes(
            bytes[offset..offset + 8]
                .try_into()
                .expect("artifact contains u64 field"),
        )
    }

    fn artifact_text(part: SidereonPreciseInterpolantArtifactErrorText) -> String {
        let mut written = 0usize;
        let mut required = 0usize;
        assert_eq!(
            unsafe {
                sidereon_last_precise_interpolant_artifact_error_text(
                    part as u32,
                    ptr::null_mut(),
                    0,
                    &mut written,
                    &mut required,
                )
            },
            SidereonStatus::Ok
        );
        let mut bytes = vec![0; required];
        assert_eq!(
            unsafe {
                sidereon_last_precise_interpolant_artifact_error_text(
                    part as u32,
                    bytes.as_mut_ptr(),
                    bytes.len(),
                    &mut written,
                    &mut required,
                )
            },
            SidereonStatus::Ok
        );
        assert_eq!(written, required);
        String::from_utf8(bytes).expect("artifact error detail is UTF-8")
    }

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
        // The binding delegates framing validation to sidereon-core, preserving
        // the core's typed declared and available lengths.
        assert_eq!(
            error,
            SidereonPreciseInterpolantArtifactErrorKind::Truncated
        );
        let mut detail = no_precise_artifact_error();
        assert_eq!(
            unsafe { sidereon_last_precise_interpolant_artifact_error(&mut detail) },
            SidereonStatus::Ok
        );
        assert_eq!(detail.kind, error as u32);
        assert_eq!(detail.declared, bytes.len() as u64);
        assert_eq!(detail.available, (bytes.len() - 1) as u64);
        assert!(
            MmapPreciseEphemerisInterpolant::from_vec(bytes[..bytes.len() - 1].to_vec()).is_err()
        );
    }

    #[test]
    fn attested_path_open_defers_corrupt_payload_failure_until_verify() {
        let sp3 = fixture_sp3();
        let mut bytes = sp3
            .precise_interpolant_store_bytes()
            .expect("build precise artifact");
        let declared = read_u64(&bytes, HEADER_CHECKSUM_OFFSET);
        let index_offset = read_u64(&bytes, HEADER_INDEX_OFFSET_OFFSET) as usize;
        let pos_kx_offset = read_u64(&bytes, index_offset + INDEX_POS_KX_OFFSET_OFFSET) as usize;
        bytes[pos_kx_offset + 1] ^= 1;
        let artifact_file = TempArtifact::write("corrupt-precise.spi", &bytes);
        let path = artifact_file.c_path();
        // sidereon-core's own outcomes for the same file.
        let core_verified_error = MmapPreciseEphemerisInterpolant::from_path(&artifact_file.0)
            .expect_err("core refuses the corrupt payload");
        let mut core_attested =
            MmapPreciseEphemerisInterpolant::from_path_attested(&artifact_file.0, declared)
                .expect("core attested open");
        let core_provenance = digest_provenance_to_c(core_attested.digest_provenance());
        let core_verify_error = core_attested
            .verify()
            .expect_err("core verify refuses the corrupt payload");
        let core_provenance_after = digest_provenance_to_c(core_attested.digest_provenance());

        let mut error = SidereonPreciseInterpolantArtifactErrorKind::None;
        let mut verified = ptr::null_mut();
        let status = unsafe {
            sidereon_precise_interpolant_artifact_from_path(
                path.as_ptr(),
                &mut error,
                &mut verified,
            )
        };
        assert_eq!(status, SidereonStatus::InvalidArgument);
        assert_eq!(error, artifact_error_kind(&core_verified_error));
        assert!(verified.is_null());

        let mut attested = ptr::null_mut();
        let status = unsafe {
            sidereon_precise_interpolant_artifact_from_path_attested(
                path.as_ptr(),
                declared,
                &mut error,
                &mut attested,
            )
        };
        assert_eq!(status, SidereonStatus::Ok);
        assert_eq!(error, SidereonPreciseInterpolantArtifactErrorKind::None);
        assert!(!attested.is_null());

        let mut provenance = SidereonDigestProvenance::Verified;
        assert_eq!(
            unsafe {
                sidereon_precise_interpolant_artifact_digest_provenance(attested, &mut provenance)
            },
            SidereonStatus::Ok
        );
        assert_eq!(provenance, core_provenance);
        let mut checksum = 0;
        assert_eq!(
            unsafe {
                sidereon_precise_interpolant_artifact_handle_checksum64(attested, &mut checksum)
            },
            SidereonStatus::Ok
        );
        assert_eq!(checksum, declared);

        assert_eq!(
            unsafe { sidereon_precise_interpolant_artifact_verify(attested, &mut error) },
            SidereonStatus::InvalidArgument
        );
        assert_eq!(error, artifact_error_kind(&core_verify_error));
        assert_eq!(
            unsafe {
                sidereon_precise_interpolant_artifact_digest_provenance(attested, &mut provenance)
            },
            SidereonStatus::Ok
        );
        assert_eq!(provenance, core_provenance_after);
        unsafe { sidereon_precise_interpolant_artifact_free(attested) };
    }

    #[test]
    fn attested_path_claim_mismatch_is_typed_without_a_handle() {
        let bytes = fixture_sp3()
            .precise_interpolant_store_bytes()
            .expect("build precise artifact");
        let declared = read_u64(&bytes, HEADER_CHECKSUM_OFFSET);
        let artifact_file = TempArtifact::write("claim-mismatch-precise.spi", &bytes);
        let path = artifact_file.c_path();
        let mut error = SidereonPreciseInterpolantArtifactErrorKind::None;
        let mut artifact = ptr::null_mut();

        let status = unsafe {
            sidereon_precise_interpolant_artifact_from_path_attested(
                path.as_ptr(),
                declared ^ 1,
                &mut error,
                &mut artifact,
            )
        };
        assert_eq!(status, SidereonStatus::InvalidArgument);
        // sidereon-core's own refusal of the wrong claim.
        let core_error =
            MmapPreciseEphemerisInterpolant::from_path_attested(&artifact_file.0, declared ^ 1)
                .expect_err("core refuses the wrong claim");
        assert_eq!(error, artifact_error_kind(&core_error));
        assert!(artifact.is_null());
    }

    #[test]
    fn pristine_attested_path_open_matches_verified_and_escalates() {
        let sp3 = fixture_sp3();
        let epoch = sp3.epochs_j2000_seconds()[10];
        let sat_id = sp3.satellites()[0];
        let bytes = sp3
            .precise_interpolant_store_bytes()
            .expect("build precise artifact");
        let declared = read_u64(&bytes, HEADER_CHECKSUM_OFFSET);
        let artifact_file = TempArtifact::write("pristine-precise.spi", &bytes);
        let path = artifact_file.c_path();
        let sat = CString::new(sat_id.to_string()).expect("satellite token");
        // sidereon-core's own attested open, before and after verify.
        let mut core_attested =
            MmapPreciseEphemerisInterpolant::from_path_attested(&artifact_file.0, declared)
                .expect("core attested open");
        let core_provenance = digest_provenance_to_c(core_attested.digest_provenance());
        core_attested
            .verify()
            .expect("core verifies the pristine artifact");
        let core_provenance_after = digest_provenance_to_c(core_attested.digest_provenance());

        let mut error = SidereonPreciseInterpolantArtifactErrorKind::None;
        let mut verified = ptr::null_mut();
        let mut attested = ptr::null_mut();
        assert_eq!(
            unsafe {
                sidereon_precise_interpolant_artifact_from_path(
                    path.as_ptr(),
                    &mut error,
                    &mut verified,
                )
            },
            SidereonStatus::Ok
        );
        assert_eq!(
            unsafe {
                sidereon_precise_interpolant_artifact_from_path_attested(
                    path.as_ptr(),
                    declared,
                    &mut error,
                    &mut attested,
                )
            },
            SidereonStatus::Ok
        );

        let mut verified_state = empty_artifact_sp3_state();
        let mut attested_state = empty_artifact_sp3_state();
        assert_eq!(
            unsafe {
                sidereon_precise_interpolant_artifact_state(
                    verified,
                    sat.as_ptr(),
                    epoch,
                    &mut verified_state,
                )
            },
            SidereonStatus::Ok
        );
        assert_eq!(
            unsafe {
                sidereon_precise_interpolant_artifact_state(
                    attested,
                    sat.as_ptr(),
                    epoch,
                    &mut attested_state,
                )
            },
            SidereonStatus::Ok
        );
        assert_eq!(
            verified_state.position_m.map(f64::to_bits),
            attested_state.position_m.map(f64::to_bits)
        );
        assert_eq!(
            verified_state.clock_s.to_bits(),
            attested_state.clock_s.to_bits()
        );

        let mut checksum = 0;
        assert_eq!(
            unsafe {
                sidereon_precise_interpolant_artifact_handle_checksum64(attested, &mut checksum)
            },
            SidereonStatus::Ok
        );
        assert_eq!(checksum, declared);
        let mut provenance = SidereonDigestProvenance::Verified;
        assert_eq!(
            unsafe {
                sidereon_precise_interpolant_artifact_digest_provenance(attested, &mut provenance)
            },
            SidereonStatus::Ok
        );
        assert_eq!(provenance, core_provenance);
        assert_eq!(
            unsafe { sidereon_precise_interpolant_artifact_verify(attested, &mut error) },
            SidereonStatus::Ok
        );
        assert_eq!(error, SidereonPreciseInterpolantArtifactErrorKind::None);
        assert_eq!(
            unsafe {
                sidereon_precise_interpolant_artifact_digest_provenance(attested, &mut provenance)
            },
            SidereonStatus::Ok
        );
        assert_eq!(provenance, core_provenance_after);

        unsafe {
            sidereon_precise_interpolant_artifact_free(verified);
            sidereon_precise_interpolant_artifact_free(attested);
        }
    }

    #[test]
    fn artifact_error_mapping_preserves_all_current_variant_fields() {
        let satellite = GnssSatelliteId::new(GnssSystem::Gps, 7).unwrap();
        let cases = [
            (
                PreciseInterpolantStoreError::Io {
                    path: PathBuf::from("artifact.spi"),
                    message: "unreadable".to_owned(),
                },
                SidereonPreciseInterpolantArtifactErrorKind::Io,
            ),
            (
                PreciseInterpolantStoreError::Parse {
                    reason: "bad index".to_owned(),
                },
                SidereonPreciseInterpolantArtifactErrorKind::Parse,
            ),
            (
                PreciseInterpolantStoreError::BadMagic {
                    found: *b"NOTMAGIC",
                },
                SidereonPreciseInterpolantArtifactErrorKind::BadMagic,
            ),
            (
                PreciseInterpolantStoreError::HeaderTruncated { available: 8 },
                SidereonPreciseInterpolantArtifactErrorKind::HeaderTruncated,
            ),
            (
                PreciseInterpolantStoreError::Truncated {
                    declared: 128,
                    available: 64,
                },
                SidereonPreciseInterpolantArtifactErrorKind::Truncated,
            ),
            (
                PreciseInterpolantStoreError::TrailingBytes {
                    declared: 128,
                    available: 129,
                },
                SidereonPreciseInterpolantArtifactErrorKind::TrailingBytes,
            ),
            (
                PreciseInterpolantStoreError::RangeOutOfBounds {
                    region: "clock node",
                    sat: Some(satellite),
                    offset: 100,
                    len: 24,
                    available: 112,
                },
                SidereonPreciseInterpolantArtifactErrorKind::RangeOutOfBounds,
            ),
            (
                PreciseInterpolantStoreError::UnsupportedVersion { version: 9 },
                SidereonPreciseInterpolantArtifactErrorKind::UnsupportedVersion,
            ),
            (
                PreciseInterpolantStoreError::UnsupportedTimeScale { tag: 255 },
                SidereonPreciseInterpolantArtifactErrorKind::UnsupportedTimeScale,
            ),
            (
                PreciseInterpolantStoreError::UnsupportedSatelliteSystem { tag: 255 },
                SidereonPreciseInterpolantArtifactErrorKind::UnsupportedSatelliteSystem,
            ),
            (
                PreciseInterpolantStoreError::DuplicateSatellite { sat: satellite },
                SidereonPreciseInterpolantArtifactErrorKind::DuplicateSatellite,
            ),
            (
                PreciseInterpolantStoreError::Checksum {
                    expected: 0x1122,
                    found: 0x3344,
                },
                SidereonPreciseInterpolantArtifactErrorKind::Corrupt,
            ),
            (
                PreciseInterpolantStoreError::SatelliteChecksum {
                    sat: satellite,
                    expected: 0x5566,
                    found: 0x7788,
                },
                SidereonPreciseInterpolantArtifactErrorKind::SatelliteChecksum,
            ),
            (
                PreciseInterpolantStoreError::AttestedChecksumMismatch {
                    claimed: 0x99aa,
                    declared: 0xbbcc,
                },
                SidereonPreciseInterpolantArtifactErrorKind::AttestedChecksumMismatch,
            ),
        ];
        for (error, expected_kind) in cases {
            let (typed, _) = artifact_error_to_c(&error);
            assert_eq!(typed.kind, expected_kind as u32, "{error:?}");
        }

        let (bad_magic, _) = artifact_error_to_c(&PreciseInterpolantStoreError::BadMagic {
            found: *b"NOTMAGIC",
        });
        assert_eq!(&bad_magic.found_magic, b"NOTMAGIC");
        let (version, _) = artifact_error_to_c(&PreciseInterpolantStoreError::UnsupportedVersion {
            version: 0x1234,
        });
        assert_eq!(version.version, 0x1234);
        let (scale, _) =
            artifact_error_to_c(&PreciseInterpolantStoreError::UnsupportedTimeScale { tag: 0xfe });
        assert_eq!(scale.tag, 0xfe);
        let (range, texts) = artifact_error_to_c(&PreciseInterpolantStoreError::RangeOutOfBounds {
            region: "clock node",
            sat: Some(satellite),
            offset: 100,
            len: 24,
            available: 112,
        });
        assert_eq!((range.offset, range.len, range.available), (100, 24, 112));
        assert!(range.has_satellite);
        assert_eq!(range.satellite.bytes, satellite_token(satellite).bytes);
        assert_eq!(
            texts,
            vec![(
                SidereonPreciseInterpolantArtifactErrorText::Region as u32,
                "clock node".to_owned()
            )]
        );
        let mut output_kind = SidereonPreciseInterpolantArtifactErrorKind::None;
        assert_eq!(
            unsafe {
                map_artifact_error(
                    "test_range_error",
                    PreciseInterpolantStoreError::RangeOutOfBounds {
                        region: "clock node",
                        sat: Some(satellite),
                        offset: 100,
                        len: 24,
                        available: 112,
                    },
                    &mut output_kind,
                )
            },
            SidereonStatus::InvalidArgument
        );
        assert_eq!(
            output_kind,
            SidereonPreciseInterpolantArtifactErrorKind::RangeOutOfBounds
        );
        assert_eq!(
            artifact_text(SidereonPreciseInterpolantArtifactErrorText::Region),
            "clock node"
        );
        let mut retained = no_precise_artifact_error();
        assert_eq!(
            unsafe { sidereon_last_precise_interpolant_artifact_error(&mut retained) },
            SidereonStatus::Ok
        );
        assert_eq!(
            (retained.offset, retained.len, retained.available),
            (100, 24, 112)
        );
        let (checksum, _) = artifact_error_to_c(&PreciseInterpolantStoreError::SatelliteChecksum {
            sat: satellite,
            expected: 0x5566,
            found: 0x7788,
        });
        assert_eq!(
            (checksum.expected_checksum64, checksum.found_checksum64),
            (0x5566, 0x7788)
        );
        assert_eq!(checksum.satellite.bytes, satellite_token(satellite).bytes);
        let (attested, _) =
            artifact_error_to_c(&PreciseInterpolantStoreError::AttestedChecksumMismatch {
                claimed: 0x99aa,
                declared: 0xbbcc,
            });
        assert_eq!(
            (attested.claimed_checksum64, attested.declared_checksum64),
            (0x99aa, 0xbbcc)
        );
    }

    #[test]
    fn bad_magic_and_header_truncation_match_core_on_all_open_routes() {
        let bad_magic = b"NOTMAGIC".to_vec();
        let expected = MmapPreciseEphemerisInterpolant::from_vec(bad_magic.clone())
            .expect_err("core rejects bad magic");
        assert_eq!(
            artifact_error_kind(&expected),
            SidereonPreciseInterpolantArtifactErrorKind::BadMagic
        );

        let mut error = SidereonPreciseInterpolantArtifactErrorKind::None;
        let mut artifact = ptr::null_mut();
        assert_eq!(
            unsafe {
                sidereon_precise_interpolant_artifact_open_owned(
                    bad_magic.as_ptr(),
                    bad_magic.len(),
                    &mut error,
                    &mut artifact,
                )
            },
            SidereonStatus::InvalidArgument
        );
        assert_eq!(error, SidereonPreciseInterpolantArtifactErrorKind::BadMagic);
        let mut detail = no_precise_artifact_error();
        assert_eq!(
            unsafe { sidereon_last_precise_interpolant_artifact_error(&mut detail) },
            SidereonStatus::Ok
        );
        assert_eq!(detail.found_magic, *b"NOTMAGIC");
        assert!(artifact_text(SidereonPreciseInterpolantArtifactErrorText::Message).is_empty());
        assert_eq!(
            unsafe { sidereon_last_precise_interpolant_artifact_error(&mut detail) },
            SidereonStatus::Ok
        );
        assert_eq!(detail.found_magic, *b"NOTMAGIC");

        assert_eq!(
            unsafe {
                sidereon_precise_interpolant_artifact_open_borrowed(
                    bad_magic.as_ptr(),
                    bad_magic.len(),
                    &mut error,
                    &mut artifact,
                )
            },
            SidereonStatus::InvalidArgument
        );
        assert_eq!(error, artifact_error_kind(&expected));

        let file = TempArtifact::write("bad-magic.spi", &bad_magic);
        let path = file.c_path();
        let mut opened = ptr::null_mut();
        for attested in [false, true] {
            let status = unsafe {
                if attested {
                    sidereon_precise_interpolant_artifact_from_path_attested(
                        path.as_ptr(),
                        0,
                        &mut error,
                        &mut opened,
                    )
                } else {
                    sidereon_precise_interpolant_artifact_from_path(
                        path.as_ptr(),
                        &mut error,
                        &mut opened,
                    )
                }
            };
            assert_eq!(status, SidereonStatus::InvalidArgument);
            assert_eq!(error, SidereonPreciseInterpolantArtifactErrorKind::BadMagic);
            assert!(opened.is_null());
            assert_eq!(
                unsafe { sidereon_last_precise_interpolant_artifact_error(&mut detail) },
                SidereonStatus::Ok
            );
            assert_eq!(detail.found_magic, *b"NOTMAGIC");
        }

        let prefix = b"PEMAP001";
        let core_header_error = MmapPreciseEphemerisInterpolant::from_vec(prefix.to_vec())
            .expect_err("core reports incomplete fixed header");
        assert_eq!(
            artifact_error_kind(&core_header_error),
            SidereonPreciseInterpolantArtifactErrorKind::HeaderTruncated
        );
        assert_eq!(
            unsafe {
                sidereon_precise_interpolant_artifact_open_owned(
                    prefix.as_ptr(),
                    prefix.len(),
                    &mut error,
                    &mut artifact,
                )
            },
            SidereonStatus::InvalidArgument
        );
        assert_eq!(
            error,
            SidereonPreciseInterpolantArtifactErrorKind::HeaderTruncated
        );
        assert_eq!(
            unsafe { sidereon_last_precise_interpolant_artifact_error(&mut detail) },
            SidereonStatus::Ok
        );
        assert_eq!(detail.available, 8);

        let valid = fixture_sp3().precise_interpolant_store_bytes().unwrap();
        assert_eq!(
            unsafe {
                sidereon_precise_interpolant_artifact_open_owned(
                    valid.as_ptr(),
                    valid.len(),
                    &mut error,
                    &mut artifact,
                )
            },
            SidereonStatus::Ok
        );
        assert_eq!(
            unsafe { sidereon_last_precise_interpolant_artifact_error(&mut detail) },
            SidereonStatus::Ok
        );
        assert_eq!(
            detail.kind,
            SidereonPreciseInterpolantArtifactErrorKind::None as u32
        );
        unsafe { sidereon_precise_interpolant_artifact_free(artifact) };

        assert_eq!(
            unsafe {
                sidereon_precise_interpolant_artifact_open_owned(
                    ptr::null(),
                    1,
                    &mut error,
                    &mut artifact,
                )
            },
            SidereonStatus::NullPointer
        );
        assert_eq!(
            unsafe { sidereon_last_precise_interpolant_artifact_error(&mut detail) },
            SidereonStatus::Ok
        );
        assert_eq!(
            detail.kind,
            SidereonPreciseInterpolantArtifactErrorKind::None as u32
        );
    }
}
