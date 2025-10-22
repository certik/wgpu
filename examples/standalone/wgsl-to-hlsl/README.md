# WGSL to HLSL Translator

A simple command-line application that translates WGSL (WebGPU Shading Language) shaders to HLSL (High-Level Shading Language) using naga.

## Usage

### Basic Translation (output to stdout)

```bash
cargo run -p wgsl-to-hlsl -- shader.wgsl
```

### Save to Output File

```bash
cargo run -p wgsl-to-hlsl -- shader.wgsl -o output.hlsl
```

### Specify HLSL Shader Model

```bash
cargo run -p wgsl-to-hlsl -- shader.wgsl --shader-model 60
```

Supported shader models: 50, 51, 60, 61, 62, 63, 64, 65, 66, 67 (default: 50)

## Options

- `input` - Input WGSL shader file (required positional argument)
- `-o, --output` - Output HLSL file path (optional, prints to stdout if not specified)
- `--shader-model` - Target HLSL shader model version (optional, default: 50)

## Examples

### Example 1: Simple Compute Shader

Input WGSL:
```wgsl
@compute @workgroup_size(1)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    // Simple compute shader
}
```

Output HLSL:
```hlsl
[numthreads(1, 1, 1)]
void main(uint3 global_id : SV_DispatchThreadID)
{
    return;
}
```

### Example 2: Vertex Shader

```bash
cargo run -p wgsl-to-hlsl -- vertex.wgsl -o vertex.hlsl --shader-model 60
```

## Building

```bash
cargo build -p wgsl-to-hlsl
```

## Testing

```bash
cargo test -p wgsl-to-hlsl
```
