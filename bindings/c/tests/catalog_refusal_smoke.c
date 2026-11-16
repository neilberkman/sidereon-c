#include "sidereon.h"
#include "constellation_fixture.h"
#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>

#define CHECK(x) do { if (!(x)) { fprintf(stderr, "%s:%d: %s\n", __FILE__, __LINE__, #x); return 1; } } while (0)
#define EXPECT(k, expected, expr) do { SidereonStatus got = (expr); counts[k]++; if (got != (expected)) { fprintf(stderr, "%s:%d: %s returned %d, expected %d\n", __FILE__, __LINE__, #expr, got, expected); return 1; } } while (0)
enum { NULL_CASE, OVERLAP, CODE, UTF8, INDEX, FREE_CASE, NCOUNTS };
static size_t counts[NCOUNTS];
static SidereonCelestrakGpResponse *gp;
static SidereonCelestrakGpResponse *gp2;
static SidereonCelestrakGpResponse *rejected_gp;
static SidereonOmm *omm;
static SidereonOmm *omm2;
static SidereonOmmArray *array;
static SidereonOmmArray *array2;
static SidereonTle *tle;
static SidereonNavcenAssessments *nav;
static size_t nav_count;
static const uint8_t bad[] = {255};
static const uint8_t value[] = "active";
static const uint8_t date[] = "2026-01-01T00:00:00";
static const uint8_t origin[] = "TEST";
static const uint8_t version[] = "3.0";
static uint8_t bytes[4096];

#define BUFFER_CASES(name, call) \
static int name##_cases(void) { \
    size_t w = 99, r = 99; uint8_t *out = NULL; size_t n = 1; size_t *wp = &w, *rp = &r; \
    EXPECT(NULL_CASE, SIDEREON_STATUS_NULL_POINTER, (call)); CHECK(w == 0 && r > 0); \
    memset(bytes, 0xa5, sizeof bytes); out = bytes; n = sizeof bytes; wp = NULL; w = r = 99; \
    EXPECT(NULL_CASE, SIDEREON_STATUS_NULL_POINTER, (call)); CHECK(r == 0 && w == 99 && bytes[0] == 0xa5); \
    wp = &w; rp = NULL; w = r = 99; \
    EXPECT(NULL_CASE, SIDEREON_STATUS_NULL_POINTER, (call)); CHECK(w == 0 && r == 99 && bytes[0] == 0xa5); \
    rp = &w; w = 99; EXPECT(OVERLAP, SIDEREON_STATUS_INVALID_ARGUMENT, (call)); CHECK(w == 99); \
    union { size_t align; uint8_t data[4096]; } region; \
    out = region.data; wp = (size_t *)region.data; rp = &r; \
    EXPECT(OVERLAP, SIDEREON_STATUS_INVALID_ARGUMENT, (call)); \
    wp = &w; rp = (size_t *)region.data; \
    EXPECT(OVERLAP, SIDEREON_STATUS_INVALID_ARGUMENT, (call)); \
    return 0; \
}
BUFFER_CASES(sidereon_celestrak_gp_source_entry, sidereon_celestrak_gp_source_entry(out,n,wp,rp))
BUFFER_CASES(sidereon_navcen_source_entry, sidereon_navcen_source_entry(out,n,wp,rp))
BUFFER_CASES(sidereon_navcen_constellation_url, sidereon_navcen_constellation_url(out,n,wp,rp))
BUFFER_CASES(sidereon_data_allowed_hosts, sidereon_data_allowed_hosts(out,n,wp,rp))
BUFFER_CASES(sidereon_celestrak_group, sidereon_celestrak_group(0,out,n,wp,rp))
BUFFER_CASES(sidereon_celestrak_constellation_url, sidereon_celestrak_constellation_url(0,5,out,n,wp,rp))
BUFFER_CASES(sidereon_celestrak_gp_url, sidereon_celestrak_gp_url(0,value,6,5,out,n,wp,rp))
BUFFER_CASES(sidereon_celestrak_gp_catalog_number_url, sidereon_celestrak_gp_catalog_number_url(25544,5,out,n,wp,rp))
BUFFER_CASES(sidereon_celestrak_gp_special_url, sidereon_celestrak_gp_special_url(0,5,out,n,wp,rp))
BUFFER_CASES(sidereon_celestrak_gp_format_as_str, sidereon_celestrak_gp_format_as_str(0,out,n,wp,rp))
BUFFER_CASES(sidereon_celestrak_gp_special_as_str, sidereon_celestrak_gp_special_as_str(0,out,n,wp,rp))
BUFFER_CASES(sidereon_celestrak_gp_response_raw, sidereon_celestrak_gp_response_raw(gp,false,0,out,n,wp,rp))
BUFFER_CASES(sidereon_celestrak_gp_response_record_json, sidereon_celestrak_gp_response_record_json(gp,false,0,out,n,wp,rp))
BUFFER_CASES(sidereon_omm_to_kvn_lenient, sidereon_omm_to_kvn_lenient(omm,out,n,wp,rp))
BUFFER_CASES(sidereon_omm_to_xml_lenient, sidereon_omm_to_xml_lenient(omm,out,n,wp,rp))

