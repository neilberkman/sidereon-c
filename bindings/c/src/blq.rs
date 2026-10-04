use super::*;
use sidereon_core::tides::{
    parse_ocean_loading_blq_blocks, write_ocean_loading_blq_blocks, BlqParseErrorKind,
    BlqWriteErrorKind, OceanLoadingBlq as CoreOceanLoadingBlq, OceanLoadingBlqBlock,
    OceanLoadingBlqComment, OceanLoadingBlqCommentPlacement, OceanTideConstituent, TideError,
};

// --- BLQ ocean-loading station blocks (sidereon_core::tides) -----------------
//
// A block keeps its station, its six coefficient rows reordered to the
// supported constituent order, and every comment and column-order header line
// in input order with its placement. The writer restates those lines and
// refuses a block the parser would not read back unchanged.

/// An owned, ordered list of BLQ station blocks. Create with sidereon_blq_parse
/// or sidereon_blq_blocks_new and release with sidereon_blq_blocks_free.
pub struct SidereonBlqBlocks {
    pub(crate) blocks: Vec<OceanLoadingBlqBlock>,
}

/// A BLQ tidal constituent, in the supported column order.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonOceanTideConstituent {
    /// `M2`.
    M2 = 0,
    /// `S2`.
    S2 = 1,
    /// `N2`.
    N2 = 2,
    /// `K2`.
    K2 = 3,
    /// `K1`.
    K1 = 4,
    /// `O1`.
    O1 = 5,
    /// `P1`.
    P1 = 6,
    /// `Q1`.
    Q1 = 7,
    /// `Mf`.
    Mf = 8,
    /// `Mm`.
    Mm = 9,
    /// `Ssa`.
    Ssa = 10,
}

/// Where a retained comment or column-order header line sits in its block.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonBlqCommentPlacement {
    /// Before the station line. The parser gives a block every line after the
    /// previous block's last coefficient row, so file header comments belong to
    /// the first block.
    BeforeStation = 0,
    /// Before the zero-based coefficient row in row, 0..=5; row 0 is between
    /// the station line and the first row. A column-order header here sets the
    /// order of this row and every later one.
    BeforeRow = 1,
    /// After the sixth coefficient row. The parser uses it only for lines after
    /// the last block of the input, and the writer accepts it only on the last
    /// block it writes.
    AfterRows = 2,
}

/// The placement of one retained BLQ line. Its text is copied with
/// sidereon_blq_blocks_comment.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct SidereonBlqComment {
    /// Placement, as SidereonBlqCommentPlacement.
    pub placement: u32,
    /// Coefficient row the line precedes, when placement is BeforeRow; 0
    /// otherwise.
    pub row: usize,
}

/// Which family of BLQ failure an operation reported.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonBlqErrorKind {
    /// No failure is recorded.
    None = 0,
    /// The parser refused a line or the whole input; parse_kind names why.
    Parse = 1,
    /// The writer refused a block it could not write so that it reads back
    /// unchanged; write_kind names why.
    Write = 2,
    /// Another engine failure. Its text is in the message.
    Other = 999,
}

/// Why the BLQ parser refused an input. Every kind but None names a
/// `BlqParseErrorKind` variant of the engine.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonBlqParseErrorKind {
    /// No parse failure.
    None = 0,
    /// The input holds nothing but whitespace.
    Empty = 1,
    /// A coefficient row appears before a station line.
    MissingStation = 2,
    /// A station ended before its six coefficient rows. Carries the station as
    /// text, expected and found.
    MissingCoefficientRows = 3,
    /// A station accumulated more than six rows. Carries the station as text.
    TooManyCoefficientRows = 4,
    /// A row or header does not hold eleven columns. Carries expected and
    /// found.
    WrongColumnCount = 5,
    /// A coefficient token is not a number. Carries the token as text.
    InvalidNumber = 6,
    /// A coefficient token is not finite. Carries the token as text.
    NonFiniteNumber = 7,
    /// A header label is not one of the eleven supported constituents, such as
    /// `SA` for `Ssa`. Carries the label as text.
    UnsupportedConstituent = 8,
    /// A header names a constituent twice. Carries the label as text.
    DuplicateConstituent = 9,
    /// The single-block parser found more than one block. Carries found.
    MultipleBlocks = 10,
}

