use crate::network::packet::AnyPacketData;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScreenShotImageBuffer(pub Vec<u8>);
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ScreenShotPacketData {
    Request,
    Response(Vec<ScreenShotImageBuffer>),
}

#[typetag::serde]
impl AnyPacketData for ScreenShotPacketData {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