#define EQUAL_CASES(name, type, left, right, args) \
static int name##_cases(void) { bool out = true; \
    EXPECT(NULL_CASE,SIDEREON_STATUS_NULL_POINTER,name(NULL,right,args &out)); CHECK(out); \
    EXPECT(NULL_CASE,SIDEREON_STATUS_NULL_POINTER,name(left,NULL,args &out)); CHECK(out); \
    EXPECT(NULL_CASE,SIDEREON_STATUS_NULL_POINTER,name(left,right,args NULL)); \
    EXPECT(OVERLAP,SIDEREON_STATUS_INVALID_ARGUMENT,name(left,right,args (bool *)(left))); \
    EXPECT(OVERLAP,SIDEREON_STATUS_INVALID_ARGUMENT,name(left,right,args (bool *)(right))); \
    return 0; }
#define NO_ARGS
#define ACCEPT_ARGS false,0,0,
#define PAIR_ARGS 0,0,
EQUAL_CASES(sidereon_celestrak_gp_response_equal, SidereonCelestrakGpResponse, gp,gp2, NO_ARGS)
EQUAL_CASES(sidereon_celestrak_gp_record_equal, SidereonCelestrakGpResponse, gp,gp2, ACCEPT_ARGS)
EQUAL_CASES(sidereon_celestrak_gp_payload_equal, SidereonCelestrakGpResponse, gp,gp2, PAIR_ARGS)
EQUAL_CASES(sidereon_celestrak_tle_record_equal, SidereonCelestrakGpResponse, gp,gp2, PAIR_ARGS)
EQUAL_CASES(sidereon_celestrak_gp_rejection_reason_equal, SidereonCelestrakGpResponse, gp,gp2, PAIR_ARGS)
EQUAL_CASES(sidereon_celestrak_gp_structural_error_equal, SidereonCelestrakGpResponse, gp,gp2, PAIR_ARGS)
EQUAL_CASES(sidereon_omm_equal, SidereonOmm, omm,omm2, NO_ARGS)
EQUAL_CASES(sidereon_omm_array_equal, SidereonOmmArray, array,array2, NO_ARGS)

static int sidereon_parse_celestrak_gp_response_cases(void) {
    SidereonCelestrakGpResponse *out=(void *)(uintptr_t)1;
    EXPECT(NULL_CASE,SIDEREON_STATUS_NULL_POINTER,sidereon_parse_celestrak_gp_response(NULL,1,0,&out)); CHECK(!out);
    EXPECT(NULL_CASE,SIDEREON_STATUS_NULL_POINTER,sidereon_parse_celestrak_gp_response(value,6,0,NULL));
    EXPECT(OVERLAP,SIDEREON_STATUS_INVALID_ARGUMENT,sidereon_parse_celestrak_gp_response((const uint8_t *)&out,sizeof out,0,&out));
    out=(void *)(uintptr_t)1; EXPECT(UTF8,SIDEREON_STATUS_INVALID_ARGUMENT,sidereon_parse_celestrak_gp_response(bad,1,SIDEREON_CELESTRAK_GP_FORMAT_JSON,&out)); CHECK(!out);
    return 0;
}
static int sidereon_celestrak_gp_response_free_cases(void) { sidereon_celestrak_gp_response_free(NULL); counts[FREE_CASE]++; return 0; }
static int sidereon_celestrak_gp_response_counts_cases(void) {
    size_t a=99,b=99;
    EXPECT(NULL_CASE,SIDEREON_STATUS_NULL_POINTER,sidereon_celestrak_gp_response_counts(NULL,&a,&b)); CHECK(a==99 && b==99);
    EXPECT(NULL_CASE,SIDEREON_STATUS_NULL_POINTER,sidereon_celestrak_gp_response_counts(gp,NULL,&b)); CHECK(b==99);
    EXPECT(NULL_CASE,SIDEREON_STATUS_NULL_POINTER,sidereon_celestrak_gp_response_counts(gp,&a,NULL)); CHECK(a==99);
    EXPECT(OVERLAP,SIDEREON_STATUS_INVALID_ARGUMENT,sidereon_celestrak_gp_response_counts(gp,&a,&a));
    EXPECT(OVERLAP,SIDEREON_STATUS_INVALID_ARGUMENT,sidereon_celestrak_gp_response_counts(gp,(size_t *)gp,&b));
    EXPECT(OVERLAP,SIDEREON_STATUS_INVALID_ARGUMENT,sidereon_celestrak_gp_response_counts(gp,&a,(size_t *)gp)); return 0;
}
#define HANDLE_CASES(name, type, call) \
static int name##_cases(void) { type *out=(void *)(uintptr_t)1; \
 EXPECT(NULL_CASE,SIDEREON_STATUS_NULL_POINTER,name(NULL,0,call &out)); CHECK(!out); \
 EXPECT(NULL_CASE,SIDEREON_STATUS_NULL_POINTER,name(gp,0,call NULL)); \
 EXPECT(OVERLAP,SIDEREON_STATUS_INVALID_ARGUMENT,name(gp,0,call (type **)gp)); \
 out=(void *)(uintptr_t)1; EXPECT(INDEX,SIDEREON_STATUS_INVALID_ARGUMENT,name(gp,1,call &out)); CHECK(!out); return 0; }