/// Why the BLQ writer refused a block. Every kind but None names a
/// `BlqWriteErrorKind` variant of the engine.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonBlqWriteErrorKind {
    /// No write failure.
    None = 0,
    /// The station identifier is empty.
    EmptyStation = 1,
    /// The station identifier contains a line break.
    StationLineBreak = 2,
    /// The station identifier has leading or trailing whitespace, which the
    /// parser trims.
    StationSurroundingWhitespace = 3,
    /// The station identifier starts with a comment marker (`$`, `#`, `!`).
    StationReadsAsComment = 4,
    /// The station identifier would read as a column-order header.
    StationReadsAsHeader = 5,
    /// The station identifier would read as a coefficient row.
    StationReadsAsCoefficientRow = 6,
    /// A coefficient is NaN or infinite. Carries row and constituent.
    NonFiniteCoefficient = 7,
    /// A retained line contains a line break or ends with a carriage return.
    /// Carries comment_index.
    CommentLineBreak = 8,
    /// A retained line is blank or has no comment marker and is not a
    /// column-order header. Carries comment_index.
    NotACommentLine = 9,
    /// A retained line names a coefficient row after the sixth. Carries
    /// comment_index.
    CommentPlacementOutOfRange = 10,
    /// A retained line is a column-order header the parser refuses. Carries
    /// comment_index and the parser's refusal in parse_kind with its payload.
    InvalidHeader = 11,
    /// A retained line follows the rows of a block that is not the last one
    /// written; the parser would read it as part of the next block. Carries
    /// comment_index.
    AfterRowsBeforeAnotherBlock = 12,
    /// Retained lines are not grouped by placement in file order. Carries
    /// comment_index, the first line placed before its predecessor.
    CommentsOutOfPlacementOrder = 13,
}

/// Typed detail of a BLQ failure. Only the fields the kinds name carry meaning,
/// each behind a present flag; the text part (a station, token or constituent
/// label) is copied with sidereon_blq_result_error_text.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct SidereonBlqError {
    /// The failure family.
    pub kind: SidereonBlqErrorKind,
    /// Whether line carries the parser's line number.
    pub has_line: bool,
    /// One-based offending line, or 0 for a whole-input failure.
    pub line: usize,
    /// Why the parser refused, as SidereonBlqParseErrorKind; also the nested
    /// refusal of a write InvalidHeader.
    pub parse_kind: u32,
    /// Whether expected carries a required count.
    pub has_expected: bool,
    /// Required count (coefficient rows or columns).
    pub expected: usize,
    /// Whether found carries a found count.
    pub has_found: bool,
    /// Found count (coefficient rows, columns or blocks).
    pub found: usize,
    /// Whether the failure carries a text part.
    pub has_text: bool,
    /// Whether block carries the refused block's index.
    pub has_block: bool,
    /// Zero-based index of the refused block in the written sequence.
    pub block: usize,
    /// Why the writer refused, as SidereonBlqWriteErrorKind.
    pub write_kind: u32,
    /// Whether row and constituent carry a non-finite coefficient's place.
    pub has_coefficient: bool,
    /// Zero-based BLQ row: amplitudes radial, west, south, then phases.
    pub row: usize,
    /// Constituent of the coefficient, as SidereonOceanTideConstituent.
    pub constituent: u32,
    /// Whether comment_index carries a retained line's index.
    pub has_comment_index: bool,
    /// Index of the retained line in the block's comments.
    pub comment_index: usize,
}

/// The fixed-width outcome of one BLQ operation.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct SidereonBlqOutcome {
    /// Whether the operation succeeded.
    pub is_ok: bool,
    /// SIDEREON_STATUS_OK on success, otherwise
    /// SIDEREON_STATUS_INVALID_ARGUMENT, which every failure maps to.
    pub status: SidereonStatus,
    /// The typed failure; kind is None when is_ok is true.
    pub error: SidereonBlqError,
}

/// Which text of a BLQ result sidereon_blq_result_error_text copies.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonBlqErrorText {
    /// The complete failure text, prefixed with the route that produced it.
    Message = 0,
    /// The station, token or constituent label the failure carries.
    Text = 1,
}

/// An owned record of one BLQ parse or write: the written text on success, or
/// the typed failure with its owned text. Release with sidereon_blq_result_free.
pub struct SidereonBlqResult {
    pub(crate) outcome: SidereonBlqOutcome,
    pub(crate) message: String,
    pub(crate) error_text: String,
    pub(crate) text: String,
}

fn constituent_to_c(constituent: OceanTideConstituent) -> u32 {
    let value = match constituent {
        OceanTideConstituent::M2 => SidereonOceanTideConstituent::M2,
        OceanTideConstituent::S2 => SidereonOceanTideConstituent::S2,
        OceanTideConstituent::N2 => SidereonOceanTideConstituent::N2,
        OceanTideConstituent::K2 => SidereonOceanTideConstituent::K2,
        OceanTideConstituent::K1 => SidereonOceanTideConstituent::K1,
        OceanTideConstituent::O1 => SidereonOceanTideConstituent::O1,
        OceanTideConstituent::P1 => SidereonOceanTideConstituent::P1,
        OceanTideConstituent::Q1 => SidereonOceanTideConstituent::Q1,
        OceanTideConstituent::Mf => SidereonOceanTideConstituent::Mf,
        OceanTideConstituent::Mm => SidereonOceanTideConstituent::Mm,
        OceanTideConstituent::Ssa => SidereonOceanTideConstituent::Ssa,
    };
    value as u32
}

fn no_blq_error() -> SidereonBlqError {
    SidereonBlqError {
        kind: SidereonBlqErrorKind::None,
        has_line: false,
        line: 0,
        parse_kind: SidereonBlqParseErrorKind::None as u32,
        has_expected: false,
        expected: 0,
        has_found: false,
        found: 0,
        has_text: false,
        has_block: false,
        block: 0,
        write_kind: SidereonBlqWriteErrorKind::None as u32,
        has_coefficient: false,
        row: 0,
        constituent: 0,
        has_comment_index: false,
        comment_index: 0,
    }
}

