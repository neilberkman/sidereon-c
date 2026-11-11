# C/Python TDM parity

This table compares the release-3.0.6 Python surface with the C binding. C uses
plain caller-owned value structs for Python value objects, an opaque `SidereonTdm`
for the message, indexed access/mutation, and the normal two-call buffer contract
for variable-length text and arrays.

| Python capability | Core mapping | C status / equivalent |
|---|---|---|
| `TdmComment` construction, text, position, equality | `TdmComment` | Lossless `SidereonTdmCommentInput`; output summary plus indexed two-call text accessor. |
| `TdmField` construction, key/value, equality | `TdmField` | Lossless `SidereonTdmFieldInput`; output summary plus indexed key/value accessors. |
| `TdmParticipant` and `TdmPath` value objects | `TdmParticipant`, `TdmPath` | Flattened segment-indexed accessors; raw metadata fields are the mutation source of truth. |
| `TdmUnit(label)` and `.label` | `TdmUnit`, `TdmUnit::as_str` | Enum plus lossless input label and `sidereon_tdm_record_text` output, including unknown labels. |
| `TdmObservable` constructors/properties | `TdmObservable` | Enum/suffix input plus lossless Other name; `sidereon_tdm_record_text` reads it. |
| `TdmScalar` construction/properties | `TdmScalar` | `value_text` and `value` in record input/output. |
| `TdmDataRecord` construction/properties | `TdmDataRecord` | ABI-stable legacy output plus lossless text accessor; separate `SidereonTdmDataRecordInput` for mutation. |
| `TdmDataSection` construction, comments/records, add/insert/remove | `TdmDataSection` and `Vec` operations | `append_segment`, `replace_data`, `insert_record`, `remove_record`, and positioned comments. Removal returns status only; callers use the lossless record accessors before removal. |
| `TdmLeniency` | `TdmLeniency` | Validated `u32` codes `SIDEREON_TDM_LENIENCY_STRICT`/`FORGIVE`. |
| `TdmPolicy` constructor/default/strict/lenient/per-rule settings | `TdmPolicy` | C policy struct and presets; public fields are the C equivalent of Python `with_*`. |
| `TdmWritePolicy` constructor/default/strict/lenient/`as_read`/per-rule settings | `TdmWritePolicy` | C write-policy struct, presets, conversion; public fields replace Python `with_*`. |
| `TdmWarning` typed detail/message | `TdmWarning` | Owned warning list with indexed lossless JSON detail/message. |
| `TdmDeparture` typed detail/message | `TdmDeparture` | Owned departure list with indexed lossless JSON detail/message. |
| `TdmInputErrorKind` | `TdmInputErrorKind` | Stable names in the engine-error JSON; C consumes this as a tagged detail rather than a standalone object. |
| `TdmErrorDetail` variant fields/message | `TdmError` | Thread-local Schema 1 engine JSON preserves typed fields and operation class; closest C equivalent to exception `.detail`. |
| `TdmParseError` | failures from `parse_kvn*` | TDM family with `operation_class: parse`. |
| `TdmWriteError` | failures from `encode_kvn*` | TDM family with `operation_class: write`. |
| `TdmValidationError` | metadata construction/edit failures | TDM family with `operation_class: validation`. |
| `TdmParseResult` message/warnings | result of `parse_kvn_with_policy` | Policy parse returns TDM plus owned warnings through two output handles. |
| `TdmWriteResult` text/departures | result of `encode_kvn_with_policy` | Policy encode returns text plus owned departures. |
| `TdmMetadataResult` metadata/departures | `TdmMetadata::from_raw_with_policy`, `replace_raw_with_policy` | Selected segment is mutated atomically and departures are returned; no redundant standalone metadata handle. |
| `TdmMetadata` construction/from-raw | `TdmMetadata::from_raw*` | Segment construction and atomic raw metadata replacement. |
| metadata comments/fields/participants/paths/properties/`get_last` | `TdmMetadata` fields and `get_last` | Lossless flattened fields/comments plus summaries/participants/paths; `get_last` is a reverse scan of returned ordered fields. |
| metadata scalar setters (`mode`, `timetag_ref`, `time_system`, `range_units`) | atomic `replace_raw*` used by Python | Edit returned raw fields and call atomic policy-aware `replace_metadata`. |
| metadata field insert/append/remove/remove-at | atomic `replace_raw*` used by Python | Edit returned ordered field rows and call atomic policy-aware `replace_metadata`. |
| metadata `add_comment` and comments setter | atomic `replace_raw*` used by Python | Edit returned positioned comment rows and call atomic policy-aware `replace_metadata`. |
| `TdmSegment` construction and metadata/data properties | `TdmSegment` | Segment summaries, append, remove, atomic metadata replacement, and data replacement. |
| `Tdm` construction and header properties | `Tdm` public fields | `sidereon_tdm_create`, header getter/setter. |
| TDM header comments/header fields | `Tdm.comments`, `Tdm.header_fields` | Lossless access and `replace_header_raw`. |
| TDM segments setter/set/replace | `Tdm.segments` | Append/remove and component replacement; replacing both components is the C equivalent of set/replace. |
| `Tdm.to_kvn*`, `encode_tdm_kvn*` | `encode_kvn`, `encode_kvn_with_policy` | Strict and policy encoders with departure result. |
| `parse_tdm_kvn*` | `parse_kvn`, `parse_kvn_with_policy` | Strict and policy parsers with warning result. |

No parser, formatter, or validation rule is implemented in C: all policy-aware
operations delegate to `sidereon_core::astro::tdm`.
