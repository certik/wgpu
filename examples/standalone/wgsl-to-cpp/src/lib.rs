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
    let debug_mode = std::env::var("WGSL_TO_CPP_DEBUG").is_ok();

    // Write C++ to a temporary file with unique name based on the output path
    let temp_dir = std::env::temp_dir();
    let cpp_filename = format!(
        "shader_{}.cpp",
        output_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("temp")
    );
    let cpp_file = temp_dir.join(cpp_filename);
    std::fs::write(&cpp_file, cpp_source)?;

    if debug_mode {
        eprintln!("[DEBUG] C++ source written to: {}", cpp_file.display());
    }

    // Get the directory containing the runtime header
    let runtime_dir = runtime_header_path
        .parent()
        .ok_or_else(|| anyhow::anyhow!("Invalid runtime header path"))?;

    // Build the command arguments
    let args = vec![
        "-std=c++17",
        "-I",
        runtime_dir.to_str().unwrap(),
        "-o",
        output_path.to_str().unwrap(),
        cpp_file.to_str().unwrap(),
    ];

    if debug_mode {
        eprintln!("[DEBUG] Compiling with: clang++ {}", args.join(" "));
    }

    // Compile with clang++
    let output = Command::new("clang++")
        .args(&args)
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow::anyhow!("Compilation failed:\n{}", stderr));
    }

    if debug_mode {
        eprintln!("[DEBUG] Binary compiled to: {}", output_path.display());
        eprintln!("[DEBUG] To keep debugging, C++ source preserved at: {}", cpp_file.display());
    }

    Ok(())
}
