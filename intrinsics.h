#include "math.h"

typedef struct BitScanResult BitScanResult;
struct BitScanResult {
    b32 found;
    u32 index;
};

BitScanResult find_least_significant_set_bit(u32 value) {
    BitScanResult result = {};

    if (value != 0) {
        result.found = 1;
        result.index = __builtin_ctzl(value);
    }

    return result;
}

inline static i32 
round_f32_to_i32(f32 value) {
    i32 result = (i32)roundf(value);
    return result;
}

inline static u32
round_f32_to_u32(f32 value) {
    u32 result = (u32)roundf(value);
    return result;
} 

inline static i32
floor_f32_to_i32(f32 value) {
    i32 result = (i32)floorf(value);
    return result;
}

inline static i32
trunc_f32_to_i32(f32 value) {
    i32 result = (i32)value;
    return result;
}

// inline f32
// sin(f32 angle) {
//     f32 result = sinf(angle);
//     return result;
// }

// inline f32
// cos(f32 angle) {
//     f32 result = cosf(angle);
//     return result;
// }

// inline f32
// atan2(f32 y, f32 x) {
//     f32 result = atan2f(y, x);
//     return result;
// }