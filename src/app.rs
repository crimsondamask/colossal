use egui::{Color32, CornerRadius, Frame, Visuals};
use egui_phosphor;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::sync::mpsc::{Receiver, Sender};

use crate::calculation_channel::*;
use crate::modbus_device::*;
use crate::ui::ui_panels::*;
use crate::ui::ModbusDeviceBuffer;

#[derive(serde::Deserialize, serde::Serialize)]
pub enum ThreadStatus {
    Healthy(String),
    Error(String),
}
/// We derive Deserialize/Serialize so we can persist app state on shutdown.
#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct ColossalApp {
    pub label: String,

    // Device tabel UI buffer.
    pub device_config_ui_buffer: ModbusDeviceBuffer,

    // Channels table selected row
    pub tabel_selected_row: Option<usize>,
    // Holder of the received data from the thread
    pub received_device_data: Option<ModbusDevice>,
    #[serde(skip)]
    pub thread_status: String,
    #[serde(skip)]
    pub status_bar_frame: Frame,
    // Device and Calc Channels======================
    #[serde(skip)]
    pub modbus_devices: Vec<ModbusDevice>,
    #[serde(skip)]
    pub calculation_channels: Vec<CalculationChannel>,
    // ===============================================
    // Thread communication channels
    #[serde(skip)]
    pub sender_main_to_thread: Sender<ModbusDeviceConfig>,
    #[serde(skip)]
    pub receiver_thread_to_main: Receiver<ModbusDevice>,
    // Thread status
    #[serde(skip)]
    pub receiver_status_to_main: Receiver<ThreadStatus>,

    // First scan latch===============================
    #[serde(skip)]
    pub first_scan: bool,
    // ===============================================
    #[serde(skip)]
    pub value: f32,
}

impl Default for ColossalApp {
    fn default() -> Self {
        let device = init_mb_tcp_device("127.0.0.1".to_owned(), 5502, "Device_1".to_owned(), 10);
        let calculation_channels = init_channel_list(5);

        // This is just a placeholder for the application startup.
        // sender and receiver will be overwritten later.
        let (_sender, receiver) = mpsc::channel(16);
        let (config_sender, _config_receiver) = mpsc::channel(16);
        let (_, receiver_status) = mpsc::channel(16);

        Self {
            // Example stuff:
            device_config_ui_buffer: ModbusDeviceBuffer::default(),
            tabel_selected_row: None,
            modbus_devices: vec![device],
            status_bar_frame: Frame::new(),
            thread_status: String::from("Status: Healthy"),
            calculation_channels,
            received_device_data: None,
            sender_main_to_thread: config_sender,
            receiver_thread_to_main: receiver,
            receiver_status_to_main: receiver_status,
            first_scan: true,
            label: "Hello World!".to_owned(),
            value: 2.7,
        }
    }
}

impl ColossalApp {
    /// Called once before the first frame.
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // This is also where you can customize the look and feel of egui using
        // `cc.egui_ctx.set_visuals` and `cc.egui_ctx.set_fonts`.

        let mut fonts = egui::FontDefinitions::default();
        fonts.font_data.insert(
            "custom_font".to_owned(),
            Arc::new(egui::FontData::from_static(include_bytes!(
                "../assets/plex.ttf"
            ))),
            //egui::FontData::from_static(include_bytes!("../assets/dejavu.ttf")),
        );
        fonts
            .families
            .entry(egui::FontFamily::Proportional)
            .or_default()
            .insert(0, "custom_font".to_owned());

        // Adding phosphor font icons
        //
        egui_phosphor::add_to_fonts(&mut fonts, egui_phosphor::Variant::Regular);
        cc.egui_ctx.set_fonts(fonts);

        // Setting the theme.
        let mut visuals = Visuals::dark();

        visuals.menu_corner_radius = CornerRadius::ZERO;
        visuals.widgets.inactive.corner_radius = CornerRadius::ZERO;
        visuals.widgets.noninteractive.corner_radius = CornerRadius::ZERO;
        visuals.widgets.active.corner_radius = CornerRadius::ZERO;
        visuals.widgets.hovered.corner_radius = CornerRadius::ZERO;
        visuals.window_corner_radius = CornerRadius::ZERO;
        visuals.override_text_color = Some(Color32::WHITE);
        cc.egui_ctx.set_visuals(visuals);

        // Load previous app state (if any).
        // Note that you must enable the `persistence` feature for this to work.
        if let Some(storage) = cc.storage {
            return eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default();
        }

        Default::default()
    }
}

