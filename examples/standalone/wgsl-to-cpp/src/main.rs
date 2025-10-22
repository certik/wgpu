use std::fs;
use std::path::{Path, PathBuf};

/// Translate WGSL shaders to C++
#[derive(argh::FromArgs)]
struct Args {
    /// input WGSL file
    #[argh(positional)]
    input: String,

    /// output C++ file (if not specified, prints to stdout)
    #[argh(option, short = 'o')]
    output: Option<String>,

    /// compile the generated C++ code
    #[argh(switch, short = 'c')]
    compile: bool,

    /// output path for compiled binary (requires --compile)
    #[argh(option)]
    compile_output: Option<String>,
}

fn get_runtime_header_path() -> PathBuf {
    // Runtime header is in the same directory as this binary's source
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    PathBuf::from(manifest_dir).join("src/wgsl_runtime.hpp")
}

fn main() -> anyhow::Result<()> {
    let args: Args = argh::from_env();

    // Read the input WGSL file
    let input_path = Path::new(&args.input);
    let wgsl_source = fs::read_to_string(input_path)?;

    // Translate to C++
    let cpp_output = wgsl_to_cpp::translate_wgsl_to_cpp_with_path(&wgsl_source, input_path)?;

    // Output the C++ code
    let cpp_file_path = if let Some(ref output_path) = args.output {
        fs::write(output_path, &cpp_output)?;
        eprintln!("C++ written to: {}", output_path);
        PathBuf::from(output_path)
    } else {
        println!("{}", cpp_output);
        // If compiling without explicit output, use temp file
        if args.compile {
            let temp = std::env::temp_dir().join("shader_output.cpp");
            fs::write(&temp, &cpp_output)?;
            temp
        } else {
            return Ok(());
        }
    };

    // Compile if requested
    if args.compile {
        let binary_output = if let Some(ref path) = args.compile_output {
            PathBuf::from(path)
        } else {
            // Default: replace .cpp with no extension (or add .out)
            let mut path = cpp_file_path.clone();
            path.set_extension("");
            if path.extension().is_none() && !path.to_string_lossy().ends_with(".out") {
                path.set_extension("out");
            }
            path
        };

        let runtime_header = get_runtime_header_path();
        wgsl_to_cpp::compile_cpp(&cpp_output, &binary_output, &runtime_header)?;
        eprintln!("Compiled binary: {}", binary_output.display());
    }

    Ok(())
}
