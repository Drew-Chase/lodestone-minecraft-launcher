use std::path::Path;

use super::*;
use crate::app::launch::manifest::Manifest;

const LIBS: &str = "/libs";

const WINDOWS: Platform = Platform {
    os: "windows",
    arch: "x86_64",
};

const LINUX: Platform = Platform { os: "linux", arch: "x86_64" };

fn manifest(json: &str) -> Manifest {
    Manifest::parse(json).expect("the fixture parses")
}

const MIXED: &str = r#"{"libraries":[
  {"name":"org.lwjgl:lwjgl:3.3.3","downloads":{"artifact":{"path":"org/lwjgl/lwjgl/3.3.3/lwjgl-3.3.3.jar","sha1":"aa","size":10,"url":"https://libs.example/lwjgl.jar"}}},
  {"name":"com.mac:only:1.0","rules":[{"action":"allow","os":{"name":"osx"}}],"downloads":{"artifact":{"path":"com/mac/only/1.0/only-1.0.jar","sha1":"bb","size":5,"url":"https://libs.example/only.jar"}}},
  {"name":"org.lwjgl:lwjgl:3.3.3:natives-windows","rules":[{"action":"allow","os":{"name":"windows"}}],"downloads":{"artifact":{"path":"org/lwjgl/lwjgl/3.3.3/lwjgl-3.3.3-natives-windows.jar","sha1":"cc","size":7,"url":"https://libs.example/lwjgl-natives.jar"}}},
  {"name":"org.lwjgl:lwjgl-old:2.9.4","natives":{"windows":"natives-windows","linux":"natives-linux"},"downloads":{"classifiers":{"natives-windows":{"path":"org/lwjgl/lwjgl-old/2.9.4/lwjgl-old-2.9.4-natives-windows.jar","sha1":"dd","size":8,"url":"https://libs.example/old-natives.jar"}}},"extract":{"exclude":["META-INF/"]}}
]}"#;

#[test]
fn windows_takes_its_jars_and_its_natives_and_nothing_else() {
    let plan = plan(&manifest(MIXED), Path::new(LIBS), WINDOWS);
    assert_eq!(
        plan.classpath,
        vec![
            Path::new("/libs/org/lwjgl/lwjgl/3.3.3/lwjgl-3.3.3.jar").to_path_buf(),
            Path::new("/libs/org/lwjgl/lwjgl/3.3.3/lwjgl-3.3.3-natives-windows.jar").to_path_buf(),
        ],
        "the new-style natives jar is a plain library, so it stays on the classpath"
    );
    assert_eq!(plan.natives.len(), 1, "only the old-style native archive is extracted");
    assert_eq!(
        plan.natives[0].archive,
        Path::new("/libs/org/lwjgl/lwjgl-old/2.9.4/lwjgl-old-2.9.4-natives-windows.jar")
    );
    assert_eq!(plan.natives[0].exclude, vec!["META-INF/".to_string()]);
    assert_eq!(
        plan.items.len(),
        3,
        "the macOS-only library is never planned: its rule does not allow Windows"
    );
}

#[test]
fn linux_gets_only_the_jar_it_is_allowed() {
    let plan = plan(&manifest(MIXED), Path::new(LIBS), LINUX);
    assert_eq!(
        plan.classpath,
        vec![Path::new("/libs/org/lwjgl/lwjgl/3.3.3/lwjgl-3.3.3.jar").to_path_buf()]
    );
    assert_eq!(plan.natives.len(), 1, "the old natives have a Linux classifier too");
    assert_eq!(
        plan.natives[0].archive,
        Path::new("/libs/org/lwjgl/lwjgl-old/2.9.4/lwjgl-old-2.9.4-natives-linux.jar"),
        "the Linux classifier is not declared, so the path comes from the coordinates"
    );
}

#[test]
fn a_loader_library_without_a_declared_file_is_fetched_from_its_repository() {
    let plan = plan(
        &manifest(r#"{"libraries":[{"name":"net.fabricmc:fabric-loader:0.16.10","url":"https://maven.fabricmc.net/"}]}"#),
        Path::new(LIBS),
        WINDOWS,
    );
    let [item] = plan.items.as_slice() else {
        panic!("one library, one item");
    };
    assert_eq!(
        item.url.as_deref(),
        Some("https://maven.fabricmc.net/net/fabricmc/fabric-loader/0.16.10/fabric-loader-0.16.10.jar")
    );
    assert_eq!(item.path, Path::new("/libs/net/fabricmc/fabric-loader/0.16.10/fabric-loader-0.16.10.jar"));
    assert_eq!(item.sha1, None);
}

#[test]
fn a_loader_library_with_no_repository_must_already_be_on_disk() {
    let plan = plan(
        &manifest(r#"{"libraries":[{"name":"com.forge:installed:1.0"}]}"#),
        Path::new(LIBS),
        WINDOWS,
    );
    assert_eq!(plan.items.len(), 1);
    assert_eq!(plan.items[0].url, None);
    assert_eq!(plan.classpath.len(), 1);
}

#[test]
fn a_jar_listed_twice_is_planned_and_put_on_the_classpath_once() {
    let json = r#"{"libraries":[
        {"name":"a:b:1.0","downloads":{"artifact":{"path":"a/b/1.0/b-1.0.jar","url":"https://x.example/b.jar"}}},
        {"name":"a:b:1.0","downloads":{"artifact":{"path":"a/b/1.0/b-1.0.jar","url":"https://x.example/b.jar"}}}
    ]}"#;
    let plan = plan(&manifest(json), Path::new(LIBS), WINDOWS);
    assert_eq!(plan.classpath.len(), 1);
    assert_eq!(plan.items.len(), 1);
}

#[test]
fn maven_paths_follow_the_repository_layout() {
    assert_eq!(
        maven_path("org.ow2.asm:asm:9.6", None).as_deref(),
        Some("org/ow2/asm/asm/9.6/asm-9.6.jar")
    );
    assert_eq!(maven_path("a:b:1", Some("natives-linux")).as_deref(), Some("a/b/1/b-1-natives-linux.jar"));
    assert_eq!(maven_path("a:b:1:cls", None).as_deref(), Some("a/b/1/b-1-cls.jar"));
    assert_eq!(maven_path("a:b:1@zip", None).as_deref(), Some("a/b/1/b-1.zip"));
    assert_eq!(maven_path("not-a-coordinate", None), None);
}
