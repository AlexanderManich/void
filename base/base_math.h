#include "math.h"

f32 power(f32 base, f32 exponent) {
    f32 result = base;
    for (u32 i = 0; i < exponent - 1; i++) {
        result *= base;
    } 

    return result;
}



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

inline Vec2 operator+(Vec2 a, Vec2 b) {
    Vec2 result;

    result.x = a.x + b.x;
    result.y = a.y + b.y;

    return result;
}

inline Vec2 operator-(Vec2 a, Vec2 b) {
    Vec2 result;

    result.x = a.x - b.x;
    result.y = a.y - b.y;

    return result;
}

inline Vec2 operator*(Vec2 a, f32 b) {
    Vec2 result;

    result.x = a.x * b;
    result.y = a.y * b;

    return result;
}

inline Vec2 operator*(f32 a, Vec2 b) {
    Vec2 result;

    result.x = a * b.x;
    result.y = a * b.y;

    return result;
}

inline Vec2 operator-(Vec2 a) {
    Vec2 result;

    result.x = -a.x;
    result.y = -a.y;

    return result;
}

inline f32 vec2_dot(Vec2 a, Vec2 b) {
    f32 result;

    result = a.x * b.x + a.y * b.y;

    return result;
}

inline f32 square(f32 a) {
    return a * a;
}

typedef struct Vec3 Vec3;
struct Vec3 {
    f32 x;
    f32 y;
    f32 z;
};

inline Vec3 vec3(f32 x, f32 y, f32 z) {
    Vec3 result;

    result.x = x;
    result.y = y;
    result.z = z;
    
    return result;
}

inline f32 vec3_len(Vec3 a) {
    f32 result;

    f32 sum = power(a.x, 2) + power(a.y, 2) + power(a.z, 2);
    result = sqrt(sum);

    return result;
}

inline f32 vec3_dot(Vec3 a, Vec3 b) {
    f32 result;
    
    result = a.x * b.x + a.y * b.y + a.z * b.z; 

    return result;
};

inline f32 vec3_lensq(Vec3 a) {
    f32 result;

    result = vec3_dot(a, a);

    return result;
}

inline f32 vec3_cross_lensq(Vec3 a, Vec3 b) {
    // ||a x b||^2 = ||a||^2 * ||b||^2 - (a dot b)^2
    return vec3_lensq(a) * vec3_lensq(b) - power(vec3_dot(a, b), 2);
}

inline Vec3 vec3_normalize2(Vec3 a) {
    Vec3 result;

    Vec3 a_sq = vec3(a.x * a.x, a.y * a.y, a.z * a.z);
    f32 lensq = vec3_lensq(a);

    result.x = a_sq.x / lensq;
    result.y = a_sq.y / lensq;
    result.z = a_sq.z / lensq;

    return result;
}

inline Vec3 vec3_normalize(Vec3 a) {
    Vec3 result = a;

    f32 len = vec3_len(result);
    result.x /= len; 
    result.y /= len;
    result.z /= len;

    return result;
}

inline Vec3 operator-(Vec3 a, Vec3 b) {
    Vec3 result;

    result.x = b.x - a.x;
    result.y = b.y - a.y;
    result.z = b.z - a.z;

    return result;
}

inline b32 operator!=(Vec3 a, Vec3 b) {
    return a.x != b.x || a.y != b.y || a.z != b.z;
}

typedef struct Mat3x3 Mat3x3;
struct Mat3x3 {
    f32 m[3][3];
};

inline Vec3 operator*(Vec3 vec, Mat3x3 mat) {
    Vec3 result = {0};

    result.x = vec.x * mat.m[0][0] + vec.y * mat.m[1][0] + vec.z * mat.m[2][0];
    result.y = vec.x * mat.m[0][1] + vec.y * mat.m[1][1] + vec.z * mat.m[2][1];
    result.z = vec.x * mat.m[0][2] + vec.y * mat.m[1][2] + vec.z * mat.m[2][2];

    return result;
}

inline Vec3 operator*(Mat3x3 mat, Vec3 vec) {
    Vec3 result = {0};

    result.x = vec.x * mat.m[0][0] + vec.y * mat.m[1][0] + vec.z * mat.m[2][0];
    result.y = vec.x * mat.m[0][1] + vec.y * mat.m[1][1] + vec.z * mat.m[2][1];
    result.z = vec.x * mat.m[0][2] + vec.y * mat.m[1][2] + vec.z * mat.m[2][2];

    return result;
};