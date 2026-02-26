use iced::{Element, Length, Padding, Color, Theme, Border, Background, Point, Font};
use iced::widget::{column, row, text, button, container, Space, Column, scrollable, horizontal_space, mouse_area};

use crate::app::{App, Message, FileNode};
use crate::theme::colors;

impl App {
    pub fn view_sidebar(&self) -> Element<'_, Message> {
        let small_btn_style = |_: &Theme, status: button::Status| -> button::Style {
            let bg = match status {
                button::Status::Hovered => colors::BG_HOVER,
                button::Status::Pressed => colors::BG_ACTIVE,
                _ => colors::BG_MEDIUM,
            };
            button::Style {
                background: Some(Background::Color(bg)),
                text_color: colors::TEXT_SECONDARY,
                border: Border {
                    radius: 3.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            }
        };

        let header = container(
            row![
                text("EXPLORER")
                    .size(11)
                    .color(colors::TEXT_SECONDARY)
                    .font(Font::with_name("system-ui")),
                horizontal_space(),
                button(text("+F").size(10).font(Font::MONOSPACE))
                    .padding(Padding::from([2, 6]))
                    .style(small_btn_style)
                    .on_press(Message::CreateNewFile),
                button(text("+D").size(10).font(Font::MONOSPACE))
                    .padding(Padding::from([2, 6]))
                    .style(small_btn_style)
                    .on_press(Message::CreateFolder),
            ]
            .spacing(4)
            .align_y(iced::Alignment::Center),
        )
        .padding(Padding::from([10, 12]))
        .width(Length::Fill)
        .style(|_| container::Style {
            background: Some(Background::Color(colors::BG_MEDIUM)),
            ..Default::default()
        });

        let file_content: Element<'_, Message> = if let Some(tree) = &self.file_tree {
            let items = self.build_file_tree_items(tree);
            let tree_view = mouse_area(
                Column::with_children(items).spacing(1).width(Length::Fill)
            )
            .on_right_press(Message::ShowContextMenu(Point::ORIGIN, None, true));

            scrollable(tree_view)
                .height(Length::Fill)
                .into()
        } else {
            container(
                column![
                    Space::with_height(40),
                    text("No folder open").size(13).color(colors::TEXT_MUTED),
                    Space::with_height(16),
                    button(text("Open Folder").size(13).color(colors::ACCENT))
                        .padding(Padding::from([8, 16]))
                        .style(|_, status| {
                            let bg = match status {
                                button::Status::Hovered => colors::BG_HOVER,
                                _ => colors::BG_LIGHT,
                            };
                            button::Style {
                                background: Some(Background::Color(bg)),
                                text_color: colors::ACCENT,
                                border: Border {
                                    color: colors::ACCENT,
                                    width: 1.0,
                                    radius: 4.0.into(),
                                },
                                ..Default::default()
                            }
                        })
                        .on_press(Message::OpenFolder),
                ]
                .align_x(iced::Alignment::Center)
                .width(Length::Fill),
            )
            .height(Length::Fill)
            .into()
        };

        let sidebar_content = column![header, file_content];

        container(sidebar_content)
            .width(Length::Fixed(self.sidebar_width))
            .height(Length::Fill)
            .style(|_| container::Style {
                background: Some(Background::Color(colors::BG_LIGHT)),
                border: Border {
                    color: colors::BORDER,
                    width: 1.0,
                    radius: 0.0.into(),
                },
                ..Default::default()
            })
            .into()
    }

    pub fn build_file_tree_items(&self, node: &FileNode) -> Vec<Element<'_, Message>> {
        let mut items = Vec::new();
        items.push(self.make_file_item(node));

        if node.expanded {
            for child in &node.children {
                items.extend(self.build_file_tree_items(child));
            }
        }

        items
    }

    pub fn make_file_item(&self, node: &FileNode) -> Element<'_, Message> {
        let icon = if node.is_dir {
            if node.expanded { "[-]" } else { "[+]" }
        } else {
            self.get_file_icon(&node.name)
        };

        let is_active = self
            .tabs
            .get(self.active_tab)
            .and_then(|t| t.path.as_ref())
            .map(|p| p == &node.path)
            .unwrap_or(false);

        let bg = if is_active {
            colors::BG_ACTIVE
        } else {
            Color::TRANSPARENT
        };

        let indent = (node.depth * 16 + 8) as f32;
        let path = node.path.clone();
        let path_for_menu = node.path.clone();
        let is_dir = node.is_dir;
        let name = node.name.clone();

        let item_btn = button(
            row![
                Space::with_width(Length::Fixed(indent)),
                text(icon).size(12).font(Font::MONOSPACE).color(colors::TEXT_MUTED),
                Space::with_width(6),
                text(name).size(13).color(if is_active {
                    colors::TEXT_PRIMARY
                } else {
                    colors::TEXT_SECONDARY
                }),
            ]
            .align_y(iced::Alignment::Center),
        )
        .width(Length::Fill)
        .padding(Padding::from([4, 0]))
        .style(move |_, status| {
            let hover_bg = match status {
                button::Status::Hovered => colors::BG_HOVER,
                _ => bg,
            };
            button::Style {
                background: Some(Background::Color(hover_bg)),
                text_color: colors::TEXT_PRIMARY,
                border: Border::default(),
                ..Default::default()
            }
        })
        .on_press(if is_dir {
            Message::ToggleFolder(path)
        } else {
            Message::FileClicked(path)
        });

        mouse_area(item_btn)
            .on_right_press(Message::ShowContextMenu(Point::ORIGIN, Some(path_for_menu), is_dir))
            .into()
    }

    pub fn get_file_icon(&self, name: &str) -> &'static str {
        let ext = name.rsplit('.').next().unwrap_or("");
        match ext {
            "rs" => " rs",
            "py" => " py",
            "js" | "ts" | "jsx" | "tsx" => " js",
            "html" => " <>",
            "css" | "scss" | "sass" => " cs",
            "json" | "toml" | "yaml" | "yml" => " {}",
            "md" => " md",
            "txt" => " tx",
            "sh" | "bash" | "zsh" => " sh",
            _ => "  .",
        }
    }
}
