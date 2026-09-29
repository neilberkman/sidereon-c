//! The SP3 merge audit trail and product coverage: the merge report's omitted
//! epochs, clock omissions, dropped input epochs, per-cell agreement and
//! per-epoch provenance, the node selection a window's interpolations make, and
//! the per-satellite coverage of a product. Every record carries its epoch as
//! the exact instant (SidereonClockEpoch) and as seconds since J2000.

use super::*;
use sidereon_core::ephemeris::{
    AgreementMetric, CellProvenance, CellSelection, ClockOmission, ClockOmissionReason,
    ContributorCoverage, DroppedEpochReason, DroppedInputEpoch, EpochWindow, InterpolationNodes,
    MergeCombine, MergeProvenance, PrecedenceTransition, ProvenanceMode, Sp3ChannelCoverage,
    Sp3Coverage, Sp3CoverageGap, Sp3CoverageSpan, TransitionReason,
};

/// How much per-epoch provenance an SP3 merge records
/// (sidereon_core::ephemeris::ProvenanceMode, or none).
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonSp3ProvenanceMode {
    /// Record no provenance (the engine default). The report then says so:
    /// sidereon_sp3_merge_report_provenance reports recorded false.
    Off = 0,
    /// Selection transitions and per-contributor coverage only.
    Summary = 1,
    /// Everything Summary records, plus one entry per accepted cell.
    Full = 2,
}

/// Which channel of a merged cell or a satellite's coverage a call reads.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonSp3Channel {
    /// Positions (orbit records).
    Position = 0,
    /// Clocks.
    Clock = 1,
}

/// How the merge arrived at the value it wrote for one channel of one cell
/// (sidereon_core::ephemeris::CellSelection).
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonSp3CellSelectionKind {
    /// One source carried the cell and was carried through. Carries source.
    SingleSource = 0,
    /// Precedence picked one source out of an agreeing set. Carries source
    /// and the members.
    Precedence = 1,
    /// The written value combines the members under rule; no single source
    /// supplied it. Carries rule and the members.
    Combined = 2,
}

/// One cell channel's selection. The members (every source in the accepted
/// consensus, ascending) are copied by
/// sidereon_sp3_merge_report_provenance_cell_members.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonSp3CellSelection {
    /// How the value was arrived at.
    pub kind: SidereonSp3CellSelectionKind,
    /// Whether source names the single source whose value was written
    /// (SingleSource and Precedence).
    pub has_source: bool,
    /// Index into the merge's input list of the source written.
    pub source: usize,
    /// Whether rule carries the combining rule (Combined).
    pub has_rule: bool,
    /// The rule that produced a Combined value.
    pub rule: SidereonSp3MergeCombine,
    /// Number of consensus members.
    pub member_count: usize,
}

/// Provenance of one accepted cell, recorded as the merge decided it.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonSp3CellProvenance {
    /// The epoch, exact.
    pub epoch: SidereonClockEpoch,
    /// The epoch as seconds since J2000; NaN when the exact epoch has no such
    /// reading.
    pub epoch_j2000_seconds: f64,
    /// Satellite token.
    pub sat_id: SidereonSatelliteToken,
    /// Whether the cell carries a position, so position is meaningful.
    pub has_position: bool,
    /// How the written position was arrived at.
    pub position: SidereonSp3CellSelection,
    /// Whether the cell carries a clock, so clock is meaningful.
    pub has_clock: bool,
    /// How the written clock was arrived at.
    pub clock: SidereonSp3CellSelection,
}

/// Why the source supplying a satellite's position changed
/// (sidereon_core::ephemeris::TransitionReason).
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonSp3TransitionReason {
    /// The previously selected source no longer carried the cell.
    SoleAvailability = 0,
    /// Precedence chose a different source that was already available.
    Precedence = 1,
    /// The previously selected source was rejected as an outlier.
    OutlierRejection = 2,
    /// The cell moved between single-source and multi-source consensus, or
    /// between combined and single-source selection.
    ConsensusChange = 3,
}

/// One change in which source supplied a satellite's position.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonSp3PrecedenceTransition {
    /// Satellite token.
    pub sat_id: SidereonSatelliteToken,
    /// The epoch at which the new source took over, exact.
    pub epoch: SidereonClockEpoch,
    /// The epoch as seconds since J2000; NaN when the exact epoch has no such
    /// reading.
    pub epoch_j2000_seconds: f64,
    /// Whether from_source names the previous supplier; false at a
    /// satellite's first accepted cell.
    pub has_from_source: bool,
    /// Source supplying the previous accepted cell.
    pub from_source: usize,
    /// Whether to_source names the new supplier; false when the new cell is
    /// combined and so has no single supplier.
    pub has_to_source: bool,
    /// Source supplying this cell.
    pub to_source: usize,
    /// Why selection changed.
    pub reason: SidereonSp3TransitionReason,
}

/// What one contributor supplied to the merged product.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonSp3ContributorCoverage {
    /// Index into the merge's input list.
    pub source: usize,
    /// Accepted cells where this source was in the position or clock consensus.
    pub cells_contributed: usize,
    /// Accepted cells whose written position came from this source alone.
    pub cells_selected: usize,
    /// Whether first_epoch is present.
    pub has_first_epoch: bool,
    /// First accepted cell this source contributed to, exact.
    pub first_epoch: SidereonClockEpoch,
    /// first_epoch as seconds since J2000; NaN when absent or unreadable.
    pub first_epoch_j2000_seconds: f64,
    /// Whether last_epoch is present.
    pub has_last_epoch: bool,
    /// Last accepted cell this source contributed to, exact.
    pub last_epoch: SidereonClockEpoch,
    /// last_epoch as seconds since J2000; NaN when absent or unreadable.
    pub last_epoch_j2000_seconds: f64,
    /// Accepted cells this source contributed nothing to.
    pub cells_absent: usize,
}

/// Whether a merge recorded provenance, in which mode, and the length of each
/// of its lists.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonSp3MergeProvenanceInfo {
    /// Whether provenance was requested and recorded. False is "not
    /// requested", never "one contributor supplied everything".
    pub recorded: bool,
    /// The mode it was recorded in; Off when not recorded.
    pub mode: SidereonSp3ProvenanceMode,
    /// Per-cell entries (zero under Summary).
    pub cell_count: usize,
    /// Selection transitions.
    pub transition_count: usize,
    /// Per-contributor coverage entries, one per input source.
    pub coverage_count: usize,
}

/// A union-grid epoch at which the merge accepted no cell, so the merged
/// product does not carry it.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonSp3MergeEpoch {
    /// The epoch, exact.
    pub epoch: SidereonClockEpoch,
    /// The epoch as seconds since J2000; NaN when the exact epoch has no such
    /// reading.
    pub epoch_j2000_seconds: f64,
}

/// Why a source's clock for a cell was not written
/// (sidereon_core::ephemeris::ClockOmissionReason).
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonSp3ClockOmissionReason {
    /// The source's datum offset to source 0 could not be estimated at this
    /// epoch; it is never extrapolated.
    DatumNotObservable = 0,
    /// Precedence writes a clock only from the preferred source, which had no
    /// clock on the reference datum. Carries the preferred source when the
    /// merge had one.
    PreferredSourceWithoutClock = 1,
    /// Clocks disagreed and no agreeing subset met the consensus rule.
    NoConsensus = 2,
}

/// One source's clock for a cell that the merge did not write.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonSp3ClockOmission {
    /// The epoch, exact.
    pub epoch: SidereonClockEpoch,
    /// The epoch as seconds since J2000; NaN when the exact epoch has no such
    /// reading.
    pub epoch_j2000_seconds: f64,
    /// Satellite token.
    pub sat_id: SidereonSatelliteToken,
    /// Index into the merge's input list of the source whose clock was not
    /// written.
    pub source: usize,
    /// Why it was not written.
    pub reason: SidereonSp3ClockOmissionReason,
    /// Whether preferred carries the preferred source of a
    /// PreferredSourceWithoutClock omission.
    pub has_preferred: bool,
    /// The preferred source.
    pub preferred: usize,
    /// Whether the merged cell carries a clock from other sources.
    pub cell_has_clock: bool,
}

/// Why an input epoch took no part in a merge
/// (sidereon_core::ephemeris::DroppedEpochReason).
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonSp3DroppedEpochReason {
    /// Not on the explicit target_epoch_interval_s grid.
    OffTargetGrid = 0,
    /// Not a whole number of the 10-nanosecond ticks an SP3 epoch record
    /// resolves.
    NotOnTickAxis = 1,
}

/// An input epoch that took no part in a merge.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonSp3DroppedInputEpoch {
    /// Index into the merge's input list.
    pub source: usize,
    /// Index into that source's epochs.
    pub epoch_index: usize,
    /// The epoch, exact.
    pub epoch: SidereonClockEpoch,
    /// The epoch as seconds since J2000; NaN when the exact epoch has no such
    /// reading.
    pub epoch_j2000_seconds: f64,
    /// Why it took no part.
    pub reason: SidereonSp3DroppedEpochReason,
}

/// Agreement statistics for one accepted cell: how tightly the consensus
/// members cluster about the value written. Absent statistics are NaN with a
/// false present flag, never 0.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonSp3AgreementMetric {
    /// The epoch, exact.
    pub epoch: SidereonClockEpoch,
    /// The epoch as seconds since J2000; NaN when the exact epoch has no such
    /// reading.
    pub epoch_j2000_seconds: f64,
    /// Satellite token.
    pub sat_id: SidereonSatelliteToken,
    /// Sources in the accepted position consensus (0 when the cell carries no
    /// position).
    pub position_members: usize,
    /// Whether position_rms_m is present (the cell carries a position).
    pub has_position_rms_m: bool,
    /// RMS 3D distance of the position members from the written position,
    /// meters; zero for a single-source cell.
    pub position_rms_m: f64,
    /// Whether position_max_m is present.
    pub has_position_max_m: bool,
    /// Largest 3D distance of a position member from the written position,
    /// meters.
    pub position_max_m: f64,
    /// Sources in the accepted clock consensus (0 when the cell carries no
    /// clock).
    pub clock_members: usize,
    /// Whether clock_rms_s is present (the cell carries a clock).
    pub has_clock_rms_s: bool,
    /// RMS deviation of the clock members from the written clock, seconds.
    pub clock_rms_s: f64,
    /// Whether clock_max_s is present.
    pub has_clock_max_s: bool,
    /// Largest absolute clock deviation from the written clock, seconds.
    pub clock_max_s: f64,
}

/// The per-satellite coverage of one SP3 product (sidereon_core Sp3Coverage).
/// Opaque to C. Create with sidereon_sp3_satellite_coverage and release with
/// sidereon_sp3_coverage_free.
pub struct SidereonSp3Coverage {
    pub(crate) inner: Sp3Coverage,
}

/// The epoch grid a product's epochs lie on.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonSp3EpochGrid {
    /// Whether the epochs lie on one grid, so interval_s is present.
    pub has_interval: bool,
    /// The grid step, seconds; NaN when absent.
    pub interval_s: f64,
    /// Whether the grid step equals the header's declared interval.
    pub agrees_with_header: bool,
    /// Epoch indices out of time order, copied by
    /// sidereon_sp3_coverage_grid_out_of_order.
    pub out_of_order_count: usize,
    /// Epoch indices no record states exactly, copied by
    /// sidereon_sp3_coverage_grid_unplaced.
    pub unplaced_count: usize,
}

/// Coverage of one channel of one satellite.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonSp3ChannelCoverage {
    /// Product epochs that carry the channel.
    pub epochs: usize,
    /// Contiguous spans, copied by sidereon_sp3_coverage_spans.
    pub span_count: usize,
    /// Gaps, copied by sidereon_sp3_coverage_gaps.
    pub gap_count: usize,
    /// Whether the channel is present at every product epoch, in one span.
    pub complete: bool,
}

/// Coverage of one satellite.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonSp3SatelliteCoverage {
    /// Satellite token.
    pub sat_id: SidereonSatelliteToken,
    /// Whether the header declares the satellite.
    pub declared: bool,
    /// Position coverage.
    pub positions: SidereonSp3ChannelCoverage,
    /// Clock coverage.
    pub clocks: SidereonSp3ChannelCoverage,
}

/// One contiguous run of product epochs that carry a channel.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonSp3CoverageSpan {
    /// Index of the first epoch of the span.
    pub first_index: usize,
    /// Index of the last epoch of the span.
    pub last_index: usize,
    /// The first epoch, exact.
    pub first_epoch: SidereonClockEpoch,
    /// first_epoch as seconds since J2000; NaN when unreadable.
    pub first_epoch_j2000_seconds: f64,
    /// The last epoch, exact.
    pub last_epoch: SidereonClockEpoch,
    /// last_epoch as seconds since J2000; NaN when unreadable.
    pub last_epoch_j2000_seconds: f64,
}

