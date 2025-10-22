use naga::back::hlsl::ShaderModel;
use wgsl_to_hlsl::{parse_shader_model, translate_wgsl_to_hlsl};

#[test]
fn test_basic_compute_shader() {
    let wgsl_source = r#"
@compute @workgroup_size(1)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    // Simple compute shader
}
"#;

    // Translate using default shader model (5.0)
    let result = translate_wgsl_to_hlsl(wgsl_source, ShaderModel::V5_0);

    // Verify it succeeded
    assert!(result.is_ok(), "Translation failed: {:?}", result.err());

    // Verify we got HLSL output
    let hlsl_output = result.unwrap();
    assert!(hlsl_output.contains("[numthreads"));
    assert!(hlsl_output.contains("void main"));
}

#[test]
fn test_vertex_shader() {
    let wgsl_source = r#"
@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> @builtin(position) vec4<f32> {
    return vec4<f32>(0.0, 0.0, 0.0, 1.0);
}
"#;

    // Translate to HLSL
    let result = translate_wgsl_to_hlsl(wgsl_source, ShaderModel::V5_0);

    // Verify it succeeded
    assert!(result.is_ok(), "Translation failed: {:?}", result.err());

    // Verify we got valid HLSL output
    let hlsl_output = result.unwrap();
    assert!(!hlsl_output.is_empty(), "Output is empty");
    assert!(hlsl_output.contains("vs_main"));
}

#[test]
fn test_fragment_shader() {
    let wgsl_source = r#"
@fragment
fn fs_main(@builtin(position) position: vec4<f32>) -> @location(0) vec4<f32> {
    return vec4<f32>(1.0, 0.0, 0.0, 1.0);
}
"#;

    // Translate to HLSL
    let result = translate_wgsl_to_hlsl(wgsl_source, ShaderModel::V5_0);

    // Verify it succeeded
    assert!(result.is_ok(), "Translation failed: {:?}", result.err());

    // Verify we got valid HLSL output
    let hlsl_output = result.unwrap();
    assert!(!hlsl_output.is_empty(), "Output is empty");
    assert!(hlsl_output.contains("fs_main"));
}

#[test]
fn test_shader_model_60() {
    let wgsl_source = r#"
@compute @workgroup_size(8, 8, 1)
fn main() {
}
"#;

    // Translate with shader model 6.0
    let result = translate_wgsl_to_hlsl(wgsl_source, ShaderModel::V6_0);

    // Verify it succeeded
    assert!(
        result.is_ok(),
        "Translation failed with shader model 6.0: {:?}",
        result.err()
    );

    // Verify we got HLSL output
    let hlsl_output = result.unwrap();
    assert!(hlsl_output.contains("[numthreads"));
}

#[test]
fn test_parse_shader_model() {
    // Test valid shader models
    assert!(parse_shader_model("50").is_ok());
    assert!(parse_shader_model("51").is_ok());
    assert!(parse_shader_model("60").is_ok());
    assert!(parse_shader_model("67").is_ok());

    // Test invalid shader model
    assert!(parse_shader_model("invalid").is_err());
    assert!(parse_shader_model("99").is_err());
}

#[test]
fn test_invalid_wgsl() {
    let invalid_wgsl = r#"
this is not valid WGSL code
"#;

    // Attempt to translate invalid WGSL
    let result = translate_wgsl_to_hlsl(invalid_wgsl, ShaderModel::V5_0);

    // Verify it failed (as expected)
    assert!(result.is_err(), "Should have failed on invalid WGSL");
}

#[test]
fn test_complex_compute_shader() {
    let wgsl_source = r#"
@group(0) @binding(0)
var<storage, read_write> data: array<u32>;

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let index = global_id.x;
    data[index] = data[index] * 2u;
}
"#;

    // Translate to HLSL
    let result = translate_wgsl_to_hlsl(wgsl_source, ShaderModel::V5_0);

    // Verify it succeeded
    assert!(result.is_ok(), "Translation failed: {:?}", result.err());

    // Verify we got HLSL output with storage buffer
    let hlsl_output = result.unwrap();
    assert!(hlsl_output.contains("[numthreads"));
    assert!(hlsl_output.contains("void main"));
}
