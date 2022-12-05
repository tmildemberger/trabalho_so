
use iced::alignment::{self, Alignment};
use iced::executor;
use iced::keyboard;
use iced::theme::{self, Theme};
use iced::widget::canvas::{Cache, Frame, Geometry};
use iced::widget::pane_grid::{self, PaneGrid};
use iced::widget::{button, column, container, row, scrollable, text, pick_list};
use iced::{
    Application, Color, Command, Element, Length, Settings, Size, Subscription
};
use iced_lazy::responsive;
use iced_native::{event, subscription, Event};
use plotters::prelude::ChartBuilder;
use plotters_iced::plotters_backend::DrawingBackend;
use plotters_iced::{Chart, ChartWidget};

use std::collections::VecDeque;
use std::fmt::Display;
use std::time::Duration;
use std::thread;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicU64, Ordering};

const MAX_POINTS: usize = 60;

pub fn main() -> iced::Result {
    // Example::run(Settings::default())
    let shared_tick = Arc::new(AtomicU64::new(0));
    let thread_tick = Arc::clone(&shared_tick);

    let shared_data = Arc::new(Mutex::new(CollectedData::default()));
    let thread_data = Arc::clone(&shared_data);

    thread::spawn(move || {
        loop {
            collect_data(&thread_data);
            thread_tick.fetch_add(1, Ordering::SeqCst);
            thread::sleep(Duration::from_secs(1));
        }
    });

    Example::run(Settings {
        window: iced::window::Settings {
            size: (800, 600),
            ..Default::default()
        },
        flags: (shared_data, shared_tick),
        text_multithreading: true,
        antialiasing: true,
        ..Default::default()
    })
}

fn collect_data(shared_data: &Arc<Mutex<CollectedData>>) {
    let mut data = shared_data.lock().unwrap();

    if (*data).cpu_usage.len() == 0 {
        for _i in 0..4 {
            (*data).cpu_usage.push(VecDeque::new());
        }
    }

    let new_cpu_points = match (*data).tick % 4 {
        0 => vec![22.0, 33.3, 75.9, 0.0],
        1 => vec![33.3, 75.9, 0.0, 22.0],
        2 => vec![75.9, 0.0, 22.0, 33.3],
        3 => vec![0.0, 22.0, 33.3, 75.9],
        _ => vec![22.0, 33.3, 75.9, 0.0],
    };
    for i in 0..4 {
        (*data).cpu_usage[i].push_front(new_cpu_points[i]);
        if (*data).cpu_usage[i].len() > MAX_POINTS {
            (*data).cpu_usage[i].pop_back();
        }
    }
    (*data).ram_usage = Some(68.4);
    (*data).disk_usage = vec![94.1, 22.2];
    
    let mut process_list = vec![];
    process_list.push(ProcessInfo {
        pid: 1,
        name: String::from("init"),
        status: String::from("Idle"),
        user: String::from("root"),
        cpu: 1.2,
        memory: 0.1,
        cmd: String::from(""),
    });
    process_list.push(ProcessInfo {
        pid: 1501,
        name: String::from("Firefox Developer Edition"),
        status: String::from("Running"),
        user: String::from("thiago"),
        cpu: 54.3,
        memory: 28.4,
        cmd: String::from("fox&"),
    });
    for _i in 0..12 {
        process_list.push(ProcessInfo {
            pid: 2222,
            name: String::from("Google Chrome"),
            status: String::from("Running"),
            user: String::from("thiago"),
            cpu: 16.8,
            memory: 58.4,
            cmd: String::from("chrome --dark-mode --profile-folder=/home/thiago/.chrome"),
        });
    }
    (*data).process_list = process_list;

    (*data).extra_infos = vec![
        String::from("Linux 64 bits"),
        String::from("Intel Core i3"),
        String::from("Nvidia 940M"),
    ];

    (*data).tick += 1;
}

#[derive(Default, Clone)]
struct ProcessInfo {
    pid: usize,
    name: String,
    status: String,
    user: String,
    cpu: f64,
    memory: f64,
    cmd: String,
}

#[derive(Default, Clone)]
struct CollectedData {
    cpu_usage: Vec<VecDeque<f64>>,
    ram_usage: Option<f64>,
    disk_usage: Vec<f64>,
    process_list: Vec<ProcessInfo>,
    extra_infos: Vec<String>,
    tick: u64,
}

#[derive(Default)]
struct LocalData {
    current_data_copy: CollectedData,
    cpu_charts: Vec<CpuUsageChart>,
    disk_charts: Vec<DiskUsageChart>,
}

struct Example {
    panes: pane_grid::State<Pane>,
    panes_created: usize,
    focus: Option<pane_grid::Pane>,
    last_tick: u64,
    shared_data: Arc<Mutex<CollectedData>>,
    shared_tick: Arc<AtomicU64>,
    local_data: LocalData,
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
    Tick,
    ChangeType(pane_grid::Pane, PaneType),
}

