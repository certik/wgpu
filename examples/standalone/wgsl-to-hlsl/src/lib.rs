use std::path::Path;

/// Translate WGSL source code to HLSL
pub fn translate_wgsl_to_hlsl(
    wgsl_source: &str,
    shader_model: naga::back::hlsl::ShaderModel,
) -> anyhow::Result<String> {
    // Parse WGSL
    let module = naga::front::wgsl::parse_str(wgsl_source)
        .map_err(|e| anyhow::anyhow!("Failed to parse WGSL:\n{}", e.emit_to_string(wgsl_source)))?;

    // Validate the module
    let info = naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::all(),
    )
    .validate(&module)?;

    // Configure HLSL output options
    let options = naga::back::hlsl::Options {
        shader_model,
        ..Default::default()
    };

    // Translate to HLSL
    let mut hlsl_output = String::new();
    let pipeline_options = Default::default();
    let mut writer = naga::back::hlsl::Writer::new(&mut hlsl_output, &options, &pipeline_options);
    writer.write(&module, &info, None)?;

    Ok(hlsl_output)
}

/// Translate WGSL source code to HLSL with a specific file path for better error messages
pub fn translate_wgsl_to_hlsl_with_path(
    wgsl_source: &str,
    input_path: &Path,
    shader_model: naga::back::hlsl::ShaderModel,
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

    // Configure HLSL output options
    let options = naga::back::hlsl::Options {
        shader_model,
        ..Default::default()
    };

    // Translate to HLSL
    let mut hlsl_output = String::new();
    let pipeline_options = Default::default();
    let mut writer = naga::back::hlsl::Writer::new(&mut hlsl_output, &options, &pipeline_options);
    writer.write(&module, &info, None)?;

    Ok(hlsl_output)
}

/// Parse shader model from string
pub fn parse_shader_model(s: &str) -> anyhow::Result<naga::back::hlsl::ShaderModel> {
    use naga::back::hlsl::ShaderModel;
    match s {
        "50" => Ok(ShaderModel::V5_0),
        "51" => Ok(ShaderModel::V5_1),
        "60" => Ok(ShaderModel::V6_0),
        "61" => Ok(ShaderModel::V6_1),
        "62" => Ok(ShaderModel::V6_2),
        "63" => Ok(ShaderModel::V6_3),
        "64" => Ok(ShaderModel::V6_4),
        "65" => Ok(ShaderModel::V6_5),
        "66" => Ok(ShaderModel::V6_6),
        "67" => Ok(ShaderModel::V6_7),
        _ => Err(anyhow::anyhow!("Invalid shader model: {s}")),
    }
}
