use iced::{Element, Length, Padding, Color, Theme, Border, Background, Font};
use iced::widget::{column, row, text, button, container, Space, Column, scrollable, horizontal_space, mouse_area};

use crate::app::{App, Message};
use crate::theme::colors;

impl App {
    pub fn view_diagnostics_panel(&self) -> Element<'_, Message> {
        let header = container(
            row![
                text("DIAGNOSTICS").size(11).color(colors::TEXT_SECONDARY).font(Font::MONOSPACE),
                horizontal_space(),
                button(text("Clear").size(10).color(colors::TEXT_SECONDARY).font(Font::MONOSPACE))
                    .padding(Padding::from([2, 8]))
                    .style(|_: &Theme, status: button::Status| {
                        let bg = match status {
                            button::Status::Hovered => colors::BG_HOVER,
                            _ => Color::TRANSPARENT,
                        };
                        button::Style {
                            background: Some(Background::Color(bg)),
                            text_color: colors::TEXT_SECONDARY,
                            border: Border { radius: 3.0.into(), ..Default::default() },
                            ..Default::default()
                        }
                    })
                    .on_press(Message::ToggleDiagnostics),
                Space::with_width(4),
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
                    .on_press(Message::ToggleDiagnostics),
            ]
            .spacing(4)
            .align_y(iced::Alignment::Center),
        )
        .padding(Padding::from([6, 12]))
        .width(Length::Fill)
        .style(|_| container::Style {
            background: Some(Background::Color(Color::from_rgb(0.10, 0.10, 0.12))),
            ..Default::default()
        });

        let mut diag_items: Vec<Element<'_, Message>> = Vec::new();

        if self.diagnostics_messages.is_empty() {
            diag_items.push(
                container(
                    text("No problems detected").size(12).font(Font::MONOSPACE).color(colors::TEXT_MUTED)
                )
                .padding(Padding::from([8, 12]))
                .into()
            );
        } else {
            for msg in &self.diagnostics_messages {
                let color = if msg.contains("error") || msg.contains("Error") {
                    Color::from_rgb(0.90, 0.35, 0.35)
                } else if msg.contains("warning") || msg.contains("Warning") {
                    Color::from_rgb(0.90, 0.75, 0.30)
                } else {
                    colors::TEXT_SECONDARY
                };
                diag_items.push(
                    text(msg.as_str()).size(12).font(Font::MONOSPACE).color(color).into()
                );
            }
        }

        let diag_column = Column::with_children(diag_items)
            .spacing(2)
            .width(Length::Fill);

        let content = column![
            header,
            container(scrollable(diag_column).height(Length::Fill))
                .width(Length::Fill)
                .height(Length::Fill)
                .padding(Padding::from([4, 12])),
        ];

