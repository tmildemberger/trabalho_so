
use iced::alignment::{self, Alignment};
use iced::executor;
use iced::keyboard;
use iced::theme::{self, Theme};
use iced::widget::pane_grid::{self, PaneGrid};
use iced::widget::{button, column, container, row, scrollable, text, pick_list};
use iced::{
    Application, Color, Command, Element, Length, Settings, Size, Subscription
};
use iced_lazy::responsive;
use iced_native::{event, subscription, Event};

use std::fmt::Display;
use std::time::{Duration, Instant};

pub fn main() -> iced::Result {
    // Example::run(Settings::default())
    Example::run(Settings {
        window: iced::window::Settings {
            size: (800, 600),
            ..Default::default()
        },
        flags: (),
        ..Default::default()
    })
}

struct Example {
    panes: pane_grid::State<Pane>,
    panes_created: usize,
    focus: Option<pane_grid::Pane>,
    now: Instant,
    ticks: usize,
}

#[derive(Debug, Clone, Copy)]
enum Message {
    Split(pane_grid::Axis, pane_grid::Pane),
    SplitFocused(pane_grid::Axis),
    FocusAdjacent(pane_grid::Direction),
    Clicked(pane_grid::Pane),
    Dragged(pane_grid::DragEvent),
    Resized(pane_grid::ResizeEvent),
    TogglePin(pane_grid::Pane),
    Maximize(pane_grid::Pane),
    Restore,
    Close(pane_grid::Pane),
    CloseFocused,
    Tick(Instant),
    ChangeType(pane_grid::Pane, PaneType),
}

impl Application for Example {
    type Message = Message;
    type Theme = Theme;
    type Executor = executor::Default;
    type Flags = ();

    fn new(_flags: Self::Flags) -> (Self, Command<Self::Message>) {
        let (panes, _) = pane_grid::State::new(Pane::new(0));

        (
            Example {
                panes,
                panes_created: 1,
                focus: None,
                now: Instant::now(),
                ticks: 0,
            },
            Command::none(),
        )
    }

    fn title(&self) -> String {
        String::from("Dashboard")
    }

    fn update(&mut self, message: Self::Message) -> Command<Self::Message> {
        match message {
            Message::Split(axis, pane) => {
                let result = self.panes.split(
                    axis,
                    &pane,
                    Pane::new(self.panes_created),
                );

                if let Some((pane, _)) = result {
                    self.focus = Some(pane);
                }

                self.panes_created += 1;
            }
            Message::SplitFocused(axis) => {
                if let Some(pane) = self.focus {
                    let result = self.panes.split(
                        axis,
                        &pane,
                        Pane::new(self.panes_created),
                    );
    
                    if let Some((pane, _)) = result {
                        self.focus = Some(pane);
                    }
    
                    self.panes_created += 1;
                }
            }
            Message::FocusAdjacent(direction) => {
                if let Some(pane) = self.focus {
                    if let Some(adjacent) =
                        self.panes.adjacent(&pane, direction)
                    {
                        self.focus = Some(adjacent);
                    }
                }
            }
            Message::Clicked(pane) => {
                self.focus = Some(pane);
            }
            Message::Resized(pane_grid::ResizeEvent { split, ratio }) => {
                self.panes.resize(&split, ratio);
            }
            Message::Dragged(pane_grid::DragEvent::Dropped {
                pane,
                target,
            }) => {
                self.panes.swap(&pane, &target);
            }
            Message::Dragged(_) => {}
            Message::TogglePin(pane) => {
                if let Some(Pane { is_pinned, .. }) = self.panes.get_mut(&pane)
                {
                    *is_pinned = !*is_pinned;
                }
            }
            Message::Maximize(pane) => self.panes.maximize(&pane),
            Message::Restore => {
                self.panes.restore()
            }
            Message::Close(pane) => {
                if let Some((_, sibling)) = self.panes.close(&pane) {
                    self.focus = Some(sibling);
                }
            }
            Message::CloseFocused => {
                if let Some(pane) = self.focus {
                    if let Some(Pane { is_pinned, .. }) = self.panes.get(&pane)
                    {
                        if !is_pinned {
                            if let Some((_, sibling)) = self.panes.close(&pane) {
                                self.focus = Some(sibling);
                            }
                        }
                    }
                }
            }
            Message::Tick(now) => {
                // let now = local_time;

                if now != self.now {
                    self.now = now;
                    self.ticks += 1;
                }
            }
            Message::ChangeType(pane, new_pane_type) => {
                if let Some(Pane { pane_type, ..}) = self.panes.get_mut(&pane)
                {
                    *pane_type = new_pane_type;
                }
            }
        }

        Command::none()
    }

