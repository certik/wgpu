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
