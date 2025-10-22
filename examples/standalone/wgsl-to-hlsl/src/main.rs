use std::fs;
use std::path::Path;

/// Translate WGSL shaders to HLSL
#[derive(argh::FromArgs)]
struct Args {
    /// input WGSL file
    #[argh(positional)]
    input: String,

    /// output HLSL file (if not specified, prints to stdout)
    #[argh(option, short = 'o')]
    output: Option<String>,

    /// HLSL shader model (50, 51, 60, 61, 62, 63, 64, 65, 66, 67)
    #[argh(option, default = "default_shader_model()")]
    shader_model: String,
}

fn default_shader_model() -> String {
    "50".to_string()
}

fn main() -> anyhow::Result<()> {
    let args: Args = argh::from_env();

    // Read the input WGSL file
    let input_path = Path::new(&args.input);
    let wgsl_source = fs::read_to_string(input_path)?;

    // Parse shader model and translate
    let shader_model = wgsl_to_hlsl::parse_shader_model(&args.shader_model)?;
    let hlsl_output =
        wgsl_to_hlsl::translate_wgsl_to_hlsl_with_path(&wgsl_source, input_path, shader_model)?;

    // Output the result
    match args.output {
        Some(output_path) => {
            fs::write(&output_path, &hlsl_output)?;
            eprintln!("HLSL written to: {}", output_path);
        }
        None => {
            println!("{}", hlsl_output);
        }
    }

    Ok(())
}