HANDLE_CASES(sidereon_celestrak_gp_record_to_omm, SidereonOmm, NO_ARGS)
#define OPS_ARGS 0,0,
HANDLE_CASES(sidereon_celestrak_gp_record_tle, SidereonTle, OPS_ARGS)
static int sidereon_celestrak_gp_record_to_element_set_cases(void) {
    SidereonOmmElementSet out; memset(&out,0x55,sizeof out); SidereonOmmElementSet before=out;
    EXPECT(NULL_CASE,SIDEREON_STATUS_NULL_POINTER,sidereon_celestrak_gp_record_to_element_set(NULL,0,&out)); CHECK(memcmp(&out,&before,sizeof out)==0);
    EXPECT(NULL_CASE,SIDEREON_STATUS_NULL_POINTER,sidereon_celestrak_gp_record_to_element_set(gp,0,NULL));
    EXPECT(OVERLAP,SIDEREON_STATUS_INVALID_ARGUMENT,sidereon_celestrak_gp_record_to_element_set(gp,0,(SidereonOmmElementSet *)gp));
    EXPECT(INDEX,SIDEREON_STATUS_INVALID_ARGUMENT,sidereon_celestrak_gp_record_to_element_set(gp,1,&out)); CHECK(memcmp(&out,&before,sizeof out)==0); return 0;
}
static int sidereon_omm_from_tle_cases(void) {
    SidereonOmm *out=(void *)(uintptr_t)1;
    EXPECT(NULL_CASE,SIDEREON_STATUS_NULL_POINTER,sidereon_omm_from_tle(NULL,value,6,&out)); CHECK(!out);
    out=(void *)(uintptr_t)1; EXPECT(NULL_CASE,SIDEREON_STATUS_NULL_POINTER,sidereon_omm_from_tle(tle,NULL,1,&out)); CHECK(!out);
    EXPECT(NULL_CASE,SIDEREON_STATUS_NULL_POINTER,sidereon_omm_from_tle(tle,value,6,NULL));
    EXPECT(OVERLAP,SIDEREON_STATUS_INVALID_ARGUMENT,sidereon_omm_from_tle(tle,(uint8_t *)&out,sizeof out,&out));
    EXPECT(OVERLAP,SIDEREON_STATUS_INVALID_ARGUMENT,sidereon_omm_from_tle(tle,value,6,(SidereonOmm **)tle));
    out=(void *)(uintptr_t)1; EXPECT(UTF8,SIDEREON_STATUS_INVALID_TOKEN,sidereon_omm_from_tle(tle,bad,1,&out)); CHECK(!out); return 0;
}
static int sidereon_omm_with_ccsds_header_cases(void) {
    SidereonOmm *out=(void *)(uintptr_t)1;
#define HEADER(o,v,vl,d,dl,r,rl,p) sidereon_omm_with_ccsds_header(o,v,vl,d,dl,r,rl,p)
    EXPECT(NULL_CASE,SIDEREON_STATUS_NULL_POINTER,HEADER(NULL,version,3,date,19,origin,4,&out)); CHECK(!out);
    out=(void *)(uintptr_t)1; EXPECT(NULL_CASE,SIDEREON_STATUS_NULL_POINTER,HEADER(omm,NULL,1,date,19,origin,4,&out)); CHECK(!out);
    out=(void *)(uintptr_t)1; EXPECT(NULL_CASE,SIDEREON_STATUS_NULL_POINTER,HEADER(omm,version,3,NULL,1,origin,4,&out)); CHECK(!out);
    out=(void *)(uintptr_t)1; EXPECT(NULL_CASE,SIDEREON_STATUS_NULL_POINTER,HEADER(omm,version,3,date,19,NULL,1,&out)); CHECK(!out);
    EXPECT(NULL_CASE,SIDEREON_STATUS_NULL_POINTER,HEADER(omm,version,3,date,19,origin,4,NULL));
    EXPECT(OVERLAP,SIDEREON_STATUS_INVALID_ARGUMENT,HEADER(omm,(uint8_t *)&out,sizeof out,date,19,origin,4,&out));
    EXPECT(OVERLAP,SIDEREON_STATUS_INVALID_ARGUMENT,HEADER(omm,version,3,(uint8_t *)&out,sizeof out,origin,4,&out));
    EXPECT(OVERLAP,SIDEREON_STATUS_INVALID_ARGUMENT,HEADER(omm,version,3,date,19,(uint8_t *)&out,sizeof out,&out));
    EXPECT(OVERLAP,SIDEREON_STATUS_INVALID_ARGUMENT,HEADER(omm,version,3,date,19,origin,4,(SidereonOmm **)omm));
    out=(void *)(uintptr_t)1; EXPECT(UTF8,SIDEREON_STATUS_INVALID_TOKEN,HEADER(omm,bad,1,date,19,origin,4,&out)); CHECK(!out);
    out=(void *)(uintptr_t)1; EXPECT(UTF8,SIDEREON_STATUS_INVALID_TOKEN,HEADER(omm,version,3,bad,1,origin,4,&out)); CHECK(!out);
    out=(void *)(uintptr_t)1; EXPECT(UTF8,SIDEREON_STATUS_INVALID_TOKEN,HEADER(omm,version,3,date,19,bad,1,&out)); CHECK(!out);
#undef HEADER
    return 0;
}
static int sidereon_omm_array_span_count_cases(void) {
    size_t out=99; EXPECT(NULL_CASE,SIDEREON_STATUS_NULL_POINTER,sidereon_omm_array_span_count(NULL,&out)); CHECK(out==99);
    EXPECT(NULL_CASE,SIDEREON_STATUS_NULL_POINTER,sidereon_omm_array_span_count(array,NULL));
    EXPECT(OVERLAP,SIDEREON_STATUS_INVALID_ARGUMENT,sidereon_omm_array_span_count(array,(size_t *)array)); return 0;
}
static int sidereon_omm_array_span_cases(void) {
    SidereonOmmRecordSpan out; memset(&out,0x55,sizeof out); SidereonOmmRecordSpan before=out;
    EXPECT(NULL_CASE,SIDEREON_STATUS_NULL_POINTER,sidereon_omm_array_span(NULL,0,&out)); CHECK(!memcmp(&out,&before,sizeof out));
    EXPECT(NULL_CASE,SIDEREON_STATUS_NULL_POINTER,sidereon_omm_array_span(array,0,NULL));
    EXPECT(OVERLAP,SIDEREON_STATUS_INVALID_ARGUMENT,sidereon_omm_array_span(array,0,(SidereonOmmRecordSpan *)array));
    EXPECT(INDEX,SIDEREON_STATUS_INVALID_ARGUMENT,sidereon_omm_array_span(array,1,&out)); CHECK(!memcmp(&out,&before,sizeof out)); return 0;
}
static int sidereon_sgp4_fit_tle_with_omm_header_cases(void) {
    SidereonSgp4FitConfig config; CHECK(sidereon_sgp4_fit_config_init(&config)==SIDEREON_STATUS_OK);
    SidereonSgp4FitSample sample; memset(&sample,0,sizeof sample);
    SidereonSgp4TleFit *out=(void *)(uintptr_t)1;
#define FIT(s,c,v,vl,d,dl,o) sidereon_sgp4_fit_tle_with_omm_header(s,1,c,v,vl,d,dl,o)
    EXPECT(NULL_CASE,SIDEREON_STATUS_NULL_POINTER,FIT(NULL,&config,date,19,origin,4,&out)); CHECK(!out);
    out=(void *)(uintptr_t)1; EXPECT(NULL_CASE,SIDEREON_STATUS_NULL_POINTER,FIT(&sample,NULL,date,19,origin,4,&out)); CHECK(!out);
    out=(void *)(uintptr_t)1; EXPECT(NULL_CASE,SIDEREON_STATUS_NULL_POINTER,FIT(&sample,&config,NULL,1,origin,4,&out)); CHECK(!out);
    out=(void *)(uintptr_t)1; EXPECT(NULL_CASE,SIDEREON_STATUS_NULL_POINTER,FIT(&sample,&config,date,19,NULL,1,&out)); CHECK(!out);
    EXPECT(NULL_CASE,SIDEREON_STATUS_NULL_POINTER,FIT(&sample,&config,date,19,origin,4,NULL));
    EXPECT(OVERLAP,SIDEREON_STATUS_INVALID_ARGUMENT,FIT((SidereonSgp4FitSample *)&out,&config,date,19,origin,4,&out));
    EXPECT(OVERLAP,SIDEREON_STATUS_INVALID_ARGUMENT,FIT(&sample,(SidereonSgp4FitConfig *)&out,date,19,origin,4,&out));
    EXPECT(OVERLAP,SIDEREON_STATUS_INVALID_ARGUMENT,FIT(&sample,&config,(uint8_t *)&out,sizeof out,origin,4,&out));
    EXPECT(OVERLAP,SIDEREON_STATUS_INVALID_ARGUMENT,FIT(&sample,&config,date,19,(uint8_t *)&out,sizeof out,&out));
    double weight=1; config.weights=&weight; config.weight_count=1;
    EXPECT(OVERLAP,SIDEREON_STATUS_INVALID_ARGUMENT,FIT(&sample,&config,date,19,origin,4,(SidereonSgp4TleFit **)&weight));
    config.weights=NULL; config.weight_count=0; config.x_scale_kind=SIDEREON_SGP4_X_SCALE_KIND_VALUES; config.x_scale_values=&weight; config.x_scale_value_count=1;
    EXPECT(OVERLAP,SIDEREON_STATUS_INVALID_ARGUMENT,FIT(&sample,&config,date,19,origin,4,(SidereonSgp4TleFit **)&weight));
    out=(void *)(uintptr_t)1; EXPECT(UTF8,SIDEREON_STATUS_INVALID_TOKEN,FIT(&sample,&config,bad,1,origin,4,&out)); CHECK(!out);
    out=(void *)(uintptr_t)1; EXPECT(UTF8,SIDEREON_STATUS_INVALID_TOKEN,FIT(&sample,&config,date,19,bad,1,&out)); CHECK(!out);
#undef FIT
    return 0;
}
static int sidereon_navcen_parse_at_cases(void) {
    SidereonNavcenAssessments *out=(void *)(uintptr_t)1;
    EXPECT(NULL_CASE,SIDEREON_STATUS_NULL_POINTER,sidereon_navcen_parse_at(NULL,1,0,&out)); CHECK(!out);
    EXPECT(NULL_CASE,SIDEREON_STATUS_NULL_POINTER,sidereon_navcen_parse_at(value,6,0,NULL));
    out=(void *)(uintptr_t)1; EXPECT(UTF8,SIDEREON_STATUS_INVALID_ARGUMENT,sidereon_navcen_parse_at(bad,1,0,&out)); CHECK(!out); return 0;
}
static int sidereon_navcen_assessments_free_cases(void) { sidereon_navcen_assessments_free(NULL); counts[FREE_CASE]++; return 0; }
static int sidereon_navcen_assessment_count_cases(void) {
    size_t out=99; EXPECT(NULL_CASE,SIDEREON_STATUS_NULL_POINTER,sidereon_navcen_assessment_count(NULL,&out)); CHECK(out==0);
    EXPECT(NULL_CASE,SIDEREON_STATUS_NULL_POINTER,sidereon_navcen_assessment_count(nav,NULL)); return 0;
}
static int sidereon_navcen_assessment_cases(void) {
    SidereonNavcenAssessment out; memset(&out,0x55,sizeof out);
    EXPECT(NULL_CASE,SIDEREON_STATUS_NULL_POINTER,sidereon_navcen_assessment(NULL,0,&out)); CHECK(out.prn==0 && !out.usable && !out.active_nanu);
    EXPECT(NULL_CASE,SIDEREON_STATUS_NULL_POINTER,sidereon_navcen_assessment(nav,0,NULL));
    EXPECT(INDEX,SIDEREON_STATUS_INVALID_ARGUMENT,sidereon_navcen_assessment(nav,nav_count,&out)); CHECK(out.prn==0 && !out.usable && !out.active_nanu); return 0;
}
#define NAV_TEXT(name) static int name##_cases(void) { size_t w=99,r=99; \
 EXPECT(NULL_CASE,SIDEREON_STATUS_NULL_POINTER,name(NULL,0,bytes,sizeof bytes,&w,&r)); CHECK(w==0 && r==0); \
 EXPECT(NULL_CASE,SIDEREON_STATUS_NULL_POINTER,name(nav,0,bytes,sizeof bytes,NULL,&r)); \
 EXPECT(NULL_CASE,SIDEREON_STATUS_NULL_POINTER,name(nav,0,bytes,sizeof bytes,&w,NULL)); \
 EXPECT(NULL_CASE,SIDEREON_STATUS_NULL_POINTER,name(nav,0,NULL,1,&w,&r)); \
 EXPECT(OVERLAP,SIDEREON_STATUS_INVALID_ARGUMENT,name(nav,0,bytes,sizeof bytes,&w,&w)); \
 size_t i=0; for (;i<nav_count;i++) { CHECK(name(nav,i,NULL,0,&w,&r)==SIDEREON_STATUS_OK); if(r) break; } \
 CHECK(i<nav_count); union { size_t align; uint8_t data[4096]; } region; \
 EXPECT(OVERLAP,SIDEREON_STATUS_INVALID_ARGUMENT,name(nav,i,region.data,sizeof region.data,(size_t *)region.data,&r)); \
 EXPECT(OVERLAP,SIDEREON_STATUS_INVALID_ARGUMENT,name(nav,i,region.data,sizeof region.data,&w,(size_t *)region.data)); \
 EXPECT(INDEX,SIDEREON_STATUS_INVALID_ARGUMENT,name(nav,nav_count,bytes,sizeof bytes,&w,&r)); CHECK(w==0 && r==0); return 0; }
