use anyhow::Result;
use std::{net::SocketAddr, time::Duration};
use tcp::connect;
use tokio_modbus::prelude::*;

#[derive(serde::Deserialize, serde::Serialize, Clone)]
pub struct ModbusChannel {
    pub id: usize,
    pub enabled: bool,
    pub name: String,
    pub address: u16,
    pub channel_type: ModbusChannelType,
    pub value: ModbusValue,
}

#[derive(serde::Deserialize, serde::Serialize, Clone)]
pub enum ModbusValue {
    Int(u16),
    Real(f32),
    Bool(bool),
}

#[derive(serde::Deserialize, serde::Serialize, Clone)]
pub enum ModbusChannelType {
    Int,
    Real,
    Coil,
}

#[derive(serde::Deserialize, serde::Serialize, Clone)]
pub enum ModbusDeviceConfig {
    Tcp(ModbusTcpConfig),
    Serial,
}

#[derive(serde::Deserialize, serde::Serialize, Clone)]
pub struct ModbusTcpConfig {
    pub ip: String,
    pub port: usize,
}

#[derive(serde::Deserialize, serde::Serialize, Clone)]
pub struct ModbusDevice {
    pub id: usize,
    pub code: String,
    pub name: String,
    pub config: ModbusDeviceConfig,
    pub channels: Vec<ModbusChannel>,
}

impl ModbusDevice {
    pub async fn connect_to_device(&mut self) -> Result<tokio_modbus::client::Context> {
        match &self.config {
            ModbusDeviceConfig::Tcp(conf) => {
                let socket_string = format!("{}:{}", conf.ip, conf.port);
                let socket_addr = socket_string.parse::<SocketAddr>()?;

                let ctx = connect(socket_addr).await?;

                Ok(ctx)
            }
            // TODO
            ModbusDeviceConfig::Serial => {
                unimplemented!();
            }
        }
    }
    pub async fn poll(&mut self, ctx: &mut tokio_modbus::client::Context) -> Result<()> {
        for channel in self.channels.iter_mut() {
            match channel.channel_type {
                ModbusChannelType::Int => {
                    let value = ctx.read_holding_registers(channel.address, 1).await??;

                    // Possibility to error-check for empty array.
                    let value = value.iter().nth(0);
                    channel.value = ModbusValue::Int(*value.unwrap());
                }
                ModbusChannelType::Real => {
                    let values = ctx.read_holding_registers(channel.address, 2).await??;
                    let value = u16_to_float(values[0], values[1]);

                    channel.value = ModbusValue::Real(value);
                }
                // TODO
                ModbusChannelType::Coil => {}
            }
        }

        Ok(())
    }
}

fn u16_to_float(reg1: u16, reg2: u16) -> f32 {
    let data_32bit_rep = ((reg1 as u32) << 16) | reg2 as u32;
    let data_array = data_32bit_rep.to_ne_bytes();
    f32::from_ne_bytes(data_array)
}

pub fn init_mb_tcp_device(
    ip: String,
    port: usize,
    name: String,
    num_channels: usize,
) -> ModbusDevice {
    let tcp_config = ModbusTcpConfig { ip, port };

    let device_config = ModbusDeviceConfig::Tcp(tcp_config);

    let mut channels: Vec<ModbusChannel> = Vec::with_capacity(num_channels);

    for i in 1..=num_channels {
        let channel = ModbusChannel {
            id: i,
            enabled: true,
            name: format!("MB{i}"),
            address: i as u16 * 2,
            channel_type: ModbusChannelType::Real,
            value: ModbusValue::Real(3.0),
        };

        channels.push(channel);
    }

    let device = ModbusDevice {
        id: 0,
        code: "MB".to_owned(),
        name,
        config: device_config,
        channels,
    };

    device
}