/// A gap in a channel's coverage.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonSp3CoverageGap {
    /// Whether after_index is present; false for a gap before the first span.
    pub has_after_index: bool,
    /// Index of the last covered epoch before the gap.
    pub after_index: usize,
    /// Whether before_index is present; false for a gap after the last span.
    pub has_before_index: bool,
    /// Index of the first covered epoch after the gap.
    pub before_index: usize,
    /// Grid epochs the gap spans that the channel does not carry, whether or
    /// not the product has an epoch there.
    pub missing_epochs: usize,
}

// --- conversions ---------------------------------------------------------------

fn exact_epoch_to_c(epoch: &Instant) -> (SidereonClockEpoch, f64) {
    (
        crate::rinex_clock::instant_to_clock_epoch(epoch),
        instant_to_j2000_seconds(epoch).unwrap_or(f64::NAN),
    )
}

fn optional_epoch_to_c(epoch: Option<&Instant>) -> (bool, SidereonClockEpoch, f64) {
    match epoch {
        Some(epoch) => {
            let (exact, seconds) = exact_epoch_to_c(epoch);
            (true, exact, seconds)
        }
        None => (false, crate::rinex_clock::absent_clock_epoch(), f64::NAN),
    }
}

fn merge_combine_to_c(rule: MergeCombine) -> SidereonSp3MergeCombine {
    match rule {
        MergeCombine::Mean => SidereonSp3MergeCombine::Mean,
        MergeCombine::Median => SidereonSp3MergeCombine::Median,
        MergeCombine::Precedence => SidereonSp3MergeCombine::Precedence,
    }
}

fn no_cell_selection() -> SidereonSp3CellSelection {
    SidereonSp3CellSelection {
        kind: SidereonSp3CellSelectionKind::SingleSource,
        has_source: false,
        source: 0,
        has_rule: false,
        rule: SidereonSp3MergeCombine::Mean,
        member_count: 0,
    }
}

fn cell_selection_to_c(selection: &CellSelection) -> SidereonSp3CellSelection {
    let mut out = no_cell_selection();
    match selection {
        CellSelection::SingleSource { source } => {
            out.kind = SidereonSp3CellSelectionKind::SingleSource;
            out.has_source = true;
            out.source = *source;
            out.member_count = 1;
        }
        CellSelection::Precedence { source, members } => {
            out.kind = SidereonSp3CellSelectionKind::Precedence;
            out.has_source = true;
            out.source = *source;
            out.member_count = members.len();
        }
        CellSelection::Combined { rule, members } => {
            out.kind = SidereonSp3CellSelectionKind::Combined;
            out.has_rule = true;
            out.rule = merge_combine_to_c(*rule);
            out.member_count = members.len();
        }
    }
    out
}

fn cell_provenance_to_c(cell: &CellProvenance) -> SidereonSp3CellProvenance {
    let (epoch, epoch_j2000_seconds) = exact_epoch_to_c(&cell.epoch);
    SidereonSp3CellProvenance {
        epoch,
        epoch_j2000_seconds,
        sat_id: satellite_token(cell.satellite),
        has_position: cell.position.is_some(),
        position: cell
            .position
            .as_ref()
            .map_or_else(no_cell_selection, cell_selection_to_c),
        has_clock: cell.clock.is_some(),
        clock: cell
            .clock
            .as_ref()
            .map_or_else(no_cell_selection, cell_selection_to_c),
    }
}

fn transition_reason_to_c(reason: TransitionReason) -> SidereonSp3TransitionReason {
    match reason {
        TransitionReason::SoleAvailability => SidereonSp3TransitionReason::SoleAvailability,
        TransitionReason::Precedence => SidereonSp3TransitionReason::Precedence,
        TransitionReason::OutlierRejection => SidereonSp3TransitionReason::OutlierRejection,
        TransitionReason::ConsensusChange => SidereonSp3TransitionReason::ConsensusChange,
    }
}

fn transition_to_c(transition: &PrecedenceTransition) -> SidereonSp3PrecedenceTransition {
    let (epoch, epoch_j2000_seconds) = exact_epoch_to_c(&transition.epoch);
    SidereonSp3PrecedenceTransition {
        sat_id: satellite_token(transition.satellite),
        epoch,
        epoch_j2000_seconds,
        has_from_source: transition.from_source.is_some(),
        from_source: transition.from_source.unwrap_or(0),
        has_to_source: transition.to_source.is_some(),
        to_source: transition.to_source.unwrap_or(0),
        reason: transition_reason_to_c(transition.reason),
    }
}

fn contributor_coverage_to_c(coverage: &ContributorCoverage) -> SidereonSp3ContributorCoverage {
    let (has_first_epoch, first_epoch, first_epoch_j2000_seconds) =
        optional_epoch_to_c(coverage.first_epoch.as_ref());
    let (has_last_epoch, last_epoch, last_epoch_j2000_seconds) =
        optional_epoch_to_c(coverage.last_epoch.as_ref());
    SidereonSp3ContributorCoverage {
        source: coverage.source,
        cells_contributed: coverage.cells_contributed,
        cells_selected: coverage.cells_selected,
        has_first_epoch,
        first_epoch,
        first_epoch_j2000_seconds,
        has_last_epoch,
        last_epoch,
        last_epoch_j2000_seconds,
        cells_absent: coverage.cells_absent,
    }
}

fn provenance_mode_to_c(mode: ProvenanceMode) -> SidereonSp3ProvenanceMode {
    match mode {
        ProvenanceMode::Summary => SidereonSp3ProvenanceMode::Summary,
        ProvenanceMode::Full => SidereonSp3ProvenanceMode::Full,
    }
}

/// The engine provenance mode a C selector names; Off is no provenance.
pub(crate) fn provenance_mode_from_c(
    fn_name: &str,
    value: u32,
) -> Result<Option<ProvenanceMode>, SidereonStatus> {
    match value {
        v if v == SidereonSp3ProvenanceMode::Off as u32 => Ok(None),
        v if v == SidereonSp3ProvenanceMode::Summary as u32 => Ok(Some(ProvenanceMode::Summary)),
        v if v == SidereonSp3ProvenanceMode::Full as u32 => Ok(Some(ProvenanceMode::Full)),
        other => {
            set_last_error(format!("{fn_name}: invalid provenance mode {other}"));
            Err(SidereonStatus::InvalidArgument)
        }
    }
}

fn provenance_info_to_c(provenance: Option<&MergeProvenance>) -> SidereonSp3MergeProvenanceInfo {
    match provenance {
        Some(provenance) => SidereonSp3MergeProvenanceInfo {
            recorded: true,
            mode: provenance_mode_to_c(provenance.mode),
            cell_count: provenance.cells.len(),
            transition_count: provenance.transitions.len(),
            coverage_count: provenance.coverage.len(),
        },
        None => SidereonSp3MergeProvenanceInfo {
            recorded: false,
            mode: SidereonSp3ProvenanceMode::Off,
            cell_count: 0,
            transition_count: 0,
            coverage_count: 0,
        },
    }
}

fn clock_omission_to_c(omission: &ClockOmission) -> SidereonSp3ClockOmission {
    let (epoch, epoch_j2000_seconds) = exact_epoch_to_c(&omission.epoch);
    let (reason, preferred) = match omission.reason {
        ClockOmissionReason::DatumNotObservable => {
            (SidereonSp3ClockOmissionReason::DatumNotObservable, None)
        }
        ClockOmissionReason::PreferredSourceWithoutClock { preferred } => (
            SidereonSp3ClockOmissionReason::PreferredSourceWithoutClock,
            preferred,
        ),
        ClockOmissionReason::NoConsensus => (SidereonSp3ClockOmissionReason::NoConsensus, None),
    };
    SidereonSp3ClockOmission {
        epoch,
        epoch_j2000_seconds,
        sat_id: satellite_token(omission.satellite),
        source: omission.source,
        reason,
        has_preferred: preferred.is_some(),
        preferred: preferred.unwrap_or(0),
        cell_has_clock: omission.cell_has_clock,
    }
}

fn dropped_input_epoch_to_c(dropped: &DroppedInputEpoch) -> SidereonSp3DroppedInputEpoch {
    let (epoch, epoch_j2000_seconds) = exact_epoch_to_c(&dropped.epoch);
    SidereonSp3DroppedInputEpoch {
        source: dropped.source,
        epoch_index: dropped.epoch_index,
        epoch,
        epoch_j2000_seconds,
        reason: match dropped.reason {
            DroppedEpochReason::OffTargetGrid => SidereonSp3DroppedEpochReason::OffTargetGrid,
            DroppedEpochReason::NotOnTickAxis => SidereonSp3DroppedEpochReason::NotOnTickAxis,
        },
    }
}

fn agreement_metric_to_c(metric: &AgreementMetric) -> SidereonSp3AgreementMetric {
    let (epoch, epoch_j2000_seconds) = exact_epoch_to_c(&metric.epoch);
    SidereonSp3AgreementMetric {
        epoch,
        epoch_j2000_seconds,
        sat_id: satellite_token(metric.satellite),
        position_members: metric.position_members,
        has_position_rms_m: metric.position_rms_m.is_some(),
        position_rms_m: metric.position_rms_m.unwrap_or(f64::NAN),
        has_position_max_m: metric.position_max_m.is_some(),
        position_max_m: metric.position_max_m.unwrap_or(f64::NAN),
        clock_members: metric.clock_members,
        has_clock_rms_s: metric.clock_rms_s.is_some(),
        clock_rms_s: metric.clock_rms_s.unwrap_or(f64::NAN),
        has_clock_max_s: metric.clock_max_s.is_some(),
        clock_max_s: metric.clock_max_s.unwrap_or(f64::NAN),
    }
}

fn channel_coverage_to_c(channel: &Sp3ChannelCoverage) -> SidereonSp3ChannelCoverage {
    SidereonSp3ChannelCoverage {
        epochs: channel.epochs,
        span_count: channel.spans.len(),
        gap_count: channel.gaps.len(),
        complete: channel.is_complete(),
    }
}

fn coverage_span_to_c(span: &Sp3CoverageSpan) -> SidereonSp3CoverageSpan {
    let (first_epoch, first_epoch_j2000_seconds) = exact_epoch_to_c(&span.first_epoch);
    let (last_epoch, last_epoch_j2000_seconds) = exact_epoch_to_c(&span.last_epoch);
    SidereonSp3CoverageSpan {
        first_index: span.first_index,
        last_index: span.last_index,
        first_epoch,
        first_epoch_j2000_seconds,
        last_epoch,
        last_epoch_j2000_seconds,
    }
}

fn coverage_gap_to_c(gap: &Sp3CoverageGap) -> SidereonSp3CoverageGap {
    SidereonSp3CoverageGap {
        has_after_index: gap.after_index.is_some(),
        after_index: gap.after_index.unwrap_or(0),
        has_before_index: gap.before_index.is_some(),
        before_index: gap.before_index.unwrap_or(0),
        missing_epochs: gap.missing_epochs,
    }
}

fn channel_from_c(fn_name: &str, channel: u32) -> Result<SidereonSp3Channel, SidereonStatus> {
    match channel {
        v if v == SidereonSp3Channel::Position as u32 => Ok(SidereonSp3Channel::Position),
        v if v == SidereonSp3Channel::Clock as u32 => Ok(SidereonSp3Channel::Clock),
        other => {
            set_last_error(format!("{fn_name}: invalid channel {other}"));
            Err(SidereonStatus::InvalidArgument)
        }
    }
}

/// Copy `values` under the variable-length output contract after clearing and
/// checking the counts.
unsafe fn copy_rows<T: Copy>(
    fn_name: &str,
    values: &[T],
    out: *mut T,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    c_try!(init_copy_counts(fn_name, out_written, out_required));
    c_try!(copy_prefix_to_c(
        fn_name,
        "out",
        values,
        out,
        len,
        out_written,
        out_required
    ));
    SidereonStatus::Ok
}

// --- merge report lists -------------------------------------------------------

