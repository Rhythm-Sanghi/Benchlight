use benchlight_platform_windows::{ensure_local_root, protected_locations};
use std::path::Path;

#[test]
fn unc_roots_fail_without_a_network_access_and_os_protections_are_available() {
    assert!(
        ensure_local_root(Path::new(r"\\benchlight-offline-test.invalid\share\source")).is_err()
    );
    assert!(
        ensure_local_root(Path::new(
            r"\\?\UNC\benchlight-offline-test.invalid\share\source"
        ))
        .is_err()
    );
    let (directories, profile) = protected_locations().unwrap();
    assert_eq!(directories.len(), 4);
    assert!(directories.iter().all(|path| path.is_absolute()));
    assert!(profile.is_absolute());
}
