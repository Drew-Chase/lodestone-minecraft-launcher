use minecraft_modloaders::fabric::{DependencyVersion, EntryPoint, FabricModJson, Person};
use std::env;

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: {} <path-to-fabric-mod.jar>", args[0]);
        eprintln!("\nExample:");
        eprintln!("  {} ./mods/example-mod-1.0.0.jar", args[0]);
        std::process::exit(1);
    }

    let jar_path = &args[1];
    println!("Reading fabric.mod.json from: {}\n", jar_path);

    match FabricModJson::from_jar(jar_path) {
        Ok(mod_json) => {
            println!("=== Mod Information ===");
            println!("ID: {}", mod_json.id);
            println!("Name: {}", mod_json.name.as_deref().unwrap_or(&mod_json.id));
            println!("Version: {}", mod_json.version);
            println!("Schema Version: {}", mod_json.schema_version);

            if let Some(desc) = &mod_json.description {
                println!("\nDescription: {}", desc);
            }

            if let Some(env) = &mod_json.environment {
                println!("Environment: {:?}", env);
            }

            // Display license information
            if let Some(license) = &mod_json.license {
                match license {
                    minecraft_modloaders::fabric::License::Single(l) => {
                        println!("License: {}", l);
                    }
                    minecraft_modloaders::fabric::License::Multiple(licenses) => {
                        println!("Licenses: {}", licenses.join(", "));
                    }
                }
            }

            // Display authors
            if let Some(authors) = &mod_json.authors {
                println!("\n=== Authors ===");
                for author in authors {
                    match author {
                        Person::Name(name) => println!("  - {}", name),
                        Person::Detailed(details) => {
                            print!("  - {}", details.name);
                            if let Some(contact) = &details.contact
                                && let Some(email) = &contact.email
                            {
                                print!(" <{}>", email);
                            }
                            println!();
                        }
                    }
                }
            }

            // Display contributors
            if let Some(contributors) = &mod_json.contributors {
                println!("\n=== Contributors ===");
                for contributor in contributors {
                    match contributor {
                        Person::Name(name) => println!("  - {}", name),
                        Person::Detailed(details) => println!("  - {}", details.name),
                    }
                }
            }

            // Display contact information
            if let Some(contact) = &mod_json.contact {
                println!("\n=== Contact Information ===");
                if let Some(homepage) = &contact.homepage {
                    println!("Homepage: {}", homepage);
                }
                if let Some(sources) = &contact.sources {
                    println!("Sources: {}", sources);
                }
                if let Some(issues) = &contact.issues {
                    println!("Issues: {}", issues);
                }
                if let Some(email) = &contact.email {
                    println!("Email: {}", email);
                }
                if let Some(irc) = &contact.irc {
                    println!("IRC: {}", irc);
                }
            }

            // Display provides (mod aliases)
            if let Some(provides) = &mod_json.provides {
                println!("\n=== Provides (Aliases) ===");
                for alias in provides {
                    println!("  - {}", alias);
                }
            }

            // Display dependencies
            if let Some(deps) = &mod_json.depends {
                println!("\n=== Dependencies (Required) ===");
                for (mod_id, version) in deps {
                    print!("  - {}: ", mod_id);
                    match version {
                        DependencyVersion::Single(v) => println!("{}", v),
                        DependencyVersion::Multiple(versions) => {
                            println!("{}", versions.join(" OR "))
                        }
                    }
                }
            }

            // Display recommended dependencies
            if let Some(recommends) = &mod_json.recommends {
                println!("\n=== Recommended Dependencies ===");
                for (mod_id, version) in recommends {
                    print!("  - {}: ", mod_id);
                    match version {
                        DependencyVersion::Single(v) => println!("{}", v),
                        DependencyVersion::Multiple(versions) => {
                            println!("{}", versions.join(" OR "))
                        }
                    }
                }
            }

            // Display suggested dependencies
            if let Some(suggests) = &mod_json.suggests {
                println!("\n=== Suggested Dependencies ===");
                for (mod_id, version) in suggests {
                    print!("  - {}: ", mod_id);
                    match version {
                        DependencyVersion::Single(v) => println!("{}", v),
                        DependencyVersion::Multiple(versions) => {
                            println!("{}", versions.join(" OR "))
                        }
                    }
                }
            }

            // Display incompatibilities
            if let Some(breaks) = &mod_json.breaks {
                println!("\n=== Breaks (Incompatible) ===");
                for (mod_id, version) in breaks {
                    print!("  - {}: ", mod_id);
                    match version {
                        DependencyVersion::Single(v) => println!("{}", v),
                        DependencyVersion::Multiple(versions) => {
                            println!("{}", versions.join(" OR "))
                        }
                    }
                }
            }

            // Display conflicts
            if let Some(conflicts) = &mod_json.conflicts {
                println!("\n=== Conflicts ===");
                for (mod_id, version) in conflicts {
                    print!("  - {}: ", mod_id);
                    match version {
                        DependencyVersion::Single(v) => println!("{}", v),
                        DependencyVersion::Multiple(versions) => {
                            println!("{}", versions.join(" OR "))
                        }
                    }
                }
            }

            // Display entrypoints
            if let Some(entrypoints) = &mod_json.entrypoints {
                println!("\n=== Entry Points ===");
                for (type_name, entries) in entrypoints {
                    println!("  {}: {} entrypoint(s)", type_name, entries.len());
                    for entry in entries {
                        match entry {
                            EntryPoint::String(class) => println!("    - {}", class),
                            EntryPoint::Object(obj) => {
                                print!("    - {}", obj.value);
                                if let Some(adapter) = &obj.adapter {
                                    print!(" (adapter: {})", adapter);
                                }
                                println!();
                            }
                        }
                    }
                }
            }

            // Display language adapters
            if let Some(adapters) = &mod_json.language_adapters {
                println!("\n=== Language Adapters ===");
                for (lang, adapter_class) in adapters {
                    println!("  {}: {}", lang, adapter_class);
                }
            }

            // Display mixins
            if let Some(mixins) = &mod_json.mixins {
                println!("\n=== Mixins ===");
                for mixin in mixins {
                    match mixin {
                        minecraft_modloaders::fabric::MixinConfig::String(path) => {
                            println!("  - {}", path);
                        }
                        minecraft_modloaders::fabric::MixinConfig::Object(obj) => {
                            print!("  - {}", obj.config);
                            if let Some(env) = &obj.environment {
                                print!(" (environment: {:?})", env);
                            }
                            println!();
                        }
                    }
                }
            }

            // Display nested JARs
            if let Some(jars) = &mod_json.jars {
                println!("\n=== Nested JARs ===");
                for jar in jars {
                    println!("  - {}", jar.file);
                }
            }

            // Display access widener
            if let Some(access_widener) = &mod_json.access_widener {
                println!("\nAccess Widener: {}", access_widener);
            }

            // Display custom fields
            if let Some(custom) = &mod_json.custom {
                println!("\n=== Custom Fields ===");
                for kvp in custom {
                    println!("  - {}", kvp.0);
                }
            }

            println!("\n✓ Successfully read fabric.mod.json");
            Ok(())
        }
        Err(e) => {
            eprintln!("✗ Error reading fabric.mod.json: {}", e);
            std::process::exit(1);
        }
    }
}