/// Copy the union-grid epochs at which the merge accepted no cell, in time
/// order. The merged product does not carry them; every position a source
/// carried there is an arc-withheld or quarantined flag, and every clock a
/// clock omission. Uses the variable-length output contract documented at the
/// top of the header.
///
/// Safety: report must be a live merge report handle; out must point to len
/// writable SidereonSp3MergeEpoch values or be NULL when len is 0; out_written
/// and out_required must point to size_t values.
#[no_mangle]
pub unsafe extern "C" fn sidereon_sp3_merge_report_omitted_epochs(
    report: *const SidereonSp3MergeReport,
    out: *mut SidereonSp3MergeEpoch,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_sp3_merge_report_omitted_epochs";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        c_try!(init_copy_counts(FN_NAME, out_written, out_required));
        let report = c_try!(require_ref(report, FN_NAME, "report"));
        let rows: Vec<SidereonSp3MergeEpoch> = report
            .inner
            .omitted_epochs
            .iter()
            .map(|epoch| {
                let (epoch, epoch_j2000_seconds) = exact_epoch_to_c(epoch);
                SidereonSp3MergeEpoch {
                    epoch,
                    epoch_j2000_seconds,
                }
            })
            .collect();
        copy_rows(FN_NAME, &rows, out, len, out_written, out_required)
    })
}

/// Copy every source clock the merge did not write, with the reason, in
/// (epoch, satellite, source) order. Uses the variable-length output contract.
///
/// Safety: report must be a live merge report handle; out must point to len
/// writable SidereonSp3ClockOmission values or be NULL when len is 0;
/// out_written and out_required must point to size_t values.
#[no_mangle]
pub unsafe extern "C" fn sidereon_sp3_merge_report_clock_omissions(
    report: *const SidereonSp3MergeReport,
    out: *mut SidereonSp3ClockOmission,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_sp3_merge_report_clock_omissions";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        c_try!(init_copy_counts(FN_NAME, out_written, out_required));
        let report = c_try!(require_ref(report, FN_NAME, "report"));
        let rows: Vec<SidereonSp3ClockOmission> = report
            .inner
            .clock_omissions
            .iter()
            .map(clock_omission_to_c)
            .collect();
        copy_rows(FN_NAME, &rows, out, len, out_written, out_required)
    })
}

/// Copy the input epochs that took no part in the merge, with the reason, in
/// (source, epoch) order. Uses the variable-length output contract.
///
/// Safety: report must be a live merge report handle; out must point to len
/// writable SidereonSp3DroppedInputEpoch values or be NULL when len is 0;
/// out_written and out_required must point to size_t values.
#[no_mangle]
pub unsafe extern "C" fn sidereon_sp3_merge_report_dropped_input_epochs(
    report: *const SidereonSp3MergeReport,
    out: *mut SidereonSp3DroppedInputEpoch,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_sp3_merge_report_dropped_input_epochs";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        c_try!(init_copy_counts(FN_NAME, out_written, out_required));
        let report = c_try!(require_ref(report, FN_NAME, "report"));
        let rows: Vec<SidereonSp3DroppedInputEpoch> = report
            .inner
            .dropped_input_epochs
            .iter()
            .map(dropped_input_epoch_to_c)
            .collect();
        copy_rows(FN_NAME, &rows, out, len, out_written, out_required)
    })
}

/// Copy the agreement statistics of every accepted cell, in output (epoch,
/// then satellite) order: one entry per cell written to the merged product.
/// Uses the variable-length output contract.
///
/// Safety: report must be a live merge report handle; out must point to len
/// writable SidereonSp3AgreementMetric values or be NULL when len is 0;
/// out_written and out_required must point to size_t values.
#[no_mangle]
pub unsafe extern "C" fn sidereon_sp3_merge_report_agreement_metrics(
    report: *const SidereonSp3MergeReport,
    out: *mut SidereonSp3AgreementMetric,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_sp3_merge_report_agreement_metrics";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        c_try!(init_copy_counts(FN_NAME, out_written, out_required));
        let report = c_try!(require_ref(report, FN_NAME, "report"));
        let rows: Vec<SidereonSp3AgreementMetric> = report
            .inner
            .agreement
            .iter()
            .map(agreement_metric_to_c)
            .collect();
        copy_rows(FN_NAME, &rows, out, len, out_written, out_required)
    })
}

// --- merge provenance ---------------------------------------------------------

/// Write whether the merge recorded per-epoch provenance, its mode and the
/// length of each list. Provenance is recorded only when
/// SidereonSp3MergeOptions.provenance_mode asked for it; recorded false means
/// it was not requested.
///
/// Safety: report must be a live merge report handle; out_info must point to a
/// SidereonSp3MergeProvenanceInfo.
#[no_mangle]
pub unsafe extern "C" fn sidereon_sp3_merge_report_provenance(
    report: *const SidereonSp3MergeReport,
    out_info: *mut SidereonSp3MergeProvenanceInfo,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_sp3_merge_report_provenance";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        let out = c_try!(require_out(out_info, FN_NAME, "out_info"));
        *out = provenance_info_to_c(None);
        let report = c_try!(require_ref(report, FN_NAME, "report"));
        *out = provenance_info_to_c(report.inner.provenance.as_ref());
        SidereonStatus::Ok
    })
}

/// Copy the per-cell provenance entries, one per accepted cell in output
/// order (Full mode only; Summary and unrecorded provenance copy nothing).
/// Uses the variable-length output contract.
///
/// Safety: report must be a live merge report handle; out must point to len
/// writable SidereonSp3CellProvenance values or be NULL when len is 0;
/// out_written and out_required must point to size_t values.
#[no_mangle]
pub unsafe extern "C" fn sidereon_sp3_merge_report_provenance_cells(
    report: *const SidereonSp3MergeReport,
    out: *mut SidereonSp3CellProvenance,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_sp3_merge_report_provenance_cells";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        c_try!(init_copy_counts(FN_NAME, out_written, out_required));
        let report = c_try!(require_ref(report, FN_NAME, "report"));
        let rows: Vec<SidereonSp3CellProvenance> = report
            .inner
            .provenance
            .as_ref()
            .map(|provenance| provenance.cells.iter().map(cell_provenance_to_c).collect())
            .unwrap_or_default();
        copy_rows(FN_NAME, &rows, out, len, out_written, out_required)
    })
}

/// Copy the consensus members of one provenance cell's channel (channel is a
/// SidereonSp3Channel value), ascending. A channel the cell does not carry
/// copies nothing; a cell index past the provenance cells is refused with
/// SIDEREON_STATUS_INVALID_ARGUMENT. Uses the variable-length output contract.
///
/// Safety: report must be a live merge report handle; out must point to len
/// writable size_t values or be NULL when len is 0; out_written and
/// out_required must point to size_t values.
#[no_mangle]
pub unsafe extern "C" fn sidereon_sp3_merge_report_provenance_cell_members(
    report: *const SidereonSp3MergeReport,
    index: usize,
    channel: u32,
    out: *mut usize,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_sp3_merge_report_provenance_cell_members";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        c_try!(init_copy_counts(FN_NAME, out_written, out_required));
        let report = c_try!(require_ref(report, FN_NAME, "report"));
        let channel = c_try!(channel_from_c(FN_NAME, channel));
        let cells = report
            .inner
            .provenance
            .as_ref()
            .map_or(&[][..], |provenance| &provenance.cells[..]);
        let Some(cell) = cells.get(index) else {
            set_last_error(format!(
                "{FN_NAME}: cell index {index} out of range ({} cells)",
                cells.len()
            ));
            return SidereonStatus::InvalidArgument;
        };
        let selection = match channel {
            SidereonSp3Channel::Position => cell.position.as_ref(),
            SidereonSp3Channel::Clock => cell.clock.as_ref(),
        };
        let members = selection.map(CellSelection::members).unwrap_or_default();
        copy_rows(FN_NAME, &members, out, len, out_written, out_required)
    })
}

/// Copy every change in which source supplied a satellite's position, in
/// output order, including each satellite's opening entry (from no source).
/// Unrecorded provenance copies nothing. Uses the variable-length output
/// contract.
///
/// Safety: report must be a live merge report handle; out must point to len
/// writable SidereonSp3PrecedenceTransition values or be NULL when len is 0;
/// out_written and out_required must point to size_t values.
#[no_mangle]
pub unsafe extern "C" fn sidereon_sp3_merge_report_provenance_transitions(
    report: *const SidereonSp3MergeReport,
    out: *mut SidereonSp3PrecedenceTransition,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_sp3_merge_report_provenance_transitions";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        c_try!(init_copy_counts(FN_NAME, out_written, out_required));
        let report = c_try!(require_ref(report, FN_NAME, "report"));
        let rows: Vec<SidereonSp3PrecedenceTransition> = report
            .inner
            .provenance
            .as_ref()
            .map(|provenance| provenance.transitions.iter().map(transition_to_c).collect())
            .unwrap_or_default();
        copy_rows(FN_NAME, &rows, out, len, out_written, out_required)
    })
}

/// Copy what each input source contributed, indexed by source order.
/// Unrecorded provenance copies nothing. Uses the variable-length output
/// contract.
///
/// Safety: report must be a live merge report handle; out must point to len
/// writable SidereonSp3ContributorCoverage values or be NULL when len is 0;
/// out_written and out_required must point to size_t values.
#[no_mangle]
pub unsafe extern "C" fn sidereon_sp3_merge_report_provenance_coverage(
    report: *const SidereonSp3MergeReport,
    out: *mut SidereonSp3ContributorCoverage,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_sp3_merge_report_provenance_coverage";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        c_try!(init_copy_counts(FN_NAME, out_written, out_required));
        let report = c_try!(require_ref(report, FN_NAME, "report"));
        let rows: Vec<SidereonSp3ContributorCoverage> = report
            .inner
            .provenance
            .as_ref()
            .map(|provenance| {
                provenance
                    .coverage
                    .iter()
                    .map(contributor_coverage_to_c)
                    .collect()
            })
            .unwrap_or_default();
        copy_rows(FN_NAME, &rows, out, len, out_written, out_required)
    })
}

// --- interpolation node selection --------------------------------------------

#[allow(clippy::too_many_arguments)]
unsafe fn selected_nodes_common(
    fn_name: &str,
    nodes: &InterpolationNodes,
    sat_id: *const c_char,
    from_j2000_s: f64,
    through_j2000_s: f64,
    out: *mut f64,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    let sat = c_try!(parse_satellite_token(fn_name, sat_id));
    let window = c_try!(EpochWindow::new(from_j2000_s, through_j2000_s)
        .map_err(|error| crate::sp3::map_sp3_argument_error(fn_name, error)));
    let selected = nodes.selected_nodes(sat, window);
    copy_rows(fn_name, &selected, out, len, out_written, out_required)
}

/// Copy the epochs (seconds since J2000, ascending) of the position nodes that
/// some query of `sat_id` in the inclusive window [from_j2000_s,
/// through_j2000_s] selects, by the product's own interpolation rule
/// (sidereon_core::ephemeris::InterpolationNodes::selected_nodes). A
/// satellite the product does not carry, or a window no query serves, copies
/// nothing. Uses the variable-length output contract.
///
/// Safety: sp3 must be a live handle; sat_id must be a null-terminated token;
/// out must point to len writable doubles or be NULL when len is 0;
/// out_written and out_required must point to size_t values.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn sidereon_sp3_selected_nodes(
    sp3: *const SidereonSp3,
    sat_id: *const c_char,
    from_j2000_s: f64,
    through_j2000_s: f64,
    out: *mut f64,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_sp3_selected_nodes";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        c_try!(init_copy_counts(FN_NAME, out_written, out_required));
        let sp3 = c_try!(require_ref(sp3, FN_NAME, "sp3"));
        let nodes = InterpolationNodes::for_sp3(&sp3.inner);
        selected_nodes_common(
            FN_NAME,
            &nodes,
            sat_id,
            from_j2000_s,
            through_j2000_s,
            out,
            len,
            out_written,
            out_required,
        )
    })
}

/// As sidereon_sp3_selected_nodes, over the merged product's nodes the merge
/// report holds for its continuity verdicts. *out_verified is false, and
/// nothing is copied, when the merge did not verify continuity
/// (SidereonSp3MergeOptions.verify_continuity_enabled).
///
/// Safety: report must be a live merge report handle; sat_id must be a
/// null-terminated token; out_verified must point to a bool; out must point to
/// len writable doubles or be NULL when len is 0; out_written and out_required
/// must point to size_t values.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn sidereon_sp3_merge_report_continuity_selected_nodes(
    report: *const SidereonSp3MergeReport,
    sat_id: *const c_char,
    from_j2000_s: f64,
    through_j2000_s: f64,
    out_verified: *mut bool,
    out: *mut f64,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_sp3_merge_report_continuity_selected_nodes";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        if !out_verified.is_null() {
            *out_verified = false;
        }
        c_try!(init_copy_counts(FN_NAME, out_written, out_required));
        let verified = c_try!(require_out(out_verified, FN_NAME, "out_verified"));
        let report = c_try!(require_ref(report, FN_NAME, "report"));
        let Some(continuity) = report.inner.continuity.as_ref() else {
            // Still check the query, so an invalid window or token is refused
            // whether or not continuity was verified.
            c_try!(parse_satellite_token(FN_NAME, sat_id));
            c_try!(EpochWindow::new(from_j2000_s, through_j2000_s)
                .map_err(|error| crate::sp3::map_sp3_argument_error(FN_NAME, error)));
            return SidereonStatus::Ok;
        };
        *verified = true;
        selected_nodes_common(
            FN_NAME,
            &continuity.nodes,
            sat_id,
            from_j2000_s,
            through_j2000_s,
            out,
            len,
            out_written,
            out_required,
        )
    })
}

