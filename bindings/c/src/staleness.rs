use super::*;

// ---------------------------------------------------------------------------

/// How a staleness selection's source epoch relates to the requested epoch.
/// Mirrors sidereon_core::staleness::DegradationKind.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonDegradationKind {
    /// A product covering the requested epoch was present; no degradation.
    Exact = 0,
    /// No product covered the requested epoch; the most-recent prior product was
    /// used as-is (the SP3 nearest-prior path).
    NearestPrior = 1,
    /// No product covered the requested day; a prior day's IONEX grid was advanced
    /// by whole days onto the requested epoch (diurnal persistence).
    DiurnalShift = 2,
}

/// Staleness provenance attached to every selection result and to a precise
/// fallback fix. Epoch fields are seconds since the J2000 epoch; staleness_s is
/// requested - source and is never negative.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonStalenessMetadata {
    /// Which degradation path produced the result.
    pub kind: SidereonDegradationKind,
    /// The requested epoch, J2000 seconds (the most-stale epoch of a range).
    pub requested_epoch_j2000_s: f64,
    /// The source product epoch the result is backed by, J2000 seconds.
    pub source_epoch_j2000_s: f64,
    /// Staleness requested - source, seconds. Zero for an exact result.
    pub staleness_s: f64,
    /// Staleness in days (staleness_s / 86400).
    pub staleness_days: f64,
}

/// Configurable staleness cap for product selection. A selection that would rely
/// on a product older than max_staleness_s is rejected rather than returning data
/// past the cap. Build one with sidereon_staleness_policy_default / _days /
/// _seconds.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonStalenessPolicy {
    /// Maximum tolerated staleness, seconds.
    pub max_staleness_s: f64,
}

/// Typed outcome of a staleness selection. SIDEREON_SELECTION_STATUS_OK is the
/// only success; every other value mirrors a sidereon_core SelectionError variant
/// or a marshaling failure. A human-readable reason, with the structured detail
/// (source epoch, staleness, and cap for a beyond-cap rejection), is available
/// from sidereon_last_error_message.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonSelectionStatus {
    /// Success; the selection handle and metadata were written.
    Ok = 0,
    /// A required pointer argument was null.
    NullPointer = 1,
    /// An argument was structurally invalid (e.g. a count too large).
    InvalidArgument = 2,
    /// A satellite or string token did not parse.
    InvalidToken = 3,
    /// An internal panic reached the FFI boundary and was contained.
    Panic = 4,
    /// The product set was empty.
    EmptyProductSet = 5,
    /// The requested range was malformed (non-finite, or end before start).
    InvalidRange = 6,
    /// No product covers or precedes the requested epoch.
    NoPriorProduct = 7,
    /// The most-recent usable product is older than the staleness cap.
    BeyondStalenessCap = 8,
    /// A product in the set was malformed, or no prior product covers the range
    /// after a whole-day diurnal shift.
    InvalidProduct = 9,
    /// The staleness cap was non-finite or negative.
    InvalidPolicy = 10,
    /// An epoch computation overflowed the i64 J2000-second axis.
    Overflow = 11,
    /// A typed IONEX epoch conversion failed.
    IonexEpoch = 12,
}

/// Which ephemeris source produced a sourced solution. Mirrors
/// sidereon_core::positioning::FixSource (precise SP3 vs broadcast).
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonFixSourceKind {
    /// A precise SP3 product produced the fix (exact or degraded).
    Precise = 0,
    /// The broadcast ephemeris path produced the fix.
    Broadcast = 1,
}

