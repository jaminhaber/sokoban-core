use sokoban_core::{Collection, Level};
use std::fs;
use std::str::FromStr;

#[test]
fn load_collection_from_xsb() {
    let xsb = fs::read_to_string("assets/Microban_155.xsb").unwrap();
    let collection = Collection::from_xsb(&xsb);

    assert_eq!(collection.len(), 155);
    assert!(!collection.is_empty());
}

#[test]
fn collection_level_access() {
    let xsb = fs::read_to_string("assets/Microban_155.xsb").unwrap();
    let collection = Collection::from_xsb(&xsb);

    // Test accessing valid indices
    assert!(collection.level(0).is_some());
    assert!(collection.level(154).is_some());

    // Test accessing invalid indices
    assert!(collection.level(155).is_none());
    assert!(collection.level(1000).is_none());
}

#[test]
fn add_level() {
    let xsb = r#"
        #####
        #@$.#
        #####
    "#;
    let mut collection = Collection::from_xsb(xsb);
    let initial_len = collection.len();

    let new_level = Level::from_str(xsb).unwrap();
    collection.add_level(new_level);

    assert_eq!(collection.len(), initial_len + 1);
    assert!(collection.level(initial_len).is_some());
}

#[test]
fn insert_level() {
    let xsb = r#"
        #####
        #@$.#
        #####
    "#;
    let mut collection = Collection::from_xsb(xsb);
    let initial_len = collection.len();

    let new_level = Level::from_str(xsb).unwrap();

    // Insert at position 0
    assert!(collection.insert_level(0, new_level.clone()).is_ok());
    assert_eq!(collection.len(), initial_len + 1);

    // Insert at end
    assert!(collection
        .insert_level(collection.len(), new_level.clone())
        .is_ok());
    assert_eq!(collection.len(), initial_len + 2);

    // Insert at invalid position
    assert!(collection.insert_level(1000, new_level).is_err());
}

#[test]
fn remove_level() {
    let xsb = fs::read_to_string("assets/Microban_155.xsb").unwrap();
    let mut collection = Collection::from_xsb(&xsb);
    let initial_len = collection.len();

    // Remove from beginning
    let removed = collection.remove_level(0);
    assert!(removed.is_some());
    assert_eq!(collection.len(), initial_len - 1);

    // Remove from end
    let removed = collection.remove_level(collection.len() - 1);
    assert!(removed.is_some());
    assert_eq!(collection.len(), initial_len - 2);

    // Remove from invalid index
    let removed = collection.remove_level(1000);
    assert!(removed.is_none());
}

#[test]
fn replace_level() {
    let xsb = fs::read_to_string("assets/Microban_155.xsb").unwrap();
    let mut collection = Collection::from_xsb(&xsb);

    let new_level_str = r#"
        #####
        #@$.#
        #####
    "#;
    let new_level = Level::from_str(new_level_str).unwrap();
    let new_level_hash = new_level.map_hash();

    // Replace valid index
    let old = collection.replace_level(0, new_level.clone());
    assert!(old.is_some());
    assert_eq!(collection.level(0).unwrap().map_hash(), new_level_hash);

    // Replace invalid index
    let old = collection.replace_level(1000, new_level);
    assert!(old.is_none());
}

#[test]
fn swap_levels() {
    let xsb = fs::read_to_string("assets/Microban_155.xsb").unwrap();
    let mut collection = Collection::from_xsb(&xsb);

    let level_0_hash = collection.level(0).unwrap().map_hash();
    let level_1_hash = collection.level(1).unwrap().map_hash();

    // Swap valid indices
    assert!(collection.swap_levels(0, 1));
    assert_eq!(collection.level(0).unwrap().map_hash(), level_1_hash);
    assert_eq!(collection.level(1).unwrap().map_hash(), level_0_hash);

    // Swap invalid indices
    assert!(!collection.swap_levels(0, 1000));
    assert!(!collection.swap_levels(1000, 2000));
}

#[test]
fn level_mut_access() {
    let xsb = r#"
        #####
        #@$.#
        #####
    "#;
    let mut collection = Collection::from_xsb(xsb);

    // Test mutable access on valid index
    assert!(collection.level_mut(0).is_some());

    // Test mutable access on invalid index
    assert!(collection.level_mut(1).is_none());
}

