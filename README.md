# wasmedgeup

A cross-platform installer for WasmEdge runtime and plugins, written in Rust.

## Features

- Cross-platform support (Linux, macOS, Windows)
- Automatic OS and architecture detection
- Plugin management
- Version management
- Progress indicators and verbose logging options
- Checksum verification for downloads

## Installation

### From Source

```bash
git clone https://github.com/Ytemiloluwa/Wasmedgeup.git
cd wasmedgeup
cargo install --path .
```

## Usage

### Installing WasmEdge Runtime

Install the latest version:
```bash
wasmedgeup install latest
```

Install a specific version:
```bash
wasmedgeup install 0.14.1
```

Install with custom path:
```bash
wasmedgeup install 0.14.1 --path /usr/local
```

### Managing Plugins

List available plugins:
```bash
wasmedgeup plugin list
```

Install plugins:
```bash
wasmedgeup plugin install wasi-nn-ggml
wasmedgeup plugin install wasmedge-tensorflow-lite@0.2.0
```

Remove plugins:
```bash
wasmedgeup plugin remove wasi-nn-ggml
```

### Other Commands

List available WasmEdge versions:
```bash
wasmedgeup list
```

Remove WasmEdge installation:
```bash
wasmedgeup remove --path ~/.wasmedge
```

## Options

- `-V, --verbose`: Enable verbose output
- `-q, --quiet`: Disable progress output
- `-p, --path`: Set installation path (default: ~/.wasmedge)
- `-t, --tmpdir`: Set temporary directory (default: /tmp)
- `-o, --os`: Override OS detection
- `-a, --arch`: Override architecture detection

## Environment Variables

The installer will create an `env` file in the installation directory with the necessary environment variables. Source this file to use WasmEdge:

```bash
source ~/.wasmedge/env
```

## Platform Support

- Linux (x86_64, aarch64)
  - Ubuntu 20.04+
  - Generic Linux distributions
- macOS (x86_64, arm64)
  - Intel processors
  - Apple Silicon
- Windows (x86_64)

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## License

This project is licensed under the Apache License 2.0 - see the LICENSE file for details. 