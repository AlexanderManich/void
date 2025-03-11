#include "lib_base.h"

u32 safe_u64_to_u32(u64 value) {
    assert(value <= max_u32);
    u32 result = (u32)value;
    return result;
}