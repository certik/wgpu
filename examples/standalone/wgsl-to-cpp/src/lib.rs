use std::path::Path;
use std::process::Command;

/// Translate WGSL source code to C++
pub fn translate_wgsl_to_cpp(wgsl_source: &str) -> anyhow::Result<String> {
    // Parse WGSL
    let module = naga::front::wgsl::parse_str(wgsl_source)
        .map_err(|e| anyhow::anyhow!("Failed to parse WGSL:\n{}", e.emit_to_string(wgsl_source)))?;

    // Validate the module
    let info = naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::all(),
    )
    .validate(&module)?;

    // Configure C++ output options
    let options = naga::back::cpp::Options::default();

    // Translate to C++
    let cpp_output = naga::back::cpp::write_string(&module, &info, &options)?;

    Ok(cpp_output)
}

/// Translate WGSL source code to C++ with a specific file path for better error messages
pub fn translate_wgsl_to_cpp_with_path(
    wgsl_source: &str,
    input_path: &Path,
) -> anyhow::Result<String> {
    // Parse WGSL
    let module = naga::front::wgsl::parse_str(wgsl_source).map_err(|e| {
        anyhow::anyhow!(
            "Failed to parse WGSL:\n{}",
            e.emit_to_string_with_path(wgsl_source, input_path)
        )
    })?;

    // Validate the module
    let info = naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::all(),
    )
    .validate(&module)?;

    // Configure C++ output options
    let options = naga::back::cpp::Options::default();

    // Translate to C++
    let cpp_output = naga::back::cpp::write_string(&module, &info, &options)?;

    Ok(cpp_output)
}

/// Compile C++ code using clang++
pub fn compile_cpp(
    cpp_source: &str,
    output_path: &Path,
    runtime_header_path: &Path,
) -> anyhow::Result<()> {
    // Write C++ to a temporary file
    let temp_dir = std::env::temp_dir();
    let cpp_file = temp_dir.join("shader.cpp");
    std::fs::write(&cpp_file, cpp_source)?;

    // Get the directory containing the runtime header
    let runtime_dir = runtime_header_path
        .parent()
        .ok_or_else(|| anyhow::anyhow!("Invalid runtime header path"))?;

    // Compile with clang++
    let output = Command::new("clang++")
        .arg("-std=c++17")
        .arg("-I")
        .arg(runtime_dir)
        .arg("-o")
        .arg(output_path)
        .arg(&cpp_file)
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow::anyhow!("Compilation failed:\n{}", stderr));
    }

    Ok(())
}
