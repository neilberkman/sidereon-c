use super::*;

// === Round-2 NTRIP sans-IO request, machine, and sourcetable =================

pub const NTRIP_FIELD_C_BYTES: usize = 129;

pub const NTRIP_MISC_C_BYTES: usize = 257;

pub struct SidereonNtripMachine {
    pub(crate) inner: sidereon_core::ntrip::NtripClientMachine,
}

pub struct SidereonNtripEvents {
    pub(crate) events: Vec<sidereon_core::ntrip::NtripEvent>,
}

pub struct SidereonNtripSourcetable {
    pub(crate) inner: sidereon_core::ntrip::Sourcetable,
}

pub struct SidereonNtripBytes {
    pub(crate) bytes: Vec<u8>,
}

#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonNtripVersion {
    Rev1 = 1,
    Rev2 = 2,
}

#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonNtripState {
    Idle = 0,
    AwaitingStatus = 1,
    AwaitingHeaders = 2,
    Streaming = 3,
    Sourcetable = 4,
    Closed = 5,
}

#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonNtripEventKind {
    Connected = 0,
    Payload = 1,
    Sourcetable = 2,
    Rejected = 3,
    StreamCorrupted = 4,
    StreamEnded = 5,
}

#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonNtripRejectionKind {
    None = 0,
    Unauthorized = 1,
    MountpointNotFound = 2,
    DigestRequired = 3,
    CasterError = 4,
    UnexpectedContentType = 5,
    HttpError = 6,
    MalformedHandshake = 7,
}

