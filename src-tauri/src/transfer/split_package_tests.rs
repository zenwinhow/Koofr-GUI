use std::process::Command;

use super::split_package::{
    MAX_PART_BYTES, MIN_PART_BYTES, SplitManifest, SplitPart, package_directory_name, restore_cmd,
    validate_part_bytes,
};

#[test]
fn package_names_preserve_the_original_name_and_transfer_identity() {
    // Given
    let transfer_id = uuid::Uuid::new_v4();

    // When
    let directory = package_directory_name("movie.mkv", transfer_id);

    // Then
    assert_eq!(directory, format!("movie.mkv.parts-{transfer_id}"));
}

#[test]
fn accepts_only_practical_custom_part_sizes() {
    assert_eq!(
        validate_part_bytes(MIN_PART_BYTES).expect("minimum part size"),
        MIN_PART_BYTES
    );
    assert_eq!(
        validate_part_bytes(MAX_PART_BYTES).expect("maximum part size"),
        MAX_PART_BYTES
    );
    assert!(validate_part_bytes(MIN_PART_BYTES - 1).is_err());
    assert!(validate_part_bytes(MAX_PART_BYTES + 1).is_err());
}

#[test]
fn manifest_requires_contiguous_parts_that_cover_the_file() {
    // Given
    let parts = vec![
        SplitPart::new(0, 64, "a".repeat(64)),
        SplitPart::new(1, 36, "b".repeat(64)),
    ];

    // When
    let manifest = SplitManifest::new("archive.tar".to_owned(), 100, 64, "c".repeat(64), parts)
        .expect("valid manifest");

    // Then
    let readme = manifest.readme();
    assert!(readme.contains("copy /b /y part-*.bin"));
    assert!(readme.contains("cat part-*.bin"));
    assert!(readme.contains("archive.tar"));
}

#[test]
fn windows_restore_script_reassembles_raw_parts_with_builtin_copy() {
    // Given
    let directory =
        std::env::temp_dir().join(format!("koofr-restore-test-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&directory).expect("create restore test directory");
    std::fs::write(directory.join("part-000001.bin"), b"first-").expect("write first part");
    std::fs::write(directory.join("part-000002.bin"), b"second").expect("write second part");
    std::fs::write(directory.join("restore.cmd"), restore_cmd()).expect("write restore script");

    // When
    let status = Command::new("cmd.exe")
        .args(["/d", "/c", "restore.cmd", "restored.bin"])
        .current_dir(&directory)
        .status()
        .expect("run restore script");

    // Then
    assert!(status.success());
    assert_eq!(
        std::fs::read(directory.join("restored.bin")).expect("read restored file"),
        b"first-second"
    );

    std::fs::remove_dir_all(directory).expect("remove restore test directory");
}