/// Fill the parse-kind fields and return the kind's text part.
fn fill_blq_parse_kind(out: &mut SidereonBlqError, kind: &BlqParseErrorKind) -> Option<String> {
    use SidereonBlqParseErrorKind as Kind;
    let (value, text) = match kind {
        BlqParseErrorKind::Empty => (Kind::Empty, None),
        BlqParseErrorKind::MissingStation => (Kind::MissingStation, None),
        BlqParseErrorKind::MissingCoefficientRows {
            station,
            expected,
            found,
        } => {
            out.has_expected = true;
            out.expected = *expected;
            out.has_found = true;
            out.found = *found;
            (Kind::MissingCoefficientRows, Some(station.clone()))
        }
        BlqParseErrorKind::TooManyCoefficientRows { station } => {
            (Kind::TooManyCoefficientRows, Some(station.clone()))
        }
        BlqParseErrorKind::WrongColumnCount { expected, found } => {
            out.has_expected = true;
            out.expected = *expected;
            out.has_found = true;
            out.found = *found;
            (Kind::WrongColumnCount, None)
        }
        BlqParseErrorKind::InvalidNumber { token } => (Kind::InvalidNumber, Some(token.clone())),
        BlqParseErrorKind::NonFiniteNumber { token } => {
            (Kind::NonFiniteNumber, Some(token.clone()))
        }
        BlqParseErrorKind::UnsupportedConstituent { constituent } => {
            (Kind::UnsupportedConstituent, Some(constituent.clone()))
        }
        BlqParseErrorKind::DuplicateConstituent { constituent } => {
            (Kind::DuplicateConstituent, Some(constituent.clone()))
        }
        BlqParseErrorKind::MultipleBlocks { found } => {
            out.has_found = true;
            out.found = *found;
            (Kind::MultipleBlocks, None)
        }
    };
    out.parse_kind = value as u32;
    out.has_text = text.is_some();
    text
}

/// Map every field of a BLQ failure into the C record and its text part.
fn blq_error_to_c(err: &TideError) -> (SidereonBlqError, String) {
    use SidereonBlqWriteErrorKind as Kind;
    let mut out = no_blq_error();
    let text = match err {
        TideError::BlqParse { line, kind } => {
            out.kind = SidereonBlqErrorKind::Parse;
            out.has_line = true;
            out.line = *line;
            fill_blq_parse_kind(&mut out, kind)
        }
        TideError::BlqWrite { block, kind } => {
            out.kind = SidereonBlqErrorKind::Write;
            out.has_block = true;
            out.block = *block;
            let mut comment = |index: usize| {
                out.has_comment_index = true;
                out.comment_index = index;
            };
            let (value, text) = match kind {
                BlqWriteErrorKind::EmptyStation => (Kind::EmptyStation, None),
                BlqWriteErrorKind::StationLineBreak => (Kind::StationLineBreak, None),
                BlqWriteErrorKind::StationSurroundingWhitespace => {
                    (Kind::StationSurroundingWhitespace, None)
                }
                BlqWriteErrorKind::StationReadsAsComment => (Kind::StationReadsAsComment, None),
                BlqWriteErrorKind::StationReadsAsHeader => (Kind::StationReadsAsHeader, None),
                BlqWriteErrorKind::StationReadsAsCoefficientRow => {
                    (Kind::StationReadsAsCoefficientRow, None)
                }
                BlqWriteErrorKind::NonFiniteCoefficient { .. } => {
                    (Kind::NonFiniteCoefficient, None)
                }
                BlqWriteErrorKind::CommentLineBreak { index } => {
                    comment(*index);
                    (Kind::CommentLineBreak, None)
                }
                BlqWriteErrorKind::NotACommentLine { index } => {
                    comment(*index);
                    (Kind::NotACommentLine, None)
                }
                BlqWriteErrorKind::CommentPlacementOutOfRange { index } => {
                    comment(*index);
                    (Kind::CommentPlacementOutOfRange, None)
                }
                BlqWriteErrorKind::InvalidHeader { index, .. } => {
                    comment(*index);
                    (Kind::InvalidHeader, None)
                }
                BlqWriteErrorKind::AfterRowsBeforeAnotherBlock { index } => {
                    comment(*index);
                    (Kind::AfterRowsBeforeAnotherBlock, None)
                }
                BlqWriteErrorKind::CommentsOutOfPlacementOrder { index } => {
                    comment(*index);
                    (Kind::CommentsOutOfPlacementOrder, None::<String>)
                }
            };
            out.write_kind = value as u32;
            match kind {
                BlqWriteErrorKind::NonFiniteCoefficient { row, constituent } => {
                    out.has_coefficient = true;
                    out.row = *row;
                    out.constituent = constituent_to_c(*constituent);
                    text
                }
                BlqWriteErrorKind::InvalidHeader { kind, .. } => {
                    fill_blq_parse_kind(&mut out, kind)
                }
                _ => text,
            }
        }
        _ => {
            out.kind = SidereonBlqErrorKind::Other;
            None
        }
    };
    (out, text.unwrap_or_default())
}