    fn subscription(&self) -> Subscription<Self::Message> {
        Subscription::batch([
            subscription::events_with(|event, status| {
                if let event::Status::Captured = status {
                    return None;
                }

                match event {
                    Event::Keyboard(keyboard::Event::KeyPressed {
                        key_code,
                        modifiers,
                    }) if modifiers.command() => handle_hotkey(key_code),
                    _ => None,
                }
            }),
            iced::time::every(Duration::from_secs(2)).map(Message::Tick)
        ])
    }

    fn view(&self) -> Element<'_, Self::Message> {
        let focus = self.focus;
        let total_panes = self.panes_created;

        let pane_grid = PaneGrid::new(&self.panes, |id, pane, maximized| {
            let is_focused = focus == Some(id);

            let title = row![
                "Pane",
                text(pane.id.to_string()).style(if is_focused {
                    PANE_ID_COLOR_FOCUSED
                } else {
                    PANE_ID_COLOR_UNFOCUSED
                }),
            ]
            .spacing(5);

            let title_bar = pane_grid::TitleBar::new(title)
                .controls(view_controls(
                    id,
                    total_panes,
                    pane.is_pinned,
                    maximized,
                    pane.pane_type,
                ))
                .padding(10)
                .style(if is_focused {
                    style::title_bar_focused
                } else {
                    style::title_bar_active
                });
            
            pane_grid::Content::new(responsive(move |size| {
                view_content(
                    id,
                    total_panes,
                    pane.is_pinned,
                    size,
                    self.ticks,
                    pane.pane_type,
                )
            }))
            .title_bar(title_bar)
            .style(if is_focused {
                style::pane_focused
            } else {
                style::pane_active
            })
        })
        .width(Length::Fill)
        .height(Length::Fill)
        .spacing(10)
        .on_click(Message::Clicked)
        .on_drag(Message::Dragged)
        .on_resize(10, Message::Resized);

        container(pane_grid)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(10)
            .into()
    }
}

fn handle_hotkey(key_code: keyboard::KeyCode) -> Option<Message> {
    use keyboard::KeyCode;
    use pane_grid::{Axis, Direction};

    let direction = match key_code {
        KeyCode::Up => Some(Direction::Up),
        KeyCode::Down => Some(Direction::Down),
        KeyCode::Left => Some(Direction::Left),
        KeyCode::Right => Some(Direction::Right),
        _ => None,
    };

    match key_code {
        KeyCode::V => Some(Message::SplitFocused(Axis::Vertical)),
        KeyCode::H => Some(Message::SplitFocused(Axis::Horizontal)),
        KeyCode::W => Some(Message::CloseFocused),
        _ => direction.map(Message::FocusAdjacent),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PaneType {
    Selection,
    CPU,
    Memory,
    Disks,
}

struct Pane {
    id: usize,
    pub is_pinned: bool,
    pane_type: PaneType,
}

impl Pane {
    fn new(id: usize) -> Self {
        Self {
            id,
            is_pinned: false,
            pane_type: PaneType::Selection,
        }
    }
}

impl PaneType {
    fn content<'a>(&self) -> Element<'a, Message> {
        match *self {
            PaneType::Selection => {
                text("Select pane type").size(16).into()
            }
            PaneType::CPU => {
                text("CPU").size(16).into()
            }
            PaneType::Memory => {
                text("Memory").size(16).into()
            }
            PaneType::Disks => {
                text("Disks").size(16).into()
            }
        }
    }
    const ALL: [PaneType; 4] = [
        PaneType::Selection,
        PaneType::CPU,
        PaneType::Memory,
        PaneType::Disks,
    ];
}

impl Display for PaneType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            PaneType::Selection => {
                write!(f, "Select pane type")
            }
            PaneType::CPU => {
                write!(f, "CPU")
            }
            PaneType::Memory => {
                write!(f, "Memory")
            }
            PaneType::Disks => {
                write!(f, "Disks")
            }
        }
    }
}

