use pumpkin_data::screen::WindowType;

use crate::inventory::Inventory;

pub trait ScreenHandler {
    fn new(window_type: WindowType, sync_id: u8) -> Self;

    fn window_type(&self) -> WindowType;

    fn add_player_hotbar_slots<I: Inventory>(&mut self, player_inventory: I) {
        for i in 0..9 {
            //self.add_slot(player_inventory.get_stack(i));
        }
    }
}
