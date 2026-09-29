use super::*;

/// Copy the complete typed broadcast-fallback reason as JSON bytes. The payload
/// owns every field of the selection or precise SPP refusal; the legacy typed
/// accessor remains available for its stable compact fields.
///
/// A size query uses `out = NULL`, `len = 0`; the returned byte sequence is not
/// NUL-terminated. `out_written` and `out_required` follow the other byte-copy
/// accessors in this binding.
///
/// Safety: `sol` must be a live handle; `out` must point to `len` writable
/// bytes or be NULL when `len` is zero; both count pointers must be writable.
#[no_mangle]
pub unsafe extern "C" fn sidereon_sourced_solution_broadcast_reason_detail(
    sol: *const SidereonSourcedSolution,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_sourced_solution_broadcast_reason_detail";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        c_try!(init_copy_counts(FN_NAME, out_written, out_required));
        let sol = c_try!(require_ref(sol, FN_NAME, "sol"));
        let payload = match &sol.source {
            FixSource::Precise(_) => {
                set_last_error(format!("{FN_NAME}: fix is from a precise source"));
                return SidereonStatus::InvalidArgument;
            }
            FixSource::Broadcast(BroadcastReason::PreciseUnavailable(error)) => {
                serde_json::json!({
                    "family": "BroadcastReason",
                    "kind": "precise_unavailable",
                    "message": error.to_string(),
                    "selection_error": selection_error_detail(error),
                })
            }
            FixSource::Broadcast(BroadcastReason::PreciseDegradedUnusable { staleness, error }) => {
                serde_json::json!({
                    "family": "BroadcastReason",
                    "kind": "precise_degraded_unusable",
                    "message": error.to_string(),
                    "staleness": {
                        "kind": degradation_kind_name(staleness.kind),
                        "requested_epoch_j2000_s": crate::engine_error::engine_f64(staleness.requested_epoch_j2000_s),
                        "source_epoch_j2000_s": crate::engine_error::engine_f64(staleness.source_epoch_j2000_s),
                        "staleness_s": crate::engine_error::engine_f64(staleness.staleness_s),
                        "staleness_days": crate::engine_error::engine_f64(staleness.staleness_days),
                    },
                    "error": crate::engine_error::spp_error_value(error),
                })
            }
        };
        let bytes = match serde_json::to_vec(&payload) {
            Ok(bytes) => bytes,
            Err(error) => {
                set_last_error(format!("{FN_NAME}: could not serialize detail: {error}"));
                return SidereonStatus::InvalidArgument;
            }
        };
        c_try!(copy_prefix_to_c(
            FN_NAME,
            "out",
            &bytes,
            out,
            len,
            out_written,
            out_required,
        ));
        SidereonStatus::Ok
    })
}

pub(crate) fn selection_error_detail(error: &SelectionError) -> serde_json::Value {
    use SelectionError as E;
    let mut detail = match error {
        E::EmptyProductSet => serde_json::json!({"kind":"empty_product_set"}),
        E::InvalidRange {
            start_epoch_j2000_s,
            end_epoch_j2000_s,
        } => serde_json::json!({
            "kind":"invalid_range",
            "start_epoch_j2000_s":crate::engine_error::engine_f64(*start_epoch_j2000_s),
            "end_epoch_j2000_s":crate::engine_error::engine_f64(*end_epoch_j2000_s),
        }),
        E::NoPriorProduct {
            requested_epoch_j2000_s,
        } => serde_json::json!({
            "kind":"no_prior_product",
            "requested_epoch_j2000_s":crate::engine_error::engine_f64(*requested_epoch_j2000_s),
        }),
        E::BeyondStalenessCap {
            requested_epoch_j2000_s,
            source_epoch_j2000_s,
            staleness_s,
            max_staleness_s,
        } => serde_json::json!({
            "kind":"beyond_staleness_cap",
            "requested_epoch_j2000_s":crate::engine_error::engine_f64(*requested_epoch_j2000_s),
            "source_epoch_j2000_s":crate::engine_error::engine_f64(*source_epoch_j2000_s),
            "staleness_s":crate::engine_error::engine_f64(*staleness_s),
            "max_staleness_s":crate::engine_error::engine_f64(*max_staleness_s),
        }),
        E::InvalidProduct(message) => {
            serde_json::json!({"kind":"invalid_product","message_detail":message})
        }
        E::InvalidPolicy { max_staleness_s } => serde_json::json!({
            "kind":"invalid_policy",
            "max_staleness_s":crate::engine_error::engine_f64(*max_staleness_s),
        }),
        E::Overflow { context } => serde_json::json!({"kind":"overflow","context":context}),
        E::IonexEpoch(error) => serde_json::json!({
            "kind":"ionex_epoch",
            "cause":crate::engine_error::core_error_value(&sidereon_core::Error::IonexEpoch(*error)),
        }),
    };
    detail["family"] = serde_json::json!("SelectionError");
    detail["message"] = serde_json::json!(error.to_string());
    detail
}

