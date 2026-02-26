use iced::{Element, Length, Padding, Color, Theme, Border, Background};
use iced::widget::{column, row, text, button, container, Space, Column, Row, horizontal_space};

use crate::app::{App, Message, TopMenu};
use crate::theme::colors;

impl App {
    pub fn view_toolbar(&self) -> Element<'_, Message> {
        let menus = [
            TopMenu::File,
            TopMenu::Edit,
            TopMenu::Selection,
            TopMenu::View,
            TopMenu::Go,
            TopMenu::Window,
            TopMenu::Help,
        ];

        let mut menu_items: Vec<Element<'_, Message>> = Vec::new();

        for menu in menus {
            let label = match menu {
                TopMenu::File => "File",
                TopMenu::Edit => "Edit",
                TopMenu::Selection => "Selection",
                TopMenu::View => "View",
                TopMenu::Go => "Go",
                TopMenu::Window => "Window",
                TopMenu::Help => "Help",
            };

            let is_active = self.active_menu == Some(menu);

            let menu_btn = button(
                text(label).size(12).color(if is_active {
                    colors::TEXT_PRIMARY
                } else {
                    colors::TEXT_SECONDARY
                }),
            )
            .padding(Padding::from([6, 10]))
            .style(move |_: &Theme, status: button::Status| {
                let bg = if is_active {
                    colors::BG_ACTIVE
                } else {
                    match status {
                        button::Status::Hovered => colors::BG_HOVER,
                        _ => colors::BG_MEDIUM,
                    }
                };
                button::Style {
                    background: Some(Background::Color(bg)),
                    text_color: colors::TEXT_PRIMARY,
                    border: Border {
                        radius: 4.0.into(),
                        ..Default::default()
                    },
                    ..Default::default()
                }
            })
            .on_press(Message::ToggleTopMenu(menu));

            menu_items.push(menu_btn.into());
        }

        menu_items.push(horizontal_space().into());

        let toolbar = Row::with_children(menu_items)
            .spacing(2)
            .padding(Padding::from([4, 8]))
            .align_y(iced::Alignment::Center);

