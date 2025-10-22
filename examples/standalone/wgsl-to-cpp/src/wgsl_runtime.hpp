// Minimal WGSL runtime for C++ code generation
// Provides basic types and operations for translated shaders

#pragma once

#include <algorithm>
#include <array>
#include <cmath>
#include <cstdint>
#include <cstring>

// Vector types
template<typename T>
struct vec2 {
    T x, y;

    vec2() : x(0), y(0) {}
    vec2(T v) : x(v), y(v) {}
    vec2(T x, T y) : x(x), y(y) {}

    T& operator[](size_t i) { return i == 0 ? x : y; }
    const T& operator[](size_t i) const { return i == 0 ? x : y; }
};

template<typename T>
struct vec3 {
    T x, y, z;

    vec3() : x(0), y(0), z(0) {}
    vec3(T v) : x(v), y(v), z(v) {}
    vec3(T x, T y, T z) : x(x), y(y), z(z) {}

    T& operator[](size_t i) { return i == 0 ? x : (i == 1 ? y : z); }
    const T& operator[](size_t i) const { return i == 0 ? x : (i == 1 ? y : z); }
};

template<typename T>
struct vec4 {
    T x, y, z, w;

    vec4() : x(0), y(0), z(0), w(0) {}
    vec4(T v) : x(v), y(v), z(v), w(v) {}
    vec4(T x, T y, T z, T w) : x(x), y(y), z(z), w(w) {}

    T& operator[](size_t i) {
        return i == 0 ? x : (i == 1 ? y : (i == 2 ? z : w));
    }
    const T& operator[](size_t i) const {
        return i == 0 ? x : (i == 1 ? y : (i == 2 ? z : w));
    }
};

// Vector operators
template<typename T>
vec2<T> operator+(const vec2<T>& a, const vec2<T>& b) {
    return vec2<T>(a.x + b.x, a.y + b.y);
}

template<typename T>
vec3<T> operator+(const vec3<T>& a, const vec3<T>& b) {
    return vec3<T>(a.x + b.x, a.y + b.y, a.z + b.z);
}

template<typename T>
vec4<T> operator+(const vec4<T>& a, const vec4<T>& b) {
    return vec4<T>(a.x + b.x, a.y + b.y, a.z + b.z, a.w + b.w);
}

template<typename T>
vec2<T> operator-(const vec2<T>& a, const vec2<T>& b) {
    return vec2<T>(a.x - b.x, a.y - b.y);
}

template<typename T>
vec3<T> operator-(const vec3<T>& a, const vec3<T>& b) {
    return vec3<T>(a.x - b.x, a.y - b.y, a.z - b.z);
}

template<typename T>
vec4<T> operator-(const vec4<T>& a, const vec4<T>& b) {
    return vec4<T>(a.x - b.x, a.y - b.y, a.z - b.z, a.w - b.w);
}

template<typename T>
vec2<T> operator-(const vec2<T>& v) {
    return vec2<T>(-v.x, -v.y);
}

template<typename T>
vec3<T> operator-(const vec3<T>& v) {
    return vec3<T>(-v.x, -v.y, -v.z);
}

template<typename T>
vec4<T> operator-(const vec4<T>& v) {
    return vec4<T>(-v.x, -v.y, -v.z, -v.w);
}

template<typename T>
vec2<T> operator*(const vec2<T>& a, T s) {
    return vec2<T>(a.x * s, a.y * s);
}

template<typename T>
vec3<T> operator*(const vec3<T>& a, T s) {
    return vec3<T>(a.x * s, a.y * s, a.z * s);
}

template<typename T>
vec4<T> operator*(const vec4<T>& a, T s) {
    return vec4<T>(a.x * s, a.y * s, a.z * s, a.w * s);
}

template<typename T>
vec2<T> operator*(T s, const vec2<T>& a) {
    return a * s;
}

template<typename T>
vec3<T> operator*(T s, const vec3<T>& a) {
    return a * s;
}

template<typename T>
vec4<T> operator*(T s, const vec4<T>& a) {
    return a * s;
}

template<typename T>
vec2<T> operator/(const vec2<T>& a, T s) {
    return vec2<T>(a.x / s, a.y / s);
}

template<typename T>
vec3<T> operator/(const vec3<T>& a, T s) {
    return vec3<T>(a.x / s, a.y / s, a.z / s);
}

template<typename T>
vec4<T> operator/(const vec4<T>& a, T s) {
    return vec4<T>(a.x / s, a.y / s, a.z / s, a.w / s);
}

