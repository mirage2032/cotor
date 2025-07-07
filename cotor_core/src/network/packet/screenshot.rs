use crate::network::packet::AnyPacket;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScreenShotImage {
    pub width: u32,
    pub height: u32,
    pub buffer: Vec<u8>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ScreenShotPacket {
    Request,
    Response(Vec<ScreenShotImage>),
}

#[typetag::serde]
impl AnyPacket for ScreenShotPacket {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_box(self: Box<Self>) -> Box<dyn std::any::Any + Send + Sync> {
        self
    }
}
