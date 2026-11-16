/* Core evaluates satellite elevation in the RTKLIB ecef2pos/satazel frame; elevation weighting changes these values. */
static const uint64_t RTK_STATIC_FLOAT[3] = {UINT64_C(0xbfef8dac549dbd88), UINT64_C(0xbfe528ffe8c612d4), UINT64_C(0x3ff117af59d28545)};
static const uint64_t RTK_STATIC_FIXED[3] = {UINT64_C(0xbfef895fef6076c7), UINT64_C(0xbfe4e3deec4c7b7e), UINT64_C(0x3ff115e70bda0544)};
static const uint64_t RTK_STATIC_RATIO = UINT64_C(0x3ffd9f9e26af453b);
static const uint64_t RTK_REF_RATIO = UINT64_C(0x4055ce66346584ff);
static const uint64_t RTK_REF_POSITION[3] = {UINT64_C(0x414f181daf5f86cc), UINT64_C(0x412c701ad35a97d0), UINT64_C(0x4152510859bc8a81)};
static const uint64_t RTK_REF_BASELINE[3] = {UINT64_C(0xbfef90fb0ac538bc), UINT64_C(0xbfe4dc4b66851386), UINT64_C(0x3ff115a6aa247f7f)};
static const uint64_t RTK_REF_COVARIANCE[9] = {UINT64_C(0x3f04714c2259df93), UINT64_C(0x3eded4741ff020e6), UINT64_C(0x3ef2f6943aef19b9), UINT64_C(0x3eded4741ff020e5), UINT64_C(0x3eec2a218ba73146), UINT64_C(0x3ed80783fb1f3ad5), UINT64_C(0x3ef2f6943aef19b9), UINT64_C(0x3ed80783fb1f3ad7), UINT64_C(0x3f090f6527135014)};
static const uint64_t RTK_WL_FLOAT[3] = {UINT64_C(0xbfefe0fc6828b48c), UINT64_C(0xbfe5c58f05db37af), UINT64_C(0x3ff106a86f04ad58)};
static const uint64_t RTK_WL_FIXED[3] = {UINT64_C(0xbfef6be70a1eaa2b), UINT64_C(0xbfe4d1177a539249), UINT64_C(0x3ff0ff8c16c0b3cd)};
static const uint64_t RTK_WL_RATIO = UINT64_C(0x4031728ec089fb89);