#[test]
fn roundtrip_single_level() {
    let xsb_original = r#"
        #####
        #@$.#
        #####
        title: Test Level
        author: Test Author
    "#;

    // Load collection from XSB
    let collection = Collection::from_xsb(xsb_original);
    assert_eq!(collection.len(), 1);

    // Serialize back to XSB
    let xsb_serialized = collection.to_xsb();

    // Reload from serialized XSB
    let collection2 = Collection::from_xsb(&xsb_serialized);
    assert_eq!(collection2.len(), 1);

    // Verify semantic equivalence (same map hash, same metadata count)
    assert_eq!(
        collection.level(0).unwrap().map_hash(),
        collection2.level(0).unwrap().map_hash()
    );
}

#[test]
fn roundtrip_multiple_levels() {
    let xsb = fs::read_to_string("assets/Microban_155.xsb").unwrap();
    let collection = Collection::from_xsb(&xsb);
    let original_count = collection.len();

    // Serialize and reload
    let xsb_serialized = collection.to_xsb();
    let collection2 = Collection::from_xsb(&xsb_serialized);

    // Verify count is preserved
    assert_eq!(collection2.len(), original_count);

    // Verify each level is semantically equivalent
    for i in 0..original_count {
        let level1 = collection.level(i).unwrap();
        let level2 = collection2.level(i).unwrap();

        // Compare map hashes (semantic equivalence)
        assert_eq!(
            level1.map_hash(),
            level2.map_hash(),
            "Level {} map hashes don't match",
            i
        );
    }
}

#[test]
fn roundtrip_all_asset_collections() {
    let assets = [
        "assets/Microban_155.xsb",
        "assets/Microban II_135.xsb",
        "assets/BoxWorld_100.xsb",
        "assets/Holland_81.xsb",
        "assets/SokHard_163.xsb",
    ];

    for asset_path in &assets {
        let xsb = match fs::read_to_string(asset_path) {
            Ok(content) => content,
            Err(_) => continue, // Skip if file not found
        };

        // Parse collection
        let collection = Collection::from_xsb(&xsb);
        let original_count = collection.len();

        // Serialize back
        let xsb_serialized = collection.to_xsb();

        // Reload
        let collection2 = Collection::from_xsb(&xsb_serialized);

        // Verify count preserved
        assert_eq!(
            collection2.len(),
            original_count,
            "Level count mismatch in {}",
            asset_path
        );

        // Spot-check first and last levels
        if original_count > 0 {
            assert_eq!(
                collection.level(0).unwrap().map_hash(),
                collection2.level(0).unwrap().map_hash(),
                "First level hash mismatch in {}",
                asset_path
            );

            if original_count > 1 {
                assert_eq!(
                    collection.level(original_count - 1).unwrap().map_hash(),
                    collection2.level(original_count - 1).unwrap().map_hash(),
                    "Last level hash mismatch in {}",
                    asset_path
                );
            }
        }
    }
}

#[test]
fn display_trait() {
    let xsb = r#"
        #####
        #@$.#
        #####
    "#;
    let collection = Collection::from_xsb(xsb);

    // Test Display implementation
    let display_output = format!("{}", collection);

    // Verify output is not empty
    assert!(!display_output.is_empty());

    // Verify we can parse it again
    let collection2 = Collection::from_xsb(&display_output);
    assert_eq!(collection2.len(), 1);
}

#[test]
fn empty_collection() {
    let xsb = r#"
        ; Just a header, no levels
    "#;
    let collection = Collection::from_xsb(xsb);

    assert_eq!(collection.len(), 0);
    assert!(collection.is_empty());
    assert!(collection.level(0).is_none());
}

#[test]
fn collection_crud_operations() {
    let mut collection = Collection::from_xsb(
        r#"
            #####
            #@$.#
            #####
        "#,
    );

    assert_eq!(collection.len(), 1);

    // Add more levels
    let level2 = Level::from_str(
        r#"
        #######
        #     #
        # .$. #
        # $.$ #
        # .$. #
        # $.$ #
        #  @  #
        #######
    "#,
    )
    .unwrap();

    collection.add_level(level2);
    assert_eq!(collection.len(), 2);

    // Remove first
    let removed = collection.remove_level(0);
    assert!(removed.is_some());
    assert_eq!(collection.len(), 1);

    // Add it back at different position
    collection.insert_level(0, removed.unwrap()).unwrap();
    assert_eq!(collection.len(), 2);
}
