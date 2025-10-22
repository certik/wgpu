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
    format!("{}_{}_{}",counter, thread_id.replace("ThreadId(", "").replace(")", ""), timestamp)
}

/// Get the path to the runtime header for tests
fn get_runtime_header_path() -> PathBuf {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    PathBuf::from(manifest_dir).join("src/wgsl_runtime.hpp")
}

/// Check if clang++ is available
fn is_clang_available() -> bool {
    Command::new("clang++")
        .arg("--version")
        .output()
        .is_ok()
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
    let cpp_code = translate_wgsl_to_cpp(wgsl_source)
        .expect("Failed to translate WGSL");

    // Wrap with main() function
    let full_cpp = format!(
        "{}\n\nint main() {{\n    compute_shader();\n    return 0;\n}}\n",
        cpp_code
    );

    // Compile
    let temp_dir = std::env::temp_dir();
    let binary_path = temp_dir.join(format!("test_basic_shader_{}", unique_test_id()));
    let runtime_header = get_runtime_header_path();

    compile_cpp(&full_cpp, &binary_path, &runtime_header)
        .expect("Failed to compile C++");

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
    let cpp_code = translate_wgsl_to_cpp(wgsl_source)
        .expect("Failed to translate WGSL");

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

    compile_cpp(&full_cpp, &binary_path, &runtime_header)
        .expect("Failed to compile C++");

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

        assert!(
            output.status.success(),
            "Shader {} execution failed",
            i
        );

        // Cleanup (skip in debug mode)
        if !is_debug_mode() {
            fs::remove_file(&binary_path).ok();
        }
    }
}

#[test]
#[ignore] // Disabled until C++ backend supports functions, loops, variables, and function calls
fn test_bisect5_newton_method() {
    // This test demonstrates a complex Newton-Raphson bisection method for finding
    // roots of a 5th degree polynomial. It's currently disabled because the C++ backend
    // doesn't yet support the required WGSL features.
    //
    // MISSING FEATURES NEEDED:
    // 1. Regular function definitions (not just entry points)
    // 2. Function parameters and return types
    // 3. Local variable declarations (var statements)
    // 4. For loops
    // 5. Function calls
    // 6. Load expressions (reading from variables)
    // 7. Module-level constants
    // 8. select() and abs() builtin functions (added to runtime, but not callable yet)
    //
    // MATHEMATICAL BACKGROUND:
    // Solves: x^5 - 3x^3 + 2x - 1 = 0
    // Using Newton-Raphson method with bisection for interval refinement
    // Expected root in [0, 1]: x ≈ 0.656
    //
    // WGSL CODE:
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

    // Translation will work (naga can parse it), but the C++ backend will generate
    // placeholder comments for unsupported features
    let result = translate_wgsl_to_cpp(wgsl_source);

    // For now, just verify it doesn't crash during translation
    // The generated C++ won't be valid/compilable until backend is extended
    match result {
        Ok(cpp_code) => {
            eprintln!("Translation succeeded, but generated C++ contains unsupported feature placeholders:");
            eprintln!("{}", cpp_code);
        }
        Err(e) => {
            eprintln!("Translation failed (expected until backend is implemented): {}", e);
        }
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
    let cpp_code = translate_wgsl_to_cpp(wgsl_source)
        .expect("Failed to translate linear solver WGSL");

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
        (3.0, 6.0, -2.0),     // 3x + 6 = 0 → x = -6/3 = -2
        (5.0, 10.0, -2.0),    // 5x + 10 = 0 → x = -10/5 = -2
        (2.0, -4.0, 2.0),     // 2x - 4 = 0 → x = -(-4)/2 = 2
        (-1.0, 3.0, 3.0),     // -x + 3 = 0 → x = -3/(-1) = 3
    ];

    for (a, b, expected) in test_cases {
        if is_debug_mode() {
            eprintln!("[DEBUG] Testing: {}x + {} = 0, expected: {}", a, b, expected);
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
        let result: f32 = stdout.trim().parse()
            .unwrap_or_else(|_| panic!("Failed to parse result '{}' for a={}, b={}", stdout.trim(), a, b));

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

    let cpp_code = translate_wgsl_to_cpp(wgsl_source)
        .expect("Failed to translate cubic Newton step WGSL");

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

    let full_cpp = format!("{cpp_code}{wrapper}", cpp_code = cpp_code, wrapper = wrapper);

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
