pub mod interactive;
pub mod registry;
pub mod state;

use num_derive::FromPrimitive;
use pumpkin_util::math::vector3::Vector3;

pub use state::BlockState;

#[derive(FromPrimitive, PartialEq, Clone, Copy)]
pub enum BlockDirection {
    Bottom = 0,
    Top,
    North,
    South,
    West,
    East,
}

pub struct InvalidBlockFace;

impl TryFrom<i32> for BlockDirection {
    type Error = InvalidBlockFace;

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Bottom),
            1 => Ok(Self::Top),
            2 => Ok(Self::North),
            3 => Ok(Self::South),
            4 => Ok(Self::West),
            5 => Ok(Self::East),
            _ => Err(InvalidBlockFace),
        }
    }
}

impl TryFrom<&str> for BlockDirection {
    type Error = InvalidBlockFace;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "bottom" => Ok(Self::Bottom),
            "top" => Ok(Self::Top),
            "north" => Ok(Self::North),
            "south" => Ok(Self::South),
            "west" => Ok(Self::West),
            "east" => Ok(Self::East),
            _ => Err(InvalidBlockFace),
        }
    }
}

impl BlockDirection {
    pub fn to_offset(&self) -> Vector3<i32> {
        match self {
            BlockDirection::Bottom => (0, -1, 0),
            BlockDirection::Top => (0, 1, 0),
            BlockDirection::North => (0, 0, -1),
            BlockDirection::South => (0, 0, 1),
            BlockDirection::West => (-1, 0, 0),
            BlockDirection::East => (1, 0, 0),
        }
        .into()
    }
    pub fn opposite(&self) -> BlockDirection {
        match self {
            BlockDirection::Bottom => BlockDirection::Top,
            BlockDirection::Top => BlockDirection::Bottom,
            BlockDirection::North => BlockDirection::South,
            BlockDirection::South => BlockDirection::North,
            BlockDirection::West => BlockDirection::East,
            BlockDirection::East => BlockDirection::West,
        }
    }
    pub fn horizontal() -> Vec<BlockDirection> {
        vec![
            BlockDirection::North,
            BlockDirection::East,
            BlockDirection::South,
            BlockDirection::West,
        ]
    }

    pub fn vertical() -> Vec<BlockDirection> {
        vec![
            BlockDirection::Bottom,
            BlockDirection::Top,
        ]
    }

    pub fn all() -> Vec<BlockDirection> {
        vec![
            BlockDirection::Bottom,
            BlockDirection::Top,
            BlockDirection::North,
            BlockDirection::South,
            BlockDirection::West,
            BlockDirection::East,
        ]
    }

    pub fn update_order() -> Vec<BlockDirection> {
        vec![
            BlockDirection::West,
            BlockDirection::East,
            BlockDirection::Bottom,
            BlockDirection::Top,
            BlockDirection::North,
            BlockDirection::South,
        ]
    }
}
