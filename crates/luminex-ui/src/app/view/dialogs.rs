use iced::{Element, Length, Padding, Color, Theme, Border, Background};
use iced::widget::{column, row, text, button, container, Space, text_input};

use crate::app::{App, Message};
use crate::theme::colors;

impl App {
    pub fn view_confirm_delete_modal(&self) -> Element<'_, Message> {
        let target_name = self.confirm_delete_target
            .as_ref()
            .and_then(|p| p.file_name())
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "this item".to_string());

        let is_dir = self.confirm_delete_target
            .as_ref()
            .map(|p| p.is_dir())
            .unwrap_or(false);

        let item_type = if is_dir { "folder" } else { "file" };

        let modal_content = column![
            text("Confirm Delete").size(16).color(colors::TEXT_PRIMARY),
            Space::with_height(12),
            text(format!("Are you sure you want to delete the {} \"{}\"?", item_type, target_name))
                .size(13)
                .color(colors::TEXT_SECONDARY),
            Space::with_height(4),
            text("You can undo this with Ctrl+Z.")
                .size(11)
                .color(colors::TEXT_MUTED),
            Space::with_height(16),
            row![
                button(
                    text("Cancel").size(13).color(colors::TEXT_PRIMARY)
                )
                .padding(Padding::from([8, 20]))
                .style(|_: &Theme, status: button::Status| {
                    let bg = match status {
                        button::Status::Hovered => colors::BG_HOVER,
                        _ => colors::BG_LIGHT,
                    };
                    button::Style {
                        background: Some(Background::Color(bg)),
                        text_color: colors::TEXT_PRIMARY,
                        border: Border {
                            color: colors::BORDER,
                            width: 1.0,
                            radius: 4.0.into(),
                        },
                        ..Default::default()
                    }
                })
                .on_press(Message::ConfirmDeleteCancel),
                Space::with_width(12),
                button(
                    text("Delete").size(13).color(Color::from_rgb(1.0, 1.0, 1.0))
                )
                .padding(Padding::from([8, 20]))
                .style(|_: &Theme, status: button::Status| {
                    let bg = match status {
                        button::Status::Hovered => Color::from_rgb(0.85, 0.25, 0.25),
                        _ => Color::from_rgb(0.75, 0.22, 0.22),
                    };
                    button::Style {
                        background: Some(Background::Color(bg)),
                        text_color: Color::WHITE,
                        border: Border {
                            radius: 4.0.into(),
                            ..Default::default()
                        },
                        ..Default::default()
                    }
                })
                .on_press(Message::ConfirmDeleteYes),
            ]
            .align_y(iced::Alignment::Center),
        ]
        .padding(24)
        .width(Length::Fixed(380.0));

        container(
            container(modal_content)
                .style(|_| container::Style {
                    background: Some(Background::Color(colors::BG_MEDIUM)),
                    border: Border {
                        color: colors::BORDER,
                        width: 1.0,
                        radius: 8.0.into(),
                    },
                    ..Default::default()
                })
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .into()
    }

    pub fn view_rename_modal(&self) -> Element<'_, Message> {
        let original_name = self.rename_target
            .as_ref()
            .and_then(|p| p.file_name())
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();

        let modal_content = column![
            text("Rename").size(16).color(colors::TEXT_PRIMARY),
            Space::with_height(12),
            text(format!("Renaming: \"{}\"", original_name))
                .size(12)
                .color(colors::TEXT_MUTED),
            Space::with_height(8),
            text_input("Enter new name...", &self.rename_input)
                .on_input(Message::RenameInputChanged)
                .on_submit(Message::RenameConfirm)
                .padding(Padding::from([8, 12]))
                .size(13),
            Space::with_height(16),
            row![
                button(
                    text("Cancel").size(13).color(colors::TEXT_PRIMARY)
                )
                .padding(Padding::from([8, 20]))
                .style(|_: &Theme, status: button::Status| {
                    let bg = match status {
                        button::Status::Hovered => colors::BG_HOVER,
                        _ => colors::BG_LIGHT,
                    };
                    button::Style {
                        background: Some(Background::Color(bg)),
                        text_color: colors::TEXT_PRIMARY,
                        border: Border {
                            color: colors::BORDER,
                            width: 1.0,
                            radius: 4.0.into(),
                        },
                        ..Default::default()
                    }
                })
                .on_press(Message::RenameCancel),
                Space::with_width(12),
                button(
                    text("Rename").size(13).color(Color::WHITE)
                )
                .padding(Padding::from([8, 20]))
                .style(|_: &Theme, status: button::Status| {
                    let bg = match status {
                        button::Status::Hovered => Color::from_rgb(0.40, 0.58, 0.95),
                        _ => colors::ACCENT,
                    };
                    button::Style {
                        background: Some(Background::Color(bg)),
                        text_color: Color::WHITE,
                        border: Border {
                            radius: 4.0.into(),
                            ..Default::default()
                        },
                        ..Default::default()
                    }
                })
                .on_press(Message::RenameConfirm),
            ]
            .align_y(iced::Alignment::Center),
        ]
        .padding(24)
        .width(Length::Fixed(380.0));

        container(
            container(modal_content)
                .style(|_| container::Style {
                    background: Some(Background::Color(colors::BG_MEDIUM)),
                    border: Border {
                        color: colors::BORDER,
                        width: 1.0,
                        radius: 8.0.into(),
                    },
                    ..Default::default()
                })
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .into()
    }

    pub fn view_goto_line_modal(&self) -> Element<'_, Message> {
        let modal_content = column![
            text("Go to Line").size(16).color(colors::TEXT_PRIMARY),
            Space::with_height(12),
            text_input("Line number...", &self.goto_line_input)
                .on_input(Message::GotoLineInputChanged)
                .on_submit(Message::GotoLineConfirm)
                .padding(Padding::from([8, 12]))
                .size(13),
            Space::with_height(16),
            row![
                button(
                    text("Cancel").size(13).color(colors::TEXT_PRIMARY)
                )
                .padding(Padding::from([8, 20]))
                .style(|_: &Theme, status: button::Status| {
                    let bg = match status {
                        button::Status::Hovered => colors::BG_HOVER,
                        _ => colors::BG_LIGHT,
                    };
                    button::Style {
                        background: Some(Background::Color(bg)),
                        text_color: colors::TEXT_PRIMARY,
                        border: Border {
                            color: colors::BORDER,
                            width: 1.0,
                            radius: 4.0.into(),
                        },
                        ..Default::default()
                    }
                })
                .on_press(Message::GotoLineCancel),
                Space::with_width(12),
                button(
                    text("Go").size(13).color(Color::WHITE)
                )
                .padding(Padding::from([8, 20]))
                .style(|_: &Theme, status: button::Status| {
                    let bg = match status {
                        button::Status::Hovered => Color::from_rgb(0.40, 0.58, 0.95),
                        _ => colors::ACCENT,
                    };
                    button::Style {
                        background: Some(Background::Color(bg)),
                        text_color: Color::WHITE,
                        border: Border {
                            radius: 4.0.into(),
                            ..Default::default()
                        },
                        ..Default::default()
                    }
                })
                .on_press(Message::GotoLineConfirm),
            ]
            .align_y(iced::Alignment::Center),
        ]
        .padding(24)
        .width(Length::Fixed(320.0));

        container(
            container(modal_content)
                .style(|_| container::Style {
                    background: Some(Background::Color(colors::BG_MEDIUM)),
                    border: Border {
                        color: colors::BORDER,
                        width: 1.0,
                        radius: 8.0.into(),
                    },
                    ..Default::default()
                })
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .into()
    }

    pub fn view_about_modal(&self) -> Element<'_, Message> {
        let modal_content = column![
            text("Luminex").size(20).color(colors::ACCENT),
            Space::with_height(8),
            text("A modern text editor built with Rust & Iced")
                .size(13)
                .color(colors::TEXT_SECONDARY),
            Space::with_height(12),
            text("Version 0.1.0").size(12).color(colors::TEXT_MUTED),
            Space::with_height(20),
            button(
                text("Close").size(13).color(Color::WHITE)
            )
            .padding(Padding::from([8, 24]))
            .style(|_: &Theme, status: button::Status| {
                let bg = match status {
                    button::Status::Hovered => Color::from_rgb(0.40, 0.58, 0.95),
                    _ => colors::ACCENT,
                };
                button::Style {
                    background: Some(Background::Color(bg)),
                    text_color: Color::WHITE,
                    border: Border {
                        radius: 4.0.into(),
                        ..Default::default()
                    },
                    ..Default::default()
                }
            })
            .on_press(Message::HideAbout),
        ]
        .padding(24)
        .width(Length::Fixed(340.0))
        .align_x(iced::Alignment::Center);

        container(
            container(modal_content)
                .style(|_| container::Style {
                    background: Some(Background::Color(colors::BG_MEDIUM)),
                    border: Border {
                        color: colors::BORDER,
                        width: 1.0,
                        radius: 8.0.into(),
                    },
                    ..Default::default()
                })
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .into()
    }
}
