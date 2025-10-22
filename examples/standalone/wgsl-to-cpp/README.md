# WGSL to C++ Translator

A command-line application that translates WGSL (WebGPU Shading Language) shaders to C++ using naga's C++ backend.

## Features

- Translates WGSL shaders to C++ code
- Supports basic compute shaders
- Optional compilation with clang++
- Minimal runtime library with vector/matrix types

## Usage

### Basic Translation (output to stdout)

```bash
cargo run -p wgsl-to-cpp -- shader.wgsl
```

### Save to Output File

```bash
cargo run -p wgsl-to-cpp -- shader.wgsl -o output.cpp
```

### Compile Generated C++

```bash
cargo run -p wgsl-to-cpp -- shader.wgsl -o output.cpp --compile
```

### Specify Compilation Output

```bash
cargo run -p wgsl-to-cpp -- shader.wgsl -o output.cpp --compile --compile-output myprogram
```

## Options

- `input` - Input WGSL shader file (required positional argument)
- `-o, --output` - Output C++ file path (optional, prints to stdout if not specified)
- `-c, --compile` - Compile the generated C++ code with clang++
- `--compile-output` - Output path for compiled binary (requires `--compile`)

## Example

### Input WGSL (simple_compute.wgsl):
```wgsl
@compute @workgroup_size(1)
fn compute_main() {
    return;
}
```

### Generated C++:
```cpp
#include "wgsl_runtime.hpp"

void compute_main() {
    return;
}
```

### Compile and Run:
```bash
# Generate C++
cargo run -p wgsl-to-cpp -- simple_compute.wgsl -o shader.cpp

# Add main() wrapper
cat >> shader.cpp << 'EOF'

int main() {
    compute_main();
    return 0;
}
EOF

# Compile
clang++ -std=c++17 -I examples/standalone/wgsl-to-cpp/src shader.cpp -o shader

# Run
./shader
```

## Runtime Library

The C++ runtime (`wgsl_runtime.hpp`) provides:
- Vector types: `vec2<T>`, `vec3<T>`, `vec4<T>`
- Matrix types: `mat2x2<T>`, `mat3x3<T>`, `mat4x4<T>`, etc.
- Basic vector arithmetic operations
- Standard C++ compatible types

## Limitations

- Only supports compute shaders currently
- Limited expression/statement support
- No GPU-specific features (textures, samplers)
- Intended for testing and validation, not production use

## Testing

```bash
cargo test -p wgsl-to-cpp
```

## Building

```bash
cargo build -p wgsl-to-cpp
```
