use crate::network::packet::types::ProcessIdentifier;
use crate::network::packet::AnyPacket;
use serde::{Deserialize, Serialize};
use std::net::{Ipv4Addr, Ipv6Addr};
use std::path::PathBuf;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OperatingSystem {
    Windows,
    Linux,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OSData {
    version: String,
    os_type: OperatingSystem,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileEntry {
    pub name: String,
    pub size: u64,
    pub is_directory: bool,
    pub modified_time: Option<std::time::SystemTime>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessEntry {
    pub pid: u32,
    pub name: String,
    pub command_line: Option<String>,
    pub user: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProcessKillResponseResult {
    Success,
    NotFound,
    Error(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PowerAction {
    Shutdown,
    Restart,
    Sleep,
    Hibernate,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpuData {
    pub model: String,
    pub cores: u32,
    pub threads: u32,
    pub architecture: String,
    pub frequency: f64, // in MHz
    pub usage: f64,     // in percentage
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryData {
    pub total: u64,     // in bytes
    pub available: u64, // in bytes
    pub used: u64,      // in bytes
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetInterfaceIpv4Data {
    pub ipv4_address: Ipv4Addr,
    pub ipv4_gateway: Ipv4Addr,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetInterfaceIpv6Data {
    pub ipv6_address: Ipv6Addr,
    pub ipv6_gateway: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkInterface {
    pub name: String,
    pub mac_address: String,
    pub ipv4_address: Vec<NetInterfaceIpv4Data>,
    pub ipv6_address: Vec<NetInterfaceIpv6Data>,
    pub ipv4_gateway: Option<Ipv4Addr>,
    pub ipv6_gateway: Option<Ipv6Addr>,
    pub speed: Option<u64>, // in bits per second
    pub is_up: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LsPacketData {
    Request((Uuid, PathBuf)),
    Response((Uuid, Vec<FileEntry>)),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BinExecPacketData {
    Request((Uuid, PathBuf)),
    Stdin((Uuid, String)),
    Stdout((Uuid, String)),
    Stderr((Uuid, String)),
    StopRequest(Uuid),
    End((Uuid, Result<(), String>)),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProcessPacketData {
    RequestList(Uuid),
    ResponseList((Uuid, Vec<ProcessEntry>)),
    KillRequest(Uuid, ProcessIdentifier),
    KillResponse(Uuid, ProcessKillResponseResult),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SystemInfoPacketData {
    Request(Uuid),
    Response {
        req_id: Uuid,
        hostname: String,
        os: OSData,
        cpu: CpuData,
        memory: MemoryData,
        username: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PowerPacketData {
    Request(Uuid, PowerAction),
    Response(Uuid, Result<(), String>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NetworkPacketData {
    RequestInterfaces(Uuid),
    ResponseInterfaces((Uuid, Vec<NetworkInterface>)),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SystemPacket {
    Ls(LsPacketData),
    BinExec(BinExecPacketData),
    Process(ProcessPacketData),
    SystemInfo(SystemInfoPacketData),
    Power(PowerPacketData),
    Network(NetworkPacketData),
}

#[typetag::serde]
impl AnyPacket for SystemPacket {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_box(self: Box<Self>) -> Box<dyn std::any::Any + Send + Sync> {
        self
    }
}
