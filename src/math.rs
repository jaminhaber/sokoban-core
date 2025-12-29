use std::ops::{Add, AddAssign, Index, IndexMut, Neg, Sub, SubAssign};

/// A 2D vector with integer components.
#[derive(Clone, Copy, Eq, PartialEq, Hash, Debug, Default, Ord, PartialOrd)]
pub struct Vec2 {
    /// The X component.
    pub x: i32,
    /// The Y component.
    pub y: i32,
}

impl Vec2 {
    /// Creates a new vector with the given components.
    pub const fn new(x: i32, y: i32) -> Self {
        Vec2 { x, y }
    }

    /// Returns a vector with both components set to 0.
    pub const fn zeros() -> Self {
        Vec2 { x: 0, y: 0 }
    }

    /// Returns the X basis vector (1, 0).
    pub const fn x_axis() -> Self {
        Vec2 { x: 1, y: 0 }
    }

    /// Returns the Y basis vector (0, 1).
    pub const fn y_axis() -> Self {
        Vec2 { x: 0, y: 1 }
    }

    /// Returns the unit X vector (1, 0).
    pub const fn unit_x() -> Self {
        Vec2 { x: 1, y: 0 }
    }

    /// Returns the unit Y vector (0, 1).
    pub const fn unit_y() -> Self {
        Vec2 { x: 0, y: 1 }
    }

    /// Returns the negative X basis vector (-1, 0).
    pub const fn neg_x_axis() -> Self {
        Vec2 { x: -1, y: 0 }
    }

    /// Returns the negative Y basis vector (0, -1).
    pub const fn neg_y_axis() -> Self {
        Vec2 { x: 0, y: -1 }
    }

    /// Computes the absolute value of each component.
    pub fn abs(self) -> Self {
        Vec2 {
            x: self.x.abs(),
            y: self.y.abs(),
        }
    }

    /// Computes the signum of each component.
    pub fn signum(self) -> Self {
        Vec2 {
            x: self.x.signum(),
            y: self.y.signum(),
        }
    }

    /// Computes the Manhattan distance between two vectors.
    pub fn manhattan_distance(self, other: Vec2) -> i32 {
        (self.x - other.x).abs() + (self.y - other.y).abs()
    }

    /// Rotates the vector 90 degrees clockwise.
    pub fn rotate_90_cw(self) -> Self {
        Vec2 {
            x: self.y,
            y: -self.x,
        }
    }

    /// Rotates the vector 90 degrees counter-clockwise.
    pub fn rotate_90_ccw(self) -> Self {
        Vec2 {
            x: -self.y,
            y: self.x,
        }
    }

    /// Returns a new vector with x and y swapped.
    pub fn yx(self) -> Self {
        Vec2 {
            x: self.y,
            y: self.x,
        }
    }

    /// Applies a function to each component with the corresponding component of
    /// another vector.
    pub fn zip_map<F>(self, other: &Vec2, f: F) -> Self
    where
        F: Fn(i32, i32) -> i32,
    {
        Vec2 {
            x: f(self.x, other.x),
            y: f(self.y, other.y),
        }
    }
}

impl Add for Vec2 {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Vec2 {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
        }
    }
}

impl Add<&Vec2> for Vec2 {
    type Output = Self;

    fn add(self, rhs: &Vec2) -> Self::Output {
        Vec2 {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
        }
    }
}

impl Add<Vec2> for &Vec2 {
    type Output = Vec2;

    fn add(self, rhs: Vec2) -> Self::Output {
        Vec2 {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
        }
    }
}

impl Add<&Vec2> for &Vec2 {
    type Output = Vec2;

    fn add(self, rhs: &Vec2) -> Vec2 {
        Vec2 {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
        }
    }
}

impl AddAssign for Vec2 {
    fn add_assign(&mut self, rhs: Self) {
        self.x += rhs.x;
        self.y += rhs.y;
    }
}

impl AddAssign<&Vec2> for Vec2 {
    fn add_assign(&mut self, rhs: &Vec2) {
        self.x += rhs.x;
        self.y += rhs.y;
    }
}

impl Sub for Vec2 {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Vec2 {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
        }
    }
}

impl Sub<&Vec2> for Vec2 {
    type Output = Self;

    fn sub(self, rhs: &Vec2) -> Self::Output {
        Vec2 {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
        }
    }
}

impl Sub<&Vec2> for &Vec2 {
    type Output = Vec2;

    fn sub(self, rhs: &Vec2) -> Self::Output {
        Vec2 {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
        }
    }
}

impl SubAssign for Vec2 {
    fn sub_assign(&mut self, rhs: Self) {
        self.x -= rhs.x;
        self.y -= rhs.y;
    }
}

impl Neg for Vec2 {
    type Output = Self;

    fn neg(self) -> Self::Output {
        Vec2 {
            x: -self.x,
            y: -self.y,
        }
    }
}

impl Index<usize> for Vec2 {
    type Output = i32;

    fn index(&self, index: usize) -> &Self::Output {
        match index {
            0 => &self.x,
            1 => &self.y,
            _ => panic!("index out of bounds"),
        }
    }
}

impl IndexMut<usize> for Vec2 {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        match index {
            0 => &mut self.x,
            1 => &mut self.y,
            _ => panic!("index out of bounds"),
        }
    }
}

impl std::fmt::Display for Vec2 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "({}, {})", self.x, self.y)
    }
}
