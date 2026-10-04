#include "sidereon.h"

#include <math.h>
#include <stddef.h>
#include <stdio.h>
#include <string.h>

int main(void) {
    const SidereonPositionCovariance covariance = {
        .ecef_m2 = {1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0},
        .enu_m2 = {1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0},
    };
    union {
        max_align_t alignment;
        unsigned char bytes[sizeof(SidereonPositionErrorMetrics)];
        SidereonPositionErrorMetrics metrics;
    } storage;
    memset(storage.bytes, 0xA5, sizeof(storage.bytes));
    SidereonErrorMetricsErrorKind error;
    memset(&error, 0xA5, sizeof(error));

    enum SidereonStatus status = sidereon_error_metrics_from_position_covariance(
        &covariance, &storage.metrics, &error);
    const SidereonPositionErrorMetrics *metrics = &storage.metrics;
    if (status != SIDEREON_STATUS_OK || error != SIDEREON_ERROR_METRICS_ERROR_KIND_NONE ||
        !isfinite(metrics->sigma_e_m) || !isfinite(metrics->sigma_n_m) ||
        !isfinite(metrics->sigma_u_m) || metrics->sigma_e_m != 1.0 ||
        metrics->sigma_n_m != 1.0 || metrics->sigma_u_m != 1.0 ||
        !metrics->cep_m.approx_valid || metrics->r95_m.approx_valid ||
        metrics->r99_m.approx_valid || !metrics->sep_m.approx_valid) {
        fprintf(stderr, "raw output initialization failed (status=%d, error=%d)\n", status,
                error);
        return 1;
    }
    return 0;
}
