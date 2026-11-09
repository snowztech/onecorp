//! The founder profile: saving, keeping and removing the photo.

use onecorp::founder::{Photo, load, name, save};
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
    let _ = fs::remove_dir_all(&dir);
}
