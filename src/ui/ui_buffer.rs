use std::fmt::Display;

use crate::ModbusDeviceConfig;

#[derive(serde::Deserialize, serde::Serialize, PartialEq, Eq)]
pub enum ModbusDeviceType {
    Tcp,
    Serial,
}

impl Display for ModbusDeviceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ModbusDeviceType::Tcp => write!(f, "Modbus TCP"),
            ModbusDeviceType::Serial => write!(f, "Modbus Serial"),
        }
    }
}
#[derive(serde::Deserialize, serde::Serialize)]
pub struct ModbusDeviceBuffer {
    pub id: usize,
    pub code: String,
    pub name: String,
    pub device_type: ModbusDeviceType,
}

impl Default for ModbusDeviceBuffer {
    fn default() -> Self {
        Self {
            id: 1,
            code: "MB".to_owned(),
            name: "PLC_1".to_owned(),
            device_type: ModbusDeviceType::Tcp,
        }
    }
}