// --- product coverage ---------------------------------------------------------

/// Build the per-satellite coverage of an SP3 product: for each satellite the
/// product declares or carries, the spans and gaps of its positions and of its
/// clocks on the product's epoch grid, and the grid itself. On success writes a
/// newly owned handle to *out_coverage; release it with
/// sidereon_sp3_coverage_free.
///
/// Safety: sp3 must be a live handle; out_coverage must point to storage for a
/// SidereonSp3Coverage*.
#[no_mangle]
pub unsafe extern "C" fn sidereon_sp3_satellite_coverage(
    sp3: *const SidereonSp3,
    out_coverage: *mut *mut SidereonSp3Coverage,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_sp3_satellite_coverage";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        let out = c_try!(require_out(out_coverage, FN_NAME, "out_coverage"));
        *out = ptr::null_mut();
        let sp3 = c_try!(require_ref(sp3, FN_NAME, "sp3"));
        write_boxed_handle(
            out,
            SidereonSp3Coverage {
                inner: sp3.inner.satellite_coverage(),
            },
        );
        SidereonStatus::Ok
    })
}

/// Release a coverage handle. Passing NULL is a no-op.
///
/// Safety: coverage must be NULL or a live handle from
/// sidereon_sp3_satellite_coverage that has not already been freed.
#[no_mangle]
pub unsafe extern "C" fn sidereon_sp3_coverage_free(coverage: *mut SidereonSp3Coverage) {
    ffi_boundary("sidereon_sp3_coverage_free", (), || {
        free_boxed(coverage);
    });
}

/// Write the epoch grid the product's epochs lie on.
///
/// Safety: coverage must be a live handle; out_grid must point to a
/// SidereonSp3EpochGrid.
#[no_mangle]
pub unsafe extern "C" fn sidereon_sp3_coverage_grid(
    coverage: *const SidereonSp3Coverage,
    out_grid: *mut SidereonSp3EpochGrid,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_sp3_coverage_grid";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        let out = c_try!(require_out(out_grid, FN_NAME, "out_grid"));
        *out = SidereonSp3EpochGrid {
            has_interval: false,
            interval_s: f64::NAN,
            agrees_with_header: false,
            out_of_order_count: 0,
            unplaced_count: 0,
        };
        let coverage = c_try!(require_ref(coverage, FN_NAME, "coverage"));
        let grid = &coverage.inner.grid;
        *out = SidereonSp3EpochGrid {
            has_interval: grid.interval_s.is_some(),
            interval_s: grid.interval_s.unwrap_or(f64::NAN),
            agrees_with_header: grid.agrees_with_header,
            out_of_order_count: grid.out_of_order.len(),
            unplaced_count: grid.unplaced.len(),
        };
        SidereonStatus::Ok
    })
}

/// Copy the indices of the product epochs that are out of time order. Uses
/// the variable-length output contract.
///
/// Safety: coverage must be a live handle; out must point to len writable
/// size_t values or be NULL when len is 0; out_written and out_required must
/// point to size_t values.
#[no_mangle]
pub unsafe extern "C" fn sidereon_sp3_coverage_grid_out_of_order(
    coverage: *const SidereonSp3Coverage,
    out: *mut usize,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_sp3_coverage_grid_out_of_order";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        c_try!(init_copy_counts(FN_NAME, out_written, out_required));
        let coverage = c_try!(require_ref(coverage, FN_NAME, "coverage"));
        copy_rows(
            FN_NAME,
            &coverage.inner.grid.out_of_order,
            out,
            len,
            out_written,
            out_required,
        )
    })
}

/// Copy the indices of the product epochs that no SP3 record states exactly.
/// Uses the variable-length output contract.
///
/// Safety: coverage must be a live handle; out must point to len writable
/// size_t values or be NULL when len is 0; out_written and out_required must
/// point to size_t values.
#[no_mangle]
pub unsafe extern "C" fn sidereon_sp3_coverage_grid_unplaced(
    coverage: *const SidereonSp3Coverage,
    out: *mut usize,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_sp3_coverage_grid_unplaced";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        c_try!(init_copy_counts(FN_NAME, out_written, out_required));
        let coverage = c_try!(require_ref(coverage, FN_NAME, "coverage"));
        copy_rows(
            FN_NAME,
            &coverage.inner.grid.unplaced,
            out,
            len,
            out_written,
            out_required,
        )
    })
}

/// Copy the coverage of every satellite, declared satellites first as the
/// header lists them, then any the records carry. Uses the variable-length
/// output contract; a satellite's spans and gaps are read by its index here.
///
/// Safety: coverage must be a live handle; out must point to len writable
/// SidereonSp3SatelliteCoverage values or be NULL when len is 0; out_written
/// and out_required must point to size_t values.
#[no_mangle]
pub unsafe extern "C" fn sidereon_sp3_coverage_satellites(
    coverage: *const SidereonSp3Coverage,
    out: *mut SidereonSp3SatelliteCoverage,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_sp3_coverage_satellites";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        c_try!(init_copy_counts(FN_NAME, out_written, out_required));
        let coverage = c_try!(require_ref(coverage, FN_NAME, "coverage"));
        let rows: Vec<SidereonSp3SatelliteCoverage> = coverage
            .inner
            .satellites
            .iter()
            .map(|satellite| SidereonSp3SatelliteCoverage {
                sat_id: satellite_token(satellite.satellite),
                declared: satellite.declared,
                positions: channel_coverage_to_c(&satellite.positions),
                clocks: channel_coverage_to_c(&satellite.clocks),
            })
            .collect();
        copy_rows(FN_NAME, &rows, out, len, out_written, out_required)
    })
}

unsafe fn coverage_channel<'a>(
    fn_name: &str,
    coverage: *const SidereonSp3Coverage,
    satellite_index: usize,
    channel: u32,
) -> Result<&'a Sp3ChannelCoverage, SidereonStatus> {
    let coverage = require_ref(coverage, fn_name, "coverage")?;
    let channel = channel_from_c(fn_name, channel)?;
    let Some(satellite) = coverage.inner.satellites.get(satellite_index) else {
        set_last_error(format!(
            "{fn_name}: satellite index {satellite_index} out of range ({} satellites)",
            coverage.inner.satellites.len()
        ));
        return Err(SidereonStatus::InvalidArgument);
    };
    Ok(match channel {
        SidereonSp3Channel::Position => &satellite.positions,
        SidereonSp3Channel::Clock => &satellite.clocks,
    })
}

/// Copy the spans of one satellite's channel (channel is a SidereonSp3Channel
/// value), in time order. Uses the variable-length output contract.
///
/// Safety: coverage must be a live handle; out must point to len writable
/// SidereonSp3CoverageSpan values or be NULL when len is 0; out_written and
/// out_required must point to size_t values.
#[no_mangle]
pub unsafe extern "C" fn sidereon_sp3_coverage_spans(
    coverage: *const SidereonSp3Coverage,
    satellite_index: usize,
    channel: u32,
    out: *mut SidereonSp3CoverageSpan,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_sp3_coverage_spans";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        c_try!(init_copy_counts(FN_NAME, out_written, out_required));
        let channel = c_try!(coverage_channel(
            FN_NAME,
            coverage,
            satellite_index,
            channel
        ));
        let rows: Vec<SidereonSp3CoverageSpan> =
            channel.spans.iter().map(coverage_span_to_c).collect();
        copy_rows(FN_NAME, &rows, out, len, out_written, out_required)
    })
}

/// Copy the gaps of one satellite's channel (channel is a SidereonSp3Channel
/// value), in time order. Uses the variable-length output contract.
///
/// Safety: coverage must be a live handle; out must point to len writable
/// SidereonSp3CoverageGap values or be NULL when len is 0; out_written and
/// out_required must point to size_t values.
#[no_mangle]
pub unsafe extern "C" fn sidereon_sp3_coverage_gaps(
    coverage: *const SidereonSp3Coverage,
    satellite_index: usize,
    channel: u32,
    out: *mut SidereonSp3CoverageGap,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_sp3_coverage_gaps";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        c_try!(init_copy_counts(FN_NAME, out_written, out_required));
        let channel = c_try!(coverage_channel(
            FN_NAME,
            coverage,
            satellite_index,
            channel
        ));
        let rows: Vec<SidereonSp3CoverageGap> =
            channel.gaps.iter().map(coverage_gap_to_c).collect();
        copy_rows(FN_NAME, &rows, out, len, out_written, out_required)
    })
}

#[cfg(test)]
mod tests {
    //! The merge audit trail, ported from sidereon-core's
    //! tests/sp3_merge_per_epoch_provenance.rs and tests/sp3_merge_coverage.rs:
    //! each fixture is merged through the C ABI and by sidereon-core, and every
    //! C record is checked against sidereon-core's own report, beside the
    //! properties the core tests state.

    use super::*;
    use sidereon_core::ephemeris::{
        merge, CellSelection, ContinuityOptions, MergeReport, OrbitClass, Sp3,
    };
    use std::ffi::{CStr, CString};

    // --- fixtures (sidereon-core tests/sp3_merge_per_epoch_provenance.rs) -----

    fn two_epoch_source(positions_km: [f64; 2]) -> Sp3 {
        let body = format!(
            "#cP2020  6 25  0  0  0.00000000       2 ORBIT IGS14 FIT  TST\n\
             ## 2111 432000.00000000   900.00000000 59025 0.0000000000000\n\
             +    1   G01  0  0  0  0  0  0  0  0  0  0  0  0  0  0  0  0\n\
             ++         0  0  0  0  0  0  0  0  0  0  0  0  0  0  0  0  0\n\
             %c G  cc GPS ccc cccc cccc cccc cccc ccccc ccccc ccccc ccccc\n\
             %c cc cc ccc ccc cccc cccc cccc cccc ccccc ccccc ccccc ccccc\n\
             %f  1.2500000  1.025000000  0.00000000000  0.000000000000000\n\
             %f  0.0000000  0.000000000  0.00000000000  0.000000000000000\n\
             %i    0    0    0    0      0      0      0      0         0\n\
             %i    0    0    0    0      0      0      0      0         0\n\
             /* TEST SP3-c FIXTURE\n\
             *  2020  6 25  0  0  0.00000000\n\
             PG01 {:13.6} -20000.000000   5000.000000    100.000000\n\
             *  2020  6 25  0 15  0.00000000\n\
             PG01 {:13.6} -20000.000000   5000.000000    100.000000\n\
             EOF\n",
            positions_km[0], positions_km[1]
        );
        Sp3::parse(body.as_bytes()).expect("parse test sp3")
    }

    fn late_source(position_km: f64) -> Sp3 {
        let body = format!(
            "#cP2020  6 25  0 15  0.00000000       1 ORBIT IGS14 FIT  TST\n\
             ## 2111 432900.00000000   900.00000000 59025 0.0000000000000\n\
             +    1   G01  0  0  0  0  0  0  0  0  0  0  0  0  0  0  0  0\n\
             ++         0  0  0  0  0  0  0  0  0  0  0  0  0  0  0  0  0\n\
             %c G  cc GPS ccc cccc cccc cccc cccc ccccc ccccc ccccc ccccc\n\
             %c cc cc ccc ccc cccc cccc cccc cccc ccccc ccccc ccccc ccccc\n\
             %f  1.2500000  1.025000000  0.00000000000  0.000000000000000\n\
             %f  0.0000000  0.000000000  0.00000000000  0.000000000000000\n\
             %i    0    0    0    0      0      0      0      0         0\n\
             %i    0    0    0    0      0      0      0      0         0\n\
             /* TEST SP3-c FIXTURE\n\
             *  2020  6 25  0 15  0.00000000\n\
             PG01 {position_km:13.6} -20000.000000   5000.000000    100.000000\n\
             EOF\n"
        );
        Sp3::parse(body.as_bytes()).expect("parse test sp3")
    }

