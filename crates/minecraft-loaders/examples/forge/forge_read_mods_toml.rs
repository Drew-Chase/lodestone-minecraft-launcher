use minecraft_modloaders::forge::{DependencyOrdering, DependencySide, ForgeModsToml};
use std::env;

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: {} <path-to-forge-mod.jar>", args[0]);
        eprintln!("\nExample:");
        eprintln!("  {} ./mods/example-mod-1.0.0.jar", args[0]);
        std::process::exit(1);
    }

    let jar_path = &args[1];
    println!("Reading mods.toml from: {}\n", jar_path);

    match ForgeModsToml::from_jar(jar_path) {
        Ok(mods_toml) => {
            println!("=== Forge Configuration ===");
            println!("Mod Loader: {}", mods_toml.mod_loader);
            println!("Loader Version: {}", mods_toml.loader_version);
            println!("License: {}", mods_toml.license);

            if let Some(show_pack) = mods_toml.show_as_resource_pack {
                println!("Show as Resource Pack: {}", show_pack);
            }

            if let Some(client_only) = mods_toml.client_side_only {
                println!("Client Side Only: {}", client_only);
            }

            if let Some(issue_url) = &mods_toml.issue_tracker_url {
                println!("Issue Tracker: {}", issue_url);
            }

            // Display properties
            if let Some(properties) = &mods_toml.properties {
                println!("\n=== Properties ===");
                for (key, value) in properties {
                    println!("  {} = {}", key, value);
                }
            }

            // Display all mods
            println!("\n=== Mods ({}) ===", mods_toml.mods.len());
            for mod_def in &mods_toml.mods {
                println!("\n  Mod ID: {}", mod_def.mod_id);

                if let Some(name) = &mod_def.display_name {
                    println!("  Name: {}", name);
                }

                if let Some(version) = &mod_def.version {
                    println!("  Version: {}", version);
                }

                if let Some(namespace) = &mod_def.namespace {
                    println!("  Namespace: {}", namespace);
                }

                if let Some(desc) = &mod_def.description {
                    println!("  Description: {}", desc);
                }

                if let Some(authors) = &mod_def.authors {
                    println!("  Authors: {}", authors);
                }

                if let Some(credits) = &mod_def.credits {
                    println!("  Credits: {}", credits);
                }

                if let Some(logo) = &mod_def.logo_file {
                    println!("  Logo: {}", logo);
                }

                if let Some(mod_url) = &mod_def.mod_url {
                    println!("  Mod URL: {}", mod_url);
                }

                if let Some(display_url) = &mod_def.display_url {
                    println!("  Display URL: {}", display_url);
                }

                if let Some(update_url) = &mod_def.update_json_url {
                    println!("  Update JSON URL: {}", update_url);
                }

                // Display features
                if let Some(features) = &mod_def.features {
                    println!("\n  Features:");
                    for (key, value) in features {
                        println!("    {} = {}", key, value);
                    }
                }

                // Display mod properties
                if let Some(modprops) = &mod_def.modproperties {
                    println!("\n  Mod Properties:");
                    for (key, value) in modprops {
                        println!("    {} = {}", key, value);
                    }
                }

                // Display dependencies for this mod
                if let Some(deps) = mods_toml.get_dependencies(&mod_def.mod_id) {
                    println!("\n  Dependencies ({}):", deps.len());
                    for dep in deps {
                        print!("    - {}", dep.mod_id);

                        if dep.mandatory {
                            print!(" [REQUIRED]");
                        } else {
                            print!(" [OPTIONAL]");
                        }

                        if let Some(version) = &dep.version_range {
                            print!(" version: {}", version);
                        }

                        if let Some(ordering) = &dep.ordering {
                            let order_str = match ordering {
                                DependencyOrdering::Before => "BEFORE",
                                DependencyOrdering::After => "AFTER",
                                DependencyOrdering::None => "NONE",
                            };
                            print!(" ordering: {}", order_str);
                        }

                        if let Some(side) = &dep.side {
                            let side_str = match side {
                                DependencySide::Client => "CLIENT",
                                DependencySide::Server => "SERVER",
                                DependencySide::Both => "BOTH",
                            };
                            print!(" side: {}", side_str);
                        }

                        if let Some(url) = &dep.referral_url {
                            print!(" referral: {}", url);
                        }

                        println!();
                    }
                }
            }

            println!("\n✓ Successfully read mods.toml");
            Ok(())
        }
        Err(e) => {
            eprintln!("✗ Error reading mods.toml: {}", e);
            std::process::exit(1);
        }
    }
}
