use serde::{Deserialize, Serialize};

/// 协议类型枚举
#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
pub enum ProtocolType {
    SSH,
    Serial,
    Telnet,
}

/// SSH 特有配置负载
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct SshConfig {
    pub host: String,
    pub port: u16,
    pub user: String,
    pub auth: AuthMethod,
    pub credential_id: Option<String>,
}

/// 串口特有配置负载
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct SerialConfig {
    pub port: String, // e.g. /dev/ttyUSB0 or COM3
    pub baud_rate: u32,
    pub data_bits: u8,
    pub stop_bits: u8,
    pub parity: String,       // N, E, O
    pub flow_control: String, // None, Software, Hardware
}

/// Telnet 特有配置负载
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct TelnetConfig {
    pub host: String,
    pub port: u16,
    pub encoding: String,
}

/// 传输层配置变体
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum TransportConfig {
    Ssh(SshConfig),
    Serial(SerialConfig),
    Telnet(TelnetConfig),
}

/// 身份认证方式（主要针对 SSH）
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Hash)]
pub enum AuthMethod {
    Password,
    Key { private_key_path: String },
    Agent,
    Interactive,
}

impl std::fmt::Display for AuthMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AuthMethod::Password => write!(f, "Password"),
            AuthMethod::Key { .. } => write!(f, "Key"),
            AuthMethod::Agent => write!(f, "Agent"),
            AuthMethod::Interactive => write!(f, "Interactive"),
        }
    }
}

/// 核心会话配置文件：仅包含元数据与协议无关属性
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SessionProfile {
    pub id: String,
    pub name: String,
    pub folder: Option<String>,
    pub color_tag: Option<[u8; 3]>,
    pub transport: TransportConfig,
}

/// 兼容层：旧的 SessionConfig 现已映射为 SessionProfile
pub type SessionConfig = SessionProfile;

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct SessionLibrary {
    pub sessions: Vec<SessionProfile>,
    pub version: String,
}