fn degradation_kind_name(kind: DegradationKind) -> &'static str {
    match kind {
        DegradationKind::Exact => "exact",
        DegradationKind::NearestPrior => "nearest_prior",
        DegradationKind::DiurnalShift => "diurnal_shift",
    }
}

/// Write which ephemeris source produced the fix to *out_kind.
///
/// Safety: sol must be a live handle; out_kind must point to a
/// SidereonFixSourceKind.
#[no_mangle]
pub unsafe extern "C" fn sidereon_sourced_solution_source_kind(
    sol: *const SidereonSourcedSolution,
    out_kind: *mut SidereonFixSourceKind,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_sourced_solution_source_kind",
        SidereonStatus::Panic,
        || {
            let out_kind = c_try!(require_out(
                out_kind,
                "sidereon_sourced_solution_source_kind",
                "out_kind"
            ));
            *out_kind = SidereonFixSourceKind::Broadcast;
            let sol = c_try!(require_ref(
                sol,
                "sidereon_sourced_solution_source_kind",
                "sol"
            ));
            *out_kind = if sol.source.is_precise() {
                SidereonFixSourceKind::Precise
            } else {
                SidereonFixSourceKind::Broadcast
            };
            SidereonStatus::Ok
        },
    )
}

/// Write whether a precise product covering the exact epoch produced the fix (no
/// degradation, zero staleness) to *out_is_precise_exact.
///
/// Safety: sol must be a live handle; out_is_precise_exact must point to a bool.
#[no_mangle]
pub unsafe extern "C" fn sidereon_sourced_solution_is_precise_exact(
    sol: *const SidereonSourcedSolution,
    out_is_precise_exact: *mut bool,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_sourced_solution_is_precise_exact",
        SidereonStatus::Panic,
        || {
            let out_is_precise_exact = c_try!(require_out(
                out_is_precise_exact,
                "sidereon_sourced_solution_is_precise_exact",
                "out_is_precise_exact"
            ));
            *out_is_precise_exact = false;
            let sol = c_try!(require_ref(
                sol,
                "sidereon_sourced_solution_is_precise_exact",
                "sol"
            ));
            *out_is_precise_exact = sol.source.is_precise_exact();
            SidereonStatus::Ok
        },
    )
}

