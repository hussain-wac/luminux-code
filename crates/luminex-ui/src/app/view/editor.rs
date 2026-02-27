use iced::{Element, Length, Padding, Color, Theme, Border, Background, Font};
use iced::widget::{column, row, text, button, container, Space, Column, Row, scrollable, horizontal_space, mouse_area, text_editor};
use crate::highlighter::{EditorHighlighter, HighlightSettings};

use crate::app::{App, Message};
use crate::theme::colors;

impl App {
    pub fn view_main_area(&self) -> Element<'_, Message> {
        let mut editor_row_items: Vec<Element<'_, Message>> = vec![self.view_editor()];

        if self.right_dock_visible {
            if self.minimap_visible {
                editor_row_items.push(self.view_minimap());
            }
            if self.outline_visible {
                editor_row_items.push(self.view_outline_panel());
            }
        }

        let editor_with_panels = Row::with_children(editor_row_items)
            .height(Length::Fill);

        let mut main_items: Vec<Element<'_, Message>> = vec![
            self.view_tabs(),
            editor_with_panels.into(),
        ];

        if self.terminal_visible {
            main_items.push(self.view_dock_divider());
            main_items.push(self.view_terminal());
        } else if self.diagnostics_visible {
            main_items.push(self.view_dock_divider());
            main_items.push(self.view_diagnostics_panel());
        }

        Column::with_children(main_items)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    pub fn view_tabs(&self) -> Element<'_, Message> {
        let mut tabs_vec: Vec<Element<'_, Message>> = Vec::new();

        for (idx, tab) in self.tabs.iter().enumerate() {
            let is_active = self.active_tab == idx;
            tabs_vec.push(self.make_tab(&tab.name, idx, is_active, tab.modified));
        }

        tabs_vec.push(horizontal_space().into());

        let tabs_row = Row::with_children(tabs_vec)
            .spacing(1)
            .align_y(iced::Alignment::End);

        container(tabs_row)
            .width(Length::Fill)
            .height(36)
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

    pub fn make_tab(
        &self,
        name: &str,
        idx: usize,
        is_active: bool,
        modified: bool,
    ) -> Element<'_, Message> {
        let display_name = if modified {
            format!("{} *", name)
        } else {
            name.to_string()
        };

        let close_btn = button(text("x").size(12).color(colors::TEXT_MUTED))
            .padding(Padding::from([2, 6]))
            .style(|_, status| {
                let bg = match status {
                    button::Status::Hovered => colors::BG_HOVER,
                    _ => Color::TRANSPARENT,
                };
                button::Style {
                    background: Some(Background::Color(bg)),
                    text_color: colors::TEXT_PRIMARY,
                    border: Border {
                        radius: 2.0.into(),
                        ..Default::default()
                    },
                    ..Default::default()
                }
            })
            .on_press(Message::CloseTab(idx));

        let tab_content = row![
            text(display_name).size(13).color(if is_active {
                colors::TEXT_PRIMARY
            } else {
                colors::TEXT_SECONDARY
            }),
            Space::with_width(8),
            close_btn,
        ]
        .align_y(iced::Alignment::Center);

        let bg = if is_active {
            colors::BG_DARK
        } else {
            colors::BG_MEDIUM
        };

        button(tab_content)
            .padding(Padding::from([8, 14]))
            .style(move |_, status| {
                let hover_bg = match status {
                    button::Status::Hovered if !is_active => colors::BG_HOVER,
                    _ => bg,
                };
                button::Style {
                    background: Some(Background::Color(hover_bg)),
                    text_color: colors::TEXT_PRIMARY,
                    border: Border {
                        color: if is_active {
                            colors::ACCENT
                        } else {
                            Color::TRANSPARENT
                        },
                        width: if is_active { 2.0 } else { 0.0 },
                        radius: 0.0.into(),
                    },
                    ..Default::default()
                }
            })
            .on_press(Message::TabSelected(idx))
            .into()
    }

    pub fn view_editor(&self) -> Element<'_, Message> {
        if let Some(tab) = self.tabs.get(self.active_tab) {
            let highlight_settings = HighlightSettings {
                language: tab.language.clone(),
            };

            let editor_bg = colors::BG_DARK;
            let selection_color = Color::from_rgba(0.25, 0.46, 0.85, 0.55);

            // Use text_editor's native scroll — do NOT wrap in scrollable().
            // Wrapping resets the scroll position on every re-render (e.g. right-click).
            let editor = text_editor(&tab.content)
                .height(Length::Fill)
                .padding(iced::Padding { top: 16.0, right: 20.0, bottom: 16.0, left: 16.0 })
                .font(Font::MONOSPACE)
                .size(self.font_size)
                .style(move |_theme: &Theme, status| {
                    let _scrollbar_color = match status {
                        text_editor::Status::Focused =>
                            Color::from_rgba(0.5, 0.5, 0.5, 0.7),
                        _ =>
                            Color::from_rgba(0.4, 0.4, 0.4, 0.4),
                    };
                    text_editor::Style {
                        background: Background::Color(editor_bg),
                        border: Border {
                            width: 0.0,
                            radius: 0.0.into(),
                            color: Color::TRANSPARENT,
                        },
                        icon: colors::TEXT_MUTED,
                        placeholder: colors::TEXT_MUTED,
                        value: colors::TEXT_PRIMARY,
                        selection: selection_color,
                    }
                })
                .highlight_with::<EditorHighlighter>(highlight_settings, |highlight, _theme| {
                    highlight.to_format(Font::MONOSPACE)
                })
                .on_action(Message::EditorAction);

            // mouse_area wraps the editor so that right-click is captured WITHOUT
            // triggering an EditorAction (which would reset scroll).
            mouse_area(editor)
                .on_right_press(Message::ShowEditorContextMenu)
                .into()
        } else {
            container(text("No file open").size(16).color(colors::TEXT_MUTED))
                .width(Length::Fill)
                .height(Length::Fill)
                .center_x(Length::Fill)
                .center_y(Length::Fill)
                .style(|_| container::Style {
                    background: Some(Background::Color(colors::BG_DARK)),
                    ..Default::default()
                })
                .into()
        }
    }

    pub fn view_minimap(&self) -> Element<'_, Message> {
        if let Some(tab) = self.tabs.get(self.active_tab) {
            let total_lines = tab.content.line_count();
            let (cursor_line, _) = tab.content.cursor_position();

            let max_minimap_lines = 200;
            let lines_to_show = total_lines.min(max_minimap_lines);

            let mut minimap_items: Vec<Element<'_, Message>> = Vec::new();

            for i in 0..lines_to_show {
                let line_idx = if total_lines <= max_minimap_lines {
                    i
                } else {
                    (i as f64 * total_lines as f64 / max_minimap_lines as f64) as usize
                };

                let line_text = tab.content.line(line_idx)
                    .map(|l| {
                        let s: &str = &l;
                        s.to_string()
                    })
                    .unwrap_or_default();

                let line_len = line_text.trim_end().len().min(80);
                let indent = line_text.len() - line_text.trim_start().len();
                let indent = indent.min(40);
                let content_len = if line_len > indent { line_len - indent } else { 0 };

                let is_current = line_idx == cursor_line;
                let alpha = if is_current { 0.8 } else { 0.25 };
                let line_color = if is_current {
                    Color::from_rgba(0.36, 0.54, 0.90, 0.6)
                } else {
                    Color::from_rgba(0.7, 0.7, 0.7, alpha)
                };

                let minimap_line: Element<'_, Message> = row![
                    Space::with_width(Length::Fixed(indent as f32 * 0.8)),
                    container(Space::new(Length::Fixed((content_len as f32 * 0.8).max(1.0)), 2))
                        .style(move |_| container::Style {
                            background: Some(Background::Color(line_color)),
                            ..Default::default()
                        }),
                ]
                .into();

                minimap_items.push(minimap_line);
            }

            let minimap_content = Column::with_children(minimap_items)
                .spacing(0)
                .width(Length::Fill);

            container(
                scrollable(minimap_content)
                    .height(Length::Fill)
            )
            .width(Length::Fixed(80.0))
            .height(Length::Fill)
            .padding(Padding::from([4, 4]))
            .style(|_| container::Style {
                background: Some(Background::Color(Color::from_rgb(0.12, 0.12, 0.14))),
                border: Border {
                    color: colors::BORDER,
                    width: 1.0,
                    radius: 0.0.into(),
                },
                ..Default::default()
            })
            .into()
        } else {
            Space::new(0, 0).into()
        }
    }

    pub fn view_outline_panel(&self) -> Element<'_, Message> {
        let header = container(
            row![
                text("OUTLINE").size(11).color(colors::TEXT_SECONDARY).font(Font::MONOSPACE),
                horizontal_space(),
                button(text("x").size(11).color(colors::TEXT_MUTED))
                    .padding(Padding::from([2, 6]))
                    .style(|_: &Theme, status: button::Status| {
                        let bg = match status {
                            button::Status::Hovered => colors::BG_HOVER,
                            _ => Color::TRANSPARENT,
                        };
                        button::Style {
                            background: Some(Background::Color(bg)),
                            text_color: colors::TEXT_PRIMARY,
                            border: Border { radius: 2.0.into(), ..Default::default() },
                            ..Default::default()
                        }
                    })
                    .on_press(Message::ToggleOutlinePanel),
            ]
            .spacing(4)
            .align_y(iced::Alignment::Center),
        )
        .padding(Padding::from([8, 10]))
        .width(Length::Fill)
        .style(|_| container::Style {
            background: Some(Background::Color(colors::BG_MEDIUM)),
            ..Default::default()
        });

        let mut outline_items: Vec<Element<'_, Message>> = Vec::new();

        if let Some(tab) = self.tabs.get(self.active_tab) {
            let line_count = tab.content.line_count();
            for i in 0..line_count {
                if let Some(line_ref) = tab.content.line(i) {
                    let line: &str = &line_ref;
                    let trimmed = line.trim();

                    let (icon, name) = if trimmed.starts_with("fn ") || trimmed.starts_with("pub fn ") || trimmed.starts_with("async fn ") || trimmed.starts_with("pub async fn ") {
                        let fn_part = if let Some(pos) = trimmed.find("fn ") {
                            &trimmed[pos + 3..]
                        } else {
                            trimmed
                        };
                        let name = fn_part.split('(').next().unwrap_or(fn_part).trim();
                        ("fn", name.to_string())
                    } else if trimmed.starts_with("struct ") || trimmed.starts_with("pub struct ") {
                        let s = trimmed.split("struct ").nth(1).unwrap_or("");
                        let name = s.split(|c: char| c == '{' || c == '<' || c == '(' || c.is_whitespace()).next().unwrap_or(s);
                        ("S", name.to_string())
                    } else if trimmed.starts_with("enum ") || trimmed.starts_with("pub enum ") {
                        let s = trimmed.split("enum ").nth(1).unwrap_or("");
                        let name = s.split(|c: char| c == '{' || c == '<' || c.is_whitespace()).next().unwrap_or(s);
                        ("E", name.to_string())
                    } else if trimmed.starts_with("impl ") || trimmed.starts_with("impl<") {
                        let s = if trimmed.starts_with("impl<") {
                            trimmed.split('>').nth(1).unwrap_or("").trim()
                        } else {
                            &trimmed[5..]
                        };
                        let name = s.split(|c: char| c == '{' || c.is_whitespace()).next().unwrap_or(s);
                        ("I", name.to_string())
                    } else if trimmed.starts_with("trait ") || trimmed.starts_with("pub trait ") {
                        let s = trimmed.split("trait ").nth(1).unwrap_or("");
                        let name = s.split(|c: char| c == '{' || c == '<' || c == ':' || c.is_whitespace()).next().unwrap_or(s);
                        ("T", name.to_string())
                    } else if trimmed.starts_with("mod ") || trimmed.starts_with("pub mod ") {
                        let s = trimmed.split("mod ").nth(1).unwrap_or("");
                        let name = s.split(|c: char| c == '{' || c == ';' || c.is_whitespace()).next().unwrap_or(s);
                        ("m", name.to_string())
                    } else if trimmed.starts_with("const ") || trimmed.starts_with("pub const ") {
                        let s = trimmed.split("const ").nth(1).unwrap_or("");
                        let name = s.split(|c: char| c == ':' || c == '=' || c.is_whitespace()).next().unwrap_or(s);
                        ("C", name.to_string())
                    } else if trimmed.starts_with("type ") || trimmed.starts_with("pub type ") {
                        let s = trimmed.split("type ").nth(1).unwrap_or("");
                        let name = s.split(|c: char| c == '=' || c == '<' || c.is_whitespace()).next().unwrap_or(s);
                        ("T", name.to_string())
                    } else if trimmed.starts_with("function ") || trimmed.starts_with("export function ") || trimmed.starts_with("async function ") {
                        let s = trimmed.split("function ").nth(1).unwrap_or("");
                        let name = s.split('(').next().unwrap_or(s).trim();
                        ("fn", name.to_string())
                    } else if trimmed.starts_with("class ") || trimmed.starts_with("export class ") {
                        let s = trimmed.split("class ").nth(1).unwrap_or("");
                        let name = s.split(|c: char| c == '{' || c == '<' || c.is_whitespace()).next().unwrap_or(s);
                        ("C", name.to_string())
                    } else if trimmed.starts_with("def ") || trimmed.starts_with("async def ") {
                        let s = if trimmed.starts_with("async def ") { &trimmed[10..] } else { &trimmed[4..] };
                        let name = s.split('(').next().unwrap_or(s).trim();
                        ("fn", name.to_string())
                    } else {
                        continue;
                    };

                    let line_num = i;
                    let (cursor_line, _) = tab.content.cursor_position();
                    let is_current = line_num == cursor_line;

                    let icon_color = match icon {
                        "fn" => Color::from_rgb(0.60, 0.80, 0.60),
                        "S" | "C" => Color::from_rgb(0.80, 0.70, 0.40),
                        "E" => Color::from_rgb(0.70, 0.55, 0.85),
                        "I" => Color::from_rgb(0.50, 0.75, 0.90),
                        "T" => Color::from_rgb(0.80, 0.60, 0.60),
                        "m" => Color::from_rgb(0.60, 0.60, 0.80),
                        _ => colors::TEXT_SECONDARY,
                    };

                    let name_color = if is_current { colors::ACCENT } else { colors::TEXT_PRIMARY };
                    let bg = if is_current { colors::BG_ACTIVE } else { Color::TRANSPARENT };

                    let item: Element<'_, Message> = container(
                        row![
                            text(icon).size(10).font(Font::MONOSPACE).color(icon_color),
                            Space::with_width(6),
                            text(name).size(11).color(name_color),
                        ]
                        .align_y(iced::Alignment::Center)
                    )
                    .padding(Padding::from([3, 10]))
                    .width(Length::Fill)
                    .style(move |_| container::Style {
                        background: Some(Background::Color(bg)),
                        ..Default::default()
                    })
                    .into();

                    outline_items.push(item);
                }
            }
        }

        if outline_items.is_empty() {
            outline_items.push(
                container(text("No symbols found").size(11).color(colors::TEXT_MUTED))
                    .padding(Padding::from([8, 10]))
                    .into()
            );
        }

        let outline_list = Column::with_children(outline_items)
            .spacing(0)
            .width(Length::Fill);

        let content = column![
            header,
            scrollable(outline_list).height(Length::Fill),
        ];

        container(content)
            .width(Length::Fixed(200.0))
            .height(Length::Fill)
            .style(|_| container::Style {
                background: Some(Background::Color(Color::from_rgb(0.12, 0.12, 0.14))),
                border: Border {
                    color: colors::BORDER,
                    width: 1.0,
                    radius: 0.0.into(),
                },
                ..Default::default()
            })
            .into()
    }
}
