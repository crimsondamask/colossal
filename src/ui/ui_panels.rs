use egui::{Color32, Frame, Margin, Sense, Stroke};
use egui_extras::{Column, TableBuilder};

use crate::ui::ModbusDeviceType;
use crate::ColossalApp;

pub fn ui_device_channels_table(app: &mut ColossalApp, ui: &mut egui::Ui) -> anyhow::Result<()> {
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

        ui.collapsing("Device Config", |ui| {
            egui::Grid::new("device_config")
                .num_columns(4)
                .min_col_width(200.)
                .show(ui, |ui| {
                    egui::ComboBox::from_label("Device Type")
                        .selected_text(format!("{}", app.device_config_ui_buffer.device_type))
                        .show_ui(ui, |ui| {
                            ui.selectable_value(
                                &mut app.device_config_ui_buffer.device_type,
                                ModbusDeviceType::Tcp,
                                "Modbus TCP",
                            );
                            ui.selectable_value(
                                &mut app.device_config_ui_buffer.device_type,
                                ModbusDeviceType::Serial,
                                "Modbus Serial",
                            );
                        });

                    ui.end_row();
                    match app.device_config_ui_buffer.device_type {
                        ModbusDeviceType::Tcp => {
                            ui.label("IP");
                            ui.text_edit_singleline(&mut app.device_config_ui_buffer.ip);
                            ui.end_row();
                            ui.label("Port");
                            ui.text_edit_singleline(&mut app.device_config_ui_buffer.port);
                        }
                        ModbusDeviceType::Serial => {}
                    }
                    ui.end_row();

                    // If clicked we update the device config.
                    if ui
                        .button(format!("{} Save", egui_phosphor::regular::FLOPPY_DISK))
                        .clicked()
                    {
                        let port = app.device_config_ui_buffer.port.parse::<usize>();
                        if let Ok(port) = port {}
                    }
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
                if let Some(received_device_data) = &app.received_device_data {
                    let num_channel_rows = received_device_data.channels.len();

                    body.rows(row_height, num_channel_rows, |mut row| {
                        let index = row.index();

                        let device_channel = &received_device_data.channels[index];
                        if let Some(selected_index) = app.tabel_selected_row {
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
                            app.tabel_selected_row = Some(index);
                        }
                    });
                }
            });
    });
    Ok(())
}

// Top colored status panel to show error messages.
pub fn ui_status_panel(app: &mut ColossalApp, ctx: &egui::Context) -> anyhow::Result<()> {
    egui::TopBottomPanel::top("status_panel")
        .exact_height(20.0)
        .frame(app.status_bar_frame)
        .show(ctx, |ui| {
            ui.vertical_centered_justified(|ui| {
                ui.label(
                    egui::RichText::new(format!("{}", app.thread_status,)).color(Color32::WHITE),
                );
            });
        });
    Ok(())
}

// Right panel to show details
pub fn ui_right_panel(app: &mut ColossalApp, ctx: &egui::Context) -> anyhow::Result<()> {
    egui::SidePanel::right("right")
        .min_width(200.)
        .show(ctx, |_ui| {
            if let Some(_selected_channel_index) = app.tabel_selected_row {}
        });
    Ok(())
}
