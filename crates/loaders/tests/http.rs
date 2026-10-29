use super::*;

#[test]
fn maven_coordinates_map_to_repository_paths() {
    assert_eq!(
        maven_jar_path("net.fabricmc:fabric-loader:0.16.14").as_deref(),
        Some("net/fabricmc/fabric-loader/0.16.14/fabric-loader-0.16.14.jar")
    );
    assert_eq!(
        maven_jar_path("net.fabricmc:sponge-mixin:0.12.5+mixin.0.8.5").as_deref(),
        Some("net/fabricmc/sponge-mixin/0.12.5+mixin.0.8.5/sponge-mixin-0.12.5+mixin.0.8.5.jar")
    );
    assert_eq!(
        maven_jar_path("org.ow2.asm:asm:9.7.1:sysv").as_deref(),
        Some("org/ow2/asm/asm/9.7.1/asm-9.7.1-sysv.jar")
    );
    assert_eq!(maven_jar_path("not-a-coordinate"), None);
}

#[test]
fn maven_bases_keep_a_single_trailing_slash() {
    assert_eq!(maven_base("https://maven.fabricmc.net/"), "https://maven.fabricmc.net/");
    assert_eq!(maven_base("https://maven.fabricmc.net"), "https://maven.fabricmc.net/");
}