/// Typed outcome of sidereon_solve_with_fallback. SIDEREON_FALLBACK_STATUS_OK is
/// the only success; PreciseSolve / BroadcastSolve name which path's SPP solve
/// failed (FallbackError::Precise / ::Broadcast). The reason is available from
/// sidereon_last_error_message.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonFallbackStatus {
    /// Success; the sourced-solution handle was written.
    Ok = 0,
    /// A required pointer argument was null.
    NullPointer = 1,
    /// An argument was structurally invalid.
    InvalidArgument = 2,
    /// A satellite or string token did not parse.
    InvalidToken = 3,
    /// An internal panic reached the FFI boundary and was contained.
    Panic = 4,
    /// A usable precise product was selected but its SPP solve failed
    /// (FallbackError::Precise); broadcast is not silently re-solved in this case.
    PreciseSolve = 5,
    /// The broadcast fallback path was taken and its SPP solve failed
    /// (FallbackError::Broadcast).
    BroadcastSolve = 6,
    /// The selected source read UT1 outside the UT1 table and its UT1 policy
    /// refused it, on either path.
    Ut1OutsideCoverage = 7,
}

/// A staleness policy with the engine default cap (3 days).
#[no_mangle]
pub extern "C" fn sidereon_staleness_policy_default() -> SidereonStalenessPolicy {
    SidereonStalenessPolicy {
        max_staleness_s: StalenessPolicy::default().max_staleness_s,
    }
}

/// A staleness policy with a cap expressed in days.
#[no_mangle]
pub extern "C" fn sidereon_staleness_policy_days(days: f64) -> SidereonStalenessPolicy {
    SidereonStalenessPolicy {
        max_staleness_s: StalenessPolicy::days(days).max_staleness_s,
    }
}

/// A staleness policy with a cap expressed in seconds.
#[no_mangle]
pub extern "C" fn sidereon_staleness_policy_seconds(seconds: f64) -> SidereonStalenessPolicy {
    SidereonStalenessPolicy {
        max_staleness_s: StalenessPolicy::seconds(seconds).max_staleness_s,
    }
}

/// Select an SP3 product usable across the range [start, end] (J2000 seconds),
/// degrading to the most-recent prior product within policy. On success writes a
/// newly owned clone of the selected product to *out_selection (a SidereonSp3
/// usable with the SP3 interpolation accessors and released with
/// sidereon_sp3_free) and the staleness provenance to *out_metadata. For an exact
/// selection the clone is byte-identical to the caller's product, so interpolating
/// it reproduces the engine bit-for-bit.
///
/// Safety: products must point to product_count readable SidereonSp3 pointers (or
/// be NULL when product_count is 0); out_selection and out_metadata must point to
/// writable storage of the documented type.
#[no_mangle]
pub unsafe extern "C" fn sidereon_select_sp3_over_range(
    products: *const *const SidereonSp3,
    product_count: usize,
    start_epoch_j2000_s: f64,
    end_epoch_j2000_s: f64,
    policy: SidereonStalenessPolicy,
    out_selection: *mut *mut SidereonSp3,
    out_metadata: *mut SidereonStalenessMetadata,
) -> SidereonSelectionStatus {
    engine_error_operation_boundary(
        "sidereon_select_sp3_over_range",
        SidereonSelectionStatus::Panic,
        || {
            let out_selection = sel_try!(require_out(
                out_selection,
                "sidereon_select_sp3_over_range",
                "out_selection"
            ));
            *out_selection = ptr::null_mut();
            let out_metadata = sel_try!(require_out(
                out_metadata,
                "sidereon_select_sp3_over_range",
                "out_metadata"
            ));
            *out_metadata = empty_staleness_metadata();
            let set = sel_try!(sp3_products_from_c(
                "sidereon_select_sp3_over_range",
                products,
                product_count
            ));
            let policy = StalenessPolicy {
                max_staleness_s: policy.max_staleness_s,
            };
            match select_sp3_over_range(&set, start_epoch_j2000_s, end_epoch_j2000_s, policy) {
                Ok(selection) => {
                    *out_metadata = staleness_metadata_to_c(selection.metadata());
                    write_boxed_handle(
                        out_selection,
                        SidereonSp3 {
                            inner: selection.sp3().clone(),
                        },
                    );
                    SidereonSelectionStatus::Ok
                }
                Err(err) => map_selection_error("sidereon_select_sp3_over_range", &err),
            }
        },
    )
}

