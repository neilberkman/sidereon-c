//! ccsds_serialize.c: the OEM segment count, IONEX map-epoch count and RINEX
//! observation version sidereon-core reads from the fixtures that test parses.

#[path = "../w6_shared.rs"]
mod shared;

use shared::*;
use sidereon_core::astro::oem;
use sidereon_core::atmosphere::ionosphere::Ionex;
use sidereon_core::rinex::observations::RinexObs;
use valgen::{read, read_bytes, tests_path};

fn main() {
    let guard = "SIDEREON_W6_CCSDS_SERIALIZE_PINS_H";
    valgen::header_start("w6_ccsds_serialize", guard);
    let kvn = oem::parse_kvn(&read(&tests_path("fixtures/oem/gps.kvn"))).expect("OEM KVN");
    let xml = oem::parse_xml(&read(&tests_path("fixtures/oem/gps.xml"))).expect("OEM XML");
    comment("OEM segment counts of the KVN and XML fixtures.");
    def_size("W6_CCSDS_OEM_KVN_SEGMENTS", kvn.segments.len());
    def_size("W6_CCSDS_OEM_XML_SEGMENTS", xml.segments.len());
    let ionex = Ionex::parse(&read_bytes(&tests_path(
        "fixtures/ionex/synthetic_2map_7x7.20i",
    )))
    .expect("IONEX");
    comment("IONEX map epochs.");
    def_size("W6_CCSDS_IONEX_EPOCHS", ionex.map_epochs_s().len());
    let obs = RinexObs::parse(&read(&tests_path(
        "fixtures/obs/ESBC00DNK_R_20201770000_01D_30S_MO_trim.rnx",
    )))
    .expect("RINEX observation");
    comment("RINEX observation header version.");
    def_bits("W6_CCSDS_RINEX_VERSION_BITS", obs.header().version);
    valgen::header_end(guard);
}
