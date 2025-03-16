typedef struct Vec2 Vec2;
struct Vec2 {
    f32 x;
    f32 y;
};

inline Vec2 vec2(f32 x, f32 y) {
    Vec2 result;

    result.x = x;
    result.y = y;

    return result;
}

inline Vec2 vec2_add(Vec2 a, Vec2 b) {
    Vec2 result;

    result.x = a.x + b.x;
    result.y = a.y + b.y;

    return result;
}

inline Vec2 vec2_sub(Vec2 a, Vec2 b) {
    Vec2 result;

    result.x = a.x - b.x;
    result.y = a.y - b.y;

    return result;
}

inline Vec2 vec2_mul(Vec2 a, f32 b) {
    Vec2 result;

    result.x = a.x * b;
    result.y = a.y * b;

    return result;
}

inline f32 vec2_dot(Vec2 a, Vec2 b) {
    f32 result;

    result = a.x * b.x + a.y * b.y;

    return result;
}