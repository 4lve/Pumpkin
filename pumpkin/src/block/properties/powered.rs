use async_trait::async_trait;
use pumpkin_macros::block_property;
use pumpkin_world::block::registry::Block;
use pumpkin_world::item::ItemStack;

use super::{BlockProperty, BlockPropertyMetadata};

#[block_property("powered")]
pub struct Powered(bool);

#[async_trait]
impl BlockProperty for Powered {
    async fn on_interact(&self, value: String, block: &Block, _item: &ItemStack) -> String {
        // Lever
        if block.id == 242 {
            if value == Self::True().value() {
                return Self::False().value();
            } else {
                return Self::True().value();
            }
        }

        value
    }

    async fn on_scheduled_tick(&self, value: String, block: &Block) -> String {
        if block.name.contains("button") {
            Self::False().value()
        } else {
            value
        }
    }
}
