use sokoban_core::*;
use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};

#[test]
fn test_boxset_in_hashmap() {
    // This test verifies that BoxSets can be properly used as HashMap keys
    // The bug was that equal sets could have different hashes, breaking HashMap lookup
    let width = 10;
    
    // Create a BoxSet and insert positions in one order
    let mut set1 = BoxSet::new(width);
    set1.insert(IVector2::new(1, 2));
    set1.insert(IVector2::new(5, 5));
    set1.insert(IVector2::new(9, 9));
    
    // Store something in a HashMap with this set as key
    let mut map = HashMap::new();
    map.insert(set1.clone(), 42);

    // Verify we can find it again
    assert_eq!(
        map.get(&set1),
        Some(&42),
        "Should be able to find BoxSet in HashMap"
    );

    // Create another BoxSet with same positions but inserted in different order
    let mut set2 = BoxSet::new(width);
    set2.insert(IVector2::new(9, 9));
    set2.insert(IVector2::new(1, 2));
    set2.insert(IVector2::new(5, 5));
    
    // Verify sets are equal
    assert_eq!(set1, set2, "BoxSets with same positions should be equal");
    
    // Verify we can find it with the second set (this would fail with broken hash)
    assert_eq!(
        map.get(&set2),
        Some(&42),
        "Should be able to find BoxSet with equal key (hash contract)"
    );
}

#[test]
fn test_boxset_hash_determinism() {
    let width = 10;

    // Create two BoxSets with same positions inserted in different orders
    let mut set1 = BoxSet::new(width);
    set1.insert(IVector2::new(1, 2));
    set1.insert(IVector2::new(5, 5));
    set1.insert(IVector2::new(9, 9));

    let mut set2 = BoxSet::new(width);
    set2.insert(IVector2::new(9, 9));
    set2.insert(IVector2::new(1, 2));
    set2.insert(IVector2::new(5, 5));

    // Verify equality
    assert_eq!(set1, set2, "BoxSets with same positions should be equal");

    // Verify hash equality
    let hash1 = {
        let mut hasher = DefaultHasher::new();
        set1.hash(&mut hasher);
        hasher.finish()
    };

    let hash2 = {
        let mut hasher = DefaultHasher::new();
        set2.hash(&mut hasher);
        hasher.finish()
    };

    assert_eq!(
        hash1, hash2,
        "Equal BoxSets must have equal hashes (hash contract)"
    );
}