NAV_TEXT(sidereon_navcen_assessment_nanu_subject)
NAV_TEXT(sidereon_navcen_assessment_nanu_type)
NAV_TEXT(sidereon_navcen_assessment_outage_start)
#define ENTRIES(X) \
 X(sidereon_celestrak_gp_source_entry) \
 X(sidereon_navcen_source_entry) \
 X(sidereon_navcen_constellation_url) \
 X(sidereon_data_allowed_hosts) \
 X(sidereon_celestrak_group) \
 X(sidereon_celestrak_constellation_url) \
 X(sidereon_celestrak_gp_url) \
 X(sidereon_celestrak_gp_catalog_number_url) \
 X(sidereon_celestrak_gp_special_url) \
 X(sidereon_celestrak_gp_format_as_str) \
 X(sidereon_celestrak_gp_special_as_str) \
 X(sidereon_parse_celestrak_gp_response) \
 X(sidereon_celestrak_gp_response_free) \
 X(sidereon_celestrak_gp_response_counts) \
 X(sidereon_celestrak_gp_response_equal) \
 X(sidereon_celestrak_gp_response_raw) \
 X(sidereon_celestrak_gp_response_record_json) \
 X(sidereon_celestrak_gp_record_to_omm) \
 X(sidereon_celestrak_gp_record_tle) \
 X(sidereon_celestrak_gp_record_to_element_set) \
 X(sidereon_celestrak_gp_record_equal) \
 X(sidereon_celestrak_gp_payload_equal) \
 X(sidereon_celestrak_tle_record_equal) \
 X(sidereon_celestrak_gp_rejection_reason_equal) \
 X(sidereon_celestrak_gp_structural_error_equal) \
 X(sidereon_omm_from_tle) \
 X(sidereon_omm_with_ccsds_header) \
 X(sidereon_omm_to_kvn_lenient) \
 X(sidereon_omm_to_xml_lenient) \
 X(sidereon_omm_equal) \
 X(sidereon_omm_array_equal) \
 X(sidereon_omm_array_span) \
 X(sidereon_omm_array_span_count) \
 X(sidereon_sgp4_fit_tle_with_omm_header) \
 X(sidereon_navcen_parse_at) \
 X(sidereon_navcen_assessment_count) \
 X(sidereon_navcen_assessment) \
 X(sidereon_navcen_assessment_nanu_type) \
 X(sidereon_navcen_assessment_nanu_subject) \
 X(sidereon_navcen_assessment_outage_start) \
 X(sidereon_navcen_assessments_free)
