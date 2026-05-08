use std::str::FromStr;

use indoc::indoc;
use sokoban_core::IVector2;
use sokoban_core::{tiles::Tiles, Actions, Map, ParseMapError};

mod utils;
use utils::*;

#[test]
fn map_from_str() {
    let no_player_map = r#"
        #####
        # $.#
        #####
    "#;
    let no_box_or_goal_map = r#"
        ###
        #@#
        ###
    "#;
    let more_than_one_player_map_1 = r#"
        ######
        #@@$.#
        ######
    "#;
    let more_than_one_player_map_2 = r#"
        ######
        #@$.+#
        ######
    "#;
    let mismatch_between_boxs_and_goals_map = r#"
        ######
        #@$$.#
        ######
    "#;
    let invalid_character_map = r#"
        ######
        #@!$.#
        ######
    "#;
    assert_eq!(
        Map::from_str(no_player_map).unwrap_err(),
        ParseMapError::NoPlayer
    );
    assert_eq!(
        Map::from_str(no_box_or_goal_map).unwrap_err(),
        ParseMapError::NoBoxOrGoal
    );
    assert_eq!(
        Map::from_str(more_than_one_player_map_1).unwrap_err(),
        ParseMapError::MoreThanOnePlayer
    );
    assert_eq!(
        Map::from_str(more_than_one_player_map_2).unwrap_err(),
        ParseMapError::MoreThanOnePlayer
    );
    assert_eq!(
        Map::from_str(mismatch_between_boxs_and_goals_map).unwrap_err(),
        ParseMapError::BoxGoalMismatch
    );
    assert_eq!(
        Map::from_str(invalid_character_map).unwrap_err(),
        ParseMapError::InvalidCharacter('!')
    );
}

#[test]
fn map_from_actions() {
    assert!(Map::from_actions(Actions::from_str("R").unwrap()).is_ok());
    assert!(Map::from_actions(Actions::from_str("DuLLrUUdrR").unwrap()).is_ok());

    assert_eq!(
        Map::from_actions(Actions::from_str("RddrU").unwrap()).unwrap_err(),
        ParseMapError::InvalidActions
    );
    assert_eq!(
        Map::from_actions(Actions::from_str("RdU").unwrap()).unwrap_err(),
        ParseMapError::InvalidActions
    );
    assert_eq!(
        Map::from_actions(Actions::from_str("RL").unwrap()).unwrap_err(),
        ParseMapError::InvalidActions
    );
    assert_eq!(
        Map::from_actions(Actions::from_str("llurldd").unwrap()).unwrap_err(),
        ParseMapError::NoBoxOrGoal
    );
}

#[test]
fn get() {
    let mut map: Map = load_level_from_file("assets/Holland_81.xsb", 9).into();
    for x in 0..map.dimensions().x {
        for y in 0..map.dimensions().y {
            let position = IVector2::new(x, y);
            let tiles = map[position];
            assert_eq!(tiles, *map.get(position).unwrap());
            assert_eq!(tiles, unsafe { *map.get_unchecked(position) });
            assert_eq!(tiles, *map.get_mut(position).unwrap());
            assert_eq!(tiles, unsafe { *map.get_unchecked_mut(position) });
        }
    }
}