fn blq_result_ok(text: String) -> SidereonBlqResult {
    SidereonBlqResult {
        outcome: SidereonBlqOutcome {
            is_ok: true,
            status: SidereonStatus::Ok,
            error: no_blq_error(),
        },
        message: String::new(),
        error_text: String::new(),
        text,
    }
}

fn blq_result_refused(fn_name: &str, err: &TideError) -> SidereonBlqResult {
    let (error, error_text) = blq_error_to_c(err);
    SidereonBlqResult {
        outcome: SidereonBlqOutcome {
            is_ok: false,
            status: SidereonStatus::InvalidArgument,
            error,
        },
        message: format!("{fn_name}: {err}"),
        error_text,
        text: String::new(),
    }
}

/// Finish an operation: set the thread-local message on a refusal, hand the
/// owned result to an optional slot, and return the operation's status.
unsafe fn finish_blq_operation(
    out_result: *mut *mut SidereonBlqResult,
    result: SidereonBlqResult,
) -> SidereonStatus {
    let status = result.outcome.status;
    if !result.outcome.is_ok {
        set_last_error(result.message.clone());
    }
    if !out_result.is_null() {
        write_boxed_handle(out_result, result);
    }
    status
}

unsafe fn blq_block_at<'a>(
    fn_name: &str,
    blocks: *const SidereonBlqBlocks,
    index: usize,
) -> Result<&'a OceanLoadingBlqBlock, SidereonStatus> {
    let blocks = require_ref(blocks, fn_name, "blocks")?;
    blocks.blocks.get(index).ok_or_else(|| {
        set_last_error(format!("{fn_name}: index {index} out of range"));
        SidereonStatus::InvalidArgument
    })
}

/// Parse every standard station block of a BLQ file. Lines starting with `$`,
/// `#` or `!` are comments; a column-order header (a `COLUMN ORDER`
/// declaration, a line of constituent labels only, or a comment in which a
/// word `ORDER` is followed to the end of the line by constituent labels) sets
/// the column order of every later row, and its labels must be the eleven
/// supported constituents, each once. Every comment and header line is kept on
/// the block it belongs to.
///
/// Returns SIDEREON_STATUS_OK with a newly owned list in *out_blocks, or
/// SIDEREON_STATUS_INVALID_ARGUMENT with *out_blocks NULL. When out_result is
/// not NULL it receives an owned SidereonBlqResult recording the outcome of a
/// well-formed call.
///
/// Safety: text points to len readable UTF-8 bytes; out_blocks points to a
/// SidereonBlqBlocks*; out_result is NULL or points to a SidereonBlqResult*.
/// Both output slots are set to NULL before any work.
#[no_mangle]
pub unsafe extern "C" fn sidereon_blq_parse(
    text: *const u8,
    len: usize,
    out_blocks: *mut *mut SidereonBlqBlocks,
    out_result: *mut *mut SidereonBlqResult,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_blq_parse";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        if !out_blocks.is_null() {
            *out_blocks = ptr::null_mut();
        }
        if !out_result.is_null() {
            *out_result = ptr::null_mut();
        }
        let out_blocks = c_try!(require_out(out_blocks, FN_NAME, "out_blocks"));
        let bytes = c_try!(require_slice(text, len, FN_NAME, "text"));
        let Ok(text) = str::from_utf8(bytes) else {
            set_last_error(format!("{FN_NAME}: text is not valid UTF-8"));
            return SidereonStatus::InvalidToken;
        };
        let result = match parse_ocean_loading_blq_blocks(text) {
            Ok(blocks) => {
                write_boxed_handle(out_blocks, SidereonBlqBlocks { blocks });
                blq_result_ok(String::new())
            }
            Err(err) => blq_result_refused(FN_NAME, &err),
        };
        finish_blq_operation(out_result, result)
    })
}

/// Create an empty BLQ block list to build with sidereon_blq_blocks_push.
///
/// Safety: out_blocks points to a SidereonBlqBlocks*.
#[no_mangle]
pub unsafe extern "C" fn sidereon_blq_blocks_new(
    out_blocks: *mut *mut SidereonBlqBlocks,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_blq_blocks_new";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        let out_blocks = c_try!(require_out(out_blocks, FN_NAME, "out_blocks"));
        write_boxed_handle(out_blocks, SidereonBlqBlocks { blocks: Vec::new() });
        SidereonStatus::Ok
    })
}

/// Release a BLQ block list. Passing NULL is a no-op.
///
/// Safety: blocks must be NULL or a live handle from a BLQ list route.
#[no_mangle]
pub unsafe extern "C" fn sidereon_blq_blocks_free(blocks: *mut SidereonBlqBlocks) {
    free_boxed(blocks);
}

/// Write the number of blocks in a BLQ list.
///
/// Safety: blocks is a live handle; out_count points to a size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_blq_blocks_count(
    blocks: *const SidereonBlqBlocks,
    out_count: *mut usize,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_blq_blocks_count";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        let out_count = c_try!(require_out(out_count, FN_NAME, "out_count"));
        *out_count = 0;
        let blocks = c_try!(require_ref(blocks, FN_NAME, "blocks"));
        *out_count = blocks.blocks.len();
        SidereonStatus::Ok
    })
}

