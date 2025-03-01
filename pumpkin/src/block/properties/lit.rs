use async_trait::async_trait;
use pumpkin_macros::block_property;
use pumpkin_world::block::registry::Block;
use pumpkin_world::item::ItemStack;

use super::{BlockProperty, BlockPropertyMetadata};

#[block_property("lit")]
pub struct Lit(bool);

#[async_trait]
impl BlockProperty for Lit {}