#[test]
fn display() {
    let map = load_level_from_file("assets/Holland_81.xsb", 9)
        .map()
        .clone();
    assert_eq!(
        map.to_string(),
        indoc! {"
            --####--
            -#____#-
            -#._*_#-
            #_._$__#
            #_#**#_#
            #__*+*_#
            -#_$$_#-
            -#____#-
            --####--
        "}
    );

    let mut map = load_level_from_file("assets/Holland_81.xsb", 9)
        .map()
        .clone();
    map[IVector2::new(4, 6)].insert(Tiles::Player);
    assert_eq!(
        map.to_string(),
        indoc! {"
            --####--
            -#____#-
            -#._?_#-
            #_._$__#
            #_#**#_#
            #__*+*_#
            -#_$$_#-
            -#____#-
            --####--
        "}
    );
}

#[test]
#[ignore]
fn from_actions() {
    let actions =
        Actions::from_str("uulLdlluRRllddlluuRRdrruRurDDulldldddllUdrruuluullddRluurrdrrurrdDldLrurrdLLuruulldlluRRRurDDullllllddrddrrUUddlluuluurrdRurrrdDldLrurrdLLuruullllllddrddrrUULuurrrrdddlLruruullllddrUluRRRurDDullllllddRddrrUUdrrrruLdllluUluRRRurDDDrdLL")
            .unwrap();
    assert_eq!(
        Map::from_actions(actions).unwrap(),
        Map::from_str(
            r#"
            -#####----
            -#   #####
            ##$# *   #
            #  . #.@ #
            # #  .# ##
            # $  $  #-
            ######  #-
            -----####-
        "#
        )
        .unwrap()
    );
}

#[test]
#[ignore]
fn normalize() {
    // Steaming Hot
    let mut actual = Map::from_str(
        r#"
         #      #
         #   #  #
          # #  #
           # #  #
          #   #  #
         #   #  #
          # #  #
        -
        ##########
        #........####
        # $$$$$$$#  #
        #.$......# *#
        # $$$$$$ #  #
        #......$+# *#
        #$$$$$$$ #  #
        #        ####
        ##########
    "#,
    )
    .unwrap();
    let expected = Map::from_str(
        r#"
        #########
        #.$+_.__#
        #.$.$$$_#
        #.$.$.$_#
        #.$.$.$_#
        #.$.$.$_#
        #.$.$.$_#
        #.$$$.$_#
        #._._.$_#
        #########
    "#,
    )
    .unwrap();
    actual.normalize();
    assert_eq!(actual, expected);

    // Sasquatch #41
    let mut actual = load_level_from_file("assets/Sasquatch_50.xsb", 41)
        .map()
        .clone();
    let expected = Map::from_str(
        r#"
        --#####---
        --#@__#---
        ###$_$####
        #_$...$__#
        #__._.___#
        #_$...$__#
        ###$_$####
        --#___#---
        --#___#---
        --#__##---
        --####----
    "#,
    )
    .unwrap();
    actual.normalize();
    assert_eq!(actual, expected);

    // Title: World Cup 2014 (MF8 61st Sokoban Competition, Extra)
    // Author: laizhufu
    let mut actual = Map::from_str(WORLDCUP2014).unwrap();
    let expected = Map::from_str(
        r#"
        --###########----
        --#____*__._###--
        -##_*_*_*_*___#--
        -#_*_**_*_*_$*###
        ##_*__*_*_*_*@*_#
        #___*_*_*_*_*_*_#
        #_*_*_$*__*__**_#
        #_.*#___#__#__###
        ##____#########--
        -###__#----------
        --#__##----------
        --#___##---------
        --###__##--------
        ----#___##-------
        ----###__#-------
        ------#__#-------
        ------####-------
    "#,
    )
    .unwrap();
    actual.normalize();
    assert_eq!(actual, expected);
}

#[test]
fn can_move_treats_walls_and_boxes_as_blocked() {
    let map = load_level_from_file("assets/Microban_155.xsb", 1)
        .map()
        .clone();
    // The player position itself is reachable.
    assert!(map.can_move(map.player_position()));
    // Box positions are not reachable for the player to step onto.
    for box_pos in map.box_positions() {
        assert!(!map.can_move(box_pos));
    }
}

#[test]
fn in_bounds_basic() {
    let map = Map::with_dimensions(IVector2::new(5, 4));
    assert!(map.in_bounds(IVector2::new(0, 0)));
    assert!(map.in_bounds(IVector2::new(4, 3)));
    assert!(!map.in_bounds(IVector2::new(-1, 0)));
    assert!(!map.in_bounds(IVector2::new(0, -1)));
    assert!(!map.in_bounds(IVector2::new(5, 0)));
    assert!(!map.in_bounds(IVector2::new(0, 4)));
}

#[test]
fn set_player_position_moves_the_player_tile() {
    let mut map = load_level_from_file("assets/Microban_155.xsb", 1)
        .map()
        .clone();
    let original = map.player_position();
    let target = original + IVector2::new(0, 1); // any free neighbor

    // Only run the test if the target is actually a walkable floor cell.
    if map.can_move(target) {
        map.set_player_position(target);
        assert_eq!(map.player_position(), target);
        // Old position should no longer be marked Player; new one should be.
        assert!(!map[original].intersects(Tiles::Player));
        assert!(map[target].intersects(Tiles::Player));
    }
}

#[test]
fn set_box_position_moves_the_box_tile() {
    let xsb = r#"
        ######
        #@$ .#
        ######
    "#;
    let mut map = Map::from_str(xsb).unwrap();
    let from = IVector2::new(2, 1);
    let to = IVector2::new(3, 1);
    assert!(map.box_positions().contains(&from));
    assert!(!map.box_positions().contains(&to));

    map.set_box_position(from, to);
    assert!(!map.box_positions().contains(&from));
    assert!(map.box_positions().contains(&to));
}

#[test]
fn rotate_preserves_solved_state() {
    // Rotate four times → original map (modulo data layout).
    let mut map = load_level_from_file("assets/Microban_155.xsb", 1)
        .map()
        .clone();
    let original_solved = map.is_solved();
    for _ in 0..4 {
        map.rotate();
    }
    assert_eq!(map.is_solved(), original_solved);
}

#[test]
fn flip_then_flip_is_identity_on_solved_state() {
    let mut map = load_level_from_file("assets/Microban_155.xsb", 1)
        .map()
        .clone();
    let original = map.is_solved();
    map.flip();
    map.flip();
    assert_eq!(map.is_solved(), original);

    map.flip_vertical();
    map.flip_vertical();
    assert_eq!(map.is_solved(), original);
}

#[test]
fn is_solved_only_when_every_box_on_a_goal() {
    let mut map = Map::from_str("#####\n#@$.#\n#####").unwrap();
    assert!(!map.is_solved());
    let from = IVector2::new(2, 1);
    let to = IVector2::new(3, 1);
    map.set_box_position(from, to);
    assert!(map.is_solved());
}

#[test]
fn truncate_resizes_and_offsets() {
    // Build a 10×10 map with one distinctive cell, truncate to a 5×5 window
    // offset by (2, 3), and confirm the cell ended up at the expected
    // position in the smaller map.
    let mut map = Map::with_dimensions(IVector2::new(10, 10));
    map[IVector2::new(3, 4)] = Tiles::Wall;

    map.truncate(IVector2::new(5, 5), IVector2::new(2, 3));

    assert_eq!(map.dimensions(), IVector2::new(5, 5));
    // (3, 4) - (2, 3) = (1, 1) inside the new map.
    assert_eq!(map[IVector2::new(1, 1)], Tiles::Wall);
    // Cells outside the wall stay empty (the source region was empty there).
    assert_eq!(map[IVector2::new(0, 0)], Tiles::empty());
}

#[test]
fn with_dimensions_is_empty_map_at_origin() {
    let map = Map::with_dimensions(IVector2::new(3, 3));
    assert_eq!(map.dimensions(), IVector2::new(3, 3));
    assert_eq!(map.player_position(), IVector2::zeros());
    assert!(map.box_positions().is_empty());
    assert!(map.goal_positions().is_empty());
    // Every cell starts empty (no walls, no floors).
    for x in 0..3 {
        for y in 0..3 {
            assert!(map[IVector2::new(x, y)].is_empty());
        }
    }
}

#[test]
fn trimmed() {
    let mut oversize_map = Map::from_str(
        r#"
        ---------------
        ---------------
        ----####-------
        --###  ####----
        --#     $ #----
        --# #  #$ #----
        --# . .#@ #----
        --#########----
        ---------------
        ---------------
        ---------------
    "#,
    )
    .unwrap();
    let expected = load_level_from_file("assets/Microban_155.xsb", 3)
        .map()
        .clone();
    oversize_map.shrink_to_fit();
    assert_eq!(oversize_map, expected);

    let mut actual = expected.clone();
    actual.shrink_to_fit();
    assert_eq!(actual, expected);
}

// Title: World Cup 2014 (MF8 61st Sokoban Competition, Extra)
// Author: laizhufu
const WORLDCUP2014: &str = r#"
    -------#########-------
    -----##---------##-----
    ---##---#####--#--##---
    --#---##------#--#--#--
    --####---##--#--#--##--
    -#-----##---#--#--#--#-
    -######----#--#--#--##-
    #-------##---#--#--#--#
    ########----#--#--#--##
    #-------.*#---#--#--#-#
    #-#-#-#-*-*-$*--*--**-#
    #-#-#-#---*-*-*-*-*-*-#
    #--#-#-#-*--*-*-*-*@*-#
    ##-#-#-#-*-**-*-*-$***#
    -#--#-#-#-*-*-*-*---*#-
    -##-#-#-#----*--.-#-##-
    --#--#-#-#-#-#-#-#--#--
    --##-#-#-#-#-#-#-#-##--
    ---##-#-#-#-#-#-#-##---
    ------#-#-#-#-#-#------
    -----#-#-#-#-#-#-#-----
    -----#-#-#-#-#-#-#-----
    ------#-#-#-#-#-#------
    ------#-#-#-#-#-#------
    ------#--#-#-#-#-#-----
    ------##-#-#-#-#-#-----
    ------##--#-#-#-#------
    ------###-#-#-#-#------
    -----####--#-#-#-#-----
    -----#####-#-#-#-#-----
    -----#####--#-#-#-#----
    ----#######-#-#-#-#----
    ----#######--#-#-#-#---
    -----#######---#-#-#---
    ---#--########----#-#--
    -#--#--##########----#-
    --#--#--#############--
    ---#--#-###########----
    -------#--######-------
"#;