/// Copy one block's coefficients, reordered to the supported constituent order:
/// row 0 radial, 1 west, 2 south.
///
/// Safety: blocks is a live handle; out_coefficients points to a
/// SidereonOceanLoadingBlq.
#[no_mangle]
pub unsafe extern "C" fn sidereon_blq_blocks_coefficients(
    blocks: *const SidereonBlqBlocks,
    index: usize,
    out_coefficients: *mut SidereonOceanLoadingBlq,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_blq_blocks_coefficients";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        let out = c_try!(require_out(out_coefficients, FN_NAME, "out_coefficients"));
        *out = SidereonOceanLoadingBlq {
            amplitude_m: [[f64::NAN; SIDEREON_PPP_OCEAN_CONSTITUENTS]; 3],
            phase_deg: [[f64::NAN; SIDEREON_PPP_OCEAN_CONSTITUENTS]; 3],
        };
        let block = c_try!(blq_block_at(FN_NAME, blocks, index));
        *out = SidereonOceanLoadingBlq {
            amplitude_m: block.coefficients.amplitude_m,
            phase_deg: block.coefficients.phase_deg,
        };
        SidereonStatus::Ok
    })
}

/// Copy one block's station identifier, trimmed. Uses the variable-length
/// output contract; the bytes are not null-terminated.
///
/// Safety: blocks is a live handle; out points to len writable bytes or is
/// NULL when len is 0; out_written and out_required point to size_t values.
#[no_mangle]
pub unsafe extern "C" fn sidereon_blq_blocks_station(
    blocks: *const SidereonBlqBlocks,
    index: usize,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_blq_blocks_station";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        c_try!(init_copy_counts(FN_NAME, out_written, out_required));
        let block = c_try!(blq_block_at(FN_NAME, blocks, index));
        c_try!(copy_prefix_to_c(
            FN_NAME,
            "out",
            block.station.as_bytes(),
            out,
            len,
            out_written,
            out_required,
        ));
        SidereonStatus::Ok
    })
}

/// Write the number of comment and column-order header lines one block keeps.
///
/// Safety: blocks is a live handle; out_count points to a size_t.
#[no_mangle]
pub unsafe extern "C" fn sidereon_blq_blocks_comment_count(
    blocks: *const SidereonBlqBlocks,
    index: usize,
    out_count: *mut usize,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_blq_blocks_comment_count";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        let out_count = c_try!(require_out(out_count, FN_NAME, "out_count"));
        *out_count = 0;
        let block = c_try!(blq_block_at(FN_NAME, blocks, index));
        *out_count = block.comments.len();
        SidereonStatus::Ok
    })
}

/// Copy one retained line of a block, in input order: its placement into
/// *out_comment, and the line exactly as read, without its terminator, on the
/// variable-length output contract (the bytes are not null-terminated).
///
/// Safety: blocks is a live handle; out_comment points to a SidereonBlqComment;
/// out points to len writable bytes or is NULL when len is 0; out_written and
/// out_required point to size_t values.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn sidereon_blq_blocks_comment(
    blocks: *const SidereonBlqBlocks,
    index: usize,
    comment_index: usize,
    out_comment: *mut SidereonBlqComment,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_blq_blocks_comment";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        let out_comment = c_try!(require_out(out_comment, FN_NAME, "out_comment"));
        *out_comment = SidereonBlqComment {
            placement: SidereonBlqCommentPlacement::BeforeStation as u32,
            row: 0,
        };
        c_try!(init_copy_counts(FN_NAME, out_written, out_required));
        let block = c_try!(blq_block_at(FN_NAME, blocks, index));
        let Some(comment) = block.comments.get(comment_index) else {
            set_last_error(format!(
                "{FN_NAME}: comment_index {comment_index} out of range"
            ));
            return SidereonStatus::InvalidArgument;
        };
        c_try!(copy_prefix_to_c(
            FN_NAME,
            "out",
            comment.line.as_bytes(),
            out,
            len,
            out_written,
            out_required,
        ));
        *out_comment = match comment.placement {
            OceanLoadingBlqCommentPlacement::BeforeStation => SidereonBlqComment {
                placement: SidereonBlqCommentPlacement::BeforeStation as u32,
                row: 0,
            },
            OceanLoadingBlqCommentPlacement::BeforeRow(row) => SidereonBlqComment {
                placement: SidereonBlqCommentPlacement::BeforeRow as u32,
                row,
            },
            OceanLoadingBlqCommentPlacement::AfterRows => SidereonBlqComment {
                placement: SidereonBlqCommentPlacement::AfterRows as u32,
                row: 0,
            },
        };
        SidereonStatus::Ok
    })
}