        container(content)
            .width(Length::Fill)
            .height(Length::Fixed(self.terminal_height))
            .style(|_| container::Style {
                background: Some(Background::Color(Color::from_rgb(0.08, 0.08, 0.10))),
                ..Default::default()
            })
            .into()
    }

    pub fn view_dock_divider(&self) -> Element<'_, Message> {
        container(Space::new(Length::Fill, 1))
            .width(Length::Fill)
            .style(|_| container::Style {
                background: Some(Background::Color(colors::BORDER)),
                ..Default::default()
            })
            .into()
    }

    pub fn view_terminal(&self) -> Element<'_, Message> {
        let term_bg = Color::from_rgb(0.07, 0.07, 0.09);
        let term_header_bg = Color::from_rgb(0.10, 0.10, 0.12);

        let tab_label = row![
            text("bash").size(11).color(colors::TEXT_PRIMARY).font(Font::MONOSPACE),
        ]
        .spacing(4)
        .align_y(iced::Alignment::Center);

        let header = container(
            row![
                container(tab_label)
                    .padding(Padding::from([5, 12]))
                    .style(move |_| container::Style {
                        background: Some(Background::Color(term_bg)),
                        border: Border {
                            color: colors::BORDER,
                            width: 1.0,
                            radius: 4.0.into(),
                        },
                        ..Default::default()
                    }),
                horizontal_space(),
                button(text("+").size(12).color(colors::TEXT_MUTED).font(Font::MONOSPACE))
                    .padding(Padding::from([2, 6]))
                    .style(|_: &Theme, status: button::Status| {
                        let bg = match status {
                            button::Status::Hovered => colors::BG_HOVER,
                            _ => Color::TRANSPARENT,
                        };
                        button::Style {
                            background: Some(Background::Color(bg)),
                            text_color: colors::TEXT_MUTED,
                            border: Border { radius: 3.0.into(), ..Default::default() },
                            ..Default::default()
                        }
                    })
                    .on_press(Message::TerminalClear),
                Space::with_width(2),
                button(text("x").size(12).color(colors::TEXT_MUTED).font(Font::MONOSPACE))
                    .padding(Padding::from([2, 6]))
                    .style(|_: &Theme, status: button::Status| {
                        let bg = match status {
                            button::Status::Hovered => colors::BG_HOVER,
                            _ => Color::TRANSPARENT,
                        };
                        button::Style {
                            background: Some(Background::Color(bg)),
                            text_color: colors::TEXT_MUTED,
                            border: Border { radius: 3.0.into(), ..Default::default() },
                            ..Default::default()
                        }
                    })
                    .on_press(Message::ToggleTerminalPanel),
            ]
            .spacing(4)
            .align_y(iced::Alignment::Center),
        )
        .padding(Padding::from([4, 8]))
        .width(Length::Fill)
        .style(move |_| container::Style {
            background: Some(Background::Color(term_header_bg)),
            border: Border {
                color: colors::BORDER,
                width: 1.0,
                radius: 0.0.into(),
            },
            ..Default::default()
        });

        let term_view = iced_term::TerminalView::show(&self.terminal)
            .map(Message::TerminalEvent);
        let filtered_term = FocusFilter::new(term_view, self.terminal_focused);

        let focus_border_color = if self.terminal_focused {
            Color::from_rgb(0.25, 0.55, 0.95)
        } else {
            Color::TRANSPARENT
        };

        let term_element: Element<'_, Message> = filtered_term.into();

        let resize_handle = mouse_area(
            container(Space::new(Length::Fill, 4))
                .style(|_| container::Style {
                    background: Some(Background::Color(Color::from_rgba(1.0, 1.0, 1.0, 0.05))),
                    ..Default::default()
                })
        )
        .on_press(Message::StartTerminalResize)
        .interaction(iced::mouse::Interaction::ResizingVertically);

        let terminal_content = Column::new()
            .push(resize_handle)
            .push(header)
            .push(
                container(term_element)
                    .width(Length::Fill)
                    .height(Length::Fill)
            )
            .width(Length::Fill)
            .height(Length::Fill);

        let terminal_container = container(terminal_content)
            .width(Length::Fill)
            .height(Length::Fixed(self.terminal_height))
            .style(move |_| container::Style {
                background: Some(Background::Color(term_bg)),
                border: Border {
                    color: focus_border_color,
                    width: if focus_border_color == Color::TRANSPARENT { 0.0 } else { 2.0 },
                    radius: 0.0.into(),
                },
                ..Default::default()
            });

        mouse_area(terminal_container)
            .on_press(Message::TerminalFocused)
            .into()
    }

    pub fn view_status_bar(&self) -> Element<'_, Message> {
        let cursor_info = if let Some(tab) = self.tabs.get(self.active_tab) {
            let (line, col) = tab.content.cursor_position();
            format!("Ln {}, Col {}", line + 1, col + 1)
        } else {
            "Ln 1, Col 1".to_string()
        };

        let file_info = self
            .tabs
            .get(self.active_tab)
            .map(|t| {
                if t.modified {
                    format!("{} [modified]", t.name)
                } else {
                    t.name.clone()
                }
            })
            .unwrap_or_else(|| "No file".to_string());

        let language_info = self
            .tabs
            .get(self.active_tab)
            .map(|t| t.language.clone())
            .unwrap_or_else(|| "text".to_string());

        let status_content = row![
            text(&self.status_message)
                .size(12)
                .color(colors::TEXT_SECONDARY),
            horizontal_space(),
            text(file_info).size(12).color(colors::TEXT_SECONDARY),
            Space::with_width(24),
            text(cursor_info).size(12).color(colors::TEXT_PRIMARY),
            Space::with_width(24),
            text(language_info).size(12).color(colors::ACCENT),
            Space::with_width(24),
            text("UTF-8").size(12).color(colors::TEXT_SECONDARY),
            Space::with_width(12),
        ]
        .padding(Padding::from([6, 12]))
        .align_y(iced::Alignment::Center);

        container(status_content)
            .width(Length::Fill)
            .height(28)
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
}

