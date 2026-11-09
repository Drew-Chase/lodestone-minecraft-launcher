use super::*;

#[test]
fn a_jar_name_reads_as_the_mod_it_names() {
    assert_eq!(pretty_name("sodium-fabric-0.6.13.jar"), "Sodium Fabric");
    assert_eq!(pretty_name("Iris-1.6.0.jar"), "Iris");
    assert_eq!(pretty_name("ae2-15.0.0.jar"), "Ae2");
    assert_eq!(pretty_name("some_mod_name_2.1.jar"), "Some Mod Name");
}

#[test]
fn a_name_without_a_version_keeps_all_of_its_words() {
    assert_eq!(pretty_name("just-a-mod.jar"), "Just A Mod");
    assert_eq!(pretty_name("noextension"), "Noextension");
}
