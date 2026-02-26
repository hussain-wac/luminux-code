pub mod dialogs;
pub mod docks;
pub mod editor;
pub mod sidebar;
pub mod terminal;

use iced::{Element, Length, Color, Theme, Border, Background, Padding};
use iced::widget::{column, row, text, button, container, Space, Column, mouse_area, stack, horizontal_space};
use std::path::PathBuf;

use crate::app::{App, Message};
use crate::theme::colors;

impl App {
    pub fn view(&self) -> Element<'_, Message> {
        let content = column![
            self.view_toolbar(),
            row![
                if self.sidebar_visible {
                    self.view_sidebar()
                } else {
                    container(Space::new(0, 0)).into()
                },
                self.view_main_area(),
            ]
            .height(Length::Fill),
            self.view_status_bar(),
        ];

        let main_view: Element<'_, Message> = container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .style(|_| container::Style {
                background: Some(Background::Color(colors::BG_DARK)),
                ..Default::default()
            })
            .into();

        let tracked_view: Element<'_, Message> = mouse_area(main_view)
            .on_move(Message::MouseMoved)
            .into();

        if self.confirm_delete_visible {
            stack![
                tracked_view,
                mouse_area(
                    container(Space::new(Length::Fill, Length::Fill))
                        .width(Length::Fill)
                        .height(Length::Fill)
                        .style(|_| container::Style {
                            background: Some(Background::Color(Color::from_rgba(0.0, 0.0, 0.0, 0.5))),
                            ..Default::default()
                        })
                )
                .on_press(Message::ConfirmDeleteCancel),
                self.view_confirm_delete_modal(),
            ]
            .into()
        } else if self.rename_visible {
            stack![
                tracked_view,
                mouse_area(
                    container(Space::new(Length::Fill, Length::Fill))
                        .width(Length::Fill)
                        .height(Length::Fill)
                        .style(|_| container::Style {
                            background: Some(Background::Color(Color::from_rgba(0.0, 0.0, 0.0, 0.5))),
                            ..Default::default()
                        })
                )
                .on_press(Message::RenameCancel),
                self.view_rename_modal(),
            ]
            .into()
        } else if self.context_menu.visible {
            stack![
                mouse_area(
                    container(Space::new(Length::Fill, Length::Fill))
                        .width(Length::Fill)
                        .height(Length::Fill)
                )
                .on_press(Message::HideContextMenu),
                tracked_view,
                self.view_context_menu(),
            ]
            .into()
        } else if self.editor_context_visible {
            stack![
                mouse_area(
                    container(Space::new(Length::Fill, Length::Fill))
                        .width(Length::Fill)
                        .height(Length::Fill)
                )
                .on_press(Message::HideEditorContextMenu),
                tracked_view,
                self.view_editor_context_menu(),
            ]
            .into()
        } else if self.goto_line_visible {
            stack![
                tracked_view,
                mouse_area(
                    container(Space::new(Length::Fill, Length::Fill))
                        .width(Length::Fill)
                        .height(Length::Fill)
                        .style(|_| container::Style {
                            background: Some(Background::Color(Color::from_rgba(0.0, 0.0, 0.0, 0.5))),
                            ..Default::default()
                        })
                )
                .on_press(Message::GotoLineCancel),
                self.view_goto_line_modal(),
            ]
            .into()
        } else if self.about_visible {
            stack![
                tracked_view,
                mouse_area(
                    container(Space::new(Length::Fill, Length::Fill))
                        .width(Length::Fill)
                        .height(Length::Fill)
                        .style(|_| container::Style {
                            background: Some(Background::Color(Color::from_rgba(0.0, 0.0, 0.0, 0.5))),
                            ..Default::default()
                        })
                )
                .on_press(Message::HideAbout),
                self.view_about_modal(),
            ]
            .into()
        } else if self.active_menu.is_some() {
            stack![
                mouse_area(
                    container(Space::new(Length::Fill, Length::Fill))
                        .width(Length::Fill)
                        .height(Length::Fill)
                )
                .on_press(Message::CloseTopMenu),
                tracked_view,
                self.view_menu_dropdown(),
            ]
            .into()
        } else {
            tracked_view
        }
    }

    pub fn view_context_menu(&self) -> Element<'_, Message> {
        let menu_btn_style = |_: &Theme, status: button::Status| -> button::Style {
            let bg = match status {
                button::Status::Hovered => colors::BG_HOVER,
                _ => colors::BG_MEDIUM,
            };
            button::Style {
                background: Some(Background::Color(bg)),
                text_color: colors::TEXT_PRIMARY,
                border: Border::default(),
                ..Default::default()
            }
        };

        let separator: Element<'_, Message> = container(Space::new(Length::Fill, 1))
            .style(|_| container::Style {
                background: Some(Background::Color(colors::BORDER)),
                ..Default::default()
            })
            .into();

        let mut items: Vec<Element<'_, Message>> = Vec::new();

        if self.context_menu.target.is_some() {
            items.push(
                button(text("New File").size(12).color(colors::TEXT_PRIMARY))
                    .width(Length::Fill)
                    .padding(Padding::from([6, 12]))
                    .style(menu_btn_style)
                    .on_press(Message::ContextNewFile)
                    .into()
            );
            items.push(
                button(text("New Folder").size(12).color(colors::TEXT_PRIMARY))
                    .width(Length::Fill)
                    .padding(Padding::from([6, 12]))
                    .style(menu_btn_style)
                    .on_press(Message::ContextNewFolder)
                    .into()
            );
            items.push(separator);
            items.push(
                button(text("Copy").size(12).color(colors::TEXT_PRIMARY))
                    .width(Length::Fill)
                    .padding(Padding::from([6, 12]))
                    .style(menu_btn_style)
                    .on_press(Message::ContextCopy)
                    .into()
            );
            items.push(
                button(text("Cut").size(12).color(colors::TEXT_PRIMARY))
                    .width(Length::Fill)
                    .padding(Padding::from([6, 12]))
                    .style(menu_btn_style)
                    .on_press(Message::ContextCut)
                    .into()
            );
            if self.clipboard_path.is_some() {
                items.push(
                    button(text("Paste").size(12).color(colors::TEXT_PRIMARY))
                        .width(Length::Fill)
                        .padding(Padding::from([6, 12]))
                        .style(menu_btn_style)
                        .on_press(Message::ContextPaste)
                        .into()
                );
            }
            items.push(
                container(Space::new(Length::Fill, 1))
                    .style(|_| container::Style {
                        background: Some(Background::Color(colors::BORDER)),
                        ..Default::default()
                    })
                    .into()
            );
            items.push(
                button(text("Rename").size(12).color(colors::TEXT_PRIMARY))
                    .width(Length::Fill)
                    .padding(Padding::from([6, 12]))
                    .style(menu_btn_style)
                    .on_press(Message::ContextRename)
                    .into()
            );
            items.push(
                button(text("Delete").size(12).color(Color::from_rgb(0.9, 0.4, 0.4)))
                    .width(Length::Fill)
                    .padding(Padding::from([6, 12]))
                    .style(menu_btn_style)
                    .on_press(Message::ContextDelete)
                    .into()
            );
        } else {
            items.push(
                button(text("New File").size(12).color(colors::TEXT_PRIMARY))
                    .width(Length::Fill)
                    .padding(Padding::from([6, 12]))
                    .style(menu_btn_style)
                    .on_press(Message::ContextNewFile)
                    .into()
            );
            items.push(
                button(text("New Folder").size(12).color(colors::TEXT_PRIMARY))
                    .width(Length::Fill)
                    .padding(Padding::from([6, 12]))
                    .style(menu_btn_style)
                    .on_press(Message::ContextNewFolder)
                    .into()
            );
            if self.clipboard_path.is_some() {
                items.push(
                    container(Space::new(Length::Fill, 1))
                        .style(|_| container::Style {
                            background: Some(Background::Color(colors::BORDER)),
                            ..Default::default()
                        })
                        .into()
                );
                items.push(
                    button(text("Paste").size(12).color(colors::TEXT_PRIMARY))
                        .width(Length::Fill)
                        .padding(Padding::from([6, 12]))
                        .style(menu_btn_style)
                        .on_press(Message::ContextPaste)
                        .into()
                );
            }
        }

        let menu_content = Column::with_children(items).width(Length::Fixed(150.0));
        let x = self.context_menu.position.x;
        let y = self.context_menu.position.y;

        let menu_box = container(menu_content)
            .padding(4)
            .style(|_| container::Style {
                background: Some(Background::Color(colors::BG_MEDIUM)),
                border: Border {
                    color: colors::BORDER,
                    width: 1.0,
                    radius: 4.0.into(),
                },
                ..Default::default()
            });

        column![
            Space::with_height(Length::Fixed(y)),
            row![
                Space::with_width(Length::Fixed(x)),
                menu_box,
            ],
        ]
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    }

    pub fn editor_menu_btn<'a>(label: &'a str, shortcut: &'a str, msg: Message, enabled: bool) -> Element<'a, Message> {
        let text_color = if enabled { colors::TEXT_PRIMARY } else { colors::TEXT_MUTED };
        let shortcut_color = colors::TEXT_MUTED;

        let btn = button(
            row![
                text(label).size(12).color(text_color),
                horizontal_space(),
                text(shortcut).size(11).color(shortcut_color),
            ]
            .width(Length::Fill)
            .align_y(iced::Alignment::Center)
        )
        .width(Length::Fill)
        .padding(Padding::from([6, 12]))
        .style(|_: &Theme, status: button::Status| {
            let bg = match status {
                button::Status::Hovered => colors::BG_HOVER,
                _ => colors::BG_MEDIUM,
            };
            button::Style {
                background: Some(Background::Color(bg)),
                text_color: colors::TEXT_PRIMARY,
                border: Border::default(),
                ..Default::default()
            }
        });

        if enabled {
            btn.on_press(msg).into()
        } else {
            btn.into()
        }
    }

    pub fn editor_menu_separator<'a>() -> Element<'a, Message> {
        container(Space::new(Length::Fill, 1))
            .style(|_| container::Style {
                background: Some(Background::Color(colors::BORDER)),
                ..Default::default()
            })
            .into()
    }

    pub fn view_editor_context_menu(&self) -> Element<'_, Message> {
        let has_selection = self.tabs.get(self.active_tab)
            .and_then(|t| t.content.selection())
            .is_some();

        let mut items: Vec<Element<'_, Message>> = Vec::new();

        items.push(Self::editor_menu_btn("Undo", "Ctrl+Z", Message::Undo, true));
        items.push(Self::editor_menu_btn("Redo", "Ctrl+Y", Message::Redo, true));
        items.push(Self::editor_menu_separator());

        items.push(Self::editor_menu_btn("Cut", "Ctrl+X", Message::EditorCut, has_selection));
        items.push(Self::editor_menu_btn("Copy", "Ctrl+C", Message::EditorCopy, has_selection));
        items.push(Self::editor_menu_btn("Paste", "Ctrl+V", Message::EditorPaste, true));
        items.push(Self::editor_menu_separator());

        items.push(Self::editor_menu_btn("Select All", "Ctrl+A", Message::EditorSelectAll, true));

        let menu_content = Column::with_children(items).width(Length::Fixed(220.0));

        let x = self.editor_context_position.x;
        let y = self.editor_context_position.y;

        let menu_box = container(menu_content)
            .padding(4)
            .style(|_| container::Style {
                background: Some(Background::Color(colors::BG_MEDIUM)),
                border: Border {
                    color: colors::BORDER,
                    width: 1.0,
                    radius: 4.0.into(),
                },
                ..Default::default()
            });

        column![
            Space::with_height(Length::Fixed(y)),
            row![
                Space::with_width(Length::Fixed(x)),
                menu_box,
            ],
        ]
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    }
}
