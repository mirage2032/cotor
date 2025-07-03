use crate::network::packet::AnyPacketData;
use serde::{Deserialize, Serialize};
use std::fmt::Display;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageLevel {
    Trace,
    Debug,
    Info,
    Warning,
    Error,
}

impl Display for MessageLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MessageLevel::Trace => write!(f, "Trace"),
            MessageLevel::Debug => write!(f, "Debug"),
            MessageLevel::Info => write!(f, "Info"),
            MessageLevel::Warning => write!(f, "Warning"),
            MessageLevel::Error => write!(f, "Error"),
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageData {
    message: String,
    level: MessageLevel,
}

impl MessageData {
    pub fn new(message: String, level: MessageLevel) -> Self {
        Self { message, level }
    }
    pub fn new_trace(message: String) -> Self {
        Self::new(message, MessageLevel::Trace)
    }
    pub fn new_debug(message: String) -> Self {
        Self::new(message, MessageLevel::Debug)
    }
    pub fn new_info(message: String) -> Self {
        Self::new(message, MessageLevel::Info)
    }
    pub fn new_warning(message: String) -> Self {
        Self::new(message, MessageLevel::Warning)
    }
    pub fn new_error(message: String) -> Self {
        Self::new(message, MessageLevel::Error)
    }
    pub fn message(&self) -> &str {
        &self.message
    }
    pub fn level(&self) -> &MessageLevel {
        &self.level
    }
}

#[typetag::serde]
impl AnyPacketData for MessageData {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_box(self: Box<Self>) -> Box<dyn std::any::Any + Send + Sync> {
        self
    }
}