#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SidereonNtripSourcetableAuth {
    None = 0,
    Basic = 1,
    Digest = 2,
    Other = 3,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonNtripConfig {
    pub host: *const c_char,
    pub port: u16,
    pub mountpoint: *const c_char,
    pub version: u32,
    pub has_credentials: bool,
    pub username: *const c_char,
    pub password: *const c_char,
    pub user_agent_product: *const c_char,
    pub has_gga_interval_s: bool,
    pub gga_interval_s: f64,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonNtripGgaPosition {
    pub lat_deg: f64,
    pub lon_deg: f64,
    pub height_m: f64,
    pub fix_quality: u8,
    pub num_satellites: u8,
    pub hdop: f64,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonNtripEventInfo {
    pub kind: u32,
    pub version: u32,
    pub chunked: bool,
    pub header_count: usize,
    pub payload_len: usize,
    pub sourcetable_record_count: usize,
    pub rejection: u32,
    pub http_status: u16,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonNtripSourcetableSummary {
    pub record_count: usize,
    pub stream_count: usize,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SidereonNtripStreamInfo {
    pub mountpoint: [c_char; NTRIP_FIELD_C_BYTES],
    pub identifier: [c_char; NTRIP_FIELD_C_BYTES],
    pub format: [c_char; NTRIP_FIELD_C_BYTES],
    pub format_details: [c_char; NTRIP_FIELD_C_BYTES],
    pub has_carrier: bool,
    pub carrier: u8,
    pub nav_system: [c_char; NTRIP_FIELD_C_BYTES],
    pub network: [c_char; NTRIP_FIELD_C_BYTES],
    pub country: [c_char; NTRIP_FIELD_C_BYTES],
    pub has_lat_deg: bool,
    pub lat_deg: f64,
    pub has_lon_deg: bool,
    pub lon_deg: f64,
    pub has_nmea_required: bool,
    pub nmea_required: bool,
    pub has_network_solution: bool,
    pub network_solution: bool,
    pub generator: [c_char; NTRIP_FIELD_C_BYTES],
    pub compression: [c_char; NTRIP_FIELD_C_BYTES],
    pub authentication: u32,
    pub has_fee: bool,
    pub fee: bool,
    pub has_bitrate: bool,
    pub bitrate: u32,
    pub misc: [c_char; NTRIP_MISC_C_BYTES],
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_ntrip_request_bytes(
    config: *const SidereonNtripConfig,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_ntrip_request_bytes",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_ntrip_request_bytes",
                out_written,
                out_required
            ));
            let config = c_try!(require_ref(
                config,
                "sidereon_ntrip_request_bytes",
                "config"
            ));
            let config = c_try!(ntrip_config_from_c("sidereon_ntrip_request_bytes", config));
            let bytes = match config.request_bytes() {
                Ok(bytes) => bytes,
                Err(err) => return map_ntrip_error("sidereon_ntrip_request_bytes", err),
            };
            c_try!(copy_prefix_to_c(
                "sidereon_ntrip_request_bytes",
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

#[no_mangle]
pub unsafe extern "C" fn sidereon_ntrip_machine_new(
    config: *const SidereonNtripConfig,
    out_machine: *mut *mut SidereonNtripMachine,
) -> SidereonStatus {
    ffi_boundary("sidereon_ntrip_machine_new", SidereonStatus::Panic, || {
        let out_machine = c_try!(require_out(
            out_machine,
            "sidereon_ntrip_machine_new",
            "out_machine"
        ));
        *out_machine = ptr::null_mut();
        let config = c_try!(require_ref(config, "sidereon_ntrip_machine_new", "config"));
        let config = c_try!(ntrip_config_from_c("sidereon_ntrip_machine_new", config));
        write_boxed_handle(
            out_machine,
            SidereonNtripMachine {
                inner: sidereon_core::ntrip::NtripClientMachine::new(config),
            },
        );
        SidereonStatus::Ok
    })
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_ntrip_machine_connection_request(
    machine: *mut SidereonNtripMachine,
    out_bytes: *mut *mut SidereonNtripBytes,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_ntrip_machine_connection_request",
        SidereonStatus::Panic,
        || {
            let out_bytes = c_try!(require_out(
                out_bytes,
                "sidereon_ntrip_machine_connection_request",
                "out_bytes"
            ));
            *out_bytes = ptr::null_mut();
            let machine = c_try!(require_mut(
                machine,
                "sidereon_ntrip_machine_connection_request",
                "machine"
            ));
            match machine.inner.connection_request() {
                Ok(bytes) => {
                    write_boxed_handle(out_bytes, SidereonNtripBytes { bytes });
                    SidereonStatus::Ok
                }
                Err(err) => map_ntrip_error("sidereon_ntrip_machine_connection_request", err),
            }
        },
    )
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_ntrip_machine_push(
    machine: *mut SidereonNtripMachine,
    data: *const u8,
    len: usize,
    out_events: *mut *mut SidereonNtripEvents,
) -> SidereonStatus {
    ffi_boundary("sidereon_ntrip_machine_push", SidereonStatus::Panic, || {
        let out_events = c_try!(require_out(
            out_events,
            "sidereon_ntrip_machine_push",
            "out_events"
        ));
        *out_events = ptr::null_mut();
        let machine = c_try!(require_mut(
            machine,
            "sidereon_ntrip_machine_push",
            "machine"
        ));
        let bytes = c_try!(require_slice(
            data,
            len,
            "sidereon_ntrip_machine_push",
            "data"
        ));
        let events = machine.inner.push(bytes);
        write_boxed_handle(out_events, SidereonNtripEvents { events });
        SidereonStatus::Ok
    })
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_ntrip_machine_finish(
    machine: *mut SidereonNtripMachine,
    out_events: *mut *mut SidereonNtripEvents,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_ntrip_machine_finish",
        SidereonStatus::Panic,
        || {
            let out_events = c_try!(require_out(
                out_events,
                "sidereon_ntrip_machine_finish",
                "out_events"
            ));
            *out_events = ptr::null_mut();
            let machine = c_try!(require_mut(
                machine,
                "sidereon_ntrip_machine_finish",
                "machine"
            ));
            let events = machine.inner.finish();
            write_boxed_handle(out_events, SidereonNtripEvents { events });
            SidereonStatus::Ok
        },
    )
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_ntrip_machine_try_gga_message(
    machine: *mut SidereonNtripMachine,
    now_s: f64,
    position: *const SidereonNtripGgaPosition,
    utc_seconds_of_day: f64,
    out_present: *mut bool,
    out_bytes: *mut *mut SidereonNtripBytes,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_ntrip_machine_try_gga_message",
        SidereonStatus::Panic,
        || {
            let out_present = c_try!(require_out(
                out_present,
                "sidereon_ntrip_machine_try_gga_message",
                "out_present"
            ));
            *out_present = false;
            let out_bytes = c_try!(require_out(
                out_bytes,
                "sidereon_ntrip_machine_try_gga_message",
                "out_bytes"
            ));
            *out_bytes = ptr::null_mut();
            let machine = c_try!(require_mut(
                machine,
                "sidereon_ntrip_machine_try_gga_message",
                "machine"
            ));
            let position = c_try!(require_ref(
                position,
                "sidereon_ntrip_machine_try_gga_message",
                "position"
            ));
            let position = ntrip_gga_position_from_c(position);
            match machine
                .inner
                .try_gga_message(now_s, &position, utc_seconds_of_day)
            {
                Ok(Some(bytes)) => {
                    *out_present = true;
                    write_boxed_handle(out_bytes, SidereonNtripBytes { bytes });
                    SidereonStatus::Ok
                }
                Ok(None) => SidereonStatus::Ok,
                Err(err) => map_ntrip_error("sidereon_ntrip_machine_try_gga_message", err),
            }
        },
    )
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_ntrip_machine_state(
    machine: *const SidereonNtripMachine,
    out_state: *mut u32,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_ntrip_machine_state",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out_state,
                "sidereon_ntrip_machine_state",
                "out_state"
            ));
            let machine = c_try!(require_ref(
                machine,
                "sidereon_ntrip_machine_state",
                "machine"
            ));
            *out = ntrip_state_to_c(machine.inner.state());
            SidereonStatus::Ok
        },
    )
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_ntrip_machine_reset(machine: *mut SidereonNtripMachine) {
    ffi_boundary("sidereon_ntrip_machine_reset", (), || {
        if let Some(machine) = machine.as_mut() {
            machine.inner.reset();
        }
    });
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_ntrip_machine_free(machine: *mut SidereonNtripMachine) {
    free_boxed(machine);
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_ntrip_events_count(
    events: *const SidereonNtripEvents,
    out_count: *mut usize,
) -> SidereonStatus {
    ffi_boundary("sidereon_ntrip_events_count", SidereonStatus::Panic, || {
        let out = c_try!(require_out(
            out_count,
            "sidereon_ntrip_events_count",
            "out_count"
        ));
        let events = c_try!(require_ref(events, "sidereon_ntrip_events_count", "events"));
        *out = events.events.len();
        SidereonStatus::Ok
    })
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_ntrip_events_event(
    events: *const SidereonNtripEvents,
    index: usize,
    out_info: *mut SidereonNtripEventInfo,
) -> SidereonStatus {
    ffi_boundary("sidereon_ntrip_events_event", SidereonStatus::Panic, || {
        let out = c_try!(require_out(
            out_info,
            "sidereon_ntrip_events_event",
            "out_info"
        ));
        let events = c_try!(require_ref(events, "sidereon_ntrip_events_event", "events"));
        let Some(event) = events.events.get(index) else {
            set_last_error(format!(
                "sidereon_ntrip_events_event: index {index} out of range ({} events)",
                events.events.len()
            ));
            return SidereonStatus::InvalidArgument;
        };
        *out = ntrip_event_info(event);
        SidereonStatus::Ok
    })
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_ntrip_events_payload(
    events: *const SidereonNtripEvents,
    index: usize,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_ntrip_events_payload",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_ntrip_events_payload",
                out_written,
                out_required
            ));
            let events = c_try!(require_ref(
                events,
                "sidereon_ntrip_events_payload",
                "events"
            ));
            let Some(event) = events.events.get(index) else {
                set_last_error(format!(
                    "sidereon_ntrip_events_payload: index {index} out of range ({} events)",
                    events.events.len()
                ));
                return SidereonStatus::InvalidArgument;
            };
            let sidereon_core::ntrip::NtripEvent::Payload(bytes) = event else {
                set_last_error("sidereon_ntrip_events_payload: event is not a payload".to_string());
                return SidereonStatus::InvalidArgument;
            };
            c_try!(copy_prefix_to_c(
                "sidereon_ntrip_events_payload",
                "out",
                bytes,
                out,
                len,
                out_written,
                out_required,
            ));
            SidereonStatus::Ok
        },
    )
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_ntrip_events_detail(
    events: *const SidereonNtripEvents,
    index: usize,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_ntrip_events_detail",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_ntrip_events_detail",
                out_written,
                out_required
            ));
            let events = c_try!(require_ref(
                events,
                "sidereon_ntrip_events_detail",
                "events"
            ));
            let Some(event) = events.events.get(index) else {
                set_last_error(format!(
                    "sidereon_ntrip_events_detail: index {index} out of range ({} events)",
                    events.events.len()
                ));
                return SidereonStatus::InvalidArgument;
            };
            let detail = ntrip_event_detail(event);
            c_try!(copy_prefix_to_c(
                "sidereon_ntrip_events_detail",
                "out",
                &detail,
                out,
                len,
                out_written,
                out_required,
            ));
            SidereonStatus::Ok
        },
    )
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_ntrip_events_sourcetable(
    events: *const SidereonNtripEvents,
    index: usize,
    out_table: *mut *mut SidereonNtripSourcetable,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_ntrip_events_sourcetable",
        SidereonStatus::Panic,
        || {
            let out_table = c_try!(require_out(
                out_table,
                "sidereon_ntrip_events_sourcetable",
                "out_table"
            ));
            *out_table = ptr::null_mut();
            let events = c_try!(require_ref(
                events,
                "sidereon_ntrip_events_sourcetable",
                "events"
            ));
            let Some(event) = events.events.get(index) else {
                set_last_error(format!(
                    "sidereon_ntrip_events_sourcetable: index {index} out of range ({} events)",
                    events.events.len()
                ));
                return SidereonStatus::InvalidArgument;
            };
            let sidereon_core::ntrip::NtripEvent::Sourcetable(table) = event else {
                set_last_error(
                    "sidereon_ntrip_events_sourcetable: event is not a sourcetable".to_string(),
                );
                return SidereonStatus::InvalidArgument;
            };
            write_boxed_handle(
                out_table,
                SidereonNtripSourcetable {
                    inner: table.clone(),
                },
            );
            SidereonStatus::Ok
        },
    )
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_ntrip_events_free(events: *mut SidereonNtripEvents) {
    free_boxed(events);
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_ntrip_bytes(
    bytes: *const SidereonNtripBytes,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary("sidereon_ntrip_bytes", SidereonStatus::Panic, || {
        c_try!(init_copy_counts(
            "sidereon_ntrip_bytes",
            out_written,
            out_required
        ));
        let bytes = c_try!(require_ref(bytes, "sidereon_ntrip_bytes", "bytes"));
        c_try!(copy_prefix_to_c(
            "sidereon_ntrip_bytes",
            "out",
            &bytes.bytes,
            out,
            len,
            out_written,
            out_required,
        ));
        SidereonStatus::Ok
    })
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_ntrip_bytes_free(bytes: *mut SidereonNtripBytes) {
    free_boxed(bytes);
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_ntrip_sourcetable_parse(
    data: *const u8,
    len: usize,
    out_table: *mut *mut SidereonNtripSourcetable,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_ntrip_sourcetable_parse",
        SidereonStatus::Panic,
        || {
            let out_table = c_try!(require_out(
                out_table,
                "sidereon_ntrip_sourcetable_parse",
                "out_table"
            ));
            *out_table = ptr::null_mut();
            let text = c_try!(text_bytes_from_c(
                "sidereon_ntrip_sourcetable_parse",
                data,
                len
            ));
            match sidereon_core::ntrip::parse_sourcetable(text) {
                Ok(inner) => {
                    write_boxed_handle(out_table, SidereonNtripSourcetable { inner });
                    SidereonStatus::Ok
                }
                Err(err) => map_ntrip_error("sidereon_ntrip_sourcetable_parse", err),
            }
        },
    )
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_ntrip_sourcetable_summary(
    table: *const SidereonNtripSourcetable,
    out_summary: *mut SidereonNtripSourcetableSummary,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_ntrip_sourcetable_summary",
        SidereonStatus::Panic,
        || {
            let out = c_try!(require_out(
                out_summary,
                "sidereon_ntrip_sourcetable_summary",
                "out_summary"
            ));
            let table = c_try!(require_ref(
                table,
                "sidereon_ntrip_sourcetable_summary",
                "table"
            ));
            *out = ntrip_sourcetable_summary(&table.inner);
            SidereonStatus::Ok
        },
    )
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_ntrip_sourcetable_streams(
    table: *const SidereonNtripSourcetable,
    out: *mut SidereonNtripStreamInfo,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_ntrip_sourcetable_streams",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_ntrip_sourcetable_streams",
                out_written,
                out_required
            ));
            let table = c_try!(require_ref(
                table,
                "sidereon_ntrip_sourcetable_streams",
                "table"
            ));
            let values: Vec<_> = table.inner.streams().map(ntrip_stream_to_c).collect();
            c_try!(copy_prefix_to_c(
                "sidereon_ntrip_sourcetable_streams",
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

#[no_mangle]
pub unsafe extern "C" fn sidereon_ntrip_sourcetable_to_text(
    table: *const SidereonNtripSourcetable,
    out: *mut u8,
    len: usize,
    out_written: *mut usize,
    out_required: *mut usize,
) -> SidereonStatus {
    ffi_boundary(
        "sidereon_ntrip_sourcetable_to_text",
        SidereonStatus::Panic,
        || {
            c_try!(init_copy_counts(
                "sidereon_ntrip_sourcetable_to_text",
                out_written,
                out_required
            ));
            let table = c_try!(require_ref(
                table,
                "sidereon_ntrip_sourcetable_to_text",
                "table"
            ));
            let text = match table.inner.to_text() {
                Ok(text) => text,
                Err(err) => return map_ntrip_error("sidereon_ntrip_sourcetable_to_text", err),
            };
            c_try!(copy_prefix_to_c(
                "sidereon_ntrip_sourcetable_to_text",
                "out",
                text.as_bytes(),
                out,
                len,
                out_written,
                out_required,
            ));
            SidereonStatus::Ok
        },
    )
}

#[no_mangle]
pub unsafe extern "C" fn sidereon_ntrip_sourcetable_free(table: *mut SidereonNtripSourcetable) {
    free_boxed(table);
}

fn ntrip_state_to_c(state: sidereon_core::ntrip::NtripState) -> u32 {
    match state {
        sidereon_core::ntrip::NtripState::Idle => SidereonNtripState::Idle as u32,
        sidereon_core::ntrip::NtripState::AwaitingStatus => {
            SidereonNtripState::AwaitingStatus as u32
        }
        sidereon_core::ntrip::NtripState::AwaitingHeaders => {
            SidereonNtripState::AwaitingHeaders as u32
        }
        sidereon_core::ntrip::NtripState::Streaming => SidereonNtripState::Streaming as u32,
        sidereon_core::ntrip::NtripState::Sourcetable => SidereonNtripState::Sourcetable as u32,
        sidereon_core::ntrip::NtripState::Closed => SidereonNtripState::Closed as u32,
    }
}

unsafe fn ntrip_config_from_c(
    fn_name: &str,
    config: &SidereonNtripConfig,
) -> Result<sidereon_core::ntrip::NtripConfig, SidereonStatus> {
    let defaults = sidereon_core::ntrip::NtripConfig::default();
    let host = parse_c_string_allow_empty(fn_name, "host", config.host)?;
    let mountpoint = parse_nullable_c_string_allow_empty(fn_name, "mountpoint", config.mountpoint)?
        .unwrap_or_default();
    let user_agent_product = parse_nullable_c_string_allow_empty(
        fn_name,
        "user_agent_product",
        config.user_agent_product,
    )?
    .unwrap_or(defaults.user_agent_product);
    let credentials = if config.has_credentials {
        Some(sidereon_core::ntrip::NtripCredentials {
            username: parse_c_string_allow_empty(fn_name, "username", config.username)?,
            password: parse_c_string_allow_empty(fn_name, "password", config.password)?,
        })
    } else {
        None
    };
    Ok(sidereon_core::ntrip::NtripConfig {
        host,
        port: config.port,
        mountpoint,
        version: ntrip_version_from_c(fn_name, config.version)?,
        credentials,
        user_agent_product,
        gga_interval_s: config.has_gga_interval_s.then_some(config.gga_interval_s),
    })
}

fn ntrip_gga_position_from_c(
    position: &SidereonNtripGgaPosition,
) -> sidereon_core::ntrip::GgaPosition {
    sidereon_core::ntrip::GgaPosition {
        lat_deg: position.lat_deg,
        lon_deg: position.lon_deg,
        height_m: position.height_m,
        fix_quality: position.fix_quality,
        num_satellites: position.num_satellites,
        hdop: position.hdop,
    }
}

fn ntrip_event_info(event: &sidereon_core::ntrip::NtripEvent) -> SidereonNtripEventInfo {
    match event {
        sidereon_core::ntrip::NtripEvent::Connected(handshake) => SidereonNtripEventInfo {
            kind: SidereonNtripEventKind::Connected as u32,
            version: ntrip_version_to_c(handshake.version),
            chunked: handshake.chunked,
            header_count: handshake.headers.len(),
            payload_len: 0,
            sourcetable_record_count: 0,
            rejection: SidereonNtripRejectionKind::None as u32,
            http_status: 0,
        },
        sidereon_core::ntrip::NtripEvent::Payload(bytes) => SidereonNtripEventInfo {
            kind: SidereonNtripEventKind::Payload as u32,
            version: 0,
            chunked: false,
            header_count: 0,
            payload_len: bytes.len(),
            sourcetable_record_count: 0,
            rejection: SidereonNtripRejectionKind::None as u32,
            http_status: 0,
        },
        sidereon_core::ntrip::NtripEvent::Sourcetable(table) => SidereonNtripEventInfo {
            kind: SidereonNtripEventKind::Sourcetable as u32,
            version: 0,
            chunked: false,
            header_count: 0,
            payload_len: 0,
            sourcetable_record_count: table.records.len(),
            rejection: SidereonNtripRejectionKind::None as u32,
            http_status: 0,
        },
        sidereon_core::ntrip::NtripEvent::Rejected(rejection) => {
            let status = match rejection {
                sidereon_core::ntrip::NtripRejection::HttpError { status, .. } => *status,
                _ => 0,
            };
            SidereonNtripEventInfo {
                kind: SidereonNtripEventKind::Rejected as u32,
                version: 0,
                chunked: false,
                header_count: 0,
                payload_len: 0,
                sourcetable_record_count: 0,
                rejection: ntrip_rejection_kind_to_c(rejection),
                http_status: status,
            }
        }
        sidereon_core::ntrip::NtripEvent::StreamCorrupted { .. } => SidereonNtripEventInfo {
            kind: SidereonNtripEventKind::StreamCorrupted as u32,
            version: 0,
            chunked: false,
            header_count: 0,
            payload_len: 0,
            sourcetable_record_count: 0,
            rejection: SidereonNtripRejectionKind::None as u32,
            http_status: 0,
        },
        sidereon_core::ntrip::NtripEvent::StreamEnded => SidereonNtripEventInfo {
            kind: SidereonNtripEventKind::StreamEnded as u32,
            version: 0,
            chunked: false,
            header_count: 0,
            payload_len: 0,
            sourcetable_record_count: 0,
            rejection: SidereonNtripRejectionKind::None as u32,
            http_status: 0,
        },
    }
}

fn ntrip_event_detail(event: &sidereon_core::ntrip::NtripEvent) -> Vec<u8> {
    match event {
        sidereon_core::ntrip::NtripEvent::Rejected(rejection) => ntrip_rejection_detail(rejection),
        sidereon_core::ntrip::NtripEvent::StreamCorrupted { detail } => detail.as_bytes().to_vec(),
        _ => Vec::new(),
    }
}

fn ntrip_stream_to_c(stream: &sidereon_core::ntrip::StrRecord) -> SidereonNtripStreamInfo {
    let (has_carrier, carrier) = ntrip_field_u8(&stream.carrier);
    let (has_lat_deg, lat_deg) = ntrip_field_f64(&stream.lat_deg);
    let (has_lon_deg, lon_deg) = ntrip_field_f64(&stream.lon_deg);
    let (has_nmea_required, nmea_required) = ntrip_field_bool(&stream.nmea_required);
    let (has_network_solution, network_solution) = ntrip_field_bool(&stream.network_solution);
    let (has_fee, fee) = ntrip_field_bool(&stream.fee);
    let (has_bitrate, bitrate) = ntrip_field_u32(&stream.bitrate);
    SidereonNtripStreamInfo {
        mountpoint: fixed_c_chars::<NTRIP_FIELD_C_BYTES>(&stream.mountpoint),
        identifier: fixed_c_chars::<NTRIP_FIELD_C_BYTES>(&stream.identifier),
        format: fixed_c_chars::<NTRIP_FIELD_C_BYTES>(&stream.format),
        format_details: fixed_c_chars::<NTRIP_FIELD_C_BYTES>(&stream.format_details),
        has_carrier,
        carrier,
        nav_system: fixed_c_chars::<NTRIP_FIELD_C_BYTES>(&stream.nav_system),
        network: fixed_c_chars::<NTRIP_FIELD_C_BYTES>(&stream.network),
        country: fixed_c_chars::<NTRIP_FIELD_C_BYTES>(&stream.country),
        has_lat_deg,
        lat_deg,
        has_lon_deg,
        lon_deg,
        has_nmea_required,
        nmea_required,
        has_network_solution,
        network_solution,
        generator: fixed_c_chars::<NTRIP_FIELD_C_BYTES>(&stream.generator),
        compression: fixed_c_chars::<NTRIP_FIELD_C_BYTES>(&stream.compression),
        authentication: ntrip_auth_to_c(&stream.authentication),
        has_fee,
        fee,
        has_bitrate,
        bitrate,
        misc: fixed_c_chars::<NTRIP_MISC_C_BYTES>(&stream.misc),
    }
}

fn map_ntrip_error(fn_name: &str, err: sidereon_core::Error) -> SidereonStatus {
    set_last_error(format!("{fn_name}: {err}"));
    SidereonStatus::InvalidArgument
}

fn ntrip_sourcetable_summary(
    table: &sidereon_core::ntrip::Sourcetable,
) -> SidereonNtripSourcetableSummary {
    SidereonNtripSourcetableSummary {
        record_count: table.records.len(),
        stream_count: table.streams().count(),
    }
}

unsafe fn parse_nullable_c_string_allow_empty(
    fn_name: &str,
    arg_name: &str,
    ptr: *const c_char,
) -> Result<Option<String>, SidereonStatus> {
    if ptr.is_null() {
        Ok(None)
    } else {
        parse_c_string_allow_empty(fn_name, arg_name, ptr).map(Some)
    }
}

fn ntrip_version_from_c(
    fn_name: &str,
    version: u32,
) -> Result<sidereon_core::ntrip::NtripVersion, SidereonStatus> {
    match version {
        x if x == SidereonNtripVersion::Rev1 as u32 => Ok(sidereon_core::ntrip::NtripVersion::Rev1),
        x if x == SidereonNtripVersion::Rev2 as u32 => Ok(sidereon_core::ntrip::NtripVersion::Rev2),
        _ => {
            set_last_error(format!("{fn_name}: invalid NTRIP version {version}"));
            Err(SidereonStatus::InvalidArgument)
        }
    }
}

fn ntrip_version_to_c(version: sidereon_core::ntrip::NtripVersion) -> u32 {
    match version {
        sidereon_core::ntrip::NtripVersion::Rev1 => SidereonNtripVersion::Rev1 as u32,
        sidereon_core::ntrip::NtripVersion::Rev2 => SidereonNtripVersion::Rev2 as u32,
    }
}

fn ntrip_rejection_kind_to_c(rejection: &sidereon_core::ntrip::NtripRejection) -> u32 {
    match rejection {
        sidereon_core::ntrip::NtripRejection::Unauthorized => {
            SidereonNtripRejectionKind::Unauthorized as u32
        }
        sidereon_core::ntrip::NtripRejection::MountpointNotFound => {
            SidereonNtripRejectionKind::MountpointNotFound as u32
        }
        sidereon_core::ntrip::NtripRejection::DigestRequired => {
            SidereonNtripRejectionKind::DigestRequired as u32
        }
        sidereon_core::ntrip::NtripRejection::CasterError { .. } => {
            SidereonNtripRejectionKind::CasterError as u32
        }
        sidereon_core::ntrip::NtripRejection::UnexpectedContentType { .. } => {
            SidereonNtripRejectionKind::UnexpectedContentType as u32
        }
        sidereon_core::ntrip::NtripRejection::HttpError { .. } => {
            SidereonNtripRejectionKind::HttpError as u32
        }
        sidereon_core::ntrip::NtripRejection::MalformedHandshake { .. } => {
            SidereonNtripRejectionKind::MalformedHandshake as u32
        }
    }
}

fn ntrip_rejection_detail(rejection: &sidereon_core::ntrip::NtripRejection) -> Vec<u8> {
    match rejection {
        sidereon_core::ntrip::NtripRejection::Unauthorized => Vec::new(),
        sidereon_core::ntrip::NtripRejection::MountpointNotFound => Vec::new(),
        sidereon_core::ntrip::NtripRejection::DigestRequired => Vec::new(),
        sidereon_core::ntrip::NtripRejection::CasterError { reason } => reason.as_bytes().to_vec(),
        sidereon_core::ntrip::NtripRejection::UnexpectedContentType { content_type } => {
            content_type.as_bytes().to_vec()
        }
        sidereon_core::ntrip::NtripRejection::HttpError { reason, .. } => {
            reason.as_bytes().to_vec()
        }
        sidereon_core::ntrip::NtripRejection::MalformedHandshake { prefix } => prefix.clone(),
    }
}

fn ntrip_auth_to_c(auth: &sidereon_core::ntrip::StrAuth) -> u32 {
    match auth {
        sidereon_core::ntrip::StrAuth::None => SidereonNtripSourcetableAuth::None as u32,
        sidereon_core::ntrip::StrAuth::Basic => SidereonNtripSourcetableAuth::Basic as u32,
        sidereon_core::ntrip::StrAuth::Digest => SidereonNtripSourcetableAuth::Digest as u32,
        sidereon_core::ntrip::StrAuth::Other(_) => SidereonNtripSourcetableAuth::Other as u32,
    }
}

fn ntrip_field_u8(field: &sidereon_core::ntrip::Field<u8>) -> (bool, u8) {
    field
        .value()
        .copied()
        .map_or((false, 0), |value| (true, value))
}

fn ntrip_field_u32(field: &sidereon_core::ntrip::Field<u32>) -> (bool, u32) {
    field
        .value()
        .copied()
        .map_or((false, 0), |value| (true, value))
}

fn ntrip_field_f64(field: &sidereon_core::ntrip::Field<f64>) -> (bool, f64) {
    field
        .value()
        .copied()
        .map_or((false, 0.0), |value| (true, value))
}

fn ntrip_field_bool(field: &sidereon_core::ntrip::Field<bool>) -> (bool, bool) {
    field
        .value()
        .copied()
        .map_or((false, false), |value| (true, value))
}
