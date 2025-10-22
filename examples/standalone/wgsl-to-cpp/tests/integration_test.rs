use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};
use wgsl_to_cpp::{compile_cpp, translate_wgsl_to_cpp};

static TEST_COUNTER: AtomicU32 = AtomicU32::new(0);

/// Check if debug mode is enabled
fn is_debug_mode() -> bool {
    std::env::var("WGSL_TO_CPP_DEBUG").is_ok()
}

/// Generate a unique ID for test binaries
fn unique_test_id() -> String {
    let counter = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
    let thread_id = format!("{:?}", thread::current().id());
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    format!(
        "{}_{}_{}",
        counter,
        thread_id.replace("ThreadId(", "").replace(")", ""),
        timestamp
    )
}

/// Get the path to the runtime header for tests
fn get_runtime_header_path() -> PathBuf {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    PathBuf::from(manifest_dir).join("src/wgsl_runtime.hpp")
}

/// Check if clang++ is available
fn is_clang_available() -> bool {
    Command::new("clang++").arg("--version").output().is_ok()
}

#[test]
fn test_basic_compute_shader() {
    let wgsl_source = r#"
@compute @workgroup_size(1)
fn main() {
}
"#;

    // Translate to C++
    let result = translate_wgsl_to_cpp(wgsl_source);

    // Verify it succeeded
    assert!(result.is_ok(), "Translation failed: {:?}", result.err());

    // Verify we got C++ output
    let cpp_output = result.unwrap();
    assert!(cpp_output.contains("#include"));
    assert!(cpp_output.contains("void main"));
}

#[test]
fn test_invalid_wgsl() {
    let invalid_wgsl = r#"
this is not valid WGSL code
"#;

    // Attempt to translate invalid WGSL
    let result = translate_wgsl_to_cpp(invalid_wgsl);

    // Verify it failed (as expected)
    assert!(result.is_err(), "Should have failed on invalid WGSL");
}

#[test]
fn test_compute_with_return() {
    let wgsl_source = r#"
@compute @workgroup_size(8)
fn main() {
    return;
}
"#;

    // Translate to C++
    let result = translate_wgsl_to_cpp(wgsl_source);

    // Verify it succeeded
    assert!(result.is_ok(), "Translation failed: {:?}", result.err());

    let cpp_output = result.unwrap();
    assert!(cpp_output.contains("void main"));
    assert!(cpp_output.contains("return"));
}