impl LocalData {
    fn update_cpus(&mut self) {
        while self.current_data_copy.cpu_usage.len() > self.cpu_charts.len() {
            self.cpu_charts.push(CpuUsageChart::new(MAX_POINTS));
        }
        for (i, chart) in self.cpu_charts.iter_mut().enumerate() {
            chart.set_data(self.current_data_copy.cpu_usage[i].clone().into_iter());
        }
    }

    fn update_disks(&mut self) {
        while self.current_data_copy.disk_usage.len() > self.disk_charts.len() {
            self.disk_charts.push(DiskUsageChart::new());
        }
        for (i, chart) in self.disk_charts.iter_mut().enumerate() {
            let used = self.current_data_copy.disk_usage[i];
            chart.set_data((used, 100.0 - used));
        }
    }
}

impl Application for Example {
    type Message = Message;
    type Theme = Theme;
    type Executor = executor::Default;
    type Flags = (Arc<Mutex<CollectedData>>, Arc<AtomicU64>);

    fn new(flags: Self::Flags) -> (Self, Command<Self::Message>) {
        let (panes, _) = pane_grid::State::new(Pane::new(0));

        (
            Example {
                panes,
                panes_created: 1,
                focus: None,
                last_tick: 0,
                shared_data: flags.0,
                shared_tick: flags.1,
                local_data: LocalData {
                    current_data_copy: CollectedData::default(),
                    cpu_charts: Vec::new(),
                    disk_charts: Vec::new(),
                },
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
            Message::Tick => {
                let current_tick = self.shared_tick.load(Ordering::SeqCst);
                if self.last_tick != current_tick {
                    self.last_tick = current_tick;
                    let data = self.shared_data.lock().unwrap();
                    self.local_data.current_data_copy = (*data).clone();
                    self.local_data.update_cpus();
                    self.local_data.update_disks();
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
            iced::time::every(Duration::from_secs_f64(1.0/60.0)).map(|_| Message::Tick)
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
                    pane.pane_type,
                    &self.local_data,
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
    Cpu,
    Memory,
    Disks,
    Info,
    Tasks,
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
    fn content<'a>(&self, data: &'a LocalData) -> Element<'a, Message> {
        match *self {
            PaneType::Selection => {
                text("Select pane type").size(16).into()
            }
            PaneType::Cpu => {
                let mut content = column![
                    text("CPU").size(24),
                ]
                .width(Length::Fill)
                .spacing(10)
                .align_items(Alignment::Center);

                let mut i = 0;
                for cpu_chart in &data.cpu_charts {
                    let row = row(vec![cpu_chart.view(i)])
                        .spacing(5)
                        .padding(5)
                        .width(Length::Fill)
                        .height(Length::Units(300))
                        .align_items(Alignment::Center);
                    content = content.push(row);
                    i += 1;
                }

                content.into()
            }
            PaneType::Memory => {
                let mut content = column![
                    text("Memory").size(24),
                ]
                .width(Length::Fill)
                .spacing(10)
                .align_items(Alignment::Center);

                if let Some(memory) = data.current_data_copy.ram_usage {
                    content = content.push(text(format!("RAM: {}%", memory)).size(16));
                }
                
                content.into()
            }
            PaneType::Disks => {
                let mut content = column![
                    text("Disks").size(24),
                ]
                .width(Length::Fill)
                .spacing(10)
                .align_items(Alignment::Center);

                let mut i = 0;
                for disk_chart in &data.disk_charts {
                    let row = row(vec![disk_chart.view(i)])
                        .spacing(5)
                        .padding(5)
                        .width(Length::Fill)
                        .height(Length::Units(300))
                        .align_items(Alignment::Center);
                    content = content.push(row);
                    i += 1;
                }

                content.into()
            }
            PaneType::Info => {
                let mut content = column![
                    text("Infos").size(24),
                ]
                .width(Length::Fill)
                .spacing(10)
                .align_items(Alignment::Center);

                for line in &data.current_data_copy.extra_infos {
                    content = content.push(text(line).size(16));
                }
                
                content.into()
            }
            PaneType::Tasks => {
                // let mut content = column![
                //     text("Tasks").size(24),
                // ]
                // .width(Length::Fill)
                // .spacing(10)
                // .align_items(Alignment::Center);
                let mut pid = column![].width(Length::Units(50)).spacing(2).align_items(Alignment::Start);
                let mut name = column![].width(Length::Units(200)).spacing(2).align_items(Alignment::Start);
                let mut status = column![].width(Length::Units(70)).spacing(2).align_items(Alignment::Start);
                let mut user = column![].width(Length::Units(70)).spacing(2).align_items(Alignment::Start);
                let mut cpu = column![].width(Length::Units(40)).spacing(2).align_items(Alignment::End);
                let mut memory = column![].width(Length::Units(45)).spacing(2).align_items(Alignment::End);
                let mut cmd = column![].width(Length::Fill).spacing(2).align_items(Alignment::Start);

                pid = pid.push(text("PID").size(16));
                name = name.push(text("Name").size(16));
                status = status.push(text("Status").size(16));
                user = user.push(text("User").size(16));
                cpu = cpu.push(text("CPU%").size(16));
                memory = memory.push(text("MEM%").size(16));
                cmd = cmd.push(text("Command").size(16));

                for task in &data.current_data_copy.process_list {
                    pid = pid.push(text(format!("{}", task.pid)).size(16));
                    name = name.push(text(format!("{}", task.name)).size(16));
                    status = status.push(text(format!("{}", task.status)).size(16));
                    user = user.push(text(format!("{}", task.user)).size(16));
                    cpu = cpu.push(text(format!("{}", task.cpu)).size(16));
                    memory = memory.push(text(format!("{}", task.memory)).size(16));
                    cmd = cmd.push(text(format!("{}", task.cmd)).size(16).height(Length::Units(16)));

                    let t: iced_native::widget::Text<iced::Renderer> = text(format!("{}", task.cmd)).size(16).height(Length::Units(16));
                    // cmd = cmd.push(t);
                    println!("{:?}", <dyn iced_native::widget::Widget<Message, iced::Renderer>>::width(&t));
                }

                let content = row![
                    pid,
                    name,
                    status,
                    user,
                    cpu,
                    memory,
                    cmd,
                ]
                .width(Length::Fill)
                .spacing(5)
                .align_items(Alignment::Start);
                
                content.into()
            }
        }
    }
    const ALL: [PaneType; 6] = [
        PaneType::Selection,
        PaneType::Cpu,
        PaneType::Memory,
        PaneType::Disks,
        PaneType::Info,
        PaneType::Tasks,
    ];
}

impl Display for PaneType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            PaneType::Selection => {
                write!(f, "Select pane type")
            }
            PaneType::Cpu => {
                write!(f, "CPU")
            }
            PaneType::Memory => {
                write!(f, "Memory")
            }
            PaneType::Disks => {
                write!(f, "Disks")
            }
            PaneType::Info => {
                write!(f, "System information")
            }
            PaneType::Tasks => {
                write!(f, "Tasks")
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
    pane_type: PaneType,
    local_data: &'a LocalData,
) -> Element<'a, Message> {
    // let button = |label, message| {
    //     button(
    //         text(label)
    //             .width(Length::Fill)
    //             .horizontal_alignment(alignment::Horizontal::Center)
    //             .size(16),
    //     )
    //     .width(Length::Fill)
    //     .padding(8)
    //     .on_press(message)
    // };

    // let mut controls = column![
    //     button(
    //         "Split horizontally",
    //         Message::Split(pane_grid::Axis::Horizontal, pane),
    //     ),
    //     button(
    //         "Split vertically",
    //         Message::Split(pane_grid::Axis::Vertical, pane),
    //     ),
    // ]
    // .spacing(5)
    // .max_width(150);

    // if total_panes > 1 && !is_pinned {
    //     controls = controls.push(
    //         button("Close", Message::Close(pane))
    //             .style(theme::Button::Destructive),
    //     )
    // }

    // let content = column![
    //     pane_type.content(local_data),
    //     text(format!("{}x{}", size.width, size.height)).size(24),
    //     controls,
    // ]
    // .width(Length::Fill)
    // .spacing(10)
    // .align_items(Alignment::Center);

    // container(scrollable(content))
    //     .width(Length::Fill)
    //     .height(Length::Fill)
    //     .padding(5)
    //     .center_y()
    //     .into()
    
    container(scrollable(pane_type.content(local_data)))
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(5)
        .center_x()
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


struct CpuUsageChart {
    cache: Cache,
    data_points: VecDeque<f64>,
    max_points: usize,
}

impl CpuUsageChart {
    fn new(max_points: usize) -> Self {
        Self {
            cache: Cache::new(),
            data_points: VecDeque::new(),
            max_points,
        }
    }

    fn set_data(&mut self, value: impl Iterator<Item = f64>) {
        self.data_points = value.collect();

        while self.data_points.len() > self.max_points {
            self.data_points.pop_back();
        }

        self.cache.clear();
    }

    fn view(&self, idx: usize) -> Element<Message> {
        container(
            column(Vec::new())
                .width(Length::Fill)
                .height(Length::Fill)
                .spacing(5)
                .push(text(format!("CPU {}", idx)))
                .push(
                    ChartWidget::new(self).height(Length::Fill),
                ),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(alignment::Horizontal::Center)
        .align_y(alignment::Vertical::Center)
        .into()
    }
}

impl Chart<Message> for CpuUsageChart {
    type State = ();
    // fn update(
    //     &mut self,
    //     event: Event,
    //     bounds: Rectangle,
    //     cursor: Cursor,
    // ) -> (event::Status, Option<Message>) {
    //     self.cache.clear();
    //     (event::Status::Ignored, None)
    // }

    #[inline]
    fn draw<F: Fn(&mut Frame)>(&self, bounds: Size, draw_fn: F) -> Geometry {
        self.cache.draw(bounds, draw_fn)
    }

    fn build_chart<DB: DrawingBackend>(&self, _state: &Self::State, mut chart: ChartBuilder<DB>) {
        use plotters::{prelude::*, style::Color};

        const PLOT_LINE_COLOR: RGBColor = RGBColor(0, 175, 255);
        let end = self.max_points;

        let mut chart = chart
            .x_label_area_size(0)
            .y_label_area_size(28)
            .margin(20)
            .build_cartesian_2d(1..self.max_points, 0f64..100.0)
            .expect("failed to build chart");

        chart
            .configure_mesh()
            .bold_line_style(plotters::style::colors::BLUE.mix(0.1))
            .light_line_style(plotters::style::colors::BLUE.mix(0.05))
            .axis_style(ShapeStyle::from(plotters::style::colors::BLUE.mix(0.45)).stroke_width(1))
            .y_labels(10)
            .y_label_style(
                ("sans-serif", 15)
                    .into_font()
                    .color(&plotters::style::colors::BLUE.mix(0.65))
                    .transform(FontTransform::Rotate90),
            )
            .y_label_formatter(&|y| format!("{}%", y))
            .draw()
            .expect("failed to draw chart mesh");

        chart
            .draw_series(
                AreaSeries::new(
                    self.data_points.iter().enumerate().map(|(x, y)| (end - x, *y)),
                    0.0,
                    PLOT_LINE_COLOR.mix(0.175),
                )
                .border_style(ShapeStyle::from(PLOT_LINE_COLOR).stroke_width(2)),
            )
            .expect("failed to draw chart data");
    }
}


struct DiskUsageChart {
    cache: Cache,
    data_points: (f64, f64),
}

impl DiskUsageChart {
    fn new() -> Self {
        Self {
            cache: Cache::new(),
            data_points: (0.0, 100.0),
        }
    }

    fn set_data(&mut self, value: (f64, f64)) {
        self.data_points = value;
        self.cache.clear();
    }

    fn view(&self, idx: usize) -> Element<Message> {
        container(
            column(Vec::new())
                .width(Length::Fill)
                .height(Length::Fill)
                .spacing(5)
                .push(text(format!("Disk {}", idx)))
                .push(
                    ChartWidget::new(self).height(Length::Fill),
                ),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(alignment::Horizontal::Center)
        .align_y(alignment::Vertical::Center)
        .into()
    }
}

impl Chart<Message> for DiskUsageChart {
    type State = ();
    // fn update(
    //     &mut self,
    //     event: Event,
    //     bounds: Rectangle,
    //     cursor: Cursor,
    // ) -> (event::Status, Option<Message>) {
    //     self.cache.clear();
    //     (event::Status::Ignored, None)
    // }

    #[inline]
    fn draw<F: Fn(&mut Frame)>(&self, bounds: Size, draw_fn: F) -> Geometry {
        self.cache.draw(bounds, draw_fn)
    }

    fn build_chart<DB: DrawingBackend>(&self, _state: &Self::State, mut chart: ChartBuilder<DB>) {
        use plotters::prelude::*;
        
        const USED_COLOR: RGBColor = RGBColor(255, 222, 153);
        const FREE_COLOR: RGBColor = RGBColor(153, 222, 255);

        let mut chart = chart
            .x_label_area_size(0)
            .y_label_area_size(0)
            .margin(5)
            .build_cartesian_2d(0..300, 0..300)
            .expect("failed to build chart");
            
        chart
            .configure_mesh()
            .disable_x_mesh()
            .disable_y_mesh()
            .draw()
            .expect("failed to draw chart mesh");

        let area = chart.plotting_area();
        let dims = area.dim_in_pixel();
        let center = (dims.0 as i32 / 2, dims.1 as i32 / 2);
        let radius = 100.0;
        let sizes = vec![self.data_points.1, self.data_points.0];
        let colors = vec![FREE_COLOR, USED_COLOR];
        let labels = vec!["Free", "Used"];

        let mut pie = Pie::new(&center, &radius, &sizes, &colors, &labels);
        pie.start_angle(-90.0);
        pie.label_style((("sans-serif", 16).into_font()).color(&(BLACK)));
        pie.percentages((("sans-serif", radius * 0.32).into_font()).color(&BLACK));
        area.draw(&pie)
            .expect("failed to draw pie graph");

    }
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