        container(toolbar)
            .width(Length::Fill)
            .style(|_| container::Style {
                background: Some(Background::Color(colors::BG_MEDIUM)),
                border: Border {
                    color: colors::BORDER,
                    width: 1.0,
                    radius: 0.0.into(),
                },
                ..Default::default()
            })
            .into()
    }

    /// Build a single dropdown menu item with label, shortcut, and action.
    pub fn menu_item<'a>(label: &'a str, shortcut: &'a str, msg: Message) -> Element<'a, Message> {
        button(
            row![
                text(label).size(12).color(colors::TEXT_PRIMARY),
                horizontal_space(),
                text(shortcut).size(11).color(colors::TEXT_MUTED),
            ]
            .width(Length::Fill)
            .align_y(iced::Alignment::Center),
        )
        .width(Length::Fill)
        .padding(Padding::from([6, 16]))
        .style(|_: &Theme, status: button::Status| {
            let bg = match status {
                button::Status::Hovered => colors::BG_HOVER,
                _ => Color::TRANSPARENT,
            };
            button::Style {
                background: Some(Background::Color(bg)),
                text_color: colors::TEXT_PRIMARY,
                border: Border::default(),
                ..Default::default()
            }
        })
        .on_press(msg)
        .into()
    }

    /// Build a disabled menu item (grayed out, no action).
    pub fn menu_separator<'a>() -> Element<'a, Message> {
        container(Space::new(Length::Fill, 1))
            .padding(Padding::from([4, 8]))
            .style(|_| container::Style {
                background: Some(Background::Color(colors::BORDER)),
                ..Default::default()
            })
            .into()
    }

    pub fn view_menu_dropdown(&self) -> Element<'_, Message> {
        let menu = match self.active_menu {
            Some(m) => m,
            None => return Space::new(0, 0).into(),
        };

        let mut items: Vec<Element<'_, Message>> = Vec::new();

        match menu {
            TopMenu::File => {
                items.push(Self::menu_item("New File", "Ctrl+N", Message::NewFile));
                items.push(Self::menu_separator());
                items.push(Self::menu_item("Open File...", "Ctrl+O", Message::OpenFile));
                items.push(Self::menu_item("Open Folder...", "", Message::OpenFolder));
                items.push(Self::menu_separator());
                items.push(Self::menu_item("Save", "Ctrl+S", Message::Save));
                items.push(Self::menu_item("Save As...", "Ctrl+Shift+S", Message::SaveAs));
                items.push(Self::menu_separator());
                items.push(Self::menu_item("Close Tab", "Ctrl+W", Message::CloseCurrentTab));
            }
            TopMenu::Edit => {
                items.push(Self::menu_item("Undo", "Ctrl+Z", Message::Undo));
                items.push(Self::menu_item("Redo", "Ctrl+Y", Message::Redo));
                items.push(Self::menu_separator());
                items.push(Self::menu_item("Cut", "Ctrl+X", Message::EditorCut));
                items.push(Self::menu_item("Copy", "Ctrl+C", Message::EditorCopy));
                items.push(Self::menu_item("Paste", "Ctrl+V", Message::EditorPaste));
                items.push(Self::menu_separator());
                items.push(Self::menu_item("Select All", "Ctrl+A", Message::EditorSelectAll));
            }
            TopMenu::Selection => {
                items.push(Self::menu_item("Select All", "Ctrl+A", Message::EditorSelectAll));
                items.push(Self::menu_item("Select Line", "", Message::SelectLine));
            }
            TopMenu::View => {
                items.push(Self::menu_item("Zoom In", "Ctrl++", Message::ZoomIn));
                items.push(Self::menu_item("Zoom Out", "Ctrl+-", Message::ZoomOut));
                items.push(Self::menu_item("Reset Zoom", "Ctrl+0", Message::ZoomReset));
                items.push(Self::menu_item("Reset All Zoom", "", Message::ZoomResetAll));
                items.push(Self::menu_separator());

                items.push(Self::menu_item("Toggle Left Dock", "Ctrl+B", Message::ToggleLeftDock));
                items.push(Self::menu_item("Toggle Right Dock", "Ctrl+Alt+B", Message::ToggleRightDock));
                items.push(Self::menu_item("Toggle Bottom Dock", "Ctrl+J", Message::ToggleBottomDock));
                items.push(Self::menu_item("Toggle All Docks", "Ctrl+Alt+Y", Message::ToggleAllDocks));
                items.push(Self::menu_separator());

                items.push(Self::menu_item("Project Panel", "Ctrl+Shift+E", Message::ToggleProjectPanel));
                items.push(Self::menu_item("Outline Panel", "Ctrl+Shift+B", Message::ToggleOutlinePanel));
                items.push(Self::menu_item("Terminal Panel", "Ctrl+`", Message::ToggleTerminalPanel));
                items.push(Self::menu_separator());

                items.push(Self::menu_item("Diagnostics", "Ctrl+Shift+M", Message::ToggleDiagnostics));
            }
            TopMenu::Go => {
                items.push(Self::menu_item("Go to Line...", "Ctrl+G", Message::ShowGotoLine));
                items.push(Self::menu_separator());
                items.push(Self::menu_item("Next Tab", "Ctrl+Tab", Message::NextTab));
                items.push(Self::menu_item("Previous Tab", "Ctrl+Shift+Tab", Message::PrevTab));
            }
            TopMenu::Window => {
                items.push(Self::menu_item("Close Window", "Ctrl+Q", Message::CloseWindow));
            }
            TopMenu::Help => {
                items.push(Self::menu_item("About Luminex", "", Message::ShowAbout));
            }
        }

        let menu_width = 280.0;
        let menu_content = Column::with_children(items)
            .width(Length::Fixed(menu_width))
            .padding(4);

        let menu_offset_x = match menu {
            TopMenu::File => 8.0,
            TopMenu::Edit => 52.0,
            TopMenu::Selection => 92.0,
            TopMenu::View => 160.0,
            TopMenu::Go => 200.0,
            TopMenu::Window => 228.0,
            TopMenu::Help => 286.0,
        };

        let menu_box = container(menu_content)
            .style(|_| container::Style {
                background: Some(Background::Color(colors::BG_MEDIUM)),
                border: Border {
                    color: colors::BORDER,
                    width: 1.0,
                    radius: 6.0.into(),
                },
                ..Default::default()
            });

        column![
            Space::with_height(Length::Fixed(32.0)),
            row![
                Space::with_width(Length::Fixed(menu_offset_x)),
                menu_box,
            ],
        ]
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    }
}
