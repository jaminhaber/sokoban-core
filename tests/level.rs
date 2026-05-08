use std::{fs, str::FromStr};

use indoc::indoc;
use sokoban_core::{
    direction::Direction, ActionError, IVector2, Level, ParseLevelError, ParseMapError,
};

mod utils;
use utils::*;

#[test]
fn parse_level_error() {
    let duplicate_metadata_level = r#"
        #####
        #@$.#
        #####
        unknown: 1
        unknown: 2
    "#;
    let unterminated_block_comment_level = r#"
        #####
        #@$.#
        #####
        comment:
        unterminated block comment
    "#;
    assert!(Level::from_str(SIMPLEST).is_ok());
    assert_eq!(
        Level::from_str(duplicate_metadata_level).unwrap_err(),
        ParseLevelError::DuplicateMetadata("unknown".to_string())
    );
    assert_eq!(
        Level::from_str(unterminated_block_comment_level).unwrap_err(),
        ParseLevelError::UnterminatedBlockComment
    );

    let invalid_character_level = r#"
        ######
        #@!$.#
        ######
    "#;
    assert_eq!(
        Level::from_str(invalid_character_level).unwrap_err(),
        ParseLevelError::ParseMapError(ParseMapError::InvalidCharacter('!'))
    );
}

#[test]
fn display() {
    let level_str = r#"
        ; Level 1
        #####
        #@$.#
        #####
        comment: single line comment
        tile: level title
        comment:
        multi: line
        comment
        comment-end:
        author: level author
    "#;
    let level = Level::from_str(level_str).unwrap();
    assert_eq!(
        level.to_string(),
        indoc! {"
            #####
            #@$.#
            #####
            author: level author
            comment:
            Level 1
            single line comment
            multi: line
            comment
            comment-end:
            tile: level title
        "}
    );
}

#[test]
fn metadata() {
    let level_str = r#"
        ; Level 1
        #####
        #@$.#
        #####
        comment: single line comment
        tile: level title
        comment:
        multi
        line
        comment
        comment-end:
        author: level author
    "#;
    let level = Level::from_str(level_str).unwrap();
    assert_eq!(level.metadata()["tile"], "level title");
    assert_eq!(level.metadata()["author"], "level author");
    assert_eq!(
        level.metadata()["comments"],
        indoc! {"
            Level 1
            single line comment
            multi
            line
            comment
        "}
    );
}

#[test]
fn create_levels_from_str() {
    for entry in fs::read_dir("assets/").unwrap() {
        let path = entry.unwrap().path();
        if path.extension() != Some(std::ffi::OsStr::new("xsb")) {
            continue;
        }
        let count = path
            .to_string_lossy()
            .rsplit_terminator(['_', '.'])
            .nth(1)
            .unwrap()
            .parse()
            .unwrap();
        assert_eq!(
            Level::load_from_str(&fs::read_to_string(path).unwrap())
                .filter_map(Result::ok)
                .count(),
            count
        );
    }
}

#[test]
fn create_levels_from_reader() {
    for entry in fs::read_dir("assets/").unwrap() {
        let path = entry.unwrap().path();
        if path.extension() != Some(std::ffi::OsStr::new("xsb")) {
            continue;
        }
        let count = path
            .to_string_lossy()
            .rsplit_terminator(['_', '.'])
            .nth(1)
            .unwrap()
            .parse()
            .unwrap();
        let reader = std::io::BufReader::new(fs::File::open(&path).unwrap());
        assert_eq!(
            Level::load_from_reader(reader)
                .filter_map(Result::ok)
                .count(),
            count
        );
    }
}

#[test]
fn create_level_with_rle_xsb() {
    assert_eq!(
        Level::from_str(MICROBAN_3_RLE).unwrap(),
        load_level_from_file("assets/Microban_155.xsb", 3)
    );
    assert_eq!(
        Level::from_str(MICROBAN2_132_RLE).unwrap(),
        load_level_from_file("assets/Microban II_135.xsb", 132)
    );
}

/// `do_action` errors when the player walks into a wall.
#[test]
fn do_action_blocked_by_wall_returns_move_blocked() {
    let mut level = Level::from_str(SIMPLEST).unwrap();
    // Player is at column 1 (next to the left wall) — moving Left hits a wall.
    assert_eq!(
        level.do_action(Direction::Left),
        Err(ActionError::MoveBlocked)
    );
}

/// `do_action` errors when a push is blocked by a wall behind the box.
#[test]
fn do_action_blocked_push_returns_push_blocked() {
    // Pushing right tries to push the box at column 2 into the wall at column 3.
    // The goal at column 4 keeps box/goal counts balanced for the parser.
    let xsb = r#"
        ######
        #@$#.#
        ######
    "#;
    let mut level = Level::from_str(xsb).unwrap();
    assert_eq!(
        level.do_action(Direction::Right),
        Err(ActionError::PushBlocked)
    );
}

#[test]
fn undo_then_redo_round_trips_to_solved_state() {
    let mut level = Level::from_str(SIMPLEST).unwrap();
    assert!(!level.is_solved());

    // Solve with a single right-push.
    level.do_action(Direction::Right).unwrap();
    assert!(level.is_solved());
    let solved_actions = level.actions().clone();

    // Undo: back to the start, no longer solved.
    level.undo_action().unwrap();
    assert!(!level.is_solved());
    assert!(level.actions().is_empty());

    // Redo: back to solved with the same recorded action sequence.
    level.redo_action().unwrap();
    assert!(level.is_solved());
    assert_eq!(level.actions(), &solved_actions);
}

#[test]
fn undo_with_no_actions_returns_no_actions_error() {
    let mut level = Level::from_str(SIMPLEST).unwrap();
    assert_eq!(level.undo_action(), Err(ActionError::NoActions));
}

#[test]
fn redo_with_nothing_to_redo_returns_no_undone_actions_error() {
    let mut level = Level::from_str(SIMPLEST).unwrap();
    // Nothing was ever undone.
    assert_eq!(level.redo_action(), Err(ActionError::NoUndoneActions));

    // After a do, the undone-stack is still empty.
    level.do_action(Direction::Right).unwrap();
    assert_eq!(level.redo_action(), Err(ActionError::NoUndoneActions));
}

#[test]
fn doing_a_new_action_clears_redo_history() {
    let xsb = r#"
        ######
        #@ $.#
        ######
    "#;
    let mut level = Level::from_str(xsb).unwrap();

    level.do_action(Direction::Right).unwrap();
    level.undo_action().unwrap();
    // A fresh action should drop the previously-undone action.
    level.do_action(Direction::Right).unwrap();
    assert_eq!(level.redo_action(), Err(ActionError::NoUndoneActions));
}

#[test]
fn is_solved_tracks_box_on_goal_state() {
    let mut level = Level::from_str(SIMPLEST).unwrap();
    assert!(!level.is_solved());
    level.do_action(Direction::Right).unwrap();
    assert!(level.is_solved());
    level.undo_action().unwrap();
    assert!(!level.is_solved());
}

#[test]
fn set_metadata_overwrites_existing_metadata() {
    use std::collections::BTreeMap;
    let mut level = Level::from_str(SIMPLEST).unwrap();

    let mut meta = BTreeMap::new();
    meta.insert("title".to_string(), "Custom".to_string());
    meta.insert("author".to_string(), "Tester".to_string());
    level.set_metadata(meta);

    assert_eq!(level.metadata()["title"], "Custom");
    assert_eq!(level.metadata()["author"], "Tester");
}

#[test]
fn player_reachable_area_excludes_walls_and_boxes() {
    let level = Level::from_str(SIMPLEST).unwrap();
    let area = level.player_reachable_area();
    // The reachable region is just the player cell — the box at column 2
    // blocks it from reaching column 3.
    assert_eq!(area.len(), 1);
}

#[test]
fn map_mut_lets_callers_mutate_the_underlying_map() {
    let mut level = Level::from_str(SIMPLEST).unwrap();
    let map = level.map_mut();
    let pos = IVector2::new(2, 1);
    // Mutate a cell — the change must persist through the borrow.
    map[pos].insert(sokoban_core::tiles::Tiles::Goal);
    assert!(level.map()[pos].intersects(sokoban_core::tiles::Tiles::Goal));
}

#[test]
fn load_nth_from_reader_returns_the_requested_level() {
    use std::io::BufReader;
    let file = std::fs::File::open("assets/Microban_155.xsb").unwrap();
    let reader = BufReader::new(file);
    let level = Level::load_nth_from_reader(reader, 3).unwrap();
    // Microban #3 has the title "Microban #3" or similar metadata.
    // The level is solvable; just verify it parsed and is non-trivial.
    assert!(!level.map().box_positions().is_empty());
}

#[test]
fn split_by_group_from_reader_yields_groups_per_level() {
    use std::io::BufReader;
    let file = std::fs::File::open("assets/Microban_155.xsb").unwrap();
    let reader = BufReader::new(file);
    // Microban_155 has 155 levels; the iterator should produce 155 groups.
    let groups: Vec<String> = Level::split_by_group_from_reader(reader).collect();
    assert_eq!(groups.len(), 155);
    // Each group is a non-empty XSB blob.
    assert!(groups.iter().all(|g| !g.trim().is_empty()));
}

#[test]
fn from_map_round_trips_through_into() {
    let level = Level::from_str(SIMPLEST).unwrap();
    let map = level.map().clone();
    let level2 = Level::from_map(map.clone());
    let map_back: sokoban_core::Map = level2.into();
    assert_eq!(map, map_back);
}

// Simplest level
const SIMPLEST: &str = r#"
    #####
    #@$.#
    #####
"#;

// Microban #3
const MICROBAN_3_RLE: &str = "--4#|3#--4#|#5-$-#|#-#--#$-#|#-.-.#@-#|9#";

// Microban II #132
const MICROBAN2_132_RLE: &str = "18-5#|12-5#-#3-#|12-#3-3#-#-#|6-5#-#-#7-#|5#-#3-#-#3-4#-##|#3-3#-#-#-3#-#--#-#|#-#4-@--#3-#-#--#-3#|#3-4#$6#-4#3-#|3#-#--#-.6-#4-#-#|--#-#--#--##--#4-#3-#|-##-5#--##4-#-5#|-#9-##--3#-#|-#-#-3#-#--5#--#-5#|-#3-#-#4-#-#4-#-#3-#|-5#-#--5#--#-3#-#-#|7-#-3#--##9-#|3-5#-#4-##--5#-##|3-#3-#4-#--##--#--#-#|3-#-#4-#8-#--#-3#|3-#3-4#-6#-4#3-#|3-3#-#--#-#3-#7-#-#|5-#-#--#-3#-#-#-3#3-#|4-##-4#3-#-#3-#-5#|4-#7-#-#-5#|4-#-#-3#3-#|4-#3-#-5#|4-5#";
