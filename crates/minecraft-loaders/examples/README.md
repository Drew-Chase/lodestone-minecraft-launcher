# Minecraft Modloader Examples

This directory contains examples for working with various Minecraft modloaders.

## Structure

Examples are organized by modloader:

```
examples/
├── fabric/          # Fabric modloader examples
│   ├── fabric_fetch_versions.rs
│   ├── fabric_download_server_jar.rs
│   ├── fabric_install_client.rs
│   ├── fabric_install_server.rs
│   ├── fabric_specific_version.rs
│   └── README.md
├── forge/           # Forge modloader examples (coming soon)
├── neoforge/        # NeoForge modloader examples (coming soon)
└── quilt/           # Quilt modloader examples (coming soon)
```

## Available Modloaders

### ✅ Fabric
Complete implementation with examples for:
- Fetching available versions
- Downloading server JARs
- Installing client instances
- Installing servers
- Working with specific versions

See [fabric/README.md](fabric/README.md) for detailed documentation.

**Run Fabric examples:**
```bash
cargo run --example fabric_fetch_versions
cargo run --example fabric_download_server_jar
cargo run --example fabric_install_client
cargo run --example fabric_install_server
cargo run --example fabric_specific_version
```

### 🚧 Forge (Coming Soon)
Forge modloader support is planned for future releases.

### 🚧 NeoForge (Coming Soon)
NeoForge modloader support is planned for future releases.

### 🚧 Quilt (Coming Soon)
Quilt modloader support is planned for future releases.

## Requirements

- Rust 1.70 or later
- tokio runtime (included in dependencies)
- Active internet connection
- Java (for client/server installation examples)

## Contributing

When adding new modloader examples, please follow the existing structure:
1. Create a subdirectory for the modloader (e.g., `examples/forge/`)
2. Prefix example names with the modloader (e.g., `forge_install_client.rs`)
3. Include a detailed README.md in the modloader subdirectory
4. Add example entries to `Cargo.toml`