    fn early_only_source() -> Sp3 {
        Sp3::parse(
            "#cP2020  6 25  0  0  0.00000000       1 ORBIT IGS14 FIT  TST\n\
             ## 2111 432000.00000000   900.00000000 59025 0.0000000000000\n\
             +    1   G01  0  0  0  0  0  0  0  0  0  0  0  0  0  0  0  0\n\
             ++         0  0  0  0  0  0  0  0  0  0  0  0  0  0  0  0  0\n\
             %c G  cc GPS ccc cccc cccc cccc cccc ccccc ccccc ccccc ccccc\n\
             %c cc cc ccc ccc cccc cccc cccc cccc ccccc ccccc ccccc ccccc\n\
             %f  1.2500000  1.025000000  0.00000000000  0.000000000000000\n\
             %f  0.0000000  0.000000000  0.00000000000  0.000000000000000\n\
             %i    0    0    0    0      0      0      0      0         0\n\
             %i    0    0    0    0      0      0      0      0         0\n\
             /* TEST SP3-c FIXTURE\n\
             *  2020  6 25  0  0  0.00000000\n\
             PG01  15000.000000 -20000.000000   5000.000000    100.000000\n\
             EOF\n"
                .as_bytes(),
        )
        .expect("parse trimmed source")
    }

    // --- fixtures (sidereon-core tests/sp3_merge_coverage.rs) -----------------

    const STEP_S: usize = 300;

    fn epoch_fields(index: usize) -> String {
        format!(
            "2020  6 25 {:2}{:3}  0.00000000",
            index / 12,
            (index % 12) * 5
        )
    }

    #[derive(Clone, Copy, PartialEq)]
    enum Record {
        Full,
        NoClock,
        ClockOnly,
        Absent,
    }

    fn product(indices: &[usize], offset_m: f64, record: impl Fn(u8, usize) -> Record) -> Sp3 {
        let first = indices[0];
        let mut text = format!(
            "#cP{}     {:3} ORBIT IGS14 FIT  TST\n\
             ## 2111 {:14.8}   300.00000000 59025 {:.13}\n\
             +    6   G01G02G03G04G05G06  0  0  0  0  0  0  0  0  0  0  0\n\
             ++         0  0  0  0  0  0  0  0  0  0  0  0  0  0  0  0  0\n\
             %c G  cc GPS ccc cccc cccc cccc cccc ccccc ccccc ccccc ccccc\n\
             %c cc cc ccc ccc cccc cccc cccc cccc ccccc ccccc ccccc ccccc\n\
             %f  1.2500000  1.025000000  0.00000000000  0.000000000000000\n\
             %f  0.0000000  0.000000000  0.00000000000  0.000000000000000\n\
             %i    0    0    0    0      0      0      0      0         0\n\
             %i    0    0    0    0      0      0      0      0         0\n\
             /* SYNTHETIC SP3 COVERAGE FIXTURE\n",
            epoch_fields(first),
            indices.len(),
            345_600.0 + (first * STEP_S) as f64,
            (first * STEP_S) as f64 / 86_400.0
        );
        for &index in indices {
            let seconds = (index * STEP_S) as f64;
            text.push_str(&format!("*  {}\n", epoch_fields(index)));
            for prn in 1..=6u8 {
                let angle = seconds * core::f64::consts::TAU / 43_200.0 + f64::from(prn) * 0.3;
                let x = 26_560.0 * angle.cos() + offset_m / 1000.0;
                let y = 26_560.0 * angle.sin() * 0.6;
                let z = 26_560.0 * angle.sin() * 0.8;
                let clock = 10.0 + f64::from(prn);
                match record(prn, index) {
                    Record::Full => {
                        text.push_str(&format!("PG{prn:02}{x:14.6}{y:14.6}{z:14.6}{clock:14.6}\n"))
                    }
                    Record::NoClock => text.push_str(&format!(
                        "PG{prn:02}{x:14.6}{y:14.6}{z:14.6}{:14.6}\n",
                        999_999.999_999
                    )),
                    Record::ClockOnly => text.push_str(&format!(
                        "PG{prn:02}{:14.6}{:14.6}{:14.6}{clock:14.6}\n",
                        0.0, 0.0, 0.0
                    )),
                    Record::Absent => {}
                }
            }
        }
        text.push_str("EOF\n");
        Sp3::parse(text.as_bytes()).expect("synthetic SP3")
    }

    fn coverage_source(first: usize, count: usize, offset_m: f64) -> Sp3 {
        let indices: Vec<usize> = (first..first + count).collect();
        product(&indices, offset_m, |_, _| Record::Full)
    }

    // --- C ABI drivers --------------------------------------------------------

    fn options() -> SidereonSp3MergeOptions {
        let mut options = unsafe { std::mem::zeroed::<SidereonSp3MergeOptions>() };
        assert_eq!(
            unsafe { sidereon_sp3_merge_options_init(&mut options) },
            SidereonStatus::Ok
        );
        options
    }

    fn precedence_options(provenance: SidereonSp3ProvenanceMode) -> SidereonSp3MergeOptions {
        let mut options = options();
        options.combine = SidereonSp3MergeCombine::Precedence as u32;
        options.precedence_scope = SidereonSp3MergePrecedenceScope::Cell as u32;
        options.min_agree = 1;
        options.provenance_mode = provenance as u32;
        options
    }

    fn coverage_precedence(scope: SidereonSp3MergePrecedenceScope) -> SidereonSp3MergeOptions {
        let mut options = options();
        options.combine = SidereonSp3MergeCombine::Precedence as u32;
        options.precedence_scope = scope as u32;
        options.min_agree = 1;
        options.position_tolerance_m = 5.0;
        options
    }

    /// Merge `sources` through the C ABI and by sidereon-core with the options
    /// the C record states.
    fn merge_both(
        sources: &[Sp3],
        options: &SidereonSp3MergeOptions,
    ) -> (
        Box<SidereonSp3>,
        Box<SidereonSp3MergeReport>,
        Sp3,
        MergeReport,
    ) {
        let handles: Vec<SidereonSp3> = sources
            .iter()
            .map(|inner| SidereonSp3 {
                inner: inner.clone(),
            })
            .collect();
        let pointers: Vec<*const SidereonSp3> = handles.iter().map(|h| h as *const _).collect();
        let mut merged = ptr::null_mut();
        let mut report = ptr::null_mut();
        assert_eq!(
            unsafe {
                sidereon_sp3_merge(
                    pointers.as_ptr(),
                    pointers.len(),
                    options,
                    &mut merged,
                    &mut report,
                )
            },
            SidereonStatus::Ok
        );
        let core_options =
            unsafe { crate::sp3::sp3_merge_options_from_c("test", options) }.expect("options");
        let (core_merged, core_report) = merge(sources, &core_options).expect("core merge");
        unsafe {
            (
                Box::from_raw(merged),
                Box::from_raw(report),
                core_merged,
                core_report,
            )
        }
    }

    unsafe fn rows<T: Copy>(
        read: impl Fn(*mut T, usize, *mut usize, *mut usize) -> SidereonStatus,
    ) -> Vec<T> {
        let mut written = usize::MAX;
        let mut required = usize::MAX;
        assert_eq!(
            read(ptr::null_mut(), 0, &mut written, &mut required),
            SidereonStatus::Ok
        );
        assert_eq!(written, 0);
        let mut out: Vec<T> = Vec::with_capacity(required);
        assert_eq!(
            read(out.as_mut_ptr(), required, &mut written, &mut required),
            SidereonStatus::Ok
        );
        assert_eq!(written, required);
        out.set_len(written);
        out
    }

    fn same_epoch(c: &SidereonClockEpoch, core: &Instant) -> bool {
        let expected = crate::rinex_clock::instant_to_clock_epoch(core);
        c.scale == expected.scale
            && c.representation == expected.representation
            && c.jd_whole.to_bits() == expected.jd_whole.to_bits()
            && c.jd_fraction.to_bits() == expected.jd_fraction.to_bits()
            && c.nanos_high == expected.nanos_high
            && c.nanos_low == expected.nanos_low
    }

    fn token(token: &SidereonSatelliteToken) -> String {
        unsafe { CStr::from_ptr(token.bytes.as_ptr()) }
            .to_string_lossy()
            .into_owned()
    }

    fn provenance_info(report: &SidereonSp3MergeReport) -> SidereonSp3MergeProvenanceInfo {
        let mut info = provenance_info_to_c(None);
        assert_eq!(
            unsafe { sidereon_sp3_merge_report_provenance(report, &mut info) },
            SidereonStatus::Ok
        );
        info
    }

    fn cells(report: &SidereonSp3MergeReport) -> Vec<SidereonSp3CellProvenance> {
        unsafe {
            rows(|out, len, w, r| {
                sidereon_sp3_merge_report_provenance_cells(report, out, len, w, r)
            })
        }
    }

    fn transitions(report: &SidereonSp3MergeReport) -> Vec<SidereonSp3PrecedenceTransition> {
        unsafe {
            rows(|out, len, w, r| {
                sidereon_sp3_merge_report_provenance_transitions(report, out, len, w, r)
            })
        }
    }

    fn contributor_coverage(
        report: &SidereonSp3MergeReport,
    ) -> Vec<SidereonSp3ContributorCoverage> {
        unsafe {
            rows(|out, len, w, r| {
                sidereon_sp3_merge_report_provenance_coverage(report, out, len, w, r)
            })
        }
    }

    fn members(
        report: &SidereonSp3MergeReport,
        index: usize,
        channel: SidereonSp3Channel,
    ) -> Vec<usize> {
        unsafe {
            rows(|out, len, w, r| {
                sidereon_sp3_merge_report_provenance_cell_members(
                    report,
                    index,
                    channel as u32,
                    out,
                    len,
                    w,
                    r,
                )
            })
        }
    }

    /// Every provenance record the C ABI copies is sidereon-core's.
    fn assert_provenance_matches_core(report: &SidereonSp3MergeReport, core: &MergeReport) {
        let info = provenance_info(report);
        let Some(provenance) = core.provenance.as_ref() else {
            assert!(!info.recorded);
            assert_eq!(info.mode, SidereonSp3ProvenanceMode::Off);
            assert!(cells(report).is_empty());
            assert!(transitions(report).is_empty());
            assert!(contributor_coverage(report).is_empty());
            return;
        };
        assert!(info.recorded);
        assert_eq!(info.mode, provenance_mode_to_c(provenance.mode));
        let c_cells = cells(report);
        assert_eq!(c_cells.len(), provenance.cells.len());
        assert_eq!(info.cell_count, provenance.cells.len());
        for (index, (c, core)) in c_cells.iter().zip(&provenance.cells).enumerate() {
            assert!(same_epoch(&c.epoch, &core.epoch));
            assert_eq!(token(&c.sat_id), core.satellite.to_string());
            for (channel, has, selection, core_selection) in [
                (
                    SidereonSp3Channel::Position,
                    c.has_position,
                    c.position,
                    core.position.as_ref(),
                ),
                (
                    SidereonSp3Channel::Clock,
                    c.has_clock,
                    c.clock,
                    core.clock.as_ref(),
                ),
            ] {
                assert_eq!(has, core_selection.is_some());
                let members = members(report, index, channel);
                let Some(core_selection) = core_selection else {
                    assert!(members.is_empty());
                    continue;
                };
                assert_eq!(members, core_selection.members());
                assert_eq!(selection.member_count, core_selection.members().len());
                assert_eq!(
                    selection.has_source.then_some(selection.source),
                    core_selection.selected_source()
                );
                match core_selection {
                    CellSelection::SingleSource { .. } => {
                        assert_eq!(selection.kind, SidereonSp3CellSelectionKind::SingleSource)
                    }
                    CellSelection::Precedence { .. } => {
                        assert_eq!(selection.kind, SidereonSp3CellSelectionKind::Precedence)
                    }
                    CellSelection::Combined { rule, .. } => {
                        assert_eq!(selection.kind, SidereonSp3CellSelectionKind::Combined);
                        assert!(selection.has_rule);
                        assert_eq!(selection.rule, merge_combine_to_c(*rule));
                    }
                }
            }
        }
        let c_transitions = transitions(report);
        assert_eq!(c_transitions.len(), provenance.transitions.len());
        for (c, core) in c_transitions.iter().zip(&provenance.transitions) {
            assert_eq!(token(&c.sat_id), core.satellite.to_string());
            assert!(same_epoch(&c.epoch, &core.epoch));
            assert_eq!(c.has_from_source.then_some(c.from_source), core.from_source);
            assert_eq!(c.has_to_source.then_some(c.to_source), core.to_source);
            assert_eq!(c.reason, transition_reason_to_c(core.reason));
        }
        let c_coverage = contributor_coverage(report);
        assert_eq!(c_coverage.len(), provenance.coverage.len());
        for (c, core) in c_coverage.iter().zip(&provenance.coverage) {
            assert_eq!(c.source, core.source);
            assert_eq!(c.cells_contributed, core.cells_contributed);
            assert_eq!(c.cells_selected, core.cells_selected);
            assert_eq!(c.cells_absent, core.cells_absent);
            assert_eq!(c.has_first_epoch, core.first_epoch.is_some());
            assert_eq!(c.has_last_epoch, core.last_epoch.is_some());
            if let Some(epoch) = &core.first_epoch {
                assert!(same_epoch(&c.first_epoch, epoch));
            }
            if let Some(epoch) = &core.last_epoch {
                assert!(same_epoch(&c.last_epoch, epoch));
            }
        }
    }