struct entry { const char *name; int (*run)(void); };
#define ENTRY(name) {#name, name##_cases},
static const struct entry entries[] = { ENTRIES(ENTRY) };

#define HANDLE_BUFFER_EXTRA(type, handle, call) do { \
    const type *h = NULL; uint8_t *o=bytes; size_t n=sizeof bytes, *wp=&w, *rp=&r; \
    w=r=99; EXPECT(NULL_CASE,SIDEREON_STATUS_NULL_POINTER,(call)); CHECK(w==0 && r==0); \
    h=handle; o=(uint8_t *)h; n=1; w=r=99; EXPECT(OVERLAP,SIDEREON_STATUS_INVALID_ARGUMENT,(call)); CHECK(w==99 && r==99); \
    o=bytes; n=sizeof bytes; wp=(size_t *)h; EXPECT(OVERLAP,SIDEREON_STATUS_INVALID_ARGUMENT,(call)); \
    wp=&w; rp=(size_t *)h; EXPECT(OVERLAP,SIDEREON_STATUS_INVALID_ARGUMENT,(call)); \
} while (0)

static int additional_cases(const char *name) {
    size_t w=99,r=99; bool equal=true;
#define IS(n) (strcmp(name, #n)==0)
#define BAD_CODE(expr) EXPECT(CODE,SIDEREON_STATUS_INVALID_ARGUMENT,(expr))
    const uint32_t below=UINT32_MAX;
    if (IS(sidereon_celestrak_group)) {
        BAD_CODE(sidereon_celestrak_group(below,bytes,sizeof bytes,&w,&r));
        BAD_CODE(sidereon_celestrak_group(7,bytes,sizeof bytes,&w,&r));
    } else if (IS(sidereon_celestrak_constellation_url)) {
        BAD_CODE(sidereon_celestrak_constellation_url(below,5,bytes,sizeof bytes,&w,&r));
        BAD_CODE(sidereon_celestrak_constellation_url(7,5,bytes,sizeof bytes,&w,&r));
        BAD_CODE(sidereon_celestrak_constellation_url(0,below,bytes,sizeof bytes,&w,&r));
        BAD_CODE(sidereon_celestrak_constellation_url(0,8,bytes,sizeof bytes,&w,&r));
    } else if (IS(sidereon_celestrak_gp_url)) {
        BAD_CODE(sidereon_celestrak_gp_url(below,value,6,5,bytes,sizeof bytes,&w,&r));
        BAD_CODE(sidereon_celestrak_gp_url(4,value,6,5,bytes,sizeof bytes,&w,&r));
        BAD_CODE(sidereon_celestrak_gp_url(1,value,6,5,bytes,sizeof bytes,&w,&r));
        BAD_CODE(sidereon_celestrak_gp_url(0,value,6,below,bytes,sizeof bytes,&w,&r));
        BAD_CODE(sidereon_celestrak_gp_url(0,value,6,8,bytes,sizeof bytes,&w,&r));
        EXPECT(NULL_CASE,SIDEREON_STATUS_NULL_POINTER,sidereon_celestrak_gp_url(0,NULL,1,5,bytes,sizeof bytes,&w,&r));
        EXPECT(UTF8,SIDEREON_STATUS_INVALID_TOKEN,sidereon_celestrak_gp_url(0,bad,1,5,bytes,sizeof bytes,&w,&r));
        EXPECT(OVERLAP,SIDEREON_STATUS_INVALID_ARGUMENT,sidereon_celestrak_gp_url(0,(uint8_t *)&w,sizeof w,5,bytes,sizeof bytes,&w,&r));
        EXPECT(OVERLAP,SIDEREON_STATUS_INVALID_ARGUMENT,sidereon_celestrak_gp_url(0,(uint8_t *)&r,sizeof r,5,bytes,sizeof bytes,&w,&r));
    } else if (IS(sidereon_celestrak_gp_catalog_number_url)) {
        BAD_CODE(sidereon_celestrak_gp_catalog_number_url(25544,below,bytes,sizeof bytes,&w,&r));
        BAD_CODE(sidereon_celestrak_gp_catalog_number_url(25544,8,bytes,sizeof bytes,&w,&r));
    } else if (IS(sidereon_celestrak_gp_special_url)) {
        BAD_CODE(sidereon_celestrak_gp_special_url(below,5,bytes,sizeof bytes,&w,&r));
        BAD_CODE(sidereon_celestrak_gp_special_url(3,5,bytes,sizeof bytes,&w,&r));
        BAD_CODE(sidereon_celestrak_gp_special_url(0,below,bytes,sizeof bytes,&w,&r));
        BAD_CODE(sidereon_celestrak_gp_special_url(0,8,bytes,sizeof bytes,&w,&r));
    } else if (IS(sidereon_celestrak_gp_special_as_str)) {
        BAD_CODE(sidereon_celestrak_gp_special_as_str(below,bytes,sizeof bytes,&w,&r));
        BAD_CODE(sidereon_celestrak_gp_special_as_str(3,bytes,sizeof bytes,&w,&r));
    } else if (IS(sidereon_celestrak_gp_format_as_str)) {
        BAD_CODE(sidereon_celestrak_gp_format_as_str(below,bytes,sizeof bytes,&w,&r));
        BAD_CODE(sidereon_celestrak_gp_format_as_str(8,bytes,sizeof bytes,&w,&r));
    } else if (IS(sidereon_parse_celestrak_gp_response)) {
        SidereonCelestrakGpResponse *out=(void *)(uintptr_t)1;
        BAD_CODE(sidereon_parse_celestrak_gp_response(value,6,below,&out)); CHECK(!out);
        out=(void *)(uintptr_t)1; BAD_CODE(sidereon_parse_celestrak_gp_response(value,6,8,&out)); CHECK(!out);
    } else if (IS(sidereon_celestrak_gp_response_raw)) {
        HANDLE_BUFFER_EXTRA(SidereonCelestrakGpResponse,gp,sidereon_celestrak_gp_response_raw(h,false,0,o,n,wp,rp));
        EXPECT(INDEX,SIDEREON_STATUS_INVALID_ARGUMENT,sidereon_celestrak_gp_response_raw(gp,false,1,bytes,sizeof bytes,&w,&r));
        EXPECT(INDEX,SIDEREON_STATUS_INVALID_ARGUMENT,sidereon_celestrak_gp_response_raw(rejected_gp,true,1,bytes,sizeof bytes,&w,&r));
    } else if (IS(sidereon_celestrak_gp_response_record_json)) {
        HANDLE_BUFFER_EXTRA(SidereonCelestrakGpResponse,gp,sidereon_celestrak_gp_response_record_json(h,false,0,o,n,wp,rp));
        EXPECT(INDEX,SIDEREON_STATUS_INVALID_ARGUMENT,sidereon_celestrak_gp_response_record_json(gp,false,1,bytes,sizeof bytes,&w,&r));
        EXPECT(INDEX,SIDEREON_STATUS_INVALID_ARGUMENT,sidereon_celestrak_gp_response_record_json(rejected_gp,true,1,bytes,sizeof bytes,&w,&r));
    } else if (IS(sidereon_omm_to_kvn_lenient)) {
        HANDLE_BUFFER_EXTRA(SidereonOmm,omm,sidereon_omm_to_kvn_lenient(h,o,n,wp,rp));
    } else if (IS(sidereon_omm_to_xml_lenient)) {
        HANDLE_BUFFER_EXTRA(SidereonOmm,omm,sidereon_omm_to_xml_lenient(h,o,n,wp,rp));
    } else if (IS(sidereon_celestrak_gp_record_equal)) {
        EXPECT(INDEX,SIDEREON_STATUS_INVALID_ARGUMENT,sidereon_celestrak_gp_record_equal(gp,gp,false,1,0,&equal));
        EXPECT(INDEX,SIDEREON_STATUS_INVALID_ARGUMENT,sidereon_celestrak_gp_record_equal(gp,gp,false,0,1,&equal));
        EXPECT(INDEX,SIDEREON_STATUS_INVALID_ARGUMENT,sidereon_celestrak_gp_record_equal(rejected_gp,rejected_gp,true,1,0,&equal));
        EXPECT(INDEX,SIDEREON_STATUS_INVALID_ARGUMENT,sidereon_celestrak_gp_record_equal(rejected_gp,rejected_gp,true,0,1,&equal));
    } else if (IS(sidereon_celestrak_gp_payload_equal)) {
        EXPECT(INDEX,SIDEREON_STATUS_INVALID_ARGUMENT,sidereon_celestrak_gp_payload_equal(gp,gp,1,0,&equal));
        EXPECT(INDEX,SIDEREON_STATUS_INVALID_ARGUMENT,sidereon_celestrak_gp_payload_equal(gp,gp,0,1,&equal));
    } else if (IS(sidereon_celestrak_tle_record_equal)) {
        EXPECT(INDEX,SIDEREON_STATUS_INVALID_ARGUMENT,sidereon_celestrak_tle_record_equal(gp,gp,1,0,&equal));
        EXPECT(INDEX,SIDEREON_STATUS_INVALID_ARGUMENT,sidereon_celestrak_tle_record_equal(gp,gp,0,1,&equal));
    } else if (IS(sidereon_celestrak_gp_rejection_reason_equal)) {
        EXPECT(INDEX,SIDEREON_STATUS_INVALID_ARGUMENT,sidereon_celestrak_gp_rejection_reason_equal(rejected_gp,rejected_gp,1,0,&equal));
        EXPECT(INDEX,SIDEREON_STATUS_INVALID_ARGUMENT,sidereon_celestrak_gp_rejection_reason_equal(rejected_gp,rejected_gp,0,1,&equal));
    } else if (IS(sidereon_celestrak_gp_structural_error_equal)) {
        EXPECT(INDEX,SIDEREON_STATUS_INVALID_ARGUMENT,sidereon_celestrak_gp_structural_error_equal(rejected_gp,rejected_gp,1,0,&equal));
        EXPECT(INDEX,SIDEREON_STATUS_INVALID_ARGUMENT,sidereon_celestrak_gp_structural_error_equal(rejected_gp,rejected_gp,0,1,&equal));
    }
#undef BAD_CODE
#undef IS
    return 0;
}