const PANE_ID_COLOR_UNFOCUSED: Color = Color::from_rgb(
    0xFF as f32 / 255.0,
    0xC7 as f32 / 255.0,
    0xC7 as f32 / 255.0
);
const PANE_ID_COLOR_FOCUSED: Color = Color::from_rgb(
    0xFF as f32 / 255.0,
    0x47 as f32 / 255.0,
    0x47 as f32 / 255.0
);

fn view_content<'a>(
    pane: pane_grid::Pane,
    total_panes: usize,
    is_pinned: bool,
    size: Size,
    ticks: usize,
    pane_type: PaneType,
) -> Element<'a, Message> {
    let button = |label, message| {
        button(
            text(label)
                .width(Length::Fill)
                .horizontal_alignment(alignment::Horizontal::Center)
                .size(16),
        )
        .width(Length::Fill)
        .padding(8)
        .on_press(message)
    };

    let mut controls = column![
        button(
            "Split horizontally",
            Message::Split(pane_grid::Axis::Horizontal, pane),
        ),
        button(
            "Split vertically",
            Message::Split(pane_grid::Axis::Vertical, pane),
        ),
    ]
    .spacing(5)
    .max_width(150);

    if total_panes > 1 && !is_pinned {
        controls = controls.push(
            button("Close", Message::Close(pane))
                .style(theme::Button::Destructive),
        )
    }

    let content = column![
        pane_type.content(),
        text(format!("{} ticks", ticks)).size(16),
        text(format!("{}x{}", size.width, size.height)).size(24),
        controls,
    ]
    .width(Length::Fill)
    .spacing(10)
    .align_items(Alignment::Center);

    container(scrollable(content))
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(5)
        .center_y()
        .into()
}

fn view_controls<'a>(
    pane: pane_grid::Pane,
    total_panes: usize,
    is_pinned: bool,
    is_maximized: bool,
    pane_type: PaneType,
) -> Element<'a, Message> {
    let mut row = row![].spacing(5);

    let pick_list = pick_list(
        &PaneType::ALL[..],
        Some(pane_type),
        move |new_type| Message::ChangeType(pane, new_type),
    );

    row = row.push(pick_list);

    if total_panes > 1 {
        let toggle = {
            let (content, message) = if is_maximized {
                ("Restore", Message::Restore)
            } else {
                ("Maximize", Message::Maximize(pane))
            };
            button(text(content).size(14))
                .style(theme::Button::Secondary)
                .padding(3)
                .on_press(message)
        };

        row = row.push(toggle);
    }
    
    let pin_button = button(
        text(if is_pinned { "Unpin" } else { "Pin" }).size(14),
    )
        .on_press(Message::TogglePin(pane))
        .padding(3);

    row = row.push(pin_button);

    let mut close = button(text("Close").size(14))
        .style(theme::Button::Destructive)
        .padding(3);
    
    if total_panes > 1 && !is_pinned {
        close = close.on_press(Message::Close(pane));
    }

    row.push(close).into()
}

mod style {
    use iced::widget::container;
    use iced::Theme;

    pub fn title_bar_active(theme: &Theme) -> container::Appearance {
        let pallete = theme.extended_palette();

        container::Appearance {
            text_color: Some(pallete.background.strong.text),
            background: Some(pallete.background.strong.color.into()),
            ..Default::default()
        }
    }

    pub fn title_bar_focused(theme: &Theme) -> container::Appearance {
        let pallete = theme.extended_palette();

        container::Appearance {
            text_color: Some(pallete.primary.strong.text),
            background: Some(pallete.primary.strong.color.into()),
            ..Default::default()
        }
    }

    pub fn pane_active(theme: &Theme) -> container::Appearance {
        let pallete = theme.extended_palette();

        container::Appearance {
            background: Some(pallete.background.weak.color.into()),
            border_width: 2.0,
            border_color: pallete.background.strong.color,
            ..Default::default()
        }
    }

    pub fn pane_focused(theme: &Theme) -> container::Appearance {
        let pallete = theme.extended_palette();

        container::Appearance {
            background: Some(pallete.background.weak.color.into()),
            border_width: 2.0,
            border_color: pallete.background.strong.color,
            ..Default::default()
        }
    }
}