    /// Every audit list the C ABI copies is sidereon-core's.
    fn assert_audit_matches_core(report: &SidereonSp3MergeReport, core: &MergeReport) {
        let omitted: Vec<SidereonSp3MergeEpoch> = unsafe {
            rows(|out, len, w, r| sidereon_sp3_merge_report_omitted_epochs(report, out, len, w, r))
        };
        assert_eq!(omitted.len(), core.omitted_epochs.len());
        for (c, core) in omitted.iter().zip(&core.omitted_epochs) {
            assert!(same_epoch(&c.epoch, core));
        }

        let omissions: Vec<SidereonSp3ClockOmission> = unsafe {
            rows(|out, len, w, r| sidereon_sp3_merge_report_clock_omissions(report, out, len, w, r))
        };
        assert_eq!(omissions.len(), core.clock_omissions.len());
        for (c, core) in omissions.iter().zip(&core.clock_omissions) {
            assert!(same_epoch(&c.epoch, &core.epoch));
            assert_eq!(token(&c.sat_id), core.satellite.to_string());
            assert_eq!(c.source, core.source);
            assert_eq!(c.cell_has_clock, core.cell_has_clock);
            let expected = clock_omission_to_c(core);
            assert_eq!(c.reason, expected.reason);
            assert_eq!(c.has_preferred, expected.has_preferred);
            assert_eq!(c.preferred, expected.preferred);
        }

        let dropped: Vec<SidereonSp3DroppedInputEpoch> = unsafe {
            rows(|out, len, w, r| {
                sidereon_sp3_merge_report_dropped_input_epochs(report, out, len, w, r)
            })
        };
        assert_eq!(dropped.len(), core.dropped_input_epochs.len());
        for (c, core) in dropped.iter().zip(&core.dropped_input_epochs) {
            assert_eq!((c.source, c.epoch_index), (core.source, core.epoch_index));
            assert!(same_epoch(&c.epoch, &core.epoch));
            assert_eq!(c.reason, dropped_input_epoch_to_c(core).reason);
        }

        let metrics: Vec<SidereonSp3AgreementMetric> = unsafe {
            rows(|out, len, w, r| {
                sidereon_sp3_merge_report_agreement_metrics(report, out, len, w, r)
            })
        };
        assert_eq!(metrics.len(), core.agreement.len());
        for (c, core) in metrics.iter().zip(&core.agreement) {
            assert!(same_epoch(&c.epoch, &core.epoch));
            assert_eq!(token(&c.sat_id), core.satellite.to_string());
            assert_eq!(c.position_members, core.position_members);
            assert_eq!(c.clock_members, core.clock_members);
            assert_eq!(
                c.has_position_rms_m.then_some(c.position_rms_m.to_bits()),
                core.position_rms_m.map(f64::to_bits)
            );
            assert_eq!(
                c.has_position_max_m.then_some(c.position_max_m.to_bits()),
                core.position_max_m.map(f64::to_bits)
            );
            assert_eq!(
                c.has_clock_rms_s.then_some(c.clock_rms_s.to_bits()),
                core.clock_rms_s.map(f64::to_bits)
            );
            assert_eq!(
                c.has_clock_max_s.then_some(c.clock_max_s.to_bits()),
                core.clock_max_s.map(f64::to_bits)
            );
        }

        let mut summary = unsafe { std::mem::zeroed::<SidereonSp3AgreementSummary>() };
        assert_eq!(
            unsafe { sidereon_sp3_merge_report_agreement_summary(report, &mut summary) },
            SidereonStatus::Ok
        );
        assert_eq!(
            summary
                .single_source_fraction_present
                .then_some(summary.single_source_fraction.to_bits()),
            core.single_source_fraction().map(f64::to_bits)
        );

        let mut count = usize::MAX;
        assert_eq!(
            unsafe {
                sidereon_sp3_merge_report_flag_count(
                    report,
                    SidereonSp3MergeFlagKind::ArcWithheld as u32,
                    &mut count,
                )
            },
            SidereonStatus::Ok
        );
        assert_eq!(count, core.arc_withheld.len());
        for (index, core_flag) in core.arc_withheld.iter().enumerate() {
            let mut flag = unsafe { std::mem::zeroed::<SidereonSp3MergeFlag>() };
            assert_eq!(
                unsafe {
                    sidereon_sp3_merge_report_flag(
                        report,
                        SidereonSp3MergeFlagKind::ArcWithheld as u32,
                        index,
                        &mut flag,
                    )
                },
                SidereonStatus::Ok
            );
            assert!(same_epoch(&flag.epoch, &core_flag.epoch));
            assert_eq!(flag.source_count, core_flag.sources.len());
        }
        assert_provenance_matches_core(report, core);
    }

    // --- sp3_merge_per_epoch_provenance.rs --------------------------------------

    #[test]
    fn provenance_is_absent_unless_requested() {
        let (_merged, report, _, core) = merge_both(
            &[two_epoch_source([15_000.0, 15_100.0])],
            &precedence_options(SidereonSp3ProvenanceMode::Off),
        );
        assert!(core.provenance.is_none());
        assert!(!provenance_info(&report).recorded);
        assert_audit_matches_core(&report, &core);
    }

    #[test]
    fn a_single_contributor_merge_records_it_for_every_epoch_with_no_mid_arc_transition() {
        let (_merged, report, _, core) = merge_both(
            &[two_epoch_source([15_000.0, 15_100.0])],
            &precedence_options(SidereonSp3ProvenanceMode::Full),
        );
        assert_audit_matches_core(&report, &core);
        let info = provenance_info(&report);
        assert_eq!(info.mode, SidereonSp3ProvenanceMode::Full);
        let c_cells = cells(&report);
        assert_eq!(c_cells.len(), 2);
        for cell in &c_cells {
            assert!(cell.has_position);
            assert!(cell.position.has_source);
            assert_eq!(cell.position.source, 0);
        }
        let c_transitions = transitions(&report);
        assert_eq!(c_transitions.len(), 1);
        assert!(!c_transitions[0].has_from_source);
        assert!(c_transitions[0].has_to_source);
        assert_eq!(c_transitions[0].to_source, 0);
        let coverage = contributor_coverage(&report);
        assert_eq!(coverage[0].cells_contributed, 2);
        assert_eq!(coverage[0].cells_selected, 2);
        assert_eq!(coverage[0].cells_absent, 0);
        assert!(coverage[0].has_first_epoch && coverage[0].has_last_epoch);
    }