int main(void) {
    const char body[]="ISS\n1 25544U 98067A   26095.55331950  .00008543  00000+0  16420-3 0  9998\n2 25544  51.6328 299.5432 0006351 274.8255  85.2008 15.48786980560492\n";
    const char incomplete[]="OBJECT_NAME = X\n";
    const char xml[]="<ndm>\n<omm></omm>\n</ndm>";
    CHECK(sidereon_parse_celestrak_gp_response((const uint8_t *)body,strlen(body),0,&gp)==SIDEREON_STATUS_OK);
    CHECK(sidereon_parse_celestrak_gp_response((const uint8_t *)body,strlen(body),0,&gp2)==SIDEREON_STATUS_OK);
    CHECK(sidereon_parse_celestrak_gp_response((const uint8_t *)incomplete,strlen(incomplete),SIDEREON_CELESTRAK_GP_FORMAT_KVN,&rejected_gp)==SIDEREON_STATUS_OK);
    size_t accepted_count=0, rejected_count=0;
    CHECK(sidereon_celestrak_gp_response_counts(rejected_gp,&accepted_count,&rejected_count)==SIDEREON_STATUS_OK && accepted_count==0 && rejected_count==1);
    CHECK(sidereon_celestrak_gp_record_to_omm(gp,0,&omm)==SIDEREON_STATUS_OK);
    CHECK(sidereon_celestrak_gp_record_to_omm(gp2,0,&omm2)==SIDEREON_STATUS_OK);
    CHECK(sidereon_celestrak_gp_record_tle(gp,0,0,0,&tle)==SIDEREON_STATUS_OK);
    CHECK(sidereon_omm_parse_xml_all((const uint8_t *)xml,strlen(xml),&array)==SIDEREON_STATUS_OK);
    CHECK(sidereon_omm_parse_xml_all((const uint8_t *)xml,strlen(xml),&array2)==SIDEREON_STATUS_OK);
    CHECK(sidereon_navcen_parse_at(CONSTELLATION_NAVCEN_HTML,CONSTELLATION_NAVCEN_HTML_LEN,0,&nav)==SIDEREON_STATUS_OK);
    CHECK(sidereon_navcen_assessment_count(nav,&nav_count)==SIDEREON_STATUS_OK);
    for (size_t i=0;i<sizeof entries/sizeof entries[0];i++) { if (entries[i].run() || additional_cases(entries[i].name)) { fprintf(stderr,"entry: %s\n",entries[i].name); return 1; } }
    printf("catalog refusals: null=%zu overlap=%zu code=%zu utf8=%zu index=%zu free=%zu\n",counts[0],counts[1],counts[2],counts[3],counts[4],counts[5]);
    sidereon_celestrak_gp_response_free(rejected_gp); sidereon_celestrak_gp_response_free(gp2); sidereon_celestrak_gp_response_free(gp); sidereon_omm_free(omm2); sidereon_omm_free(omm); sidereon_tle_free(tle); sidereon_omm_array_free(array2); sidereon_omm_array_free(array); sidereon_navcen_assessments_free(nav); return 0;
}