/// Single-epoch convenience over sidereon_select_sp3_over_range.
///
/// Safety: as sidereon_select_sp3_over_range.
#[no_mangle]
pub unsafe extern "C" fn sidereon_select_sp3(
    products: *const *const SidereonSp3,
    product_count: usize,
    requested_epoch_j2000_s: f64,
    policy: SidereonStalenessPolicy,
    out_selection: *mut *mut SidereonSp3,
    out_metadata: *mut SidereonStalenessMetadata,
) -> SidereonSelectionStatus {
    sidereon_select_sp3_over_range(
        products,
        product_count,
        requested_epoch_j2000_s,
        requested_epoch_j2000_s,
        policy,
        out_selection,
        out_metadata,
    )
}

/// Select an IONEX product usable across the range [start, end] (integer UTC
/// seconds since J2000, as for sidereon_ionex_slant_delay), degrading to a whole-day diurnal-shifted prior product within policy. On
/// success writes a newly owned product to *out_selection (a SidereonIonex usable
/// with sidereon_ionex_slant_delay and released with sidereon_ionex_free) and the
/// staleness provenance to *out_metadata. For an exact selection the product is
/// byte-identical to the caller's, so the slant delay reproduces the engine
/// bit-for-bit.
///
/// Safety: products must point to product_count readable SidereonIonex pointers
/// (or be NULL when product_count is 0); out_selection and out_metadata must point
/// to writable storage of the documented type.
#[no_mangle]
pub unsafe extern "C" fn sidereon_select_ionex_over_range(
    products: *const *const SidereonIonex,
    product_count: usize,
    start_epoch_j2000_s: i64,
    end_epoch_j2000_s: i64,
    policy: SidereonStalenessPolicy,
    out_selection: *mut *mut SidereonIonex,
    out_metadata: *mut SidereonStalenessMetadata,
) -> SidereonSelectionStatus {
    engine_error_operation_boundary(
        "sidereon_select_ionex_over_range",
        SidereonSelectionStatus::Panic,
        || {
            let out_selection = sel_try!(require_out(
                out_selection,
                "sidereon_select_ionex_over_range",
                "out_selection"
            ));
            *out_selection = ptr::null_mut();
            let out_metadata = sel_try!(require_out(
                out_metadata,
                "sidereon_select_ionex_over_range",
                "out_metadata"
            ));
            *out_metadata = empty_staleness_metadata();
            let set = sel_try!(ionex_products_from_c(
                "sidereon_select_ionex_over_range",
                products,
                product_count
            ));
            let policy = StalenessPolicy {
                max_staleness_s: policy.max_staleness_s,
            };
            match select_ionex_over_range(
                &set,
                crate::ionex::ionex_epoch_from_j2000_seconds(start_epoch_j2000_s),
                crate::ionex::ionex_epoch_from_j2000_seconds(end_epoch_j2000_s),
                policy,
            ) {
                Ok(selection) => {
                    *out_metadata = staleness_metadata_to_c(selection.metadata());
                    write_boxed_handle(
                        out_selection,
                        SidereonIonex {
                            inner: selection.ionex().clone(),
                        },
                    );
                    SidereonSelectionStatus::Ok
                }
                Err(err) => map_selection_error("sidereon_select_ionex_over_range", &err),
            }
        },
    )
}

/// Single-epoch convenience over sidereon_select_ionex_over_range.
///
/// Safety: as sidereon_select_ionex_over_range.
#[no_mangle]
pub unsafe extern "C" fn sidereon_select_ionex(
    products: *const *const SidereonIonex,
    product_count: usize,
    requested_epoch_j2000_s: i64,
    policy: SidereonStalenessPolicy,
    out_selection: *mut *mut SidereonIonex,
    out_metadata: *mut SidereonStalenessMetadata,
) -> SidereonSelectionStatus {
    sidereon_select_ionex_over_range(
        products,
        product_count,
        requested_epoch_j2000_s,
        requested_epoch_j2000_s,
        policy,
        out_selection,
        out_metadata,
    )
}

