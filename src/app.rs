use egui::{Color32, CornerRadius, Frame, Margin, RichText, Sense, Stroke, TextEdit, Visuals};
use egui_extras::{Column, TableBuilder};
use egui_phosphor;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::sync::mpsc::{Receiver, Sender};

use crate::calculation_channel::*;
use crate::modbus_device::*;
use crate::ui::ModbusDeviceBuffer;

#[derive(serde::Deserialize, serde::Serialize)]
enum ThreadStatus {
    Healthy(String),
    Error(String),
}
/// We derive Deserialize/Serialize so we can persist app state on shutdown.
#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct ColossalApp {
    label: String,

    // Device tabel UI buffer.
    device_config_ui_buffer: ModbusDeviceBuffer,

    // Channels table selected row
    tabel_selected_row: Option<usize>,
    // Holder of the received data from the thread
    received_device_data: Option<ModbusDevice>,
    #[serde(skip)]
    thread_status: String,
    #[serde(skip)]
    status_bar_frame: Frame,
    // Device and Calc Channels======================
    #[serde(skip)]
    modbus_devices: Vec<ModbusDevice>,
    #[serde(skip)]
    calculation_channels: Vec<CalculationChannel>,
    // ===============================================
    // Thread communication channels
    #[serde(skip)]
    sender_main_to_thread: Sender<ModbusDevice>,
    #[serde(skip)]
    receiver_thread_to_main: Receiver<ModbusDevice>,
    // Thread status
    #[serde(skip)]
    receiver_status_to_main: Receiver<ThreadStatus>,

    // First scan latch===============================
    #[serde(skip)]
    first_scan: bool,
    // ===============================================
    #[serde(skip)]
    value: f32,
}

