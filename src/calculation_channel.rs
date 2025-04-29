use crate::modbus_device::*;
use anyhow::{anyhow, Result};
use egui::TextBuffer;
use regex::Regex;
use rhai::{Engine, EvalAltResult};

#[derive(serde::Deserialize, serde::Serialize, Clone)]
pub struct CalculationChannel {
    pub enabled: bool,
    pub id: usize,
    pub name: String,
    pub calculation: String,
    pub value: f64,
    pub error: Option<String>,
}

impl CalculationChannel {
    // Evaluate the channel calculation and store it in the value member.
    pub fn evaluate(&mut self, devices: &Vec<ModbusDevice>) -> Result<()> {
        let re = Regex::new(r"\bMB\D{0,4}(\d{0,8})(?:\d|$)")?;
        let hay = &self.calculation;
        let mut calculation_string: String = self.calculation.clone();
        let matches: Vec<_> = re.find_iter(hay.as_str()).map(|m| m.as_str()).collect();

        let mut replace_value = String::new();

        //println!("{:?}", matches);
        for m in matches {
            let match_str = format!("{}", m);
            for device in devices {
                for channel in &device.channels {
                    if channel.name == match_str.trim() {
                        match channel.value {
                            ModbusValue::Int(v) => {
                                replace_value = format!("{}", v);
                            }
                            ModbusValue::Real(v) => {
                                replace_value = format!("{}", v);
                            }
                            ModbusValue::Bool(_) => {
                                unimplemented!()
                            }
                        }

                        calculation_string =
                            calculation_string.replace(match_str.as_str(), &replace_value);
                        let engine = Engine::new();
                        let result = engine.eval::<f64>(&calculation_string);

                        match result {
                            Ok(v) => {
                                self.value = v;
                            }
                            Err(e) => {
                                anyhow::bail!("{e}");
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }
}

// A convenience method to construct a channel array.
pub fn init_channel_list(n: usize) -> Vec<CalculationChannel> {
    let mut channel_list = Vec::with_capacity(n);

    for i in 1..=n {
        // The 0.0 is a temporal fix for a weird bug.
        let calculation = format!("MB{i} + MB{i} + 0.0");
        let channel = CalculationChannel {
            enabled: true,
            id: i,
            name: format!("CH{i}"),
            calculation,
            value: 0.0,
            error: None,
        };

        channel_list.push(channel);
    }

    channel_list
}
