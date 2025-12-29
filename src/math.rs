//! A 2D vector type for grid positions.

use std::ops::{Add, AddAssign, Index, IndexMut, Neg, Sub, SubAssign};

/// A 2D vector with integer components.
#[derive(Clone, Copy, Eq, PartialEq, Hash, Debug, Default, Ord, PartialOrd)]
pub struct IVector2 {
    /// The X component.
    pub x: i32,
    /// The Y component.
    pub y: i32,
}

impl IVector2 {
    /// Creates a new vector with the given components.
    pub const fn new(x: i32, y: i32) -> Self {
        IVector2 { x, y }
    }

    /// Returns a vector with both components set to 0.
    pub const fn zeros() -> Self {
        IVector2 { x: 0, y: 0 }
    }

    /// Returns the X basis vector (1, 0).
    pub const fn x_axis() -> Self {
        IVector2 { x: 1, y: 0 }
    }

    /// Returns the Y basis vector (0, 1).
    pub const fn y_axis() -> Self {
        IVector2 { x: 0, y: 1 }
    }

    /// Returns the unit X vector (1, 0).
    pub const fn unit_x() -> Self {
        IVector2 { x: 1, y: 0 }
    }

    /// Returns the unit Y vector (0, 1).
    pub const fn unit_y() -> Self {
        IVector2 { x: 0, y: 1 }
    }

    /// Returns the negative X basis vector (-1, 0).
    pub const fn neg_x_axis() -> Self {
        IVector2 { x: -1, y: 0 }
    }

    /// Returns the negative Y basis vector (0, -1).
    pub const fn neg_y_axis() -> Self {
        IVector2 { x: 0, y: -1 }
    }

    /// Computes the absolute value of each component.
    pub fn abs(self) -> Self {
        IVector2 {
            x: self.x.abs(),
            y: self.y.abs(),
        }
    }

    /// Computes the signum of each component.
    pub fn signum(self) -> Self {
        IVector2 {
            x: self.x.signum(),
            y: self.y.signum(),
        }
    }

    /// Computes the Manhattan distance between two vectors.
    pub fn manhattan_distance(self, other: IVector2) -> i32 {
        (self.x - other.x).abs() + (self.y - other.y).abs()
    }

    /// Rotates the vector 90 degrees clockwise.
    pub fn rotate_90_cw(self) -> Self {
        IVector2 {
            x: self.y,
            y: -self.x,
        }
    }

    /// Rotates the vector 90 degrees counter-clockwise.
    pub fn rotate_90_ccw(self) -> Self {
        IVector2 {
            x: -self.y,
            y: self.x,
        }
    }

    /// Returns a new vector with x and y swapped.
    pub fn yx(self) -> Self {
        IVector2 {
            x: self.y,
            y: self.x,
        }
    }

    /// Applies a function to each component with the corresponding component of
    /// another vector.
    pub fn zip_map<F>(self, other: &IVector2, f: F) -> Self
    where
        F: Fn(i32, i32) -> i32,
    {
        IVector2 {
            x: f(self.x, other.x),
            y: f(self.y, other.y),
        }
    }

    /// Returns the sum of all components.
    pub fn sum(self) -> i32 {
        self.x + self.y
    }

    /// Returns an iterator over the components [x, y].
    pub fn iter(&self) -> impl Iterator<Item = &i32> {
        [&self.x, &self.y].into_iter()
    }

    /// Returns a mutable iterator over the components [x, y].
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut i32> {
        [&mut self.x, &mut self.y].into_iter()
    }
}

impl Add for IVector2 {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        IVector2 {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
        }
    }
}

impl Add<&IVector2> for IVector2 {
    type Output = Self;

    fn add(self, rhs: &IVector2) -> Self::Output {
        IVector2 {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
        }
    }
}

impl Add<IVector2> for &IVector2 {
    type Output = IVector2;

    fn add(self, rhs: IVector2) -> Self::Output {
        IVector2 {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
        }
    }
}

impl Add<&IVector2> for &IVector2 {
    type Output = IVector2;

    fn add(self, rhs: &IVector2) -> IVector2 {
        IVector2 {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
        }
    }
}

impl AddAssign for IVector2 {
    fn add_assign(&mut self, rhs: Self) {
        self.x += rhs.x;
        self.y += rhs.y;
    }
}

impl AddAssign<&IVector2> for IVector2 {
    fn add_assign(&mut self, rhs: &IVector2) {
        self.x += rhs.x;
        self.y += rhs.y;
    }
}

impl Sub for IVector2 {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        IVector2 {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
        }
    }
}

impl Sub<&IVector2> for IVector2 {
    type Output = Self;

    fn sub(self, rhs: &IVector2) -> Self::Output {
        IVector2 {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
        }
    }
}

impl Sub<&IVector2> for &IVector2 {
    type Output = IVector2;

    fn sub(self, rhs: &IVector2) -> Self::Output {
        IVector2 {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
        }
    }
}

impl SubAssign for IVector2 {
    fn sub_assign(&mut self, rhs: Self) {
        self.x -= rhs.x;
        self.y -= rhs.y;
    }
}

impl Neg for IVector2 {
    type Output = Self;

    fn neg(self) -> Self::Output {
        IVector2 {
            x: -self.x,
            y: -self.y,
        }
    }
}

impl Index<usize> for IVector2 {
    type Output = i32;

    fn index(&self, index: usize) -> &Self::Output {
        match index {
            0 => &self.x,
            1 => &self.y,
            _ => panic!("index out of bounds"),
        }
    }
}

impl IndexMut<usize> for IVector2 {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        match index {
            0 => &mut self.x,
            1 => &mut self.y,
            _ => panic!("index out of bounds"),
        }
    }
}

impl std::fmt::Display for IVector2 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "({}, {})", self.x, self.y)
    }
}