/// Write the staleness metadata of the source that produced the fix to
/// *out_metadata and whether it is present to *out_present. Present (precise fix):
/// the precise product's staleness. Absent (broadcast fix): the broadcast fix is
/// not backed by a precise product, so *out_present is false and *out_metadata is
/// zeroed; the precise product that was tried, if any, is exposed via
/// sidereon_sourced_solution_broadcast_reason.
///
/// Safety: sol must be a live handle; out_metadata and out_present must point to
/// writable storage of the documented type, and the output ranges must be
/// disjoint.
#[no_mangle]
pub unsafe extern "C" fn sidereon_sourced_solution_staleness(
    sol: *const SidereonSourcedSolution,
    out_metadata: *mut SidereonStalenessMetadata,
    out_present: *mut bool,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_sourced_solution_staleness",
        SidereonStatus::Panic,
        || {
            if !out_metadata.is_null() && !out_present.is_null() {
                let outputs = [
                    Some((
                        c_try!(super::checked_output_range(
                            "sidereon_sourced_solution_staleness",
                            out_metadata,
                            1,
                            "out_metadata"
                        )),
                        "out_metadata",
                    )),
                    Some((
                        c_try!(super::checked_output_range(
                            "sidereon_sourced_solution_staleness",
                            out_present,
                            1,
                            "out_present"
                        )),
                        "out_present",
                    )),
                ];
                c_try!(super::reject_overlapping_optional_outputs(
                    "sidereon_sourced_solution_staleness",
                    &outputs
                ));
            }
            let out_metadata = c_try!(require_out(
                out_metadata,
                "sidereon_sourced_solution_staleness",
                "out_metadata"
            ));
            *out_metadata = empty_staleness_metadata();
            let out_present = c_try!(require_out(
                out_present,
                "sidereon_sourced_solution_staleness",
                "out_present"
            ));
            *out_present = false;
            let sol = c_try!(require_ref(
                sol,
                "sidereon_sourced_solution_staleness",
                "sol"
            ));
            if let Some(metadata) = sol.source.staleness() {
                *out_metadata = staleness_metadata_to_c(metadata);
                *out_present = true;
            }
            SidereonStatus::Ok
        },
    )
}

/// Read the broadcast-fallback reason for a broadcast-sourced fix. Writes the
/// reason kind to *out_kind. For PreciseUnavailable, the precise selection's typed
/// rejection is written to *out_precise_unavailable_reason. For
/// PreciseDegradedUnusable, the tried precise product's staleness is written to
/// *out_attempted_staleness with *out_has_attempted_staleness set true.
///
/// Returns SIDEREON_STATUS_INVALID_ARGUMENT if the fix came from a precise source
/// (use sidereon_sourced_solution_staleness for that case).
///
/// Safety: sol must be a live handle; every out pointer must point to writable
/// storage of the documented type, and the output ranges must be disjoint.
#[no_mangle]
pub unsafe extern "C" fn sidereon_sourced_solution_broadcast_reason(
    sol: *const SidereonSourcedSolution,
    out_kind: *mut SidereonBroadcastReasonKind,
    out_precise_unavailable_reason: *mut SidereonSelectionStatus,
    out_attempted_staleness: *mut SidereonStalenessMetadata,
    out_has_attempted_staleness: *mut bool,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_sourced_solution_broadcast_reason",
        SidereonStatus::Panic,
        || {
            if !out_kind.is_null()
                && !out_precise_unavailable_reason.is_null()
                && !out_attempted_staleness.is_null()
                && !out_has_attempted_staleness.is_null()
            {
                let outputs = [
                    if out_kind.is_null() {
                        None
                    } else {
                        Some((
                            c_try!(super::checked_output_range(
                                "sidereon_sourced_solution_broadcast_reason",
                                out_kind,
                                1,
                                "out_kind"
                            )),
                            "out_kind",
                        ))
                    },
                    if out_precise_unavailable_reason.is_null() {
                        None
                    } else {
                        Some((
                            c_try!(super::checked_output_range(
                                "sidereon_sourced_solution_broadcast_reason",
                                out_precise_unavailable_reason,
                                1,
                                "out_precise_unavailable_reason"
                            )),
                            "out_precise_unavailable_reason",
                        ))
                    },
                    if out_attempted_staleness.is_null() {
                        None
                    } else {
                        Some((
                            c_try!(super::checked_output_range(
                                "sidereon_sourced_solution_broadcast_reason",
                                out_attempted_staleness,
                                1,
                                "out_attempted_staleness"
                            )),
                            "out_attempted_staleness",
                        ))
                    },
                    if out_has_attempted_staleness.is_null() {
                        None
                    } else {
                        Some((
                            c_try!(super::checked_output_range(
                                "sidereon_sourced_solution_broadcast_reason",
                                out_has_attempted_staleness,
                                1,
                                "out_has_attempted_staleness"
                            )),
                            "out_has_attempted_staleness",
                        ))
                    },
                ];
                c_try!(super::reject_overlapping_optional_outputs(
                    "sidereon_sourced_solution_broadcast_reason",
                    &outputs
                ));
            }
            let out_kind = c_try!(require_out(
                out_kind,
                "sidereon_sourced_solution_broadcast_reason",
                "out_kind"
            ));
            *out_kind = SidereonBroadcastReasonKind::PreciseUnavailable;
            let out_precise_unavailable_reason = c_try!(require_out(
                out_precise_unavailable_reason,
                "sidereon_sourced_solution_broadcast_reason",
                "out_precise_unavailable_reason"
            ));
            *out_precise_unavailable_reason = SidereonSelectionStatus::Ok;
            let out_attempted_staleness = c_try!(require_out(
                out_attempted_staleness,
                "sidereon_sourced_solution_broadcast_reason",
                "out_attempted_staleness"
            ));
            *out_attempted_staleness = empty_staleness_metadata();
            let out_has_attempted_staleness = c_try!(require_out(
                out_has_attempted_staleness,
                "sidereon_sourced_solution_broadcast_reason",
                "out_has_attempted_staleness"
            ));
            *out_has_attempted_staleness = false;
            let sol = c_try!(require_ref(
                sol,
                "sidereon_sourced_solution_broadcast_reason",
                "sol"
            ));
            match &sol.source {
                FixSource::Precise(_) => {
                    set_last_error(
                        "sidereon_sourced_solution_broadcast_reason: fix is from a precise source"
                            .to_string(),
                    );
                    SidereonStatus::InvalidArgument
                }
                FixSource::Broadcast(reason) => {
                    match reason {
                        BroadcastReason::PreciseUnavailable(selection_error) => {
                            *out_kind = SidereonBroadcastReasonKind::PreciseUnavailable;
                            *out_precise_unavailable_reason =
                                selection_error_to_status(selection_error);
                        }
                        BroadcastReason::PreciseDegradedUnusable { staleness, .. } => {
                            *out_kind = SidereonBroadcastReasonKind::PreciseDegradedUnusable;
                            *out_attempted_staleness = staleness_metadata_to_c(*staleness);
                            *out_has_attempted_staleness = true;
                        }
                    }
                    SidereonStatus::Ok
                }
            }
        },
    )
}

