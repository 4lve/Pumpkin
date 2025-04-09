use pumpkin_data::screen::WindowType;

use crate::{inventory::Inventory, slot::Slot};

pub trait ScreenHandler {
    fn new(window_type: WindowType, sync_id: u8) -> Self;

    fn window_type(&self) -> WindowType;

    fn size(&self) -> usize;

    fn add_slot<S: Slot>(&mut self, mut slot: S) -> S {
        slot.set_index(self.size());
        slot
    }

    fn add_player_hotbar_slots<I: Inventory>(&mut self, player_inventory: I) {
        for i in 0..9 {
            //self.add_slot(player_inventory.get_stack(i));
        }
    }
}