/// Append a block with a station identifier, taken exactly as given, and
/// coefficients in the supported constituent order. The writer, not this call,
/// refuses a station or coefficient it cannot write back unchanged.
///
/// Safety: blocks is a live handle no other call uses meanwhile; station is a
/// null-terminated UTF-8 string; coefficients points to a
/// SidereonOceanLoadingBlq.
#[no_mangle]
pub unsafe extern "C" fn sidereon_blq_blocks_push(
    blocks: *mut SidereonBlqBlocks,
    station: *const c_char,
    coefficients: *const SidereonOceanLoadingBlq,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_blq_blocks_push";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        let blocks = c_try!(require_mut(blocks, FN_NAME, "blocks"));
        let station = c_try!(parse_c_string_allow_empty(FN_NAME, "station", station));
        let coefficients = c_try!(require_ref(coefficients, FN_NAME, "coefficients"));
        blocks.blocks.push(OceanLoadingBlqBlock {
            station,
            coefficients: CoreOceanLoadingBlq {
                amplitude_m: coefficients.amplitude_m,
                phase_deg: coefficients.phase_deg,
            },
            comments: Vec::new(),
        });
        SidereonStatus::Ok
    })
}

/// Append a comment or column-order header line to a block at a placement, the
/// line taken exactly as given. The writer, not this call, refuses a line it
/// cannot write back as the same comment at the same place.
///
/// Safety: blocks is a live handle no other call uses meanwhile; comment points
/// to a SidereonBlqComment; line is a null-terminated UTF-8 string.
#[no_mangle]
pub unsafe extern "C" fn sidereon_blq_blocks_push_comment(
    blocks: *mut SidereonBlqBlocks,
    index: usize,
    comment: *const SidereonBlqComment,
    line: *const c_char,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_blq_blocks_push_comment";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        let blocks = c_try!(require_mut(blocks, FN_NAME, "blocks"));
        let comment = c_try!(require_ref(comment, FN_NAME, "comment"));
        let line = c_try!(parse_c_string_allow_empty(FN_NAME, "line", line));
        let placement = match comment.placement {
            value if value == SidereonBlqCommentPlacement::BeforeStation as u32 => {
                OceanLoadingBlqCommentPlacement::BeforeStation
            }
            value if value == SidereonBlqCommentPlacement::BeforeRow as u32 => {
                OceanLoadingBlqCommentPlacement::BeforeRow(comment.row)
            }
            value if value == SidereonBlqCommentPlacement::AfterRows as u32 => {
                OceanLoadingBlqCommentPlacement::AfterRows
            }
            other => {
                set_last_error(format!("{FN_NAME}: invalid placement {other}"));
                return SidereonStatus::InvalidArgument;
            }
        };
        let Some(block) = blocks.blocks.get_mut(index) else {
            set_last_error(format!("{FN_NAME}: index {index} out of range"));
            return SidereonStatus::InvalidArgument;
        };
        block
            .comments
            .push(OceanLoadingBlqComment { placement, line });
        SidereonStatus::Ok
    })
}

/// Write every block as one BLQ file and take an owned record of the attempt.
/// Each retained line is written at its placement, the station line from the
/// third column (where the provider's files put it and RTKLIB `readblq` reads
/// it), and each row in the column order its retained header declares, which
/// stays in force for the blocks after it as it does when the parser reads the
/// file; parsing the text gives back equal blocks. A block the parser would not
/// read back unchanged is refused with its index and reason.
///
/// Returns SIDEREON_STATUS_OK whenever the call itself is well formed and hands
/// back a newly owned SidereonBlqResult.
///
/// Safety: blocks is a live handle; out_result points to a SidereonBlqResult*,
/// set to NULL before any work.
#[no_mangle]
pub unsafe extern "C" fn sidereon_blq_blocks_to_text_result(
    blocks: *const SidereonBlqBlocks,
    out_result: *mut *mut SidereonBlqResult,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_blq_blocks_to_text_result";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        let out_result = c_try!(require_out(out_result, FN_NAME, "out_result"));
        *out_result = ptr::null_mut();
        let blocks = c_try!(require_ref(blocks, FN_NAME, "blocks"));
        let result = match write_ocean_loading_blq_blocks(&blocks.blocks) {
            Ok(text) => blq_result_ok(text),
            Err(err) => blq_result_refused(FN_NAME, &err),
        };
        write_boxed_handle(out_result, result);
        SidereonStatus::Ok
    })
}

/// Write one block on its own as a standard six-row BLQ block, in the column
/// order its own retained header declares or the standard order, and take an
/// owned record of the attempt. A block the parser would not read back
/// unchanged is refused.
///
/// Safety: blocks is a live handle; out_result points to a SidereonBlqResult*,
/// set to NULL before any work.
#[no_mangle]
pub unsafe extern "C" fn sidereon_blq_block_to_text_result(
    blocks: *const SidereonBlqBlocks,
    index: usize,
    out_result: *mut *mut SidereonBlqResult,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_blq_block_to_text_result";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        let out_result = c_try!(require_out(out_result, FN_NAME, "out_result"));
        *out_result = ptr::null_mut();
        let block = c_try!(blq_block_at(FN_NAME, blocks, index));
        let result = match block.to_blq_block() {
            Ok(text) => blq_result_ok(text),
            Err(err) => blq_result_refused(FN_NAME, &err),
        };
        write_boxed_handle(out_result, result);
        SidereonStatus::Ok
    })
}

