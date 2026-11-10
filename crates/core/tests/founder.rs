//! The founder profile: saving, keeping and removing the photo.

use onecorp::founder::{Photo, load, name, remove, save};
use std::fs;

#[test]
fn a_bad_photo_never_loses_the_one_you_had() {
    let dir = std::env::temp_dir().join(format!("onecorp-founder-test-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    let good = dir.join("me.png");
    fs::write(&good, b"png").unwrap();
    let bad = dir.join("notes.txt");
    fs::write(&bad, b"text").unwrap();
    // Integration tests run in their own process, so this can't leak into other tests.
    unsafe { std::env::set_var("ONECORP_HOME", dir.join("home")) };

    assert!(save("  ", Photo::Keep).is_err());
    let f = save("Lucas", Photo::Use(&good)).unwrap();
    assert!(f.photo.as_ref().unwrap().exists());
    assert!(save("Lucas", Photo::Use(&bad)).is_err());
    assert!(load().unwrap().photo.is_some(), "the old photo survives a bad one");
    assert_eq!(save("Lucas O.", Photo::Keep).unwrap().name, "Lucas O.");
    assert!(load().unwrap().photo.is_some());
    save("Lucas", Photo::Remove).unwrap();
    assert!(load().unwrap().photo.is_none());
    assert_eq!(name(), "Lucas");

    save("Lucas", Photo::Use(&good)).unwrap();
    remove().unwrap();
    assert!(load().is_none() && !dir.join("home/founder-photo.png").exists(), "removing you takes the photo too");
    remove().unwrap();

    // A company's look: a color, and a logo that replaces the last one.
    let repo = dir.join("repo");
    fs::create_dir_all(&repo).unwrap();
    std::process::Command::new("git").arg("-C").arg(&repo).args(["init", "-q"]).status().unwrap();
    let c = onecorp::store::Company::create("acme", &repo, "").unwrap();
    assert_eq!(c.look().unwrap(), ("#27272a".to_string(), None));
    c.set_look("#d97757", Some(&good), false).unwrap();
    assert!(c.look().unwrap().1.unwrap().ends_with("logo.png"));
    assert!(c.set_look("orange", None, true).is_err());
    assert!(c.set_look("#d97757", Some(&bad), false).is_err());
    assert!(c.look().unwrap().1.is_some(), "a bad file keeps the logo you had");
    c.set_look("#3a9b7a", None, false).unwrap();
    assert_eq!(c.look().unwrap(), ("#3a9b7a".to_string(), None));
    let _ = fs::remove_dir_all(&dir);
}