template<typename T>
vec2<T> operator/(const vec2<T>& a, const vec2<T>& b) {
    return vec2<T>(a.x / b.x, a.y / b.y);
}

template<typename T>
vec3<T> operator/(const vec3<T>& a, const vec3<T>& b) {
    return vec3<T>(a.x / b.x, a.y / b.y, a.z / b.z);
}

template<typename T>
vec4<T> operator/(const vec4<T>& a, const vec4<T>& b) {
    return vec4<T>(a.x / b.x, a.y / b.y, a.z / b.z, a.w / b.w);
}

// Matrix types (column-major like WGSL)
template<typename T>
struct mat2x2 {
    vec2<T> cols[2];

    mat2x2() {}
    mat2x2(vec2<T> c0, vec2<T> c1) : cols{c0, c1} {}

    vec2<T>& operator[](size_t i) { return cols[i]; }
    const vec2<T>& operator[](size_t i) const { return cols[i]; }
};

template<typename T>
struct mat3x3 {
    vec3<T> cols[3];

    mat3x3() {}
    mat3x3(vec3<T> c0, vec3<T> c1, vec3<T> c2) : cols{c0, c1, c2} {}

    vec3<T>& operator[](size_t i) { return cols[i]; }
    const vec3<T>& operator[](size_t i) const { return cols[i]; }
};

template<typename T>
struct mat4x4 {
    vec4<T> cols[4];

    mat4x4() {}
    mat4x4(vec4<T> c0, vec4<T> c1, vec4<T> c2, vec4<T> c3) : cols{c0, c1, c2, c3} {}

    vec4<T>& operator[](size_t i) { return cols[i]; }
    const vec4<T>& operator[](size_t i) const { return cols[i]; }
};

// Common matrix sizes
template<typename T>
struct mat3x2 {
    vec2<T> cols[3];
    vec2<T>& operator[](size_t i) { return cols[i]; }
    const vec2<T>& operator[](size_t i) const { return cols[i]; }
};

template<typename T>
struct mat4x2 {
    vec2<T> cols[4];
    vec2<T>& operator[](size_t i) { return cols[i]; }
    const vec2<T>& operator[](size_t i) const { return cols[i]; }
};

template<typename T>
struct mat2x3 {
    vec3<T> cols[2];
    vec3<T>& operator[](size_t i) { return cols[i]; }
    const vec3<T>& operator[](size_t i) const { return cols[i]; }
};

template<typename T>
struct mat4x3 {
    vec3<T> cols[4];
    vec3<T>& operator[](size_t i) { return cols[i]; }
    const vec3<T>& operator[](size_t i) const { return cols[i]; }
};

template<typename T>
struct mat2x4 {
    vec4<T> cols[2];
    vec4<T>& operator[](size_t i) { return cols[i]; }
    const vec4<T>& operator[](size_t i) const { return cols[i]; }
};

template<typename T>
struct mat3x4 {
    vec4<T> cols[3];
    vec4<T>& operator[](size_t i) { return cols[i]; }
    const vec4<T>& operator[](size_t i) const { return cols[i]; }
};

// WGSL builtin functions

// abs() - absolute value for vectors
// Note: For scalar abs(), std::abs() is used directly (already available in <cmath>)
template<typename T>
inline vec2<T> abs(const vec2<T>& v) {
    return vec2<T>(std::abs(v.x), std::abs(v.y));
}

template<typename T>
inline vec3<T> abs(const vec3<T>& v) {
    return vec3<T>(std::abs(v.x), std::abs(v.y), std::abs(v.z));
}

template<typename T>
inline vec4<T> abs(const vec4<T>& v) {
    return vec4<T>(std::abs(v.x), std::abs(v.y), std::abs(v.z), std::abs(v.w));
}

// select() - ternary conditional: select(false_value, true_value, condition)
// In WGSL: select(a, b, cond) returns b if cond is true, otherwise a
template<typename T>
inline T select(const T& false_val, const T& true_val, bool cond) {
    return cond ? true_val : false_val;
}

// Vector variants
template<typename T>
inline vec2<T> select(const vec2<T>& false_val, const vec2<T>& true_val, bool cond) {
    return cond ? true_val : false_val;
}

template<typename T>
inline vec3<T> select(const vec3<T>& false_val, const vec3<T>& true_val, bool cond) {
    return cond ? true_val : false_val;
}

template<typename T>
inline vec4<T> select(const vec4<T>& false_val, const vec4<T>& true_val, bool cond) {
    return cond ? true_val : false_val;
}

// Additional helpers matching WGSL builtins