/// Release an owned BLQ result. Passing NULL is a no-op.
///
/// Safety: result may be NULL; otherwise it must be a live SidereonBlqResult
/// handle this binding produced, passed here exactly once.
#[no_mangle]
pub unsafe extern "C" fn sidereon_blq_result_free(result: *mut SidereonBlqResult) {
    ffi_boundary("sidereon_blq_result_free", (), || {
        free_boxed(result);
    });
}

/// Copy the fixed-width outcome of an owned BLQ result. *out_outcome is written
/// before the result pointer is validated.
///
/// Safety: result is a live handle; out_outcome points to a SidereonBlqOutcome.
#[no_mangle]
pub unsafe extern "C" fn sidereon_blq_result_get_outcome(
    result: *const SidereonBlqResult,
    out_outcome: *mut SidereonBlqOutcome,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_blq_result_get_outcome";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        let out = c_try!(require_out(out_outcome, FN_NAME, "out_outcome"));
        *out = SidereonBlqOutcome {
            is_ok: false,
            status: SidereonStatus::InvalidArgument,
            error: no_blq_error(),
        };
        let result = c_try!(require_ref(result, FN_NAME, "result"));
        *out = result.outcome;
        SidereonStatus::Ok
    })
}

/// Copy the text a write result holds. A refused result has no text: the call
/// returns SIDEREON_STATUS_INVALID_ARGUMENT, reports a required length of zero,
/// writes nothing, and sets the thread-local message to the result's own
/// failure text. Uses the variable-length output contract; the bytes are not
/// null-terminated.
///
/// Safety: result is a live handle; out points to len writable bytes or is
/// NULL when len is 0; out_written and out_required point to size_t values.
#[no_mangle]
pub unsafe extern "C" fn sidereon_blq_result_get_text(
    result: *const SidereonBlqResult,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_blq_result_get_text";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        c_try!(init_copy_counts(FN_NAME, out_written, out_required));
        let result = c_try!(require_ref(result, FN_NAME, "result"));
        if !result.outcome.is_ok {
            set_last_error(result.message.clone());
            return result.outcome.status;
        }
        c_try!(copy_prefix_to_c(
            FN_NAME,
            "out",
            result.text.as_bytes(),
            out,
            len,
            out_written,
            out_required,
        ));
        SidereonStatus::Ok
    })
}

