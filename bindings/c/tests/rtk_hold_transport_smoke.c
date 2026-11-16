#include "sidereon.h"
#include "rtk_hold_transport_fixture.h"
#include <stdbool.h>
#include <stdio.h>
#include <string.h>

#define REQUIRE(x) do { if (!(x)) { char msg[512] = {0}; sidereon_last_error_message(msg, sizeof(msg)); fprintf(stderr,"%s:%d: %s: %s\n",__FILE__,__LINE__,#x,msg); return 1; } } while(0)
static int run_case(int fdma) {
    static const char *const ids[6] = {"G01","G02","G03","G04","G05","G06"};
    const double base[3] = {3512900.0,780500.0,5248700.0};
    SidereonRtkArcObservation base_obs[3][6] = {{{0}}}, rover_obs[3][6] = {{{0}}};
    SidereonRtkArcPositionEntry positions[3][6] = {{{0}}};
    SidereonRtkArcEpoch epochs[3] = {{0}};
    SidereonRtkFloatMapEntry wavelengths[6], offsets[5];
    for (int j=0;j<6;j++) { wavelengths[j]=(SidereonRtkFloatMapEntry){ids[j], fdma && j==3 ? 0.24 : 0.19}; if(j>0) offsets[j-1]=(SidereonRtkFloatMapEntry){ids[j],0.05}; }
    for (int e=0;e<3;e++) {
        int count=0;
        for (int j=0;j<6;j++) {
            if (!fdma && e==0 && j==3) continue; /* Promote previously unheld G04. */
            const HoldSatellite *v=&hold_cases[fdma][e].sat[j];
            base_obs[e][count]=(SidereonRtkArcObservation){.sat_id=ids[j],.ambiguity_id=ids[j],.code_m=v->base_code,.phase_m=v->base_phase};
            rover_obs[e][count]=(SidereonRtkArcObservation){.sat_id=ids[j],.ambiguity_id=ids[j],.code_m=v->rover_code,.phase_m=v->rover_phase};
            positions[e][count].id=ids[j]; memcpy(positions[e][count].pos,v->pos,sizeof v->pos); count++;
        }
        epochs[e].base=base_obs[e]; epochs[e].base_count=(size_t)count; epochs[e].rover=rover_obs[e]; epochs[e].rover_count=(size_t)count;
        epochs[e].satellite_positions=positions[e]; epochs[e].satellite_position_count=(size_t)count;
        epochs[e].has_velocity_mps=true; epochs[e].has_prediction_time=true; epochs[e].prediction_time_s=e*30.0;
        if(e) for(int a=0;a<3;a++) epochs[e].velocity_mps[a]=(hold_cases[fdma][e].rover[a]-hold_cases[fdma][e-1].rover[a])/30.0;
    }
    SidereonRtkArcReferenceEntry reference={SIDEREON_GNSS_SYSTEM_GPS,"G01"};
    SidereonRtkArcConfig cfg={0}; memcpy(cfg.base_m,base,sizeof base);
    for(int a=0;a<3;a++) cfg.initial_baseline_m[a]=hold_cases[fdma][0].rover[a]-base[a];
    cfg.reference_mode=SIDEREON_RTK_ARC_REFERENCE_MODE_PER_SYSTEM; cfg.reference_per_system=&reference; cfg.reference_per_system_count=1;
    REQUIRE(sidereon_rtk_measurement_model_init(&cfg.model)==SIDEREON_STATUS_OK); cfg.model.sagnac=false;
    cfg.baseline_prior_sigma_m=100.0; cfg.ambiguity_prior_sigma_m=1000.0;
    cfg.wavelengths_m=fdma ? wavelengths : wavelengths+1; cfg.wavelength_count=fdma ? 6 : 5;
    cfg.offsets_m=offsets; cfg.offset_count=5;
    REQUIRE(sidereon_rtk_arc_update_options_init(&cfg.update_options)==SIDEREON_STATUS_OK);
    cfg.update_options.hold_sigma_m=1e-4; cfg.update_options.dynamics_velocity_propagated=true;
    cfg.preprocessing.has_elevation_mask_deg=true; cfg.preprocessing.elevation_mask_deg=15.0;
    SidereonRtkArcSolution *solution=NULL;
    REQUIRE(sidereon_solve_rtk_arc(epochs,2,&cfg,&solution)==SIDEREON_STATUS_OK && solution);
    SidereonRtkArcEpochMetadata meta; REQUIRE(sidereon_rtk_arc_solution_epoch_metadata(solution,1,&meta)==SIDEREON_STATUS_OK);
    REQUIRE(meta.elevation_mask_converged && meta.reference_satellite_count==1 && meta.released_hold_count>=2);
    SidereonSatelliteToken ref[2]; size_t written=0, required=0;
    REQUIRE(sidereon_rtk_arc_solution_epoch_reference_satellites(solution,1,ref,2,&written,&required)==SIDEREON_STATUS_OK && written==1 && strcmp((char *)ref[0].bytes,"G04")==0);
    /* RTKLIB mask oracle epoch 2 rejects G01 at this rover position. */
    SidereonSatelliteToken masked[8];
    REQUIRE(sidereon_rtk_arc_solution_epoch_elevation_masked_satellites(solution,1,masked,8,&written,&required)==SIDEREON_STATUS_OK && written==1 && strcmp((char *)masked[0].bytes,"G01")==0);
    SidereonReleasedRtkHold released[8];
    REQUIRE(sidereon_rtk_arc_solution_epoch_released_holds(solution,1,released,8,&written,&required)==SIDEREON_STATUS_OK && written==meta.released_hold_count);
    for(size_t i=0;i<written;i++) REQUIRE(released[i].reason==(fdma ? SIDEREON_RTK_HOLD_RELEASE_REASON_INCOMPATIBLE_WAVELENGTHS : SIDEREON_RTK_HOLD_RELEASE_REASON_UNHELD_REFERENCE));
    sidereon_rtk_arc_solution_free(solution); return 0;
}
int main(void) { if(run_case(0)||run_case(1)) return 1; puts("RTK hold transport fixture: both release reasons"); return 0; }
