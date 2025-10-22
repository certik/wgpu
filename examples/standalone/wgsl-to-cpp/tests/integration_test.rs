use wgsl_to_cpp::translate_wgsl_to_cpp;

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
