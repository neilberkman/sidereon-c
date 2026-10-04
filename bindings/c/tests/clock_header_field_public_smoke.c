#include "sidereon.h"
#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

enum { HAS_VERSION=1, HAS_SYSTEM=2, HAS_COUNT=4, HAS_INTEGER=8,
       HAS_START=16, HAS_STOP=32, HAS_CONSTRAINT=64, HAS_XYZ=128 };

typedef struct ExpectedHeader {
    uint32_t kind, reading;
    uint8_t flags;
    size_t text_count;
    const char *const *texts;
} ExpectedHeader;

static void require_true(bool ok, const char *message) {
    if (!ok) { fprintf(stderr, "clock_header_field_public_smoke: %s\n", message); exit(1); }
}
static void assert_part(const SidereonClockHeaderRecords *records, size_t row, size_t part,
                       const char *expected) {
    uint8_t bytes[256]; size_t written=0, required=0;
    enum SidereonStatus status=sidereon_clock_header_records_field_text(records,row,part,bytes,sizeof(bytes),&written,&required);
    size_t n=strlen(expected);
    require_true(status==SIDEREON_STATUS_OK && written==n && required==n && memcmp(bytes,expected,n)==0,
                 "typed text part changed");
}
static void check_fixture(const char *path, bool a18, const ExpectedHeader *expected, size_t n) {
    FILE *f=fopen(path,"rb"); require_true(f!=NULL,"cannot open clock fixture");
    require_true(fseek(f,0,SEEK_END)==0,"cannot seek fixture"); long end=ftell(f);
    require_true(end>=0 && fseek(f,0,SEEK_SET)==0,"cannot size fixture");
    uint8_t *text=malloc((size_t)end); require_true(text!=NULL,"cannot allocate fixture");
    require_true(fread(text,1,(size_t)end,f)==(size_t)end && fclose(f)==0,"cannot read fixture");
    SidereonRinexClock *clock=NULL;
    require_true(sidereon_rinex_clock_parse(text,(size_t)end,&clock)==SIDEREON_STATUS_OK && clock!=NULL,
                 "public strict parser rejected fixture");
    free(text);
    SidereonClockHeaderRecords *records=NULL;
    require_true(sidereon_rinex_clock_header_records(clock,&records)==SIDEREON_STATUS_OK && records!=NULL,
                 "public header snapshot failed");
    size_t count=0; require_true(sidereon_clock_header_records_count(records,&count)==SIDEREON_STATUS_OK && count==n,
                                 "header record count changed");
    for(size_t i=0;i<n;i++) {
        SidereonClockHeaderRecord got;
        require_true(sidereon_clock_header_records_get(records,i,&got)==SIDEREON_STATUS_OK,
                     "header record get failed");
        require_true(got.field_kind==expected[i].kind && got.reading==expected[i].reading &&
                     got.text_part_count==expected[i].text_count,"field kind, reading or text count changed");
        for(size_t p=0;p<expected[i].text_count;p++) assert_part(records,i,p,expected[i].texts[p]);
        uint8_t flags=(got.has_version?HAS_VERSION:0)|(got.has_system_code?HAS_SYSTEM:0)|
            (got.has_count?HAS_COUNT:0)|(got.has_integer?HAS_INTEGER:0)|
            (got.has_start?HAS_START:0)|(got.has_stop?HAS_STOP:0)|
            (got.has_constraint_s?HAS_CONSTRAINT:0)|(got.has_xyz_mm?HAS_XYZ:0);
        require_true(flags==expected[i].flags,"typed field presence changed");
        if(a18 && i==0) require_true(got.version==3.04,"A18 version changed");
        if(!a18) {
            if(i==0) require_true(got.version==3.04,"A17 version changed");
            if(i==5) require_true(got.system_code==71 && got.count==4,"A17 observation type fields changed");
            if(i==7) require_true(got.integer==10,"A17 leap seconds changed");
            if(i==10) require_true(got.count==2,"A17 data type count changed");
            if(i==12 || i==14) {
                require_true(got.count==1,"A17 clock-ref count changed");
                require_true(got.start.year==1994 && got.start.month==7 && got.start.day==14 &&
                    got.start.hour==(i==12?0:21) && got.start.minute==0 && got.start.second==0.0 &&
                    got.stop.year==1994 && got.stop.month==7 && got.stop.day==14 &&
                    got.stop.hour==(i==12?20:21) && got.stop.minute==59 && got.stop.second==0.0,
                    "A17 clock-ref epochs changed");
            }
            if(i==13 || i==15) require_true(got.constraint_s==-0.123456789012,"A17 clock constraint changed");
            if(i==16) require_true(got.count==4,"A17 solution-station count changed");
            if(i>=17 && i<=21) {
                static const int64_t xyz[5][3]={{1234567890,-1234567890,-1234567890},{-1234567890,1234567890,-1234567890},{1234567890,-1234567890,1234567890},{-1234567890,1234567890,-1234567890},{1234567890,-1234567890,-1234567890}};
                require_true(memcmp(got.xyz_mm,xyz[i-17],sizeof(got.xyz_mm))==0,"A17 station coordinates changed");
            }
            if(i==22) require_true(got.count==27,"A17 satellite count changed");
        } else {
            if(i==4) require_true(got.integer==10,"A18 GNSS leap seconds changed");
            if(i==5) require_true(got.count==2,"A18 data type count changed");
        }
    }
    sidereon_clock_header_records_free(records); sidereon_rinex_clock_free(clock);
}