template<typename T>
inline T clamp(T x, T min_val, T max_val) {
    return std::clamp(x, min_val, max_val);
}

template<typename T>
inline vec2<T> clamp(const vec2<T>& x, const vec2<T>& min_val, const vec2<T>& max_val) {
    return vec2<T>(
        std::clamp(x.x, min_val.x, max_val.x),
        std::clamp(x.y, min_val.y, max_val.y)
    );
}

template<typename T>
inline vec3<T> clamp(const vec3<T>& x, const vec3<T>& min_val, const vec3<T>& max_val) {
    return vec3<T>(
        std::clamp(x.x, min_val.x, max_val.x),
        std::clamp(x.y, min_val.y, max_val.y),
        std::clamp(x.z, min_val.z, max_val.z)
    );
}

template<typename T>
inline vec4<T> clamp(const vec4<T>& x, const vec4<T>& min_val, const vec4<T>& max_val) {
    return vec4<T>(
        std::clamp(x.x, min_val.x, max_val.x),
        std::clamp(x.y, min_val.y, max_val.y),
        std::clamp(x.z, min_val.z, max_val.z),
        std::clamp(x.w, min_val.w, max_val.w)
    );
}

template<typename T>
inline T mix(const T& a, const T& b, const T& t) {
    return a * (static_cast<T>(1) - t) + b * t;
}

template<typename T>
inline vec2<T> mix(const vec2<T>& a, const vec2<T>& b, T t) {
    return a * (static_cast<T>(1) - t) + b * t;
}

template<typename T>
inline vec3<T> mix(const vec3<T>& a, const vec3<T>& b, T t) {
    return a * (static_cast<T>(1) - t) + b * t;
}

template<typename T>
inline vec4<T> mix(const vec4<T>& a, const vec4<T>& b, T t) {
    return a * (static_cast<T>(1) - t) + b * t;
}

template<typename T>
inline T smoothstep(T edge0, T edge1, T x) {
    T t = clamp((x - edge0) / (edge1 - edge0), static_cast<T>(0), static_cast<T>(1));
    return t * t * (static_cast<T>(3) - static_cast<T>(2) * t);
}

template<typename T>
inline T dot(const vec2<T>& a, const vec2<T>& b) {
    return a.x * b.x + a.y * b.y;
}

template<typename T>
inline T dot(const vec3<T>& a, const vec3<T>& b) {
    return a.x * b.x + a.y * b.y + a.z * b.z;
}

template<typename T>
inline T dot(const vec4<T>& a, const vec4<T>& b) {
    return a.x * b.x + a.y * b.y + a.z * b.z + a.w * b.w;
}

template<typename T>
inline T length(const vec2<T>& v) {
    return static_cast<T>(std::sqrt(dot(v, v)));
}

template<typename T>
inline T length(const vec3<T>& v) {
    return static_cast<T>(std::sqrt(dot(v, v)));
}

template<typename T>
inline T length(const vec4<T>& v) {
    return static_cast<T>(std::sqrt(dot(v, v)));
}

inline vec3<float> pow(const vec3<float>& base, const vec3<float>& exp) {
    return vec3<float>(
        std::pow(base.x, exp.x),
        std::pow(base.y, exp.y),
        std::pow(base.z, exp.z)
    );
}

inline float fwidth(float) {
    return 0.0f;
}

template<typename Dest, typename Src>
inline Dest bitcast(const Src& value) {
    static_assert(sizeof(Dest) == sizeof(Src), "bitcast requires same size");
    Dest result;
    std::memcpy(&result, &value, sizeof(Dest));
    return result;
}

// Component-wise select for vector conditions
template<typename T>
inline vec2<T> select(const vec2<T>& false_val, const vec2<T>& true_val, const vec2<bool>& cond) {
    return vec2<T>(
        cond.x ? true_val.x : false_val.x,
        cond.y ? true_val.y : false_val.y
    );
}

template<typename T>
inline vec3<T> select(const vec3<T>& false_val, const vec3<T>& true_val, const vec3<bool>& cond) {
    return vec3<T>(
        cond.x ? true_val.x : false_val.x,
        cond.y ? true_val.y : false_val.y,
        cond.z ? true_val.z : false_val.z
    );
}

template<typename T>
inline vec4<T> select(const vec4<T>& false_val, const vec4<T>& true_val, const vec4<bool>& cond) {
    return vec4<T>(
        cond.x ? true_val.x : false_val.x,
        cond.y ? true_val.y : false_val.y,
        cond.z ? true_val.z : false_val.z,
        cond.w ? true_val.w : false_val.w
    );
}