struct FocusFilter<'a, Message, Theme, Renderer> {
    content: Element<'a, Message, Theme, Renderer>,
    is_focused: bool,
}

impl<'a, Message, Theme, Renderer> FocusFilter<'a, Message, Theme, Renderer> {
    pub fn new(content: impl Into<Element<'a, Message, Theme, Renderer>>, is_focused: bool) -> Self {
        Self {
            content: content.into(),
            is_focused,
        }
    }
}

impl<'a, Message, Theme, Renderer> iced::advanced::Widget<Message, Theme, Renderer>
    for FocusFilter<'a, Message, Theme, Renderer>
where
    Renderer: iced::advanced::Renderer,
{
    fn size(&self) -> iced::Size<Length> {
        self.content.as_widget().size()
    }

    fn layout(
        &self,
        tree: &mut iced::advanced::widget::Tree,
        renderer: &Renderer,
        limits: &iced::advanced::layout::Limits,
    ) -> iced::advanced::layout::Node {
        self.content.as_widget().layout(&mut tree.children[0], renderer, limits)
    }

    fn draw(
        &self,
        tree: &iced::advanced::widget::Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &iced::advanced::renderer::Style,
        layout: iced::advanced::Layout<'_>,
        cursor: iced::advanced::mouse::Cursor,
        viewport: &iced::Rectangle,
    ) {
        self.content.as_widget().draw(
            &tree.children[0],
            renderer,
            theme,
            style,
            layout,
            cursor,
            viewport,
        )
    }

    fn children(&self) -> Vec<iced::advanced::widget::Tree> {
        vec![iced::advanced::widget::Tree::new(&self.content)]
    }

    fn diff(&self, tree: &mut iced::advanced::widget::Tree) {
        tree.diff_children(std::slice::from_ref(&self.content))
    }

    fn operate(
        &self,
        tree: &mut iced::advanced::widget::Tree,
        layout: iced::advanced::Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn iced::advanced::widget::operation::Operation,
    ) {
        self.content.as_widget().operate(&mut tree.children[0], layout, renderer, operation)
    }

    fn on_event(
        &mut self,
        tree: &mut iced::advanced::widget::Tree,
        event: iced::Event,
        layout: iced::advanced::Layout<'_>,
        cursor: iced::advanced::mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn iced::advanced::Clipboard,
        shell: &mut iced::advanced::Shell<'_, Message>,
        viewport: &iced::Rectangle,
    ) -> iced::event::Status {
        if let iced::Event::Keyboard(_) = event {
            if !self.is_focused {
                return iced::event::Status::Ignored;
            }
        }

        self.content.as_widget_mut().on_event(
            &mut tree.children[0],
            event,
            layout,
            cursor,
            renderer,
            clipboard,
            shell,
            viewport,
        )
    }

    fn mouse_interaction(
        &self,
        tree: &iced::advanced::widget::Tree,
        layout: iced::advanced::Layout<'_>,
        cursor: iced::advanced::mouse::Cursor,
        viewport: &iced::Rectangle,
        renderer: &Renderer,
    ) -> iced::mouse::Interaction {
        self.content.as_widget().mouse_interaction(
            &tree.children[0],
            layout,
            cursor,
            viewport,
            renderer,
        )
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut iced::advanced::widget::Tree,
        layout: iced::advanced::Layout<'_>,
        renderer: &Renderer,
        translation: iced::Vector,
    ) -> Option<iced::advanced::overlay::Element<'b, Message, Theme, Renderer>> {
        self.content.as_widget_mut().overlay(
            &mut tree.children[0],
            layout,
            renderer,
            translation,
        )
    }
}

impl<'a, Message, Theme, Renderer> From<FocusFilter<'a, Message, Theme, Renderer>>
    for Element<'a, Message, Theme, Renderer>
where
    Message: 'a,
    Theme: 'a,
    Renderer: iced::advanced::Renderer + 'a,
{
    fn from(filter: FocusFilter<'a, Message, Theme, Renderer>) -> Self {
        Element::new(filter)
    }
}
