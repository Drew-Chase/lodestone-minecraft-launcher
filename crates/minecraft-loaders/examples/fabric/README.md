# Fabric Loader Examples

This directory contains examples demonstrating how to use the Fabric modloader API.

## Running Examples

All examples can be run using `cargo run --example <example_name>`:

```bash
# Fetch and display available Fabric versions
cargo run --example fabric_fetch_versions

# Download a Fabric server JAR
cargo run --example fabric_download_server_jar

# Install Fabric client
cargo run --example fabric_install_client

# Install Fabric server
cargo run --example fabric_install_server

# Work with specific versions
cargo run --example fabric_specific_version
```

## Examples Overview

### 1. `fabric_fetch_versions.rs`
Demonstrates how to fetch and display available Fabric versions from the API.

**What it does:**
- Fetches all available versions from Fabric Meta API
- Displays latest stable versions (game, loader, installer)
- Shows counts of available versions
- Lists recent stable game and loader versions

**Usage:**
```bash
cargo run --example fabric_fetch_versions
```

### 2. `fabric_download_server_jar.rs`
Shows how to download a Fabric server JAR file.

**What it does:**
- Fetches available versions
- Gets the latest stable game and loader versions
- Downloads the Fabric server JAR to `./server` directory
- Displays download progress and file size

**Usage:**
```bash
cargo run --example fabric_download_server_jar
```

**Output:** `./server/fabric-server-<version>-<loader>.jar`

### 3. `fabric_install_client.rs`
Demonstrates installing Fabric client using the installer.

**What it does:**
- Fetches latest versions
- Downloads the Fabric installer
- Runs the installer to set up a Fabric client instance
- Creates a complete client installation in `./fabric-instance`

**Requirements:**
- Java must be installed and available in PATH
- Alternatively, specify a custom Java path in the code

**Usage:**
```bash
cargo run --example fabric_install_client
```

**Output:** `./fabric-instance/` (complete Fabric client installation)

### 4. `fabric_install_server.rs`
Shows how to install a complete Fabric server.

**What it does:**
- Fetches latest versions
- Downloads the Fabric installer
- Runs the installer to set up a Fabric server
- Downloads the Minecraft server JAR
- Creates a complete server installation in `./fabric-server`

**Requirements:**
- Java must be installed and available in PATH
- Alternatively, specify a custom Java path in the code

**Usage:**
```bash
cargo run --example fabric_install_server
```

**Output:** `./fabric-server/` (complete Fabric server installation)

**Starting the server:**
```bash
cd fabric-server
java -jar fabric-server-launch.jar nogui
```

### 5. `fabric_specific_version.rs`
Demonstrates working with specific Fabric versions.

**What it does:**
- Shows how to find specific Minecraft and loader versions
- Downloads server JAR for specific versions (e.g., Minecraft 1.20.1 with Fabric Loader 0.15.0)
- Lists available loader versions for reference
- Demonstrates fallback to latest version if specific version not found

**Usage:**
```bash
cargo run --example fabric_specific_version
```

**Customization:**
Edit the example to specify your desired versions:
```rust
let minecraft_version = "1.20.1";
let loader_version_string = "0.15.0";
```

## API Usage Patterns

### Fetching Versions
```rust
use minecraft_modloaders::fabric::FabricVersions;

let versions = FabricVersions::fetch().await?;
```

### Getting Latest Versions
```rust
let latest_game = versions.get_latest_game_version()?;
let latest_loader = versions.get_latest_loader()?;
let latest_installer = versions.get_latest_installer()?;
```

### Finding Specific Versions
```rust
let game = versions.find_game_version("1.20.1")?;
let loader = versions.find_loader("0.15.0")?;
```

### Downloading Server JAR
```rust
let loader = versions.get_latest_loader()?;
let path = loader.download_server_jar("1.20.1", "./server.jar").await?;
```

### Installing Client
```rust
let installer = versions.get_latest_installer()?;
installer.install_client(
    "1.20.1",
    "0.15.0",
    "./instance",
    "java"
).await?;
```

### Installing Server
```rust
let installer = versions.get_latest_installer()?;
let server_jar = installer.install_server(
    "1.20.1",
    "0.15.0",
    "./server",
    "java"
).await?;
```

## Notes

- All downloads are performed asynchronously using tokio
- The API client automatically retries failed requests
- Downloaded files are verified for successful HTTP status codes
- Examples include cleanup code where appropriate
- Java path can be customized for non-standard installations

## Requirements

- Rust 1.70 or later
- tokio runtime (included in examples)
- Active internet connection
- Java (for install_client and install_server examples)

## Troubleshooting

**"Java not found" error:**
- Ensure Java is installed: `java -version`
- Add Java to your PATH, or specify full path in code:
  ```rust
  let java_path = PathBuf::from("C:/Program Files/Java/jdk-17/bin/java.exe");
  ```

**Network errors:**
- Check internet connection
- Verify Fabric Meta API is accessible: https://meta.fabricmc.net/v2/versions/

**Permission errors:**
- Ensure write permissions for output directories
- Try running with elevated privileges if needed