#[test]
fn test_compile_and_run_basic_shader() {
    // Skip if clang++ not available
    if !is_clang_available() {
        eprintln!("Skipping test_compile_and_run_basic_shader: clang++ not available");
        return;
    }

    let wgsl_source = r#"
@compute @workgroup_size(1)
fn compute_shader() {
    return;
}
"#;

    // Translate to C++
    let cpp_code = translate_wgsl_to_cpp(wgsl_source).expect("Failed to translate WGSL");

    // Wrap with main() function
    let full_cpp = format!(
        "{}\n\nint main() {{\n    compute_shader();\n    return 0;\n}}\n",
        cpp_code
    );

    // Compile
    let temp_dir = std::env::temp_dir();
    let binary_path = temp_dir.join(format!("test_basic_shader_{}", unique_test_id()));
    let runtime_header = get_runtime_header_path();

    compile_cpp(&full_cpp, &binary_path, &runtime_header).expect("Failed to compile C++");

    // Run the compiled binary
    if is_debug_mode() {
        eprintln!("[DEBUG] Running binary: {}", binary_path.display());
    }

    let output = Command::new(&binary_path)
        .output()
        .expect("Failed to execute binary");

    // Verify it ran successfully
    assert!(
        output.status.success(),
        "Binary execution failed with status: {:?}\nstderr: {}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );

    // Cleanup (skip in debug mode)
    if !is_debug_mode() {
        fs::remove_file(&binary_path).ok();
    }
}

#[test]
fn test_compile_shader_with_output() {
    // Skip if clang++ not available
    if !is_clang_available() {
        eprintln!("Skipping test_compile_shader_with_output: clang++ not available");
        return;
    }

    let wgsl_source = r#"
@compute @workgroup_size(8, 8, 1)
fn process() {
}
"#;

    // Translate to C++
    let cpp_code = translate_wgsl_to_cpp(wgsl_source).expect("Failed to translate WGSL");

    // Wrap with main() that prints output
    let full_cpp = format!(
        "{}\n\n#include <iostream>\n\nint main() {{\n    std::cout << \"Starting shader...\" << std::endl;\n    process();\n    std::cout << \"Shader completed!\" << std::endl;\n    return 0;\n}}\n",
        cpp_code
    );

    // Compile
    let temp_dir = std::env::temp_dir();
    // Use a unique name to avoid conflicts with other tests
    let binary_path = temp_dir.join(format!("test_shader_output_{}", unique_test_id()));
    let runtime_header = get_runtime_header_path();

    compile_cpp(&full_cpp, &binary_path, &runtime_header).expect("Failed to compile C++");

    // Run and capture output
    if is_debug_mode() {
        eprintln!("[DEBUG] Running binary: {}", binary_path.display());
    }

    let output = Command::new(&binary_path)
        .output()
        .expect("Failed to execute binary");

    // Verify execution
    assert!(
        output.status.success(),
        "Binary execution failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Verify output
    let stdout = String::from_utf8_lossy(&output.stdout);
    if is_debug_mode() {
        eprintln!("[DEBUG] Binary output:\n{}", stdout);
    }
    assert!(stdout.contains("Starting shader..."));
    assert!(stdout.contains("Shader completed!"));

    // Cleanup (skip in debug mode)
    if !is_debug_mode() {
        fs::remove_file(&binary_path).ok();
    }
}

#[test]
fn test_compile_multiple_shaders() {
    // Skip if clang++ not available
    if !is_clang_available() {
        eprintln!("Skipping test_compile_multiple_shaders: clang++ not available");
        return;
    }

    // Test multiple simple shaders to ensure they all compile
    let shaders = vec![
        r#"@compute @workgroup_size(1) fn shader1() {}"#,
        r#"@compute @workgroup_size(64) fn shader2() { return; }"#,
        r#"@compute @workgroup_size(8, 8, 1) fn shader3() {}"#,
    ];

    let temp_dir = std::env::temp_dir();
    let runtime_header = get_runtime_header_path();

    for (i, wgsl) in shaders.iter().enumerate() {
        let cpp_code = translate_wgsl_to_cpp(wgsl)
            .unwrap_or_else(|e| panic!("Shader {} failed to translate: {}", i, e));

        let full_cpp = format!("{}\n\nint main() {{ return 0; }}\n", cpp_code);
        let binary_path = temp_dir.join(format!("test_shader_{}_{}", i, unique_test_id()));

        compile_cpp(&full_cpp, &binary_path, &runtime_header)
            .unwrap_or_else(|e| panic!("Shader {} failed to compile: {}", i, e));

        if is_debug_mode() {
            eprintln!("[DEBUG] Running binary {}: {}", i, binary_path.display());
        }

        let output = Command::new(&binary_path)
            .output()
            .unwrap_or_else(|e| panic!("Shader {} failed to execute: {}", i, e));

        assert!(output.status.success(), "Shader {} execution failed", i);

        // Cleanup (skip in debug mode)
        if !is_debug_mode() {
            fs::remove_file(&binary_path).ok();
        }
    }
}

#[test]
fn test_bisect5_newton_method() {
    if !is_clang_available() {
        eprintln!("Skipping test_bisect5_newton_method: clang++ not available");
        return;
    }

    let wgsl_source = r#"
const eps: f32 = 1e-6;

// Newton bisection for 5th degree polynomial
// Finds root of: a*x^5 + b*x^4 + c*x^3 + d*x^2 + e*x + f = 0
fn bisect5(a: f32, b: f32, c: f32, d: f32, e: f32, f: f32, t_in: vec2<f32>, v: vec2<f32>) -> f32 {
    var t = t_in;
    var x = (t.x + t.y) * 0.5;
    let s = select(-1.0, 1.0, v.x < v.y);

    for (var i = 0; i < 32; i++) {
        // Evaluate polynomial and derivative using Horner's method
        var y = a * x + b;
        var q = a * x + y;
        y = y * x + c;
        q = q * x + y;
        y = y * x + d;
        q = q * x + y;
        y = y * x + e;
        q = q * x + y;
        y = y * x + f;

        t = select(vec2(t.x, x), vec2(x, t.y), s * y < 0.0);
        var next = x - y / q;
        next = select((t.x + t.y) * 0.5, next, next >= t.x && next <= t.y);
        if (abs(next - x) < eps) {
            return next;
        }
        x = next;
    }
    return x;
}

@compute @workgroup_size(1)
fn main() {
    // Test polynomial: x^5 - 3x^3 + 2x - 1 = 0
    // Coefficients: a=1, b=0, c=-3, d=0, e=2, f=-1
    let root = bisect5(1.0, 0.0, -3.0, 0.0, 2.0, -1.0, vec2(0.0, 1.0), vec2(0.0, 1.0));
}
"#;

    let cpp_code = translate_wgsl_to_cpp(wgsl_source).expect("Failed to translate bisect5 WGSL");

    assert!(
        !cpp_code.contains("/* expr */") && !cpp_code.contains("// Unsupported statement"),
        "Generated C++ still contains unsupported placeholders:\n{}",
        cpp_code
    );

    let bisect_function_name = cpp_code
        .lines()
        .find_map(|line| {
            let trimmed = line.trim_start();
            if let Some(rest) = trimmed.strip_prefix("float ") {
                Some(rest.split('(').next().unwrap_or("").trim().to_string())
            } else {
                None
            }
        })
        .expect("bisect5 function not found in generated C++");

    let renamed_cpp = cpp_code.replacen("void main()", "void shader_main()", 1);

    let harness_cpp = format!(
        "{original}

#include <iostream>
#include <iomanip>

int main() {{
    shader_main();
    float root = {fn_name}(1.0f, 0.0f, -3.0f, 0.0f, 2.0f, -1.0f, vec2<float>(0.0f, 1.0f), vec2<float>(0.0f, 1.0f));
    std::cout << std::fixed << std::setprecision(6) << root << std::endl;
    return 0;
}}
",
        original = renamed_cpp,
        fn_name = bisect_function_name
    );

    if is_debug_mode() {
        eprintln!("[DEBUG] Generated bisect5 C++:\n{}", harness_cpp);
    }

    fn reference_bisect5(
        a: f32,
        b: f32,
        c: f32,
        d: f32,
        e: f32,
        f: f32,
        t0: f32,
        t1: f32,
        v0: f32,
        v1: f32,
    ) -> f32 {
        let mut lower = t0;
        let mut upper = t1;
        let mut x = 0.5 * (lower + upper);
        let s = if v0 < v1 { 1.0 } else { -1.0 };
        let eps = 1e-6f32;

        for _ in 0..32 {
            let mut y = a * x + b;
            let mut q = a * x + y;
            y = y * x + c;
            q = q * x + y;
            y = y * x + d;
            q = q * x + y;
            y = y * x + e;
            q = q * x + y;
            y = y * x + f;

            if s * y < 0.0 {
                lower = x;
            } else {
                upper = x;
            }

            let mut next = x - y / q;
            if !(next >= lower && next <= upper) {
                next = 0.5 * (lower + upper);
            }

            if (next - x).abs() < eps {
                return next;
            }

            x = next;
        }

        x
    }

    let temp_dir = std::env::temp_dir();
    let binary_path = temp_dir.join(format!("test_bisect5_full_{}", unique_test_id()));
    let runtime_header = get_runtime_header_path();

    compile_cpp(&harness_cpp, &binary_path, &runtime_header)
        .expect("Failed to compile bisect5 C++");

    if is_debug_mode() {
        eprintln!("[DEBUG] Running bisect5 binary: {}", binary_path.display());
    }

    let output = Command::new(&binary_path)
        .output()
        .expect("Failed to execute bisect5 binary");

    assert!(
        output.status.success(),
        "bisect5 execution failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let root: f32 = stdout.trim().parse().expect("Failed to parse bisect5 root");
    let expected = reference_bisect5(1.0, 0.0, -3.0, 0.0, 2.0, -1.0, 0.0, 1.0, 0.0, 1.0);
    let error = (root - expected).abs();
    assert!(
        error < 0.001,
        "bisect5 root inaccurate: got {}, expected {} (error = {})",
        root,
        expected,
        error
    );

    if !is_debug_mode() {
        let _ = fs::remove_file(&binary_path);
    }
}

#[test]
fn test_bezier_distance_accuracy() {
    if !is_clang_available() {
        eprintln!("Skipping test_bezier_distance_accuracy: clang++ not available");
        return;
    }

    let wgsl_source = r#"
struct Uniforms {
    resolution: vec2<f32>,
    p0: vec2<f32>,
    p1: vec2<f32>,
    p2: vec2<f32>,
    p3: vec2<f32>,
}

@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

fn sdSegment(p: vec2<f32>, a: vec2<f32>, b: vec2<f32>) -> f32 {
    let pa = p - a;
    let ba = b - a;
    let h = clamp(dot(pa, ba) / dot(ba, ba), 0.0, 1.0);
    return length(pa - ba * h);
}

fn sdPoint(p: vec2<f32>, pt: vec2<f32>) -> f32 {
    return length(p - pt);
}

const eps: f32 = 0.0001;

// Return true if x is not a NaN nor an infinite
fn wg_isfinite(x: f32) -> bool {
    return (bitcast<u32>(x) & 0x7f800000u) != 0x7f800000u;
}

fn poly5(a: f32, b: f32, c: f32, d: f32, e: f32, f: f32, t: f32) -> f32 {
    return ((((a * t + b) * t + c) * t + d) * t + e) * t + f;
}

// Newton bisection for 5th degree polynomial
fn bisect5(a: f32, b: f32, c: f32, d: f32, e: f32, f: f32, t_in: vec2<f32>, v: vec2<f32>) -> f32 {
    var t = t_in;
    var x = (t.x + t.y) * 0.5;
    let s = select(-1.0, 1.0, v.x < v.y);

    for (var i = 0; i < 32; i++) {
        // Evaluate polynomial and derivative using Horner's method
        var y = a * x + b;
        var q = a * x + y;
        y = y * x + c;
        q = q * x + y;
        y = y * x + d;
        q = q * x + y;
        y = y * x + e;
        q = q * x + y;
        y = y * x + f;

        t = select(vec2(t.x, x), vec2(x, t.y), s * y < 0.0);
        var next = x - y / q;
        next = select((t.x + t.y) * 0.5, next, next >= t.x && next <= t.y);
        if (abs(next - x) < eps) {
            return next;
        }
        x = next;
    }
    return x;
}

// Quadratic root finder: solve ax²+bx+c=0 (clamped to [0,1])
fn root_find2(a: f32, b: f32, c: f32) -> array<f32, 6> {
    var result: array<f32, 6>;
    result[5] = 0.0; // count

    let disc = b * b - 4.0 * a * c;
    if (disc < 0.0) {
        return result;
    }
    if (disc == 0.0) {
        let s = -0.5 * b / a;
        if (wg_isfinite(s)) {
            result[0] = s;
            result[5] = 1.0;
        }
        return result;
    }

    let h = sqrt(disc);
    let q = -0.5 * (b + select(-h, h, b > 0.0));
    var v = vec2(q / a, c / q);
    if (v.x > v.y) {
        v = v.yx;
    }

    var count = 0;
    if (wg_isfinite(v.x) && v.x >= 0.0 && v.x <= 1.0) {
        result[count] = v.x;
        count++;
    }
    if (wg_isfinite(v.y) && v.y >= 0.0 && v.y <= 1.0) {
        result[count] = v.y;
        count++;
    }
    result[5] = f32(count);
    return result;
}

// Find roots using bisection on quintic
fn cy_find5(r4: array<f32, 6>, a: f32, b: f32, c: f32, d: f32, e: f32, f: f32) -> array<f32, 6> {
    var result: array<f32, 6>;
    var count = 0;
    let n = i32(r4[5]);

    var px = 0.0;
    var py = poly5(a, b, c, d, e, f, 0.0);

    for (var i = 0; i <= n; i++) {
        let x = select(r4[i], 1.0, i == n);
        let y = poly5(a, b, c, d, e, f, x);

        if (py * y <= 0.0 && !(py * y == 0.0)) {
            let v = bisect5(a, b, c, d, e, f, vec2(px, x), vec2(py, y));
            result[count] = v;
            count++;
        }
        px = x;
        py = y;
    }

    result[5] = f32(count);
    return result;
}

// Hierarchical root finder for 5th degree polynomial
fn root_find5(da: f32, db: f32, dc: f32, dd: f32, de: f32, df: f32) -> array<f32, 6> {
    // Degree 2
    let r2 = root_find2(10.0 * da, 4.0 * db, dc);

    // Degree 3
    let r3 = cy_find5(r2, 0.0, 0.0, 10.0 * da, 6.0 * db, 3.0 * dc, dd);

    // Degree 4
    let r4 = cy_find5(r3, 0.0, 5.0 * da, 4.0 * db, 3.0 * dc, dd + dd, de);

    // Degree 5
    let r = cy_find5(r4, da, db, dc, dd, de, df);

    return r;
}

fn dot2(v: vec2<f32>) -> f32 {
    return dot(v, v);
}

// Cubic Bezier distance function
fn bezier(p: vec2<f32>, p0: vec2<f32>, p1: vec2<f32>, p2: vec2<f32>, p3: vec2<f32>) -> f32 {
    // Start by testing distance to boundary points
    let dp0 = p0 - p;
    let dp3 = p3 - p;
    var dist = min(dot2(dp0), dot2(dp3));

    // Bezier cubic points to polynomial coefficients
    let a = -p0 + 3.0 * (p1 - p2) + p3;
    let b = 3.0 * (p0 - 2.0 * p1 + p2);
    let c = 3.0 * (p1 - p0);
    let d = p0;

    // Solve D'(t)=0 where D(t) is distance squared
    let dmp = d - p;
    let da = 3.0 * dot(a, a);
    let db = 5.0 * dot(a, b);
    let dc = 4.0 * dot(a, c) + 2.0 * dot(b, b);
    let dd = 3.0 * (dot(a, dmp) + dot(b, c));
    let de = 2.0 * dot(b, dmp) + dot(c, c);
    let df = dot(c, dmp);

    let roots = root_find5(da, db, dc, dd, de, df);
    let count = i32(roots[5]);

    for (var i = 0; i < count; i++) {
        let t = roots[i];
        let dp = ((a * t + b) * t + c) * t + dmp;
        dist = min(dist, dot2(dp));
    }

    return sqrt(dist);
}

// Distance field debug visualization (Inigo Quilez colorscheme)
fn df_debug(d: f32) -> vec3<f32> {
    var col = vec3<f32>(0.0, 0.2, 0.5);
    col *= 0.7 + 0.3 * cos(120.0 * abs(d));
    return mix(col, vec3<f32>(1.0), 1.0 - smoothstep(0.0, 0.02, abs(d)));
}

fn sat(x: f32) -> f32 {
    return clamp(x, 0.0, 1.0);
}

// Render control points and tangent segments
fn points_segments(c_in: vec3<f32>, p: vec2<f32>, p0: vec2<f32>, p1: vec2<f32>, p2: vec2<f32>, p3: vec2<f32>) -> vec3<f32> {
    var c = c_in;

    // Points
    let d0 = dot2(p - p0);
    let d1 = dot2(p - p1);
    let d2 = dot2(p - p2);
    let d3 = dot2(p - p3);
    let d = 0.02 - sqrt(min(min(min(d0, d1), d2), d3));
    c = mix(c, vec3<f32>(1.0, 0.5, 0.0), sat(0.5 + d / fwidth(d)));

    // Segments
    let s0 = sdSegment(p, p0, p1);
    let s1 = sdSegment(p, p2, p3);
    let s = 0.005 - min(s0, s1);
    c = mix(c, vec3<f32>(1.0, 0.5, 0.0), sat(0.5 + s / fwidth(s)) * 0.5);

    return c;
}

@vertex
fn vs_main(@builtin(vertex_index) in_vertex_index: u32) -> @builtin(position) vec4<f32> {
    let positions = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>( 3.0, -1.0),
        vec2<f32>(-1.0,  3.0)
    );
    return vec4<f32>(positions[in_vertex_index], 0.0, 1.0);
}

@fragment
fn fs_main(@builtin(position) pos: vec4<f32>) -> @location(0) vec4<f32> {
    let fragCoord = vec2<f32>(pos.x, uniforms.resolution.y - pos.y);
    var uv = (fragCoord * 2.0 - uniforms.resolution) / uniforms.resolution.y;
    let aspect = uniforms.resolution.x / uniforms.resolution.y;
    uv.x *= aspect;
    let scale = max(aspect, 1.0);
    uv /= scale;

    // Calculate distance to cubic Bezier curve
    let d = bezier(uv, uniforms.p0, uniforms.p1, uniforms.p2, uniforms.p3);

    // Apply distance field visualization
    var o = df_debug(d);

    // Overlay control points and tangent segments
    o = points_segments(o, uv, uniforms.p0, uniforms.p1, uniforms.p2, uniforms.p3);

    // Apply gamma correction
    return vec4<f32>(pow(o, vec3<f32>(1.0 / 2.2)), 1.0);
}
"#;

    let cpp_code = translate_wgsl_to_cpp(wgsl_source).expect("Failed to translate Bezier WGSL");

    let bezier_name = cpp_code
        .lines()
        .find_map(|line| {
            let trimmed = line.trim_start();
            if let Some(rest) = trimmed.strip_prefix("float ") {
                let name = rest.split('(').next()?.trim();
                if name.starts_with("bezier") {
                    return Some(name.to_string());
                }
            }
            None
        })
        .expect("bezier function not found");

    let harness = format!(
        r#"{cpp}

#include <iostream>
#include <iomanip>

struct TestCase {{
    const char* name;
    vec2<float> point;
    vec2<float> p0;
    vec2<float> p1;
    vec2<float> p2;
    vec2<float> p3;
}};

int main() {{
    const float K = 0.5522847498307935f;
    TestCase cases[] = {{
        {{"horizontal", vec2<float>(0.0f, 0.5f), vec2<float>(-1.0f, 0.0f), vec2<float>(-0.33333334f, 0.0f), vec2<float>(0.33333334f, 0.0f), vec2<float>(1.0f, 0.0f)}},
        {{"vertical", vec2<float>(0.5f, 0.0f), vec2<float>(0.0f, -1.0f), vec2<float>(0.0f, -0.33333334f), vec2<float>(0.0f, 0.33333334f), vec2<float>(0.0f, 1.0f)}},
        {{"diagonal", vec2<float>(0.0f, 1.0f), vec2<float>(-1.0f, -1.0f), vec2<float>(-0.33333334f, -0.33333334f), vec2<float>(0.33333334f, 0.33333334f), vec2<float>(1.0f, 1.0f)}},
        {{"circle_quarter", vec2<float>(0.0f, 0.0f), vec2<float>(1.0f, 0.0f), vec2<float>(1.0f, K), vec2<float>(K, 1.0f), vec2<float>(0.0f, 1.0f)}}
    }};

    for (const auto& tc : cases) {{
        float d = {func}(tc.point, tc.p0, tc.p1, tc.p2, tc.p3);
        std::cout << tc.name << " " << std::setprecision(10) << d << std::endl;
    }}
    return 0;
}}
"#,
        cpp = cpp_code,
        func = bezier_name
    );

    let temp_dir = std::env::temp_dir();
    let binary_path = temp_dir.join(format!("test_bezier_distance_{}", unique_test_id()));
    let runtime_header = get_runtime_header_path();

    compile_cpp(&harness, &binary_path, &runtime_header)
        .expect("Failed to compile bezier distance test C++");

    let output = Command::new(&binary_path)
        .output()
        .expect("Failed to execute bezier distance binary");

    assert!(
        output.status.success(),
        "bezier distance binary failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut results = std::collections::HashMap::new();
    for line in stdout.lines() {
        let mut parts = line.split_whitespace();
        if let (Some(name), Some(value_str)) = (parts.next(), parts.next()) {
            if let Ok(value) = value_str.parse::<f32>() {
                results.insert(name.to_string(), value);
            }
        }
    }

    let reference = [
        ("horizontal", 0.5_f32, 0.005_f32),
        ("vertical", 0.5_f32, 0.005_f32),
        ("diagonal", (1.0_f32 / std::f32::consts::SQRT_2), 0.01_f32),
        ("circle_quarter", 1.0_f32, 0.05_f32),
    ];

    for (name, expected, tolerance) in reference {
        let actual = results
            .get(name)
            .unwrap_or_else(|| panic!("Missing result for {}", name));
        let error = (actual - expected).abs();
        assert!(
            error <= tolerance,
            "{}: expected {}, got {} (error {})",
            name,
            expected,
            actual,
            error
        );
    }

    if !is_debug_mode() {
        let _ = fs::remove_file(&binary_path);
    }
}

#[test]
fn test_linear_equation_solver() {
    // Skip if clang++ not available
    if !is_clang_available() {
        eprintln!("Skipping test_linear_equation_solver: clang++ not available");
        return;
    }

    // MATHEMATICAL BACKGROUND:
    // Solve linear equation: ax + b = 0
    // Solution: x = -b/a
    //
    // This test translates a WGSL function that solves linear equations
    // and tests it with multiple coefficient pairs via command line arguments.

    let wgsl_source = r#"
fn solve_linear(a: f32, b: f32) -> f32 {
    return -b / a;
}

@compute @workgroup_size(1)
fn compute_main() {
}
"#;

    // Translate to C++
    let cpp_code =
        translate_wgsl_to_cpp(wgsl_source).expect("Failed to translate linear solver WGSL");

    // Wrap with main() that takes command line args and calls the WGSL function
    let full_cpp = format!(
        "{}

#include <iostream>
#include <cstdlib>
#include <cmath>

int main(int argc, char* argv[]) {{
    if (argc != 3) {{
        std::cerr << \"Usage: \" << argv[0] << \" <a> <b>\" << std::endl;
        return 1;
    }}

    float a = std::atof(argv[1]);
    float b = std::atof(argv[2]);

    // Call the WGSL-generated function
    float solution = solve_linear(a, b);

    // Output in parseable format
    std::cout << solution << std::endl;

    return 0;
}}
",
        cpp_code
    );

    // Compile once
    let temp_dir = std::env::temp_dir();
    let binary_path = temp_dir.join(format!("test_linear_solver_{}", unique_test_id()));
    let runtime_header = get_runtime_header_path();

    compile_cpp(&full_cpp, &binary_path, &runtime_header)
        .expect("Failed to compile linear solver C++");

    // Test cases: (a, b, expected_solution)
    // Solution: x = -b/a
    let test_cases = vec![
        (3.0, 6.0, -2.0),  // 3x + 6 = 0 → x = -6/3 = -2
        (5.0, 10.0, -2.0), // 5x + 10 = 0 → x = -10/5 = -2
        (2.0, -4.0, 2.0),  // 2x - 4 = 0 → x = -(-4)/2 = 2
        (-1.0, 3.0, 3.0),  // -x + 3 = 0 → x = -3/(-1) = 3
    ];

    for (a, b, expected) in test_cases {
        if is_debug_mode() {
            eprintln!(
                "[DEBUG] Testing: {}x + {} = 0, expected: {}",
                a, b, expected
            );
        }

        let output = Command::new(&binary_path)
            .arg(a.to_string())
            .arg(b.to_string())
            .output()
            .expect("Failed to execute linear solver binary");

        assert!(
            output.status.success(),
            "Linear solver failed for a={}, b={}: {}",
            a,
            b,
            String::from_utf8_lossy(&output.stderr)
        );

        let stdout = String::from_utf8_lossy(&output.stdout);
        let result: f32 = stdout.trim().parse().unwrap_or_else(|_| {
            panic!(
                "Failed to parse result '{}' for a={}, b={}",
                stdout.trim(),
                a,
                b
            )
        });

        let error = (result - expected).abs();
        assert!(
            error < 0.001,
            "Solution incorrect for {}x + {} = 0: got {}, expected {} (error = {})",
            a,
            b,
            result,
            expected,
            error
        );

        if is_debug_mode() {
            eprintln!("[DEBUG] Result: {} (error = {})", result, error);
        }
    }

    if is_debug_mode() {
        eprintln!("[DEBUG] All test cases passed!");
    }

    // Cleanup (skip in debug mode)
    if !is_debug_mode() {
        fs::remove_file(&binary_path).ok();
    }
}

#[test]
fn test_cubic_newton_step() {
    if !is_clang_available() {
        eprintln!("Skipping test_cubic_newton_step: clang++ not available");
        return;
    }

    let wgsl_source = r#"
fn newton_step_cubic(a: f32, b: f32, c: f32, d: f32, x0: f32, lower: f32, upper: f32) -> f32 {
    let fx = ((a * x0 + b) * x0 + c) * x0 + d;
    let doubled_b = b + b;
    let tripled_a = (a + a) + a;
    let derivative = (tripled_a * x0 + doubled_b) * x0 + c;
    let eps = 1e-6;
    if (derivative < eps && derivative > -eps) {
        return x0;
    }

    let next = x0 - fx / derivative;

    if (next > upper) {
        return upper;
    }
    if (next < lower) {
        return lower;
    }
    return next;
}

@compute @workgroup_size(1)
fn compute_main() {
}
"#;

    let cpp_code =
        translate_wgsl_to_cpp(wgsl_source).expect("Failed to translate cubic Newton step WGSL");

    let wrapper = r#"

#include <iostream>
#include <iomanip>
#include <cstdlib>
#include <cmath>

int main(int argc, char* argv[]) {
    if (argc != 8) {
        std::cerr << "Usage: " << argv[0] << " <a> <b> <c> <d> <x0> <lower> <upper>" << std::endl;
        return 1;
    }

    float a = std::atof(argv[1]);
    float b = std::atof(argv[2]);
    float c = std::atof(argv[3]);
    float d = std::atof(argv[4]);
    float x0 = std::atof(argv[5]);
    float lower = std::atof(argv[6]);
    float upper = std::atof(argv[7]);

    float next = newton_step_cubic(a, b, c, d, x0, lower, upper);

    std::cout << std::fixed << std::setprecision(6) << next << std::endl;

    return 0;
}
"#;

    let full_cpp = format!(
        "{cpp_code}{wrapper}",
        cpp_code = cpp_code,
        wrapper = wrapper
    );

    let temp_dir = std::env::temp_dir();
    let binary_path = temp_dir.join(format!("test_cubic_newton_step_{}", unique_test_id()));
    let runtime_header = get_runtime_header_path();

    compile_cpp(&full_cpp, &binary_path, &runtime_header)
        .expect("Failed to compile cubic Newton step C++");

    struct TestCase {
        coeffs: (f32, f32, f32, f32),
        x0: f32,
        bounds: (f32, f32),
        description: &'static str,
    }

    let test_cases = vec![
        TestCase {
            coeffs: (1.0, -6.0, 11.0, -6.0),
            x0: 1.5,
            bounds: (-10.0, 10.0),
            description: "Root near 1",
        },
        TestCase {
            coeffs: (1.0, -6.0, 11.0, -6.0),
            x0: 2.6,
            bounds: (-10.0, 10.0),
            description: "Root near 3",
        },
        TestCase {
            coeffs: (1.0, 0.0, 0.0, -1.0),
            x0: 0.0,
            bounds: (-10.0, 10.0),
            description: "Derivative near zero",
        },
        TestCase {
            coeffs: (1.0, 0.0, 0.0, -1.0),
            x0: 20.0,
            bounds: (-10.0, 10.0),
            description: "Clamp upper bound",
        },
        TestCase {
            coeffs: (1.0, 0.0, 0.0, -1.0),
            x0: -20.0,
            bounds: (-10.0, 10.0),
            description: "Clamp lower bound",
        },
    ];

    fn expected_step(a: f32, b: f32, c: f32, d: f32, x0: f32, lower: f32, upper: f32) -> f32 {
        let fx = ((a * x0 + b) * x0 + c) * x0 + d;
        let doubled_b = b + b;
        let tripled_a = (a + a) + a;
        let derivative = (tripled_a * x0 + doubled_b) * x0 + c;
        let eps = 1e-6f32;
        if derivative < eps && derivative > -eps {
            return x0;
        }
        let next = x0 - fx / derivative;
        if next > upper {
            return upper;
        }
        if next < lower {
            return lower;
        }
        next
    }

    let tolerance = 1e-3f32;

    for case in test_cases {
        let (a, b, c, d) = case.coeffs;
        let (lower, upper) = case.bounds;
        let x0 = case.x0;
        let expected = expected_step(a, b, c, d, x0, lower, upper);

        if is_debug_mode() {
            eprintln!(
                "[DEBUG] {}: coefficients=({}, {}, {}, {}), x0={}, bounds=({}, {}), expected next={} ",
                case.description,
                a,
                b,
                c,
                d,
                x0,
                lower,
                upper,
                expected
            );
        }

        let output = Command::new(&binary_path)
            .arg(a.to_string())
            .arg(b.to_string())
            .arg(c.to_string())
            .arg(d.to_string())
            .arg(x0.to_string())
            .arg(lower.to_string())
            .arg(upper.to_string())
            .output()
            .expect("Failed to execute cubic Newton step binary");

        assert!(
            output.status.success(),
            "Cubic step execution failed for {}: {}",
            case.description,
            String::from_utf8_lossy(&output.stderr)
        );

        let stdout = String::from_utf8_lossy(&output.stdout);
        let result: f32 = stdout
            .trim()
            .parse()
            .unwrap_or_else(|_| panic!("Failed to parse result '{}'", stdout.trim()));

        let error = (result - expected).abs();
        assert!(
            error < tolerance,
            "{}: got {}, expected {} (error = {})",
            case.description,
            result,
            expected,
            error
        );

        if is_debug_mode() {
            eprintln!("[DEBUG] Result: {} (error = {})", result, error);
        }
    }

    if !is_debug_mode() {
        fs::remove_file(&binary_path).ok();
    }
}

#[test]
fn test_bisect5_single_step() {
    if !is_clang_available() {
        eprintln!("Skipping test_bisect5_single_step: clang++ not available");
        return;
    }

    let wgsl_source = r#"
fn bisect5_single_step(
    a: f32,
    b: f32,
    c: f32,
    d: f32,
    e: f32,
    f: f32,
    left: f32,
    right: f32,
) -> f32 {
    let epsilon = 1e-6;
    let mid = 0.5 * (left + right);
    let left_value = (((((a * left + b) * left + c) * left + d) * left + e) * left + f);
    let mid_value = (((((a * mid + b) * mid + c) * mid + d) * mid + e) * mid + f);
    let product = left_value * mid_value;

    if (product * product < epsilon) {
        return mid;
    }

    if (product > epsilon) {
        return 0.5 * (mid + right);
    }

    return 0.5 * (left + mid);
}

@compute @workgroup_size(1)
fn compute_main() {
}
"#;

    let cpp_code =
        translate_wgsl_to_cpp(wgsl_source).expect("Failed to translate bisect5 single step WGSL");

    let wrapper = r#"

#include <iostream>
#include <iomanip>
#include <cstdlib>
#include <cmath>

int main(int argc, char* argv[]) {
    if (argc != 9) {
        std::cerr << "Usage: " << argv[0]
                  << " <a> <b> <c> <d> <e> <f> <left> <right>" << std::endl;
        return 1;
    }

    float a = std::atof(argv[1]);
    float b = std::atof(argv[2]);
    float c = std::atof(argv[3]);
    float d = std::atof(argv[4]);
    float e = std::atof(argv[5]);
    float f = std::atof(argv[6]);
    float left = std::atof(argv[7]);
    float right = std::atof(argv[8]);

    float mid = bisect5_single_step(a, b, c, d, e, f, left, right);

    std::cout << std::fixed << std::setprecision(6) << mid << std::endl;

    return 0;
}
"#;

    let full_cpp = format!(
        "{cpp_code}{wrapper}",
        cpp_code = cpp_code,
        wrapper = wrapper
    );

    let temp_dir = std::env::temp_dir();
    let binary_path = temp_dir.join(format!("test_bisect5_single_step_{}", unique_test_id()));
    let runtime_header = get_runtime_header_path();

    compile_cpp(&full_cpp, &binary_path, &runtime_header)
        .expect("Failed to compile bisect5 single step C++");

    struct TestCase {
        coeffs: (f32, f32, f32, f32, f32, f32),
        interval: (f32, f32),
        description: &'static str,
    }

    let test_cases = vec![
        TestCase {
            coeffs: (1.0, 0.0, -3.0, 0.0, 2.0, -1.0),
            interval: (0.0, 2.0),
            description: "Primary root bracket",
        },
        TestCase {
            coeffs: (1.0, -2.0, 0.0, -1.0, 0.0, 1.0),
            interval: (-1.0, 1.0),
            description: "Opposite signs across origin",
        },
        TestCase {
            coeffs: (0.5, -1.0, 0.5, -0.5, 0.25, -0.01),
            interval: (0.0, 1.0),
            description: "Small coefficients",
        },
        TestCase {
            coeffs: (1.0, 0.0, 0.0, 0.0, 0.0, -1.0),
            interval: (0.0, 1.0),
            description: "Midpoint is exact root",
        },
    ];

    fn eval_poly(coeffs: (f32, f32, f32, f32, f32, f32), x: f32) -> f32 {
        let (a, b, c, d, e, f) = coeffs;
        ((((a * x + b) * x + c) * x + d) * x + e) * x + f
    }

    fn expected_mid(coeffs: (f32, f32, f32, f32, f32, f32), left: f32, right: f32) -> f32 {
        let epsilon = 1e-6f32;
        let mid = 0.5 * (left + right);
        let f_left = eval_poly(coeffs, left);
        let f_mid = eval_poly(coeffs, mid);
        let product = f_left * f_mid;

        if product * product < epsilon {
            return mid;
        }
        if product > epsilon {
            return 0.5 * (mid + right);
        }
        0.5 * (left + mid)
    }

    let tolerance = 1e-3f32;

    for case in test_cases {
        let (left, right) = case.interval;
        let expected = expected_mid(case.coeffs, left, right);

        if is_debug_mode() {
            let (a, b, c, d, e, f_val) = case.coeffs;
            eprintln!(
                "[DEBUG] {}: coeffs=({}, {}, {}, {}, {}, {}), interval=({}, {}), expected mid={}",
                case.description, a, b, c, d, e, f_val, left, right, expected
            );
        }

        let output = Command::new(&binary_path)
            .arg(case.coeffs.0.to_string())
            .arg(case.coeffs.1.to_string())
            .arg(case.coeffs.2.to_string())
            .arg(case.coeffs.3.to_string())
            .arg(case.coeffs.4.to_string())
            .arg(case.coeffs.5.to_string())
            .arg(left.to_string())
            .arg(right.to_string())
            .output()
            .expect("Failed to execute bisect5 single step binary");

        assert!(
            output.status.success(),
            "bisect5 single step execution failed for {}: {}",
            case.description,
            String::from_utf8_lossy(&output.stderr)
        );

        let stdout = String::from_utf8_lossy(&output.stdout);
        let result: f32 = stdout
            .trim()
            .parse()
            .unwrap_or_else(|_| panic!("Failed to parse result '{}'", stdout.trim()));

        let error = (result - expected).abs();
        assert!(
            error < tolerance,
            "{}: got {}, expected {} (error = {})",
            case.description,
            result,
            expected,
            error
        );

        if is_debug_mode() {
            eprintln!("[DEBUG] Result: {} (error = {})", result, error);
        }
    }

    if !is_debug_mode() {
        fs::remove_file(&binary_path).ok();
    }
}

#[test]
fn test_bisect5_solver() {
    if !is_clang_available() {
        eprintln!("Skipping test_bisect5_solver: clang++ not available");
        return;
    }

    let wgsl_source = r#"
fn bisect5(
    a: f32,
    b: f32,
    c: f32,
    d: f32,
    e: f32,
    f: f32,
    left: f32,
    right: f32,
) -> f32 {
    let epsilon = 1e-6;
    let mid1 = 0.5 * (left + right);
    let left_value1 = (((((a * left + b) * left + c) * left + d) * left + e) * left + f);
    let mid_value1 = (((((a * mid1 + b) * mid1 + c) * mid1 + d) * mid1 + e) * mid1 + f);
    if (left_value1 * left_value1 < epsilon) {
        return left;
    }

    if (mid_value1 * mid_value1 < epsilon) {
        return mid1;
    }

    let product1 = left_value1 * mid_value1;

    if (product1 > epsilon) {
        let left2 = mid1;
        let right2 = right;
        let mid2 = 0.5 * (left2 + right2);
        let left_value2 = (((((a * left2 + b) * left2 + c) * left2 + d) * left2 + e) * left2 + f);
        let mid_value2 = (((((a * mid2 + b) * mid2 + c) * mid2 + d) * mid2 + e) * mid2 + f);
        if (left_value2 * left_value2 < epsilon) {
            return left2;
        }

        if (mid_value2 * mid_value2 < epsilon) {
            return mid2;
        }

        let product2 = left_value2 * mid_value2;

        if (product2 > epsilon) {
            let left3 = mid2;
            let right3 = right2;
            let mid3 = 0.5 * (left3 + right3);
            let left_value3 = (((((a * left3 + b) * left3 + c) * left3 + d) * left3 + e) * left3 + f);
            let mid_value3 = (((((a * mid3 + b) * mid3 + c) * mid3 + d) * mid3 + e) * mid3 + f);
            if (left_value3 * left_value3 < epsilon) {
                return left3;
            }

            if (mid_value3 * mid_value3 < epsilon) {
                return mid3;
            }

            let product3 = left_value3 * mid_value3;

            if (product3 > epsilon) {
                return 0.5 * (mid3 + right3);
            }

            return 0.5 * (left3 + mid3);
        }

        let left3 = left2;
        let right3 = mid2;
        let mid3 = 0.5 * (left3 + right3);
        let left_value3 = (((((a * left3 + b) * left3 + c) * left3 + d) * left3 + e) * left3 + f);
        let mid_value3 = (((((a * mid3 + b) * mid3 + c) * mid3 + d) * mid3 + e) * mid3 + f);
        if (left_value3 * left_value3 < epsilon) {
            return left3;
        }

        if (mid_value3 * mid_value3 < epsilon) {
            return mid3;
        }

        let product3 = left_value3 * mid_value3;

        if (product3 > epsilon) {
            return 0.5 * (mid3 + right3);
        }

        return 0.5 * (left3 + mid3);
    }

    let left2 = left;
    let right2 = mid1;
    let mid2 = 0.5 * (left2 + right2);
    let left_value2 = (((((a * left2 + b) * left2 + c) * left2 + d) * left2 + e) * left2 + f);
    let mid_value2 = (((((a * mid2 + b) * mid2 + c) * mid2 + d) * mid2 + e) * mid2 + f);
    if (left_value2 * left_value2 < epsilon) {
        return left2;
    }

    if (mid_value2 * mid_value2 < epsilon) {
        return mid2;
    }

    let product2 = left_value2 * mid_value2;

    if (product2 > epsilon) {
        let left3 = mid2;
        let right3 = right2;
        let mid3 = 0.5 * (left3 + right3);
        let left_value3 = (((((a * left3 + b) * left3 + c) * left3 + d) * left3 + e) * left3 + f);
        let mid_value3 = (((((a * mid3 + b) * mid3 + c) * mid3 + d) * mid3 + e) * mid3 + f);
        if (left_value3 * left_value3 < epsilon) {
            return left3;
        }

        if (mid_value3 * mid_value3 < epsilon) {
            return mid3;
        }

        let product3 = left_value3 * mid_value3;

        if (product3 > epsilon) {
            return 0.5 * (mid3 + right3);
        }

        return 0.5 * (left3 + mid3);
    }

    let left3 = left2;
    let right3 = mid2;
    let mid3 = 0.5 * (left3 + right3);
    let left_value3 = (((((a * left3 + b) * left3 + c) * left3 + d) * left3 + e) * left3 + f);
    let mid_value3 = (((((a * mid3 + b) * mid3 + c) * mid3 + d) * mid3 + e) * mid3 + f);
    if (left_value3 * left_value3 < epsilon) {
        return left3;
    }

    if (mid_value3 * mid_value3 < epsilon) {
        return mid3;
    }

    let product3 = left_value3 * mid_value3;

    if (product3 > epsilon) {
        return 0.5 * (mid3 + right3);
    }

    return 0.5 * (left3 + mid3);
}

@compute @workgroup_size(1)
fn compute_main() {
}
"#;

    let cpp_code = translate_wgsl_to_cpp(wgsl_source).expect("Failed to translate bisect5 WGSL");

    assert!(
        !cpp_code.contains("/* expr */") && !cpp_code.contains("// Unsupported statement"),
        "Generated C++ still contains unsupported placeholders:\n{}",
        cpp_code
    );

    let bisect_function_name = cpp_code
        .lines()
        .find_map(|line| {
            let trimmed = line.trim_start();
            if let Some(rest) = trimmed.strip_prefix("float ") {
                let name = rest.split('(').next()?.trim();
                if name.starts_with("bisect5") {
                    return Some(name.to_string());
                }
            }
            None
        })
        .expect("bisect5 function not found in generated C++");

    let wrapper_template = r#"

#include <iostream>
#include <iomanip>
#include <cstdlib>
#include <cmath>

int main(int argc, char* argv[]) {
    if (argc != 9) {
        std::cerr << "Usage: " << argv[0]
                  << " <a> <b> <c> <d> <e> <f> <left> <right>" << std::endl;
        return 1;
    }

    float a = std::atof(argv[1]);
    float b = std::atof(argv[2]);
    float c = std::atof(argv[3]);
    float d = std::atof(argv[4]);
    float e = std::atof(argv[5]);
    float f = std::atof(argv[6]);
    float left = std::atof(argv[7]);
    float right = std::atof(argv[8]);

    float root = __B5_FN__(a, b, c, d, e, f, left, right);

    std::cout << std::fixed << std::setprecision(6) << root << std::endl;

    return 0;
}
"#;

    let wrapper = wrapper_template.replace("__B5_FN__", &bisect_function_name);

    let full_cpp = format!(
        "{cpp_code}{wrapper}",
        cpp_code = cpp_code,
        wrapper = wrapper
    );

    let temp_dir = std::env::temp_dir();
    let binary_path = temp_dir.join(format!("test_bisect5_solver_{}", unique_test_id()));
    let runtime_header = get_runtime_header_path();

    compile_cpp(&full_cpp, &binary_path, &runtime_header).expect("Failed to compile bisect5 C++");

    struct TestCase {
        coeffs: (f32, f32, f32, f32, f32, f32),
        interval: (f32, f32),
        description: &'static str,
    }

    let test_cases = vec![
        TestCase {
            coeffs: (1.0, 0.0, -3.0, 0.0, 2.0, -1.0),
            interval: (1.0, 2.0),
            description: "x^5 - 3x^3 + 2x - 1",
        },
        TestCase {
            coeffs: (1.0, -2.0, 1.0, 0.0, -1.0, 0.0),
            interval: (0.0, 2.0),
            description: "Polynomial with root at x=1",
        },
        TestCase {
            coeffs: (1.0, 0.0, 0.0, 0.0, 0.0, -2.0),
            interval: (1.0, 2.0),
            description: "x^5 - 2",
        },
    ];

    fn eval_poly(coeffs: (f32, f32, f32, f32, f32, f32), x: f32) -> f32 {
        let (a, b, c, d, e, f) = coeffs;
        ((((a * x + b) * x + c) * x + d) * x + e) * x + f
    }

    fn bisect5_reference(coeffs: (f32, f32, f32, f32, f32, f32), left: f32, right: f32) -> f32 {
        let epsilon = 1e-6f32;
        let poly = |x: f32| eval_poly(coeffs, x);

        let mid1 = 0.5 * (left + right);
        let left_value1 = poly(left);
        if left_value1 * left_value1 < epsilon {
            return left;
        }
        let mid_value1 = poly(mid1);
        if mid_value1 * mid_value1 < epsilon {
            return mid1;
        }
        let product1 = left_value1 * mid_value1;

        if product1 > epsilon {
            let left2 = mid1;
            let right2 = right;
            let mid2 = 0.5 * (left2 + right2);
            let left_value2 = poly(left2);
            if left_value2 * left_value2 < epsilon {
                return left2;
            }
            let mid_value2 = poly(mid2);
            if mid_value2 * mid_value2 < epsilon {
                return mid2;
            }
            let product2 = left_value2 * mid_value2;

            if product2 > epsilon {
                let left3 = mid2;
                let right3 = right2;
                let mid3 = 0.5 * (left3 + right3);
                let left_value3 = poly(left3);
                if left_value3 * left_value3 < epsilon {
                    return left3;
                }
                let mid_value3 = poly(mid3);
                if mid_value3 * mid_value3 < epsilon {
                    return mid3;
                }
                let product3 = left_value3 * mid_value3;
                if product3 > epsilon {
                    return 0.5 * (mid3 + right3);
                }
                return 0.5 * (left3 + mid3);
            }

            let left3 = left2;
            let right3 = mid2;
            let mid3 = 0.5 * (left3 + right3);
            let left_value3 = poly(left3);
            if left_value3 * left_value3 < epsilon {
                return left3;
            }
            let mid_value3 = poly(mid3);
            if mid_value3 * mid_value3 < epsilon {
                return mid3;
            }
            let product3 = left_value3 * mid_value3;
            if product3 > epsilon {
                return 0.5 * (mid3 + right3);
            }
            return 0.5 * (left3 + mid3);
        }

        let left2 = left;
        let right2 = mid1;
        let mid2 = 0.5 * (left2 + right2);
        let left_value2 = poly(left2);
        if left_value2 * left_value2 < epsilon {
            return left2;
        }
        let mid_value2 = poly(mid2);
        if mid_value2 * mid_value2 < epsilon {
            return mid2;
        }
        let product2 = left_value2 * mid_value2;

        if product2 > epsilon {
            let left3 = mid2;
            let right3 = right2;
            let mid3 = 0.5 * (left3 + right3);
            let left_value3 = poly(left3);
            if left_value3 * left_value3 < epsilon {
                return left3;
            }
            let mid_value3 = poly(mid3);
            if mid_value3 * mid_value3 < epsilon {
                return mid3;
            }
            let product3 = left_value3 * mid_value3;
            if product3 > epsilon {
                return 0.5 * (mid3 + right3);
            }
            return 0.5 * (left3 + mid3);
        }

        let left3 = left2;
        let right3 = mid2;
        let mid3 = 0.5 * (left3 + right3);
        let left_value3 = poly(left3);
        if left_value3 * left_value3 < epsilon {
            return left3;
        }
        let mid_value3 = poly(mid3);
        if mid_value3 * mid_value3 < epsilon {
            return mid3;
        }
        let product3 = left_value3 * mid_value3;
        if product3 > epsilon {
            return 0.5 * (mid3 + right3);
        }
        0.5 * (left3 + mid3)
    }

    let tolerance = 1e-4f32;

    for case in test_cases {
        let (a, b, c, d, e, f_val) = case.coeffs;
        let (left, right) = case.interval;

        if is_debug_mode() {
            eprintln!(
                "[DEBUG] {}: coeffs=({}, {}, {}, {}, {}, {}), interval=({}, {})",
                case.description, a, b, c, d, e, f_val, left, right
            );
        }

        let output = Command::new(&binary_path)
            .arg(a.to_string())
            .arg(b.to_string())
            .arg(c.to_string())
            .arg(d.to_string())
            .arg(e.to_string())
            .arg(f_val.to_string())
            .arg(left.to_string())
            .arg(right.to_string())
            .output()
            .expect("Failed to execute bisect5 solver binary");

        assert!(
            output.status.success(),
            "bisect5 solver execution failed for {}: {}",
            case.description,
            String::from_utf8_lossy(&output.stderr)
        );

        let stdout = String::from_utf8_lossy(&output.stdout);
        let result: f32 = stdout
            .trim()
            .parse()
            .unwrap_or_else(|_| panic!("Failed to parse result '{}'", stdout.trim()));

        let expected = bisect5_reference(case.coeffs, left, right);
        let poly_value = eval_poly(case.coeffs, result);
        let expected_error = (result - expected).abs();

        assert!(
            expected_error < tolerance,
            "{}: got {}, expected ~{} (error = {})",
            case.description,
            result,
            expected,
            expected_error
        );

        assert!(
            poly_value.abs() < 0.5,
            "{}: polynomial residual too large at root approximation: {}",
            case.description,
            poly_value
        );

        if is_debug_mode() {
            eprintln!(
                "[DEBUG] {} result: {}, residual: {}",
                case.description, result, poly_value
            );
        }
    }

    if !is_debug_mode() {
        fs::remove_file(&binary_path).ok();
    }
}
