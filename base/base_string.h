#ifndef BASE_STRING_H
#define BASE_STRING_H

// #include "string.h"

typedef struct String8 String8;
struct String8 {
    u8 *str;
    u64 size;
};

u64 calc_cstring_length(char *cstr);

String8 str8(u8 *str, u64 size);

#define str8_cstring(s)  str8((u8 *)(s), calc_cstring_length(s))
#define str8_lit(s)      (String8){(u8 *)(s), sizeof(s) - 1}
#define str8_lit_comp(s) {(u8 *)(s), sizeof(s) - 1}
#define str8_varg(s)     (int)((s).size), ((s).str)

b32 str8_compare(const String8 str1, const String8 str2);

String8 str8_last_slash_prefix(String8 string);
String8 str8_last_slash_suffix(String8 string);

String8 str8_concat(Arena *arena, String8 str1, String8 str2);
#endif // BASE_STRING_H