    #[test]
    fn a_forced_precedence_switch_records_one_transition_naming_both_sides() {
        let (_merged, report, _, core) = merge_both(
            &[early_only_source(), late_source(15_100.0)],
            &precedence_options(SidereonSp3ProvenanceMode::Full),
        );
        assert_audit_matches_core(&report, &core);
        let c_cells = cells(&report);
        assert_eq!(c_cells.len(), 2);
        assert_eq!(c_cells[0].position.source, 0);
        assert_eq!(c_cells[1].position.source, 1);
        let changes: Vec<_> = transitions(&report)
            .into_iter()
            .filter(|transition| transition.has_from_source)
            .collect();
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].from_source, 0);
        assert_eq!(changes[0].to_source, 1);
        assert_eq!(
            changes[0].reason,
            SidereonSp3TransitionReason::SoleAvailability
        );
        let coverage = contributor_coverage(&report);
        assert_eq!(coverage[0].cells_contributed, 1);
        assert_eq!(coverage[0].cells_absent, 1);
        assert_eq!(coverage[1].cells_contributed, 1);
        assert_eq!(coverage[1].cells_absent, 1);
    }

    #[test]
    fn outlier_rejection_is_recorded_as_its_own_reason() {
        let mut options = options();
        options.combine = SidereonSp3MergeCombine::Precedence as u32;
        options.precedence_scope = SidereonSp3MergePrecedenceScope::Cell as u32;
        options.min_agree = 2;
        options.position_tolerance_m = 1.0;
        options.outlier_reject_enabled = 1;
        options.outlier_reject_position_tolerance_m = 1.0;
        options.outlier_reject_clock_tolerance_s = 1.0e-6;
        options.provenance_mode = SidereonSp3ProvenanceMode::Full as u32;
        let (_merged, report, _, core) = merge_both(
            &[
                two_epoch_source([15_000.0, 25_000.0]),
                two_epoch_source([15_000.0, 15_100.0]),
                two_epoch_source([15_000.0, 15_100.0]),
            ],
            &options,
        );
        assert_audit_matches_core(&report, &core);
        let c_cells = cells(&report);
        assert_eq!(c_cells[0].position.source, 0);
        assert_eq!(c_cells[1].position.source, 1);
        let changes: Vec<_> = transitions(&report)
            .into_iter()
            .filter(|transition| transition.has_from_source)
            .collect();
        assert_eq!(changes.len(), 1);
        assert_eq!(
            changes[0].reason,
            SidereonSp3TransitionReason::OutlierRejection
        );
        assert!(!core.position_outliers.is_empty());
    }

    #[test]
    fn a_combined_cell_names_no_single_supplier() {
        let mut options = options();
        options.provenance_mode = SidereonSp3ProvenanceMode::Full as u32;
        let (_merged, report, _, core) = merge_both(
            &[
                two_epoch_source([15_000.0, 15_100.0]),
                two_epoch_source([15_000.0, 15_100.0]),
            ],
            &options,
        );
        assert_audit_matches_core(&report, &core);
        for (index, cell) in cells(&report).iter().enumerate() {
            assert_eq!(cell.position.kind, SidereonSp3CellSelectionKind::Combined);
            assert!(!cell.position.has_source);
            assert!(cell.position.has_rule);
            assert_eq!(cell.position.rule, SidereonSp3MergeCombine::Mean);
            assert_eq!(
                members(&report, index, SidereonSp3Channel::Position),
                vec![0, 1]
            );
        }
        for coverage in contributor_coverage(&report) {
            assert_eq!(coverage.cells_selected, 0);
            assert_eq!(coverage.cells_contributed, 2);
        }
        // A cell index past the provenance cells is refused.
        let mut written = 0usize;
        let mut required = 0usize;
        assert_eq!(
            unsafe {
                sidereon_sp3_merge_report_provenance_cell_members(
                    &*report,
                    2,
                    SidereonSp3Channel::Position as u32,
                    ptr::null_mut(),
                    0,
                    &mut written,
                    &mut required,
                )
            },
            SidereonStatus::InvalidArgument
        );
    }

    #[test]
    fn summary_and_full_modes_agree_on_every_transition_they_both_describe() {
        let sources = [
            two_epoch_source([15_000.0, 15_100.0]),
            late_source(15_100.0),
        ];
        let (_full_merged, full, _, core_full) = merge_both(
            &sources,
            &precedence_options(SidereonSp3ProvenanceMode::Full),
        );
        let (_summary_merged, summary, _, core_summary) = merge_both(
            &sources,
            &precedence_options(SidereonSp3ProvenanceMode::Summary),
        );
        assert_audit_matches_core(&full, &core_full);
        assert_audit_matches_core(&summary, &core_summary);
        assert_eq!(
            provenance_info(&summary).mode,
            SidereonSp3ProvenanceMode::Summary
        );
        assert!(cells(&summary).is_empty());
        assert!(!cells(&full).is_empty());
        let full_transitions = transitions(&full);
        let summary_transitions = transitions(&summary);
        assert_eq!(full_transitions.len(), summary_transitions.len());
        for (a, b) in full_transitions.iter().zip(&summary_transitions) {
            assert_eq!(token(&a.sat_id), token(&b.sat_id));
            assert_eq!(
                a.epoch_j2000_seconds.to_bits(),
                b.epoch_j2000_seconds.to_bits()
            );
            assert_eq!(
                (
                    a.has_from_source,
                    a.from_source,
                    a.has_to_source,
                    a.to_source,
                    a.reason
                ),
                (
                    b.has_from_source,
                    b.from_source,
                    b.has_to_source,
                    b.to_source,
                    b.reason
                )
            );
        }
    }

    #[test]
    fn the_merged_product_is_byte_identical_whether_or_not_provenance_is_enabled() {
        let sources = [
            two_epoch_source([15_000.0, 15_100.0]),
            late_source(15_100.5),
        ];
        let (without, _, _, _) = merge_both(
            &sources,
            &precedence_options(SidereonSp3ProvenanceMode::Off),
        );
        let (with, _, _, _) = merge_both(
            &sources,
            &precedence_options(SidereonSp3ProvenanceMode::Full),
        );
        assert_eq!(
            without.inner.to_sp3_string().expect("write"),
            with.inner.to_sp3_string().expect("write")
        );
    }

    #[test]
    fn an_invalid_provenance_mode_is_refused() {
        let mut options = options();
        options.provenance_mode = 3;
        assert!(unsafe { crate::sp3::sp3_merge_options_from_c("test", &options) }.is_err());
    }

    // --- sp3_merge_coverage.rs --------------------------------------------------

    fn coverage_of(sp3: &SidereonSp3) -> Box<SidereonSp3Coverage> {
        let mut coverage = ptr::null_mut();
        assert_eq!(
            unsafe { sidereon_sp3_satellite_coverage(sp3, &mut coverage) },
            SidereonStatus::Ok
        );
        unsafe { Box::from_raw(coverage) }
    }

    /// Every coverage record the C ABI copies is sidereon-core's.
    fn assert_coverage_matches_core(sp3: &SidereonSp3) {
        let core = sp3.inner.satellite_coverage();
        let coverage = coverage_of(sp3);
        let mut grid = unsafe { std::mem::zeroed::<SidereonSp3EpochGrid>() };
        assert_eq!(
            unsafe { sidereon_sp3_coverage_grid(&*coverage, &mut grid) },
            SidereonStatus::Ok
        );
        assert_eq!(
            grid.has_interval.then_some(grid.interval_s.to_bits()),
            core.grid.interval_s.map(f64::to_bits)
        );
        assert_eq!(grid.agrees_with_header, core.grid.agrees_with_header);
        let out_of_order: Vec<usize> = unsafe {
            rows(|out, len, w, r| {
                sidereon_sp3_coverage_grid_out_of_order(&*coverage, out, len, w, r)
            })
        };
        assert_eq!(out_of_order, core.grid.out_of_order);
        let unplaced: Vec<usize> = unsafe {
            rows(|out, len, w, r| sidereon_sp3_coverage_grid_unplaced(&*coverage, out, len, w, r))
        };
        assert_eq!(unplaced, core.grid.unplaced);
        let satellites: Vec<SidereonSp3SatelliteCoverage> = unsafe {
            rows(|out, len, w, r| sidereon_sp3_coverage_satellites(&*coverage, out, len, w, r))
        };
        assert_eq!(satellites.len(), core.satellites.len());
        for (index, (c, core)) in satellites.iter().zip(&core.satellites).enumerate() {
            assert_eq!(token(&c.sat_id), core.satellite.to_string());
            assert_eq!(c.declared, core.declared);
            for (channel, c_channel, core_channel) in [
                (SidereonSp3Channel::Position, c.positions, &core.positions),
                (SidereonSp3Channel::Clock, c.clocks, &core.clocks),
            ] {
                assert_eq!(c_channel.epochs, core_channel.epochs);
                assert_eq!(c_channel.complete, core_channel.is_complete());
                let spans: Vec<SidereonSp3CoverageSpan> = unsafe {
                    rows(|out, len, w, r| {
                        sidereon_sp3_coverage_spans(
                            &*coverage,
                            index,
                            channel as u32,
                            out,
                            len,
                            w,
                            r,
                        )
                    })
                };
                assert_eq!(spans.len(), core_channel.spans.len());
                assert_eq!(c_channel.span_count, spans.len());
                for (c_span, core_span) in spans.iter().zip(&core_channel.spans) {
                    assert_eq!(
                        (c_span.first_index, c_span.last_index),
                        (core_span.first_index, core_span.last_index)
                    );
                    assert!(same_epoch(&c_span.first_epoch, &core_span.first_epoch));
                    assert!(same_epoch(&c_span.last_epoch, &core_span.last_epoch));
                }
                let gaps: Vec<SidereonSp3CoverageGap> = unsafe {
                    rows(|out, len, w, r| {
                        sidereon_sp3_coverage_gaps(
                            &*coverage,
                            index,
                            channel as u32,
                            out,
                            len,
                            w,
                            r,
                        )
                    })
                };
                assert_eq!(c_channel.gap_count, gaps.len());
                let c_gaps: Vec<(Option<usize>, Option<usize>, usize)> = gaps
                    .iter()
                    .map(|gap| {
                        (
                            gap.has_after_index.then_some(gap.after_index),
                            gap.has_before_index.then_some(gap.before_index),
                            gap.missing_epochs,
                        )
                    })
                    .collect();
                let core_gaps: Vec<(Option<usize>, Option<usize>, usize)> = core_channel
                    .gaps
                    .iter()
                    .map(|gap| (gap.after_index, gap.before_index, gap.missing_epochs))
                    .collect();
                assert_eq!(c_gaps, core_gaps);
            }
        }
    }

    fn continuity_json(report: &SidereonSp3MergeReport) -> serde_json::Value {
        let bytes: Vec<u8> = unsafe {
            rows(|out, len, w, r| sidereon_sp3_merge_report_continuity_json(report, out, len, w, r))
        };
        serde_json::from_slice(&bytes).expect("continuity JSON")
    }

    #[test]
    fn a_hold_out_residual_is_attributed_to_every_node_its_prediction_used() {
        let mut options = coverage_precedence(SidereonSp3MergePrecedenceScope::Cell);
        options.verify_continuity_enabled = 1;
        let (merged, report, _, core) = merge_both(
            &[coverage_source(0, 72, 0.0), coverage_source(60, 24, 0.8)],
            &options,
        );
        assert_audit_matches_core(&report, &core);
        let continuity = core.continuity.as_ref().expect("verification requested");
        let json = continuity_json(&report);
        assert_eq!(
            json,
            crate::sp3::merge_continuity_report_json(continuity),
            "the C JSON is sidereon-core's report"
        );
        assert_eq!(json["attested"], false);
        let violations = json["violations"].as_array().expect("violations");
        assert_eq!(violations.len(), continuity.violations.len());
        let inside_b: Vec<_> = violations
            .iter()
            .filter(|violation| {
                violation["from_sources"] == serde_json::json!([1])
                    && violation["to_sources"] == serde_json::json!([1])
            })
            .collect();
        assert!(!inside_b.is_empty());
        for violation in inside_b {
            assert_eq!(violation["defect"]["kind"], "hold_out_residual");
            let nodes = violation["defect"]["node_epochs_j2000_s"]
                .as_array()
                .expect("nodes");
            assert_eq!(nodes.len(), 11);
            assert_eq!(violation["crosses_contributors"], true);
            assert_eq!(violation["sources"], serde_json::json!([0, 1]));
            let cells = violation["cells"].as_array().expect("cells");
            assert_eq!(cells.len(), 12);
            assert_eq!(
                cells
                    .iter()
                    .filter(|cell| cell["role"] == "held_out")
                    .count(),
                1
            );
            assert!(cells.iter().any(|cell| cell["role"] == "interpolation_node"
                && cell["selection"]
                    == serde_json::json!({"kind": "precedence", "source": 0, "members": [0, 1]})));
        }

        // The window selections the verdicts read are the merged product's own.
        let epochs = merged.inner.epochs_j2000_seconds();
        let sat = CString::new("G01").unwrap();
        for (from, through) in [(epochs[51], epochs[66]), (epochs[67], epochs[70])] {
            let mut verified = false;
            let verified_ptr: *mut bool = &mut verified;
            let from_report: Vec<f64> = unsafe {
                rows(|out, len, w, r| {
                    sidereon_sp3_merge_report_continuity_selected_nodes(
                        &*report,
                        sat.as_ptr(),
                        from,
                        through,
                        verified_ptr,
                        out,
                        len,
                        w,
                        r,
                    )
                })
            };
            assert!(verified);
            let window = EpochWindow::new(from, through).expect("window");
            let g01 = sidereon_core::GnssSatelliteId::new(sidereon_core::GnssSystem::Gps, 1)
                .expect("G01");
            assert_eq!(from_report, continuity.nodes.selected_nodes(g01, window));
            let from_product: Vec<f64> = unsafe {
                rows(|out, len, w, r| {
                    sidereon_sp3_selected_nodes(
                        &*merged,
                        sat.as_ptr(),
                        from,
                        through,
                        out,
                        len,
                        w,
                        r,
                    )
                })
            };
            assert_eq!(
                from_product,
                InterpolationNodes::for_sp3(&merged.inner).selected_nodes(g01, window)
            );
        }
    }

    #[test]
    fn the_undisplaced_merge_attests_and_an_unverified_merge_says_so() {
        let mut options = coverage_precedence(SidereonSp3MergePrecedenceScope::Cell);
        options.verify_continuity_enabled = 1;
        let sources = [coverage_source(0, 72, 0.0), coverage_source(60, 24, 0.0)];
        let (_merged, report, _, core) = merge_both(&sources, &options);
        assert_audit_matches_core(&report, &core);
        let json = continuity_json(&report);
        assert_eq!(json["attested"], true);
        assert_eq!(json["violations"], serde_json::json!([]));

        options.verify_continuity_enabled = 0;
        let (_merged, report, _, _) = merge_both(&sources, &options);
        assert_eq!(continuity_json(&report), serde_json::Value::Null);
        let sat = CString::new("G01").unwrap();
        let mut verified = true;
        let verified_ptr: *mut bool = &mut verified;
        let nodes: Vec<f64> = unsafe {
            rows(|out, len, w, r| {
                sidereon_sp3_merge_report_continuity_selected_nodes(
                    &*report,
                    sat.as_ptr(),
                    0.0,
                    1.0,
                    verified_ptr,
                    out,
                    len,
                    w,
                    r,
                )
            })
        };
        assert!(!verified);
        assert!(nodes.is_empty());
    }

    #[test]
    fn coverage_separates_positions_from_clocks_and_names_each_omitted_clock() {
        let (merged, report, core_merged, core) = merge_both(
            &[coverage_source(0, 72, 0.0), coverage_source(60, 24, 0.0)],
            &coverage_precedence(SidereonSp3MergePrecedenceScope::Cell),
        );
        assert_audit_matches_core(&report, &core);
        assert_coverage_matches_core(&merged);
        assert_eq!(merged.inner.epochs, core_merged.epochs);
        assert_eq!(merged.inner.epochs.len(), 84);
        assert!(core.omitted_epochs.is_empty());
        assert!(core.arc_withheld.is_empty());

        let coverage = coverage_of(&merged);
        let satellites: Vec<SidereonSp3SatelliteCoverage> = unsafe {
            rows(|out, len, w, r| sidereon_sp3_coverage_satellites(&*coverage, out, len, w, r))
        };
        assert_eq!(satellites.len(), 6);
        for satellite in &satellites {
            assert!(satellite.declared);
            assert_eq!(satellite.positions.epochs, 84);
            assert!(satellite.positions.complete);
            assert_eq!(satellite.clocks.epochs, 72);
            assert_eq!(satellite.clocks.span_count, 1);
        }
        let omissions: Vec<SidereonSp3ClockOmission> = unsafe {
            rows(|out, len, w, r| {
                sidereon_sp3_merge_report_clock_omissions(&*report, out, len, w, r)
            })
        };
        assert_eq!(omissions.len(), 6 * 12);
        for omission in &omissions {
            assert_eq!(
                omission.reason,
                SidereonSp3ClockOmissionReason::DatumNotObservable
            );
            assert_eq!(omission.source, 1);
            assert!(!omission.cell_has_clock);
        }
    }

    #[test]
    fn satellite_arc_precedence_omits_and_reports_empty_epochs() {
        let sources = [coverage_source(0, 72, 0.0), coverage_source(60, 24, 0.0)];
        let (merged, report, _, core) = merge_both(
            &sources,
            &coverage_precedence(SidereonSp3MergePrecedenceScope::SatelliteArc),
        );
        assert_audit_matches_core(&report, &core);
        assert_coverage_matches_core(&merged);
        assert_eq!(merged.inner.epochs.len(), 72);
        let omitted: Vec<SidereonSp3MergeEpoch> = unsafe {
            rows(|out, len, w, r| {
                sidereon_sp3_merge_report_omitted_epochs(&*report, out, len, w, r)
            })
        };
        assert_eq!(omitted.len(), 12);
        for (c, expected) in omitted.iter().zip(&sources[1].epochs[12..]) {
            assert!(same_epoch(&c.epoch, expected));
        }
        let mut count = 0usize;
        assert_eq!(
            unsafe {
                sidereon_sp3_merge_report_flag_count(
                    &*report,
                    SidereonSp3MergeFlagKind::ArcWithheld as u32,
                    &mut count,
                )
            },
            SidereonStatus::Ok
        );
        assert_eq!(count, 6 * 12);
        for index in 0..count {
            let sources: Vec<usize> = unsafe {
                rows(|out, len, w, r| {
                    sidereon_sp3_merge_report_flag_sources(
                        &*report,
                        SidereonSp3MergeFlagKind::ArcWithheld as u32,
                        index,
                        out,
                        len,
                        w,
                        r,
                    )
                })
            };
            assert_eq!(sources, vec![1]);
        }
    }

    #[test]
    fn an_explicit_target_reports_every_input_epoch_it_drops() {
        let mut options = coverage_precedence(SidereonSp3MergePrecedenceScope::Cell);
        options.target_epoch_interval_s_enabled = 1;
        options.target_epoch_interval_s = 900.0;
        let (merged, report, _, core) = merge_both(&[coverage_source(0, 12, 0.0)], &options);
        assert_audit_matches_core(&report, &core);
        assert_eq!(merged.inner.epochs.len(), 4);
        let dropped: Vec<SidereonSp3DroppedInputEpoch> = unsafe {
            rows(|out, len, w, r| {
                sidereon_sp3_merge_report_dropped_input_epochs(&*report, out, len, w, r)
            })
        };
        assert_eq!(dropped.len(), 8);
        for entry in &dropped {
            assert_eq!(entry.source, 0);
            assert_ne!(entry.epoch_index % 3, 0);
            assert_eq!(entry.reason, SidereonSp3DroppedEpochReason::OffTargetGrid);
        }
    }

    #[test]
    fn a_target_on_the_tick_axis_is_the_engine_decision() {
        // A 450.5 s target is a whole number of SP3 ticks the binding no longer
        // refuses on its own rule; a sub-tick target is the engine's refusal.
        let mut options = coverage_precedence(SidereonSp3MergePrecedenceScope::Cell);
        options.target_epoch_interval_s_enabled = 1;
        options.target_epoch_interval_s = 450.5;
        let (merged, _report, core_merged, _) =
            merge_both(&[coverage_source(0, 12, 0.0)], &options);
        assert_eq!(merged.inner.epochs, core_merged.epochs);
        options.target_epoch_interval_s = 1.0e-9;
        let source = SidereonSp3 {
            inner: coverage_source(0, 12, 0.0),
        };
        let pointers = [&source as *const SidereonSp3];
        let mut out_sp3 = ptr::null_mut();
        let mut out_report = ptr::null_mut();
        assert_eq!(
            unsafe {
                sidereon_sp3_merge(
                    pointers.as_ptr(),
                    1,
                    &options,
                    &mut out_sp3,
                    &mut out_report,
                )
            },
            SidereonStatus::InvalidArgument
        );
        assert!(out_sp3.is_null() && out_report.is_null());
        options.target_epoch_interval_s = 100_000.0;
        assert_eq!(
            unsafe {
                sidereon_sp3_merge(
                    pointers.as_ptr(),
                    1,
                    &options,
                    &mut out_sp3,
                    &mut out_report,
                )
            },
            SidereonStatus::InvalidArgument
        );
        assert!(out_sp3.is_null() && out_report.is_null());
    }

    #[test]
    fn each_source_clock_left_out_of_a_clocked_cell_is_reported() {
        let indices: Vec<usize> = (0..12).collect();
        let a = product(&indices, 0.0, |_, _| Record::Full);
        let b = product(&indices, 0.0, |_, _| Record::Full);
        let c = product(&indices, 0.0, |prn, _| {
            if prn <= 4 {
                Record::Full
            } else {
                Record::NoClock
            }
        });
        let (merged, report, _, core) = merge_both(&[a, b, c], &options());
        assert_audit_matches_core(&report, &core);
        assert_eq!(merged.inner.epochs.len(), 12);
        let omissions: Vec<SidereonSp3ClockOmission> = unsafe {
            rows(|out, len, w, r| {
                sidereon_sp3_merge_report_clock_omissions(&*report, out, len, w, r)
            })
        };
        assert_eq!(omissions.len(), 12 * 4);
        for omission in &omissions {
            assert_eq!(omission.source, 2);
            assert_eq!(
                omission.reason,
                SidereonSp3ClockOmissionReason::DatumNotObservable
            );
            assert!(omission.cell_has_clock);
        }
    }

    #[test]
    fn coverage_of_a_product_reports_spans_and_gaps_per_channel() {
        let indices = [0, 1, 2, 3, 4, 6, 7, 8];
        let sp3 = SidereonSp3 {
            inner: product(&indices, 0.0, |prn, index| match (prn, index) {
                (1, _) => Record::Full,
                (2, 2 | 3) => Record::Absent,
                (2, _) => Record::Full,
                (3, 7 | 8) => Record::NoClock,
                (3, _) => Record::Full,
                (4, 0 | 1) => Record::ClockOnly,
                (4, _) => Record::Full,
                (_, _) => Record::Absent,
            }),
        };
        assert_coverage_matches_core(&sp3);
        let coverage = coverage_of(&sp3);
        let mut grid = unsafe { std::mem::zeroed::<SidereonSp3EpochGrid>() };
        assert_eq!(
            unsafe { sidereon_sp3_coverage_grid(&*coverage, &mut grid) },
            SidereonStatus::Ok
        );
        assert!(grid.has_interval);
        assert_eq!(grid.interval_s, 300.0);
        assert!(grid.agrees_with_header);
        let g01_gaps: Vec<SidereonSp3CoverageGap> = unsafe {
            rows(|out, len, w, r| {
                sidereon_sp3_coverage_gaps(
                    &*coverage,
                    0,
                    SidereonSp3Channel::Position as u32,
                    out,
                    len,
                    w,
                    r,
                )
            })
        };
        assert_eq!(g01_gaps.len(), 1);
        assert!(g01_gaps[0].has_after_index && g01_gaps[0].has_before_index);
        assert_eq!(
            (
                g01_gaps[0].after_index,
                g01_gaps[0].before_index,
                g01_gaps[0].missing_epochs
            ),
            (4, 5, 0)
        );
        // A satellite index past the list is refused.
        let mut written = 0usize;
        let mut required = 0usize;
        assert_eq!(
            unsafe {
                sidereon_sp3_coverage_spans(
                    &*coverage,
                    6,
                    SidereonSp3Channel::Clock as u32,
                    ptr::null_mut(),
                    0,
                    &mut written,
                    &mut required,
                )
            },
            SidereonStatus::InvalidArgument
        );
    }

    #[test]
    fn coverage_reports_epochs_out_of_order() {
        let sp3 = SidereonSp3 {
            inner: product(&[0, 2, 1, 3], 0.0, |_, _| Record::Full),
        };
        assert_coverage_matches_core(&sp3);
        let coverage = coverage_of(&sp3);
        let out_of_order: Vec<usize> = unsafe {
            rows(|out, len, w, r| {
                sidereon_sp3_coverage_grid_out_of_order(&*coverage, out, len, w, r)
            })
        };
        assert_eq!(out_of_order, vec![2]);
    }

    #[test]
    fn a_standalone_check_reports_every_defect_under_an_explicit_speed_bound() {
        // The 0.8 m merge product, checked alone with an explicit bound low
        // enough to flag every pair, and with the MEO class bound.
        let (merged, _, _, _) = merge_both(
            &[coverage_source(0, 72, 0.0), coverage_source(60, 24, 0.8)],
            &coverage_precedence(SidereonSp3MergePrecedenceScope::Cell),
        );
        let mut meo = unsafe { std::mem::zeroed::<SidereonSp3ContinuityOptions>() };
        assert_eq!(
            unsafe {
                sidereon_sp3_continuity_options_for_orbit_class(
                    SidereonSp3OrbitClass::MeoGnss as u32,
                    &mut meo,
                )
            },
            SidereonStatus::Ok
        );
        let mut explicit = meo;
        explicit.speed_bound_kind = SidereonSp3SpeedBoundKind::ExplicitMaxSpeed as u32;
        explicit.explicit_max_speed_m_s = 1.0;
        for options in [meo, explicit] {
            let core_options =
                crate::sp3::continuity_options_struct_from_c("test", &options).expect("options");
            let core_report = sidereon_core::ephemeris::check_continuity(
                &merged.inner.precise_ephemeris_samples(),
                &core_options,
            )
            .expect("valid continuity options");
            let bytes: Vec<u8> = unsafe {
                rows(|out, len, w, r| {
                    sidereon_sp3_continuity_report_json(&*merged, &options, out, len, w, r)
                })
            };
            let json: serde_json::Value = serde_json::from_slice(&bytes).expect("report JSON");
            assert_eq!(json, crate::sp3::continuity_report_json(&core_report));
            assert_eq!(json["pairs_checked"], core_report.pairs_checked);

            let epochs = merged.inner.epochs_j2000_seconds();
            let window = EpochWindow::new(epochs[60], epochs[70]).expect("window");
            let stencil =
                sidereon_core::ephemeris::StencilExtent::for_sp3(&merged.inner).expect("stencil");
            let verdict: Vec<u8> = unsafe {
                rows(|out, len, w, r| {
                    sidereon_sp3_continuity_verdict_json_with_options(
                        &*merged, &options, epochs[60], epochs[70], out, len, w, r,
                    )
                })
            };
            let verdict: serde_json::Value = serde_json::from_slice(&verdict).expect("verdict");
            assert_eq!(
                verdict,
                crate::sp3::window_continuity_verdict_json(
                    core_report.verdict_for_window(window, stencil)
                )
            );
        }
        // Every adjacent pair moves faster than 1 m/s.
        let explicit_report = sidereon_core::ephemeris::check_continuity(
            &merged.inner.precise_ephemeris_samples(),
            &crate::sp3::continuity_options_struct_from_c("test", &explicit).expect("options"),
        )
        .expect("valid continuity options");
        assert!(!explicit_report.attested());
    }

    // --- continuity options -----------------------------------------------------

    #[test]
    fn continuity_options_initialize_to_the_engine_class_settings_and_refuse_nan_bounds() {
        for (class, core_class) in [
            (SidereonSp3OrbitClass::MeoGnss, OrbitClass::MeoGnss),
            (
                SidereonSp3OrbitClass::Geosynchronous,
                OrbitClass::Geosynchronous,
            ),
            (SidereonSp3OrbitClass::Leo, OrbitClass::Leo),
        ] {
            let mut options = unsafe { std::mem::zeroed::<SidereonSp3ContinuityOptions>() };
            assert_eq!(
                unsafe {
                    sidereon_sp3_continuity_options_for_orbit_class(class as u32, &mut options)
                },
                SidereonStatus::Ok
            );
            assert_eq!(
                crate::sp3::continuity_options_struct_from_c("test", &options).expect("options"),
                ContinuityOptions::for_orbit_class(core_class)
            );
        }
        let mut options = unsafe { std::mem::zeroed::<SidereonSp3ContinuityOptions>() };
        assert_eq!(
            unsafe { sidereon_sp3_continuity_options_for_orbit_class(9, &mut options) },
            SidereonStatus::InvalidArgument
        );
        assert_eq!(
            unsafe {
                sidereon_sp3_continuity_options_for_orbit_class(
                    SidereonSp3OrbitClass::MeoGnss as u32,
                    &mut options,
                )
            },
            SidereonStatus::Ok
        );
        let mut explicit = options;
        explicit.speed_bound_kind = SidereonSp3SpeedBoundKind::ExplicitMaxSpeed as u32;
        explicit.explicit_max_speed_m_s = 6_000.0;
        assert_eq!(
            crate::sp3::continuity_options_struct_from_c("test", &explicit)
                .expect("explicit")
                .speed_bound,
            Some(sidereon_core::ephemeris::SpeedBound::ExplicitMaxSpeed(
                6_000.0
            ))
        );
        explicit.explicit_max_speed_m_s = f64::NAN;
        assert!(crate::sp3::continuity_options_struct_from_c("test", &explicit).is_err());
        let mut residual = options;
        residual.residual_tolerance_m = f64::NAN;
        assert!(crate::sp3::continuity_options_struct_from_c("test", &residual).is_err());
        residual.residual_tolerance_enabled = 0;
        assert_eq!(
            crate::sp3::continuity_options_struct_from_c("test", &residual)
                .expect("no residual check")
                .residual_tolerance_m,
            None
        );
    }
}
