#include "sidereon.h"
#include <stdint.h>
#include <stdio.h>
#include <string.h>

/* NULL and overlapping output checks for every newly introduced function.
 * Invalid code is applicable only to the ADD constructor's system argument. */
#define EXPECT(expr, want) do { SidereonStatus got = (expr); if (got != (want)) { fprintf(stderr, "%s: got %d expected %d\n", #expr, (int)got, (int)(want)); return 1; } } while (0)
#define NULLP SIDEREON_STATUS_NULL_POINTER
#define BAD SIDEREON_STATUS_INVALID_ARGUMENT
static int chi2(void) { EXPECT(sidereon_chi2_isf(.01, 2, NULL), NULLP); return 0; }
static int add(void) { SidereonAraimSatelliteIsmModel out; EXPECT(sidereon_araim_satellite_ism_model_add_v3_0(0,.5,1,1,0,0,NULL),NULLP); EXPECT(sidereon_araim_satellite_ism_model_add_v3_0(UINT32_MAX,.5,1,1,0,0,&out),BAD); return 0; }
#define COV_CASE(name, type) static int name(void) { size_t w=0,r=0; type out[2]; EXPECT(sidereon_coverage_grid_##name(NULL,0,out,2,&w,&r),NULLP); EXPECT(sidereon_coverage_grid_##name((const SidereonCoverageGrid *)out,0,out,2,&w,&r),BAD); return 0; }
COV_CASE(try_visible_mask, bool)
COV_CASE(try_access_counts, size_t)
#define PPP_CASE(name, type) static int name(void) { type *h=(type *)(uintptr_t)16; bool out; EXPECT(sidereon_ppp_##name(NULL,&out),NULLP); EXPECT(sidereon_ppp_##name(h,NULL),NULLP); EXPECT(sidereon_ppp_##name(h,(bool *)h),BAD); return 0; }
PPP_CASE(float_solution_posterior_variance_factor_estimated, SidereonPppFloatSolution)
PPP_CASE(fixed_solution_posterior_variance_factor_estimated, SidereonPppFixedSolution)
#define RTK_CASE(name, type) static int name(void) { size_t w=0,r=0; type out[2]; EXPECT(sidereon_rtk_arc_solution_epoch_##name(NULL,0,out,2,&w,&r),NULLP); EXPECT(sidereon_rtk_arc_solution_epoch_##name((const SidereonRtkArcSolution *)out,0,out,2,&w,&r),BAD); return 0; }
RTK_CASE(reference_satellites, SidereonSatelliteToken)
RTK_CASE(elevation_masked_satellites, SidereonSatelliteToken)
RTK_CASE(released_holds, SidereonReleasedRtkHold)
static int static_mask(void) { const SidereonRtkStaticArcSolution *h=(void *)(uintptr_t)16; bool out; EXPECT(sidereon_rtk_static_arc_solution_elevation_mask_converged(NULL,&out),NULLP); EXPECT(sidereon_rtk_static_arc_solution_elevation_mask_converged(h,(bool *)h),BAD); return 0; }
static int shared_dop(void) { const SidereonSppSolution *h=(void *)(uintptr_t)16; SidereonDop out; EXPECT(sidereon_spp_solution_rtklib_single_clock_dop(NULL,&out),NULLP); EXPECT(sidereon_spp_solution_rtklib_single_clock_dop(h,(SidereonDop *)h),BAD); return 0; }
#define RAIM_CASE(name) static int name(void) { SidereonPppRaimGeometryRow rows[1]={{.satellite_id="G01",.line_of_sight={1,0,0}}}; double weights[1]={1}; SidereonPppRaimConfig cfg; SidereonGeodetic geo={0}; EXPECT(sidereon_ppp_raim_config_default(&cfg),SIDEREON_STATUS_OK); EXPECT(sidereon_ppp_raim_##name(rows,weights,1,geo,cfg,NULL),NULLP); EXPECT(sidereon_ppp_raim_##name(rows,weights,1,geo,cfg,(SidereonPppRaimProtectionLevels *)rows),BAD); return 0; }
RAIM_CASE(protection_levels_with_weights)
RAIM_CASE(fault_free_accuracy_bounds)
struct entry { const char *name; int (*test)(void); };
#define ENTRY(x) {#x,x}
static const struct entry entries[] = { ENTRY(chi2), ENTRY(add), ENTRY(try_visible_mask), ENTRY(try_access_counts), ENTRY(float_solution_posterior_variance_factor_estimated), ENTRY(fixed_solution_posterior_variance_factor_estimated), ENTRY(reference_satellites), ENTRY(elevation_masked_satellites), ENTRY(released_holds), ENTRY(static_mask), ENTRY(shared_dop), ENTRY(protection_levels_with_weights), ENTRY(fault_free_accuracy_bounds) };
int main(void) { for (size_t i=0;i<sizeof(entries)/sizeof(entries[0]);i++) if(entries[i].test()) { fprintf(stderr,"entry: %s\n",entries[i].name); return 1; } puts("parity refusal table: 13 entry points"); return 0; }