/// Copy the receiver solution out of a sourced solution into a newly owned
/// SidereonSppSolution, so the full spp solution accessors apply. Release the new
/// handle with sidereon_spp_solution_free.
///
/// Safety: sol must be a live handle; out_solution must point to storage for a
/// SidereonSppSolution*.
#[no_mangle]
pub unsafe extern "C" fn sidereon_sourced_solution_solution(
    sol: *const SidereonSourcedSolution,
    out_solution: *mut *mut SidereonSppSolution,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_sourced_solution_solution",
        SidereonStatus::Panic,
        || {
            let out_solution = c_try!(require_out(
                out_solution,
                "sidereon_sourced_solution_solution",
                "out_solution"
            ));
            *out_solution = ptr::null_mut();
            let sol = c_try!(require_ref(
                sol,
                "sidereon_sourced_solution_solution",
                "sol"
            ));
            write_boxed_handle(
                out_solution,
                SidereonSppSolution {
                    inner: sol.solution.clone(),
                },
            );
            SidereonStatus::Ok
        },
    )
}

/// Release a sourced-solution handle from sidereon_solve_with_fallback. Passing
/// NULL is a no-op.
///
/// Safety: sol must be NULL or a live handle from sidereon_solve_with_fallback
/// that has not already been freed.
#[no_mangle]
pub unsafe extern "C" fn sidereon_sourced_solution_free(sol: *mut SidereonSourcedSolution) {
    ffi_boundary("sidereon_sourced_solution_free", (), || {
        free_boxed(sol);
    });
}

// === Fault detection and exclusion (FDE) ===================================
//
// A RAIM-driven solve-and-exclude loop: solve, run the residual chi-square RAIM
// test, drop the worst satellite, and repeat until the measurement set is
// self-consistent or the exclusion budget is spent. The whole loop, the
// per-iteration solve, and the candidate validation are the engine's own
// (sidereon_core::quality::fde_spp, which chains sidereon_core::positioning::solve
// and sidereon_core::quality::validate_receiver_solution); this binding only
// marshals the options and surfaces the surviving solution, the excluded
// satellites, and the iteration count. No detection or exclusion logic lives here.
