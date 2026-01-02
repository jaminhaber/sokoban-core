//! A collection of maps.

use crate::{CollectionError, IVector2, Level, Map, Tiles};
use itertools::Itertools;
use std::fmt;

/// A collection of maps.
pub struct Collection {
    header: String,
    levels: Vec<Level>,
}

impl Collection {
    /// Returns the name of the collection.
    pub fn header(&self) -> &str {
        &self.header
    }

    /// Returns the levels of the collection.
    pub fn levels(&self) -> &[Level] {
        &self.levels
    }

    /// Returns the level at the given index.
    pub fn level(&self, index: usize) -> Option<Level> {
        self.levels.get(index).map(|l| l.clone())
    }

    /// Returns a mutable reference to the level at the given index.
    pub fn level_mut(&mut self, index: usize) -> Option<&mut Level> {
        self.levels.get_mut(index)
    }

    /// Returns the number of levels in the collection.
    pub fn len(&self) -> usize {
        self.levels.len()
    }

    /// Returns true if the collection is empty.
    pub fn is_empty(&self) -> bool {
        self.levels.is_empty()
    }

    /// Adds a level to the end of the collection.
    pub fn add_level(&mut self, level: Level) {
        self.levels.push(level);
    }

    /// Inserts a level at the given index.
    ///
    /// Returns an error if the index is out of bounds.
    pub fn insert_level(&mut self, index: usize, level: Level) -> Result<(), CollectionError> {
        if index > self.levels.len() {
            return Err(CollectionError::IndexOutOfBounds);
        }
        self.levels.insert(index, level);
        Ok(())
    }

    /// Removes and returns the level at the given index.
    pub fn remove_level(&mut self, index: usize) -> Option<Level> {
        if index < self.levels.len() {
            Some(self.levels.remove(index))
        } else {
            None
        }
    }

    /// Replaces the level at the given index with a new level,
    /// returning the old level.
    pub fn replace_level(&mut self, index: usize, level: Level) -> Option<Level> {
        if index < self.levels.len() {
            Some(std::mem::replace(&mut self.levels[index], level))
        } else {
            None
        }
    }

    /// Swaps two levels at the given indices.
    ///
    /// Returns true if the swap was successful, false if either index is out of
    /// bounds.
    pub fn swap_levels(&mut self, a: usize, b: usize) -> bool {
        if a < self.levels.len() && b < self.levels.len() {
            self.levels.swap(a, b);
            true
        } else {
            false
        }
    }

    /// Serializes the collection to XSB format.
    pub fn to_xsb(&self) -> String {
        let mut output = String::new();

        // Add header
        if !self.header.is_empty() {
            output.push_str(&self.header);
            output.push('\n');
        }

        // Add each level
        for (index, level) in self.levels.iter().enumerate() {
            // Add blank line between levels (but not before the first one if header exists)
            if index > 0 || self.header.is_empty() {
                output.push('\n');
            }

            // Convert level map to XSB format and add to output
            let map_xsb = map_to_xsb(level.map());
            output.push_str(&map_xsb);

            // Add level metadata directly from the level
            for key in level.metadata().keys().sorted() {
                let value = &level.metadata()[key];
                if key == "comments" && value.lines().count() > 1 {
                    output.push_str("comment:\n");
                    for line in value.lines() {
                        output.push_str(line);
                        output.push('\n');
                    }
                    output.push_str("comment-end:\n");
                    continue;
                }
                output.push_str(key);
                output.push_str(": ");
                output.push_str(value);
                output.push('\n');
            }
        }

        output
    }

    /// Constructs a collection from an XSB file.
    pub fn from_xsb(xsb: &str) -> Self {
        let levels = Level::load_from_str(xsb);

        let mut header = String::new();
        // Extract and preserve header (lines starting with `;`)
        for line in xsb.lines() {
            if line.starts_with(';') {
                header.push_str(line);
                header.push('\n');
            } else if !line.trim().is_empty() {
                break;
            }
        }

        Self {
            header,
            levels: levels.filter_map(|l| l.ok()).collect_vec(),
        }
    }
}

impl fmt::Display for Collection {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.to_xsb())
    }
}

fn map_to_xsb(map: &Map) -> String {
    let mut map = map.clone();
    // Trim empty edges
    map = trim_empty_edges(map);

    // Convert to string and clean up
    let map_str = map.to_string().replace('_', " ").replace('-', " ");

    // Find minimum leading padding
    let min_padding = map_str
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| line.len() - line.trim_start().len())
        .min()
        .unwrap_or(0);

    // Build output with normalized padding
    let mut output = String::new();
    for line in map_str.lines().filter(|line| !line.trim().is_empty()) {
        let trimmed = line.get(min_padding..).unwrap_or(line).trim_end();
        output.push_str(trimmed);
        output.push('\n');
    }
    output
}

fn trim_empty_edges(map: Map) -> Map {
    let dims = map.dimensions();

    // Find bounds of non-floor tiles
    let mut min_x = dims.x;
    let mut max_x = -1;
    let mut min_y = dims.y;
    let mut max_y = -1;

    for x in 0..dims.x {
        for y in 0..dims.y {
            let pos = IVector2::new(x, y);
            if let Some(tile) = map.get(pos) {
                // Check if tile is not just floor (empty)
                if *tile != Tiles::Floor {
                    min_x = min_x.min(x);
                    max_x = max_x.max(x);
                    min_y = min_y.min(y);
                    max_y = max_y.max(y);
                }
            }
        }
    }

    // If no non-floor tiles found, return a minimal map
    if max_x < 0 || max_y < 0 {
        return Map::with_dimensions(IVector2::new(3, 3));
    }

    // Calculate new dimensions with the trimmed bounds
    let new_width = max_x - min_x + 1;
    let new_height = max_y - min_y + 1;
    let mut new_map = Map::with_dimensions(IVector2::new(new_width, new_height));

    // Copy tiles to the new map
    for x in 0..new_width {
        for y in 0..new_height {
            let old_pos = IVector2::new(x + min_x, y + min_y);
            let new_pos = IVector2::new(x, y);
            if let Some(tile) = map.get(old_pos) {
                new_map[new_pos] = *tile;
            }
        }
    }

    new_map
}