/// Copy the failure message or the failure's text part, selected by a
/// SidereonBlqErrorText value. A successful result copies nothing. Uses the
/// variable-length output contract; the bytes are copied verbatim and not
/// null-terminated.
///
/// Safety: result is a live handle; out points to len writable bytes or is
/// NULL when len is 0; out_written and out_required point to size_t values.
#[no_mangle]
pub unsafe extern "C" fn sidereon_blq_result_error_text(
    result: *const SidereonBlqResult,
    part: u32,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    const FN_NAME: &str = "sidereon_blq_result_error_text";
    ffi_boundary(FN_NAME, SidereonStatus::Panic, || {
        c_try!(init_copy_counts(FN_NAME, out_written, out_required));
        let result = c_try!(require_ref(result, FN_NAME, "result"));
        let text = match part {
            value if value == SidereonBlqErrorText::Message as u32 => &result.message,
            value if value == SidereonBlqErrorText::Text as u32 => &result.error_text,
            other => {
                set_last_error(format!("{FN_NAME}: invalid part {other}"));
                return SidereonStatus::InvalidArgument;
            }
        };
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

#[cfg(test)]
mod blq_c_tests {
    use super::*;

    const ONSA: &str = concat!(
        "$$ Ocean loading displacement\n",
        "$$ COLUMN ORDER:  M2  S2  N2  K2  K1  O1  P1  Q1  MF  MM SSA\n",
        "  ONSA\n",
        "$$ ONSA,                 RADI TANG  lon/lat:   11.9255   57.3953    45.000\n",
        "  .00344 .00121 .00078 .00031 .00189 .00116 .00064 .00004 .00090 .00048 .00041\n",
        "  .00143 .00035 .00035 .00008 .00053 .00051 .00018 .00009 .00013 .00006 .00003\n",
        "  .00086 .00023 .00023 .00006 .00029 .00025 .00010 .00008 .00005 .00003 .00001\n",
        "   -64.7  -52.0  -96.2  -55.2  -58.8 -151.4  -65.6 -138.1    8.4    5.2    2.1\n",
        "    85.5  114.5   56.5  113.6   99.4   19.1   94.1  -10.4 -167.4 -170.0 -177.7\n",
        "   109.5  147.0   92.7  148.8   50.5  -55.1   50.5 -113.9   44.8    1.9    0.4\n",
    );

    #[test]
    fn a_block_keeps_its_comments_and_writes_back_to_an_equal_block() {
        // sidereon-core's own reading of the same text.
        let core = parse_ocean_loading_blq_blocks(ONSA).expect("core parses ONSA");
        let core_comment = &core[0].comments[2];
        unsafe {
            let mut blocks = ptr::null_mut();
            let mut result = ptr::null_mut();
            assert_eq!(
                sidereon_blq_parse(ONSA.as_ptr(), ONSA.len(), &mut blocks, &mut result),
                SidereonStatus::Ok
            );
            assert!((*result).outcome.is_ok);
            sidereon_blq_result_free(result);
            let mut count = 0;
            sidereon_blq_blocks_count(blocks, &mut count);
            assert_eq!(count, core.len());
            sidereon_blq_blocks_comment_count(blocks, 0, &mut count);
            assert_eq!(count, core[0].comments.len());
            let mut comment = SidereonBlqComment {
                placement: 99,
                row: 99,
            };
            let mut written = 0;
            let mut required = 0;
            assert_eq!(
                sidereon_blq_blocks_comment(
                    blocks,
                    0,
                    2,
                    &mut comment,
                    ptr::null_mut(),
                    0,
                    &mut written,
                    &mut required
                ),
                SidereonStatus::Ok
            );
            assert_eq!(
                comment,
                match core_comment.placement {
                    OceanLoadingBlqCommentPlacement::BeforeStation => SidereonBlqComment {
                        placement: SidereonBlqCommentPlacement::BeforeStation as u32,
                        row: 0,
                    },
                    OceanLoadingBlqCommentPlacement::BeforeRow(row) => SidereonBlqComment {
                        placement: SidereonBlqCommentPlacement::BeforeRow as u32,
                        row,
                    },
                    OceanLoadingBlqCommentPlacement::AfterRows => SidereonBlqComment {
                        placement: SidereonBlqCommentPlacement::AfterRows as u32,
                        row: 0,
                    },
                }
            );
            assert_eq!(required, core_comment.line.len());

            let mut written_result = ptr::null_mut();
            assert_eq!(
                sidereon_blq_blocks_to_text_result(blocks, &mut written_result),
                SidereonStatus::Ok
            );
            let owned = &*written_result;
            assert!(owned.outcome.is_ok);
            let reparsed = parse_ocean_loading_blq_blocks(&owned.text).expect("reparse");
            assert_eq!(reparsed, (*blocks).blocks);
            sidereon_blq_result_free(written_result);
            sidereon_blq_blocks_free(blocks);
        }
    }

    #[test]
    fn a_block_the_parser_would_not_read_back_is_refused_typed() {
        unsafe {
            let mut blocks = ptr::null_mut();
            assert_eq!(sidereon_blq_blocks_new(&mut blocks), SidereonStatus::Ok);
            let mut coefficients = SidereonOceanLoadingBlq {
                amplitude_m: [[0.001; SIDEREON_PPP_OCEAN_CONSTITUENTS]; 3],
                phase_deg: [[10.0; SIDEREON_PPP_OCEAN_CONSTITUENTS]; 3],
            };
            coefficients.phase_deg[1][4] = f64::NAN;
            let station = CString::new("ONSA").expect("station");
            assert_eq!(
                sidereon_blq_blocks_push(blocks, station.as_ptr(), &coefficients),
                SidereonStatus::Ok
            );
            // sidereon-core's own refusal of the same block list.
            let Err(TideError::BlqWrite {
                block: core_block,
                kind:
                    BlqWriteErrorKind::NonFiniteCoefficient {
                        row: core_row,
                        constituent: core_constituent,
                    },
            }) = write_ocean_loading_blq_blocks(&(*blocks).blocks)
            else {
                panic!("sidereon-core writes a non-finite coefficient");
            };
            let mut result = ptr::null_mut();
            assert_eq!(
                sidereon_blq_blocks_to_text_result(blocks, &mut result),
                SidereonStatus::Ok
            );
            let error = (*result).outcome.error;
            assert!(!(*result).outcome.is_ok);
            assert_eq!(error.kind, SidereonBlqErrorKind::Write);
            assert_eq!(
                error.write_kind,
                SidereonBlqWriteErrorKind::NonFiniteCoefficient as u32
            );
            assert!(error.has_block && error.block == core_block);
            assert!(error.has_coefficient && error.row == core_row);
            assert_eq!(error.constituent, constituent_to_c(core_constituent));
            sidereon_blq_result_free(result);

            let mut parsed = ptr::null_mut();
            let unsupported = "$$ COLUMN ORDER: M2 S2 N2 K2 K1 O1 P1 Q1 MF MM SA\n";
            // sidereon-core's own refusal of the same header.
            let Err(TideError::BlqParse {
                line: core_line,
                kind:
                    BlqParseErrorKind::UnsupportedConstituent {
                        constituent: core_text,
                    },
            }) = parse_ocean_loading_blq_blocks(unsupported)
            else {
                panic!("sidereon-core reads an unsupported constituent");
            };
            assert_eq!(
                sidereon_blq_parse(
                    unsupported.as_ptr(),
                    unsupported.len(),
                    &mut parsed,
                    &mut result
                ),
                SidereonStatus::InvalidArgument
            );
            assert!(parsed.is_null());
            let error = (*result).outcome.error;
            assert_eq!(error.kind, SidereonBlqErrorKind::Parse);
            assert_eq!(
                error.parse_kind,
                SidereonBlqParseErrorKind::UnsupportedConstituent as u32
            );
            assert!(error.has_line && error.line == core_line && error.has_text);
            assert_eq!((*result).error_text, core_text);
            sidereon_blq_result_free(result);
            sidereon_blq_blocks_free(blocks);
        }
    }
}
