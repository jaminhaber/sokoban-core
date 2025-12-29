//! A direction.

use std::ops::Neg;

use crate::math::Vec2;

/// A direction.
#[derive(Clone, Copy, Eq, PartialEq, Hash, Debug)]
pub enum Direction {
    /// Upward direction (negative Y-axis).
    Up,
    /// Downward direction (positive Y-axis).
    Down,
    /// Leftward direction (negative X-axis).
    Left,
    /// Rightward direction (positive X-axis).
    Right,
}

impl Direction {
    /// Returns an iterator over all directions.
    pub fn iter() -> std::array::IntoIter<Direction, 4> {
        [Self::Up, Self::Down, Self::Left, Self::Right].into_iter()
    }

    /// Rotate the direction 90° clockwise.
    ///
    /// # Examples
    ///
    /// ```
    /// # use soukoban::direction::Direction;
    /// assert_eq!(Direction::Up.rotate(), Direction::Right);
    ///
    /// // Rotate the direction 90° counter clockwis.
    /// assert_eq!(-Direction::Right.rotate(), Direction::Up);
    /// ```
    pub fn rotate(self) -> Direction {
        match self {
            Self::Up => Self::Right,
            Self::Right => Self::Down,
            Self::Down => Self::Left,
            Self::Left => Self::Up,
        }
    }

    /// Flip the direction.
    ///
    /// # Examples
    ///
    /// ```
    /// # use soukoban::direction::Direction;
    /// assert_eq!(Direction::Left.flip(), Direction::Right);
    /// assert_eq!(Direction::Up.flip(), Direction::Down);
    /// ```
    pub fn flip(self) -> Direction {
        match self {
            Self::Up => Self::Down,
            Self::Down => Self::Up,
            Self::Left => Self::Right,
            Self::Right => Self::Left,
        }
    }
}

impl Neg for Direction {
    type Output = Self;

    fn neg(self) -> Self::Output {
        self.flip()
    }
}

impl From<Direction> for Vec2 {
    fn from(direction: Direction) -> Self {
        use Direction as E;
        match direction {
            E::Up => -Vec2::unit_y(),
            E::Down => Vec2::unit_y(),
            E::Left => -Vec2::unit_x(),
            E::Right => Vec2::unit_x(),
        }
    }
}

impl TryFrom<Vec2> for Direction {
    type Error = ();

    fn try_from(vector: Vec2) -> Result<Self, Self::Error> {
        use Direction::*;
        match vector {
            v if v == -Vec2::unit_y() => Ok(Up),
            v if v == Vec2::unit_y() => Ok(Down),
            v if v == -Vec2::unit_x() => Ok(Left),
            v if v == Vec2::unit_x() => Ok(Right),
            _ => Err(()),
        }
    }
}
