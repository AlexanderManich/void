#ifndef BASE_CORE_H
#define BASE_CORE_H

#include <stdint.h>
#include <string.h>

#if SLOW
#define assert(expr) if (!(expr)) {*(int *)0 = 0;}
#endif // SLOW

typedef int8_t   i8;
typedef int16_t  i16;
typedef int32_t  i32;
typedef int64_t  i64;
typedef uint8_t  u8;
typedef uint16_t u16;
typedef uint32_t u32;
typedef uint64_t u64;
typedef float    f32;
typedef double   f64;

static u32 max_u32 = 0xffffffff;

u32 safe_u64_to_u32(u64 value);

typedef int32_t b32;
#define false 0
#define true  1

#define KB(n) ((n)     * 1024LL)
#define MB(n) ((KB(n)) * 1024LL)
#define GB(n) ((MB(n)) * 1024LL)
#define TB(n) ((GB(n)) * 1024LL)

#define memory_zero(s, n) (memset(s, 0, n))
#define array_count(arr) ((sizeof(arr)) / (sizeof((arr[0]))))

#define align_pow2(x,b) (((x) + (b) - 1) & (~((b) - 1)))

#endif // BASE_CORE_H