impl eframe::App for ColossalApp {
    /// Called by the frame work to save state before shutdown.
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }

    /// Called each time the UI needs repainting, which may be many times per second.
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Make sure the UI keeps updating when there is no
        // user input.
        ctx.request_repaint();
        // We spawn the polling thread at startup.
        if self.first_scan {
            let devices = self.modbus_devices.clone();
            let calculation_channels = self.calculation_channels.clone();

            // Configuration update channel.
            // We send it from the GUI main to the thread
            // in case of configuration changes.
            let (sender_main_to_thread, receiver_main_to_thread): (
                Sender<ModbusDeviceConfig>,
                Receiver<ModbusDeviceConfig>,
            ) = mpsc::channel(16);

            // Data polling channel. This is the main channel
            // that carries all the polling data.
            let (sender_thread_to_main, receiver_thread_to_main): (
                Sender<ModbusDevice>,
                Receiver<ModbusDevice>,
            ) = mpsc::channel(16);

            // Thread health status.
            // We use it to send any error messages to the main thread
            let (sender_status_to_main, receiver_status_to_main): (
                Sender<ThreadStatus>,
                Receiver<ThreadStatus>,
            ) = mpsc::channel(16);

            self.sender_main_to_thread = sender_main_to_thread;
            self.receiver_thread_to_main = receiver_thread_to_main;
            self.receiver_status_to_main = receiver_status_to_main;

            // We spawn a thread to keep polling the device
            // The thread will stay alive for the entirety
            // of the app lifetime and will try to reconnect the
            // device
            std::thread::spawn(move || {
                let devices = devices;
                tokio::runtime::Builder::new_multi_thread()
                    .enable_all()
                    .build()
                    .unwrap()
                    .block_on(async move {
                        //let mut device = devices[0].clone();
                        async_pool_thread(
                            calculation_channels,
                            receiver_main_to_thread,
                            sender_thread_to_main,
                            sender_status_to_main,
                            devices,
                        )
                        .await
                    });
            });
            self.first_scan = false;
        }

        let top_panel_frame = Frame {
            fill: Color32::from_rgb(54, 77, 99),
            ..Default::default()
        };
        egui::TopBottomPanel::top("top_panel")
            .frame(top_panel_frame)
            .show(ctx, |ui| {
                // The top panel is often a good place for a menu bar:

                egui::menu::bar(ui, |ui| {
                    // NOTE: no File->Quit on web pages!
                    let is_web = cfg!(target_arch = "wasm32");
                    if !is_web {
                        ui.menu_button("File", |ui| {
                            if ui.button("Quit").clicked() {
                                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                            }
                        });
                        ui.add_space(16.0);
                    }
                });
            });

        if let Ok(status_msg) = self.receiver_status_to_main.try_recv() {
            match status_msg {
                ThreadStatus::Healthy(msg) => {
                    self.status_bar_frame = Frame {
                        fill: Color32::DARK_GREEN,

                        ..Default::default()
                    };

                    self.thread_status = format!("{} {}", egui_phosphor::regular::HEARTBEAT, msg);
                }
                ThreadStatus::Error(e) => {
                    self.status_bar_frame = Frame {
                        fill: Color32::DARK_RED,

                        ..Default::default()
                    };
                    self.thread_status = format!("{} {}", egui_phosphor::regular::WARNING, e);
                }
            }
        }

        match ui_status_panel(self, ctx) {
            Ok(_) => {}
            Err(e) => println!("{e}"),
        }

        match ui_right_panel(self, ctx) {
            Ok(_) => {}
            Err(e) => println!("{e}"),
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            // The central panel the region left after adding TopPanel's and SidePanel's

            // Check for any data coming from the thread.
            if let Ok(received_device_data) = self.receiver_thread_to_main.try_recv() {
                self.received_device_data = Some(received_device_data);
            }

            match ui_device_channels_table(self, ui) {
                Ok(_) => {}
                Err(e) => println!("{e}"),
            }
        });
    }
}

async fn async_pool_thread(
    calculation_channels: Vec<CalculationChannel>,
    mut receiver_main_to_thread: Receiver<ModbusDeviceConfig>,
    sender_thread_to_main: Sender<ModbusDevice>,
    sender_status_to_main: Sender<ThreadStatus>,
    mut devices: Vec<ModbusDevice>,
) {
    loop {
        let ctx = devices[0].connect_to_device().await;
        match ctx {
            Ok(mut ctx) => {
                loop {
                    //
                    if let Ok(_msg) = receiver_main_to_thread.try_recv() {
                        sender_status_to_main
                            .send(ThreadStatus::Healthy("Config update.".to_owned()))
                            .await
                            .unwrap();
                        println!("Thread received a message");
                    }
                    match devices[0].poll(&mut ctx).await {
                        Ok(_) => {
                            sender_status_to_main
                                .send(ThreadStatus::Healthy(format!("Healthy",)))
                                .await
                                .unwrap();
                            match sender_thread_to_main.send(devices[0].clone()).await {
                                Ok(_) => {}
                                Err(e) => {
                                    println!("Sender error: {e}");
                                    sender_status_to_main
                                        .send(ThreadStatus::Error(
                                            "Could not send data back to main.".to_owned(),
                                        ))
                                        .await
                                        .unwrap();
                                }
                            }
                            for channel in &devices[0].channels {
                                println!("{:?}", channel.value);
                            }
                        }
                        Err(e) => {
                            println!("{e}");
                            sender_status_to_main
                                .send(ThreadStatus::Error(format!("Poll error: {e}")))
                                .await
                                .unwrap();

                            break;
                        }
                    }
                    // Evaluate each calculation channel.
                    for mut channel in calculation_channels.clone() {
                        // Use the reference to devices so we are sure
                        // we are working with the updated values from
                        // the poll function.
                        match channel.evaluate(&devices) {
                            Ok(_) => {
                                println!("Calculation result: {}", channel.value);
                            }
                            Err(e) => {
                                sender_status_to_main
                                    .send(ThreadStatus::Error(format!(
                                        "Calculation evaluation error: {e}"
                                    )))
                                    .await
                                    .unwrap();
                            }
                        }
                    }
                    std::thread::sleep(Duration::from_millis(1000));
                }
            }
            Err(e) => {
                sender_status_to_main
                    .send(ThreadStatus::Error(format!("{e}")))
                    .await
                    .unwrap();
                println!("{e}");
                continue;
            }
        }

        // We wait for a while before the next connection attempt.
        std::thread::sleep(Duration::from_millis(5000));
    }
}
