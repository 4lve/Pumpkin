use crate::{ItemStackSerializer, VarInt};

use pumpkin_data::packet::clientbound::PLAY_CONTAINER_SET_CONTENT;
use pumpkin_macros::packet;
use serde::Serialize;

#[derive(Serialize)]
#[packet(PLAY_CONTAINER_SET_CONTENT)]
pub struct CSetContainerContent<'a> {
    window_id: VarInt,
    state_id: VarInt,
    count: VarInt,
    slot_data: &'a [ItemStackSerializer],
    carried_item: &'a ItemStackSerializer,
}

impl<'a> CSetContainerContent<'a> {
    pub fn new(
        window_id: VarInt,
        state_id: VarInt,
        slots: &'a [ItemStackSerializer],
        carried_item: &'a ItemStackSerializer,
    ) -> Self {
        Self {
            window_id,
            state_id,
            slot_data: slots,
            carried_item,
        }
    }
}