impl Default for ColossalApp {
    fn default() -> Self {
        let device = init_mb_tcp_device("127.0.0.1".to_owned(), 5502, "Device_1".to_owned(), 10);
        let calculation_channels = init_channel_list(5);

        // This is just a placeholder for the application startup.
        // sender and receiver will be overwritten later.
        let (sender, receiver) = mpsc::channel(16);
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
            sender_main_to_thread: sender,
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

    fn device_channels_table(&mut self, ui: &mut egui::Ui) {
        let table_frame = Frame {
            stroke: Stroke::new(1.0, Color32::LIGHT_YELLOW),
            inner_margin: Margin::symmetric(10, 10),
            ..Default::default()
        };

        // Surround the table with a frame
        table_frame.show(ui, |ui| {
            ui.vertical_centered_justified(|ui| {
                // Table title
                ui.label(format!(
                    "{} Device Channels",
                    egui_phosphor::regular::PLUGS_CONNECTED
                ))
            });
            ui.separator();

            egui::Grid::new("device_config")
                .num_columns(3)
                .show(ui, |ui| {
                    egui::ComboBox::from_label("Device Type:")
                        .selected_text(format!("{}", self.device_config_ui_buffer.device_type))
                        .show_ui(ui, |ui| {
                            ui.selectable_value(
                                &mut self.device_config_ui_buffer.device_type,
                                crate::ModbusDeviceType::Tcp,
                                "Modbus TCP",
                            );
                            ui.selectable_value(
                                &mut self.device_config_ui_buffer.device_type,
                                crate::ModbusDeviceType::Serial,
                                "Modbus Serial",
                            );
                        });
                });

            ui.separator();
            let channels_table_avl_height = 200.0;

            // We build the device channels table.
            let mut device_channels_table = TableBuilder::new(ui)
                .striped(true)
                .resizable(false)
                .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                .column(Column::exact(80.))
                .column(Column::exact(80.))
                .column(Column::exact(80.))
                .column(Column::exact(80.))
                .column(Column::exact(80.))
                .column(Column::remainder())
                .vscroll(true)
                .auto_shrink(false)
                .min_scrolled_height(0.0)
                .max_scroll_height(channels_table_avl_height)
                .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysVisible);

            // The table rows should sense the user clicks for selection.
            device_channels_table = device_channels_table.sense(Sense::click());

            device_channels_table
                .header(30.0, |mut header| {
                    header.col(|ui| {
                        ui.strong("INDEX");
                    });
                    header.col(|ui| {
                        ui.strong("NAME");
                    });
                    header.col(|ui| {
                        ui.strong("TYPE");
                    });
                    header.col(|ui| {
                        ui.strong("ADDRESS");
                    });
                    header.col(|ui| {
                        ui.strong("VALUE");
                    });
                    header.col(|ui| {
                        ui.strong("DESCRIPTION");
                    });
                })
                .body(|body| {
                    let row_height = 20.0;
                    if let Some(received_device_data) = &self.received_device_data {
                        let num_channel_rows = received_device_data.channels.len();

                        body.rows(row_height, num_channel_rows, |mut row| {
                            let index = row.index();

                            let device_channel = &received_device_data.channels[index];
                            if let Some(selected_index) = self.tabel_selected_row {
                                if selected_index == index {
                                    row.set_selected(true);
                                }
                            }

                            row.col(|ui| {
                                ui.label(format!("{index}"));
                            });
                            // Channel name
                            row.col(|ui| {
                                ui.label(format!("{device_channel}"));
                            });
                            // Channel type
                            row.col(|ui| {
                                ui.label(format!("{}", &device_channel.channel_type));
                            });
                            row.col(|ui| {
                                ui.label(format!("{}", &device_channel.address));
                            });
                            row.col(|ui| {
                                ui.label(format!("{}", &device_channel.value));
                            });
                            row.col(|ui| {
                                ui.label(format!("{}", &device_channel.description));
                            });

                            if row.response().clicked() {
                                self.tabel_selected_row = Some(index);
                            }
                        });
                    }
                });
        });
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
            let (sender_main_to_thread, mut receiver_main_to_thread): (
                Sender<ModbusDevice>,
                Receiver<ModbusDevice>,
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

            std::thread::spawn(move || {
                let mut devices = devices;
                tokio::runtime::Builder::new_multi_thread()
                    .enable_all()
                    .build()
                    .unwrap()
                    .block_on(async move {
                        //let mut device = devices[0].clone();
                        loop {
                            let ctx = devices[0].connect_to_device().await;
                            match ctx {
                                Ok(mut ctx) => {
                                    loop {
                                        //
                                        if let Ok(msg) = receiver_main_to_thread.try_recv() {
                                            sender_status_to_main
                                                .send(ThreadStatus::Healthy(
                                                    "Config update.".to_owned(),
                                                ))
                                                .await
                                                .unwrap();
                                            println!("Thread received a message");
                                        }
                                        match devices[0].poll(&mut ctx).await {
                                            Ok(_) => {
                                                sender_status_to_main
                                                    .send(ThreadStatus::Healthy(
                                                        format!("Healthy",),
                                                    ))
                                                    .await
                                                    .unwrap();
                                                match sender_thread_to_main
                                                    .send(devices[0].clone())
                                                    .await
                                                {
                                                    Ok(_) => {}
                                                    Err(e) => {
                                                        println!("Sender error: {e}");
                                                        sender_status_to_main
                                                            .send(ThreadStatus::Error(
                                                                "Could not send data back to main."
                                                                    .to_owned(),
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
                                                    .send(ThreadStatus::Error(format!(
                                                        "Poll error: {e}"
                                                    )))
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
                                                    println!(
                                                        "Calculation result: {}",
                                                        channel.value
                                                    );
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
        egui::TopBottomPanel::top("status_panel")
            .exact_height(20.0)
            .frame(self.status_bar_frame)
            .show(ctx, |ui| {
                ui.vertical_centered_justified(|ui| {
                    ui.label(
                        egui::RichText::new(format!("{}", self.thread_status,))
                            .color(Color32::WHITE),
                    );
                });
            });
        egui::SidePanel::right("right")
            .min_width(200.)
            .show(ctx, |ui| {
                if let Some(selected_channel_index) = self.tabel_selected_row {}
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            // The central panel the region left after adding TopPanel's and SidePanel's

            // Check for any data coming from the thread.
            if let Ok(received_device_data) = self.receiver_thread_to_main.try_recv() {
                self.received_device_data = Some(received_device_data);
            }

            self.device_channels_table(ui);
        });
    }
}

fn _powered_by_egui_and_eframe(ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        ui.label("Powered by ");
        ui.hyperlink_to("egui", "https://github.com/emilk/egui");
        ui.label(" and ");
        ui.hyperlink_to(
            "eframe",
            "https://github.com/emilk/egui/tree/master/crates/eframe",
        );
        ui.label(".");
    });
}
