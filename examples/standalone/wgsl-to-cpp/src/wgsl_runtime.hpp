// Minimal WGSL runtime for C++ code generation
// Provides basic types and operations for translated shaders

#pragma once

#include <cstdint>
#include <cmath>
#include <array>
#include <algorithm>

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