static const char *a17_0[]={"C","G"}, *a17_1[]={"TORINEXC V9.9","USNO","19960403  001000 UTC"};
static const char *a17_2[]={"EXAMPLE OF A CLOCK DATA ANALYSIS FILE"}, *a17_3[]={"IN THIS CASE ANALYSIS RESULTS FROM GPS ONLY ARE INCLUDED"}, *a17_4[]={"No re-alignment of the clocks has been applied."};
static const char *a17_5[]={"C1W","L1W","C2W","L2W"}, *a17_6[]={"GPS"};
static const char *a17_8[]={"G","CC2NONCC","p1c1bias.hist @ goby.nrl.navy.mil"}, *a17_9[]={"G","PAGES","igs05.atx @ igscb.jpl.nasa.gov"};
static const char *a17_10[]={"AS","AR"}, *a17_11[]={"USN","USNO USING GIPSY/OASIS-II"}, *a17_13[]={"USNO","40451S003"}, *a17_15[]={"TIDB","50103M108"};
static const char *a17_16[]={"ITRF96"}, *a17_17[]={"GOLD","40405S031"}, *a17_18[]={"AREQ","42202M005"}, *a17_19[]={"TIDB","50103M108"}, *a17_20[]={"HARK","30302M007"}, *a17_21[]={"USNO","40451S003"};
static const char *a17_23[]={"G01","G02","G03","G04","G05","G06","G07","G08","G09","G10","G13","G14","G15","G16","G17","G18"};
static const char *a17_24[]={"G19","G21","G22","G23","G24","G25","G26","G27","G29","G30","G31"};
static const ExpectedHeader rows17[]={
 {1,0,HAS_VERSION,2,a17_0},{2,0,0,3,a17_1},{3,0,0,1,a17_2},{3,0,0,1,a17_3},{3,0,0,1,a17_4},
 {4,0,HAS_SYSTEM|HAS_COUNT,4,a17_5},{5,0,0,1,a17_6},{6,0,HAS_INTEGER,0,NULL},{8,1,0,3,a17_8},{9,1,0,3,a17_9},
 {10,0,HAS_COUNT,2,a17_10},{13,0,0,2,a17_11},{14,1,HAS_COUNT|HAS_START|HAS_STOP,0,NULL},{15,0,HAS_CONSTRAINT,2,a17_13},
 {14,1,HAS_COUNT|HAS_START|HAS_STOP,0,NULL},{15,0,HAS_CONSTRAINT,2,a17_15},{16,0,HAS_COUNT,1,a17_16},
 {17,0,HAS_XYZ,2,a17_17},{17,0,HAS_XYZ,2,a17_18},{17,0,HAS_XYZ,2,a17_19},{17,0,HAS_XYZ,2,a17_20},{17,0,HAS_XYZ,2,a17_21},
 {18,0,HAS_COUNT,0,NULL},{19,0,0,16,a17_23},{19,0,0,11,a17_24},{20,0,0,0,NULL}};
static const char *a18_0[]={"C",""}, *a18_1[]={"TORINEXC V9.9","USNO","19960403  001000 UTC"};
static const char *a18_2[]={"EXAMPLE OF A CLOCK DATA FILE"}, *a18_3[]={"IN THIS CASE CALIBRATION/DISCONTINUITY DATA GIVEN"}, *a18_5[]={"CR","DR"};
static const char *a18_6[]={"USNO","40451S003"}, *a18_7[]={"UTC(USNO) MASTER CLOCK VIA CONTINUOUS CABLE MONITOR"};
static const ExpectedHeader rows18[]={
 {1,0,HAS_VERSION,2,a18_0},{2,0,0,3,a18_1},{3,0,0,1,a18_2},{3,0,0,1,a18_3},{7,0,HAS_INTEGER,0,NULL},
 {10,0,HAS_COUNT,2,a18_5},{11,1,0,2,a18_6},{12,0,0,1,a18_7},{20,0,0,0,NULL}};
int main(int argc,char **argv){require_true(argc==3,"usage: clock_header_field_public_smoke A17 A18");check_fixture(argv[1],false,rows17,sizeof(rows17)/sizeof(rows17[0]));check_fixture(argv[2],true,rows18,sizeof(rows18)/sizeof(rows18[0]));puts("clock_header_field_public_smoke: OK");return 0;}