/// Select an IONEX product across an exact, scale-tagged interval. The
/// endpoints retain their split/nanosecond precision through the core
/// selection call. `out_epoch_error` carries any of the seven typed IONEX
/// epoch conversion causes.
#[no_mangle]
pub unsafe extern "C" fn sidereon_select_ionex_over_instant_range(
    products: *const *const SidereonIonex,
    product_count: usize,
    start: *const SidereonClockEpoch,
    end: *const SidereonClockEpoch,
    policy: SidereonStalenessPolicy,
    out_selection: *mut *mut SidereonIonex,
    out_metadata: *mut SidereonStalenessMetadata,
    out_epoch_error: *mut SidereonIonexEpochError,
) -> SidereonSelectionStatus {
    const FN: &str = "sidereon_select_ionex_over_instant_range";
    engine_error_operation_boundary(FN, SidereonSelectionStatus::Panic, || {
        if !out_epoch_error.is_null() {
            *out_epoch_error = no_ionex_epoch_error();
        }
        let out_selection = match require_out(out_selection, FN, "out_selection") {
            Ok(out) => out,
            Err(status) => return selection_status_from_ffi(status),
        };
        *out_selection = ptr::null_mut();
        let out_metadata = match require_out(out_metadata, FN, "out_metadata") {
            Ok(out) => out,
            Err(status) => return selection_status_from_ffi(status),
        };
        *out_metadata = empty_staleness_metadata();
        let start = match require_ref(start, FN, "start")
            .and_then(|epoch| clock_epoch_from_c(FN, "start", epoch))
        {
            Ok(epoch) => epoch,
            Err(status) => return selection_status_from_ffi(status),
        };
        let end = match require_ref(end, FN, "end")
            .and_then(|epoch| clock_epoch_from_c(FN, "end", epoch))
        {
            Ok(epoch) => epoch,
            Err(status) => return selection_status_from_ffi(status),
        };
        let set = match ionex_products_from_c(FN, products, product_count) {
            Ok(set) => set,
            Err(status) => return selection_status_from_ffi(status),
        };
        let policy = StalenessPolicy {
            max_staleness_s: policy.max_staleness_s,
        };
        match select_ionex_over_range(&set, start, end, policy) {
            Ok(selection) => {
                *out_metadata = staleness_metadata_to_c(selection.metadata());
                write_boxed_handle(
                    out_selection,
                    SidereonIonex {
                        inner: selection.ionex().clone(),
                    },
                );
                SidereonSelectionStatus::Ok
            }
            Err(SelectionError::IonexEpoch(error)) => {
                if !out_epoch_error.is_null() {
                    *out_epoch_error = ionex_epoch_error_to_c(&error);
                }
                map_selection_error(FN, &SelectionError::IonexEpoch(error))
            }
            Err(error) => map_selection_error(FN, &error),
        }
    })
}

/// Select one IONEX product at an exact scale-tagged instant.
#[no_mangle]
pub unsafe extern "C" fn sidereon_select_ionex_at_instant(
    products: *const *const SidereonIonex,
    product_count: usize,
    requested: *const SidereonClockEpoch,
    policy: SidereonStalenessPolicy,
    out_selection: *mut *mut SidereonIonex,
    out_metadata: *mut SidereonStalenessMetadata,
    out_epoch_error: *mut SidereonIonexEpochError,
) -> SidereonSelectionStatus {
    sidereon_select_ionex_over_instant_range(
        products,
        product_count,
        requested,
        requested,
        policy,
        out_selection,
        out_metadata,
        out_epoch_error,
    )
}

fn selection_status_from_ffi(status: SidereonStatus) -> SidereonSelectionStatus {
    marshal_status_to_selection(status)
}

/// Collect an array of IONEX handle pointers into an owned product slice.
unsafe fn ionex_products_from_c(
    fn_name: &str,
    products: *const *const SidereonIonex,
    product_count: usize,
) -> Result<Vec<Ionex>, SidereonStatus> {
    let raw = require_slice(products, product_count, fn_name, "products")?;
    let mut set = Vec::with_capacity(raw.len());
    for (idx, &handle_ptr) in raw.iter().enumerate() {
        let handle = require_ref(handle_ptr, fn_name, &format!("products[{idx}]"))?;
        set.push(handle.inner.clone());
    }
    Ok(set)
}
