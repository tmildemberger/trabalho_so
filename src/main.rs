
use iced::alignment::{self, Alignment};
use iced::executor;
use iced::keyboard;
use iced::theme::{self, Theme};
use iced::widget::canvas::{Cache, Frame, Geometry};
use iced::widget::pane_grid::{self, PaneGrid};
use iced::widget::canvas;
use iced::widget::{button, column, container, row, scrollable, text, pick_list};
use iced::{
    Application, Color, Command, Element, Length, Settings, Size, Subscription
};
use iced_native::Rectangle;
use iced::mouse;
use iced::widget::canvas::Cursor;
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

const MAX_POINTS: usize = 30;

pub fn main() -> iced::Result {
    // Example::run(Settings::default())
    let shared_tick = Arc::new(AtomicU64::new(0));
    let shared_data = Arc::new(Mutex::new(CollectedData::default()));
    
    {
        let thread_tick = Arc::clone(&shared_tick);
        let thread_data = Arc::clone(&shared_data);
    
        thread::spawn(move || {
            loop {
                collect_infos(&thread_data);
                thread_tick.fetch_add(1, Ordering::SeqCst);
                thread::sleep(Duration::from_secs(1));
            }
        });
    }

    {
        let thread_tick = Arc::clone(&shared_tick);
        let thread_data = Arc::clone(&shared_data);
    
        thread::spawn(move || {
            loop {
                collect_tasks(&thread_data);
                thread_tick.fetch_add(1, Ordering::SeqCst);
                thread::sleep(Duration::from_secs(2));
            }
        });
    }

    {
        let thread_tick = Arc::clone(&shared_tick);
        let thread_data = Arc::clone(&shared_data);
    
        thread::spawn(move || {
            loop {
                collect_memory(&thread_data);
                thread_tick.fetch_add(1, Ordering::SeqCst);
                thread::sleep(Duration::from_secs(2));
            }
        });
    }

    {
        let thread_tick = Arc::clone(&shared_tick);
        let thread_data = Arc::clone(&shared_data);
    
        thread::spawn(move || {
            loop {
                collect_cpu(&thread_data);
                thread_tick.fetch_add(1, Ordering::SeqCst);
                thread::sleep(Duration::from_secs(2));
            }
        });
    }

    {
        let thread_tick = Arc::clone(&shared_tick);
        let thread_data = Arc::clone(&shared_data);
    
        thread::spawn(move || {
            loop {
                collect_disks(&thread_data);
                thread_tick.fetch_add(1, Ordering::SeqCst);
                thread::sleep(Duration::from_secs(2));
            }
        });
    }

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

fn collect_tasks(shared_data: &Arc<Mutex<CollectedData>>) {
    use std::process::{Command, Stdio};
    use std::io::Read;
    let process = Command::new("/usr/bin/ps")
                        .stdout(Stdio::piped())
                        .args(["-ew", "-o", "pid,ni,s,user,%cpu,%mem,args"])
                        .spawn()
                        .expect("failed to execute process");

    let mut s = String::new();
    process.stdout.unwrap().read_to_string(&mut s).unwrap();
    let mut lines = s.lines();
    lines.next();

    let mut process_list = vec![];

    let mut last_whitespace = false;

    for line in lines {
        let fields: Vec<_> = line.trim().splitn(7, |c: char| {
            if c.is_whitespace() {
                if last_whitespace {
                    return false
                }
                last_whitespace = true;
                true
            } else {
                last_whitespace = false;
                false
            }
        }).map(str::trim).collect();
        last_whitespace = false;
        if fields.len() == 7 {
            // println!("{:?}", fields);
            process_list.push(ProcessInfo {
                pid: fields[0].parse().unwrap(),
                nice: fields[1].to_owned(),
                status: fields[2].to_owned(),
                user: fields[3].to_owned(),
                cpu: fields[4].parse().unwrap(),
                memory: fields[5].parse().unwrap(),
                cmd: fields[6].to_owned(),
            });
        }
    }

    let mut data = shared_data.lock().unwrap();
    data.process_list = process_list;
    data.updated_tasks = true;
    data.tick += 1;
}

fn collect_memory(shared_data: &Arc<Mutex<CollectedData>>) {
    use std::process::{Command, Stdio};
    use std::io::Read;
    let process0 = Command::new("/usr/bin/sudo")
                        .stdout(Stdio::piped())
                        .args(["/usr/bin/dmidecode", "-t", "memory"])
                        .spawn()
                        .expect("failed to execute process0");

    let mut s0 = String::new();
    process0.stdout.unwrap().read_to_string(&mut s0).unwrap();
    let lines = s0.lines();
    let tech = lines.map(str::trim).find_map(|s| {
        if s.starts_with("Type: ") {
            let (_a, b) = s.split_at(6);
            Some(b.to_string())
        } else {
            None
        }
    }).unwrap_or_else(|| String::from("DDR3"));

    let process1 = Command::new("/usr/bin/free")
                        .stdout(Stdio::piped())
                        .args(["-b", "-t"])
                        .spawn()
                        .expect("failed to execute process1");

    let mut s1 = String::new();
    process1.stdout.unwrap().read_to_string(&mut s1).unwrap();
    let mut lines = s1.lines();
    lines.next();
    let mem_line = lines.next().unwrap();
    let swap_line = lines.next().unwrap();

    let mut last_whitespace = false;

    let mem_fields: Vec<_> = mem_line.trim().splitn(7, |c: char| {
        if c.is_whitespace() {
            if last_whitespace {
                return false
            }
            last_whitespace = true;
            true
        } else {
            last_whitespace = false;
            false
        }
    }).map(str::trim).collect();
    last_whitespace = false;
    let swap_fields: Vec<_> = swap_line.trim().splitn(4, |c: char| {
        if c.is_whitespace() {
            if last_whitespace {
                return false
            }
            last_whitespace = true;
            true
        } else {
            last_whitespace = false;
            false
        }
    }).map(str::trim).collect();
    let mem_data = if mem_fields.len() == 7 {
        let total: u64 = mem_fields[1].parse().unwrap();
        let used: u64 = mem_fields[2].parse().unwrap();
        let free: u64 = mem_fields[3].parse().unwrap();
        let buff: u64 = mem_fields[5].parse().unwrap();
        (total, used, free, buff)
    } else {
        panic!("wrong");
    };
    let swap_data = if swap_fields.len() == 4 {
        let total: u64 = swap_fields[1].parse().unwrap();
        let used: u64 = swap_fields[2].parse().unwrap();
        let free: u64 = swap_fields[3].parse().unwrap();
        let buff: u64 = 0;
        (total, used, free, buff)
    } else {
        panic!("panic");
    };

    let mem_total = format!("{:.2}G", (mem_data.0 as f64) / (1024.*1024.*1024.));
    let swap_total = format!("{:.2}G", (swap_data.0 as f64) / (1024.*1024.*1024.));
    let mem_used = ((mem_data.1 as f64) / (mem_data.0 as f64)) * 100.;
    let mem_buff = ((mem_data.3 as f64) / (mem_data.0 as f64)) * 100.;
    let swap_used = ((swap_data.1 as f64) / (swap_data.0 as f64)) * 100.;
    let swap_buff = 0.0_f64;

    let ram_usage = (mem_used, mem_buff, swap_used, swap_buff, tech, mem_total, swap_total);

    let mut data = shared_data.lock().unwrap();
    data.ram_usage = ram_usage;
    data.updated_memory = true;
    data.tick += 1;
}

fn collect_cpu(shared_data: &Arc<Mutex<CollectedData>>) {
    use std::process::{Command, Stdio};
    use std::io::Read;
    let process0 = Command::new("/usr/bin/top")
                        .stdout(Stdio::piped())
                        .args(["-b", "-d", "0.5", "-n", "2"])
                        .spawn()
                        .expect("failed to execute process0");

    let mut s0 = String::new();
    process0.stdout.unwrap().read_to_string(&mut s0).unwrap();
    let mut a = s0.split("top - ");
    a.next();
    let s0 = a.next().unwrap();
    let lines = s0.lines();

    let mut last_whitespace = false;
    let usages = lines.filter(|l| l.starts_with("%Cpu")).map(|l| {
        last_whitespace = false;
        // println!("{}", l.trim().splitn(7, ',').find(|s| s.contains("id")).trim().unwrap());
        100. - l.trim().splitn(7, ',').find(|s| s.contains("id")).unwrap().trim().split(' ').next().unwrap().trim().replace(',', ".").parse::<f64>().unwrap()
    });

    let cpu_usage: Vec<_> = usages.collect();

    let mut data = shared_data.lock().unwrap();
    while data.cpu_usage.len() < cpu_usage.len() {
        data.cpu_usage.push(VecDeque::new());
    }

    for (i, usage) in cpu_usage.into_iter().enumerate() {
        data.cpu_usage[i].push_front(usage);
        if data.cpu_usage[i].len() > MAX_POINTS {
            data.cpu_usage[i].pop_back();
        }
    }
    // println!("{:?}", data.cpu_usage);
    data.updated_cpu = true;
    data.tick += 1;
}

fn collect_disks(shared_data: &Arc<Mutex<CollectedData>>) {
    use std::process::{Command, Stdio};
    use std::io::Read;
    // let process0 = Command::new("/usr/bin/ls")
    //                     .stdout(Stdio::piped())
    //                     .args(["/sys/block"])
    //                     .spawn()
    //                     .expect("failed to execute process0");

    // let mut s0 = String::new();
    // process0.stdout.unwrap().read_to_string(&mut s0).unwrap();
    // let devices = s0.trim().split(|c: char| {
    //     if c.is_whitespace() {
    //         if last_whitespace {
    //             return false
    //         }
    //         last_whitespace = true;
    //         true
    //     } else {
    //         last_whitespace = false;
    //         false
    //     }
    // }).map(str::to_string);
    // last_whitespace = false;


    let process1 = Command::new("/usr/bin/df")
                        .stdout(Stdio::piped())
                        .args(["-T"])
                        .spawn()
                        .expect("failed to execute process1");

    let mut s1 = String::new();
    process1.stdout.unwrap().read_to_string(&mut s1).unwrap();
    let mut lines = s1.lines();
    lines.next();

    let mut last_whitespace = false;
    let mut v: Vec<Vec<&str>> = vec![];
    for line in lines.filter(|l| l.contains("ext") || l.contains("fat")) {
        v.push(line.trim().split(|c: char| {
            if c.is_whitespace() {
                if last_whitespace {
                    return false
                }
                last_whitespace = true;
                true
            } else {
                last_whitespace = false;
                false
            }
        }).collect());
        last_whitespace = false;
    }
    let partitions: Vec<_> = v.into_iter().map(|s| {
        // println!("_{}_ _{}_ _{}_", s[5].trim().split('%').next().unwrap(), s[0], s[2]);
        (s[5].trim().split('%').next().unwrap().parse::<f64>().unwrap(), s[0].to_string(), format!("{:.0}G", s[2].trim().parse::<f64>().unwrap() / (1024. * 1024.)))
    }).collect();

    // let cpu_usage: Vec<_> = usages.collect();

    let mut data = shared_data.lock().unwrap();
    data.disk_usage = partitions;
    data.updated_disks = true;
    data.tick += 1;
}

fn collect_infos(shared_data: &Arc<Mutex<CollectedData>>) {
    use std::process::{Command, Stdio};
    use std::io::Read;
    
    let process1 = Command::new("/usr/bin/uname")
                        .stdout(Stdio::piped())
                        .args(["-a"])
                        .spawn()
                        .expect("failed to execute process1");
    
    let process2 = Command::new("/usr/bin/lshw")
                        .stdout(Stdio::piped())
                        .stderr(Stdio::null())
                        .args(["-short"])
                        .spawn()
                        .expect("failed to execute process1");

    let mut s1 = String::new();
    process1.stdout.unwrap().read_to_string(&mut s1).unwrap();
    let lines1 = s1.lines().map(str::to_string);
    
    let mut s2 = String::new();
    process2.stdout.unwrap().read_to_string(&mut s2).unwrap();
    let lines2 = s2.lines().map(str::to_string);
    
    let lines = lines1.chain(std::iter::once(String::from("\n"))).chain(lines2).collect();
    let mut data = shared_data.lock().unwrap();
    data.extra_infos = lines;
    data.tick += 1;
}

#[derive(Default, Clone)]
pub struct ProcessInfo {
    pid: usize,
    nice: String,
    status: String,
    user: String,
    cpu: f64,
    memory: f64,
    cmd: String,
}

#[derive(Default, Clone)]
struct CollectedData {
    cpu_usage: Vec<VecDeque<f64>>,
    ram_usage: (f64, f64, f64, f64, String, String, String),
    disk_usage: Vec<(f64, String, String)>,
    process_list: Vec<ProcessInfo>,
    extra_infos: Vec<String>,
    updated_tasks: bool,
    updated_memory: bool,
    updated_cpu: bool,
    updated_disks: bool,
    tick: u64,
}

#[derive(Default)]
struct LocalData {
    current_data_copy: CollectedData,
    cpu_charts: Vec<CpuUsageChart>,
    disk_charts: Vec<DiskUsageChart>,
    memory_chart: Option<MemoryUsageChart>,
    tasks_chart: Option<tasks::TasksListChart>,
}

struct Example {
    panes: pane_grid::State<Pane>,
    panes_created: usize,
    focus: Option<pane_grid::Pane>,
    tasks_pane: Option<pane_grid::Pane>,
    last_tick: u64,
    shared_data: Arc<Mutex<CollectedData>>,
    shared_tick: Arc<AtomicU64>,
    local_data: LocalData,
    show_title_bar: bool,
}

#[derive(Debug, Clone, Copy)]
enum Message {
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
    UnFocus,
    Tick,
    ChangeType(pane_grid::Pane, PaneType),
    ChangeTypeFocused(PaneType),
    DraggedTask(usize, f32),
    SortTasks(usize),
    ToggleTitleBar,
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
            let used = &self.current_data_copy.disk_usage[i];
            chart.set_data((used.0, 100.0 - used.0), used.1.clone(), used.2.clone());
        }
    }

    fn update_memory(&mut self) {
        if self.memory_chart.is_none() {
            self.memory_chart = Some(MemoryUsageChart::new());
        }
        if let Some(memory_chart) = &mut self.memory_chart {
            let mem_data = &self.current_data_copy.ram_usage;
            memory_chart.set_data((mem_data.0, mem_data.1), (mem_data.2, mem_data.3), mem_data.4.clone(), mem_data.5.clone(), mem_data.6.clone());
        }
    }

    fn update_tasks(&mut self) {
        if self.tasks_chart.is_none() {
            self.tasks_chart = Some(tasks::TasksListChart::new());
        }
        if let Some(tasks_chart) = &mut self.tasks_chart {
            let process_data = &self.current_data_copy.process_list;
            tasks_chart.set_data(process_data);
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
                tasks_pane: None,
                last_tick: 0,
                shared_data: flags.0,
                shared_tick: flags.1,
                local_data: LocalData {
                    current_data_copy: CollectedData::default(),
                    cpu_charts: Vec::new(),
                    disk_charts: Vec::new(),
                    memory_chart: None,
                    tasks_chart: None,
                },
                show_title_bar: false,
            },
            Command::none(),
        )
    }

    fn title(&self) -> String {
        String::from("Dashboard")
    }

    fn update(&mut self, message: Self::Message) -> Command<Self::Message> {
        match message {
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
            Message::UnFocus => {
                self.focus = None;
            }
            Message::Tick => {
                let current_tick = self.shared_tick.load(Ordering::SeqCst);
                if self.last_tick != current_tick {
                    self.last_tick = current_tick;
                    {
                        let mut data = self.shared_data.lock().unwrap();
                        self.local_data.current_data_copy = data.clone();
                        data.updated_tasks = false;
                        data.updated_memory = false;
                        data.updated_cpu = false;
                        data.updated_disks = false;
                    }
                    self.local_data.update_cpus();
                    self.local_data.update_disks();

                    if self.local_data.current_data_copy.updated_cpu {
                        self.local_data.current_data_copy.updated_cpu = false;
                        self.local_data.update_cpus();
                    }
                    if self.local_data.current_data_copy.updated_disks {
                        self.local_data.current_data_copy.updated_disks = false;
                        self.local_data.update_disks();
                    }
                    if self.local_data.current_data_copy.updated_memory {
                        self.local_data.current_data_copy.updated_memory = false;
                        self.local_data.update_memory();
                    }
                    if self.local_data.current_data_copy.updated_tasks {
                        self.local_data.current_data_copy.updated_tasks = false;
                        self.local_data.update_tasks();
                    }
                }
            }
            Message::ChangeType(pane, new_pane_type) => {
                if let Some(Pane { pane_type, ..}) = self.panes.get_mut(&pane)
                {
                    if new_pane_type != PaneType::Tasks {
                        if *pane_type == PaneType::Tasks {
                            self.tasks_pane = None;
                        }
                        *pane_type = new_pane_type;
                    }
                    if new_pane_type == PaneType::Tasks && self.tasks_pane.is_none() {
                        *pane_type = new_pane_type;
                        self.tasks_pane = Some(pane);
                    }
                }
            }
            Message::ChangeTypeFocused(new_pane_type) => {
                if let Some(pane) = &mut self.focus {
                    if let Some(Pane { pane_type, ..}) = self.panes.get_mut(pane) {
                        if new_pane_type != PaneType::Tasks {
                            if *pane_type == PaneType::Tasks {
                                self.tasks_pane = None;
                            }
                            *pane_type = new_pane_type;
                        }
                        if new_pane_type == PaneType::Tasks && self.tasks_pane.is_none() {
                            *pane_type = new_pane_type;
                            self.tasks_pane = Some(*pane);
                        }
                    }
                }
            }
            Message::DraggedTask(selected, new_sep) => {
                if let Some(tasks_chart) = &mut self.local_data.tasks_chart {
                    tasks_chart.separators[selected] = new_sep;
                }
            }
            Message::SortTasks(selected) => {
                if let Some(tasks_chart) = &mut self.local_data.tasks_chart {
                    tasks_chart.sort_by(selected);
                }
            }
            Message::ToggleTitleBar => {
                self.show_title_bar = !self.show_title_bar;
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
                    }) => {
                        if modifiers.command() {
                            handle_hotkey(key_code, true)
                        } else if modifiers.shift() {
                            handle_hotkey(key_code, false)
                        } else {
                            None
                        }
                    },
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
            
            let grid = pane_grid::Content::new(responsive(move |size| {
                view_content(
                    id,
                    total_panes,
                    pane.is_pinned,
                    size,
                    pane.pane_type,
                    &self.local_data,
                )
            }));

            if self.show_title_bar {
                grid
                .title_bar(title_bar)
                .style(if is_focused {
                    style::pane_focused
                } else {
                    style::pane_active
                })
            } else {
                grid.style(if is_focused {
                    style::pane_focused
                } else {
                    style::pane_active
                })
            }
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

fn handle_hotkey(key_code: keyboard::KeyCode, control: bool) -> Option<Message> {
    use keyboard::KeyCode;
    use pane_grid::{Axis, Direction};

    if control {
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
            KeyCode::U => Some(Message::UnFocus),
            _ => direction.map(Message::FocusAdjacent),
        }
    } else {
        match key_code {
            KeyCode::C => Some(Message::ChangeTypeFocused(PaneType::Cpu)),
            KeyCode::M => Some(Message::ChangeTypeFocused(PaneType::Memory)),
            KeyCode::T => Some(Message::ChangeTypeFocused(PaneType::Tasks)),
            KeyCode::D => Some(Message::ChangeTypeFocused(PaneType::Disks)),
            KeyCode::I => Some(Message::ChangeTypeFocused(PaneType::Info)),
            KeyCode::B => Some(Message::ToggleTitleBar),
            _ => None,
        }
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

const USED_COLOR: Color = Color::from_rgb(
    1.0,
    222_f32 / 255.0,
    153_f32 / 255.0
);
const FREE_COLOR: Color = Color::from_rgb(
    153_f32 / 255.0,
    222_f32 / 255.0,
    1.0
);

const MEM_USED_COLOR: Color = Color::from_rgb(
    175_f32 / 255.0,
    175_f32 / 255.0,
    175_f32 / 255.0
);
const MEM_BUFF_COLOR: Color = Color::from_rgb(
    175_f32 / 255.0,
    175_f32 / 255.0,
    1.0
);
const MEM_FREE_COLOR: Color = Color::from_rgb(
    0_f32 / 255.0,
    175_f32 / 255.0,
    1.0
);

#[derive(Debug)]
struct ColoredRect {
    color: Color,
}
use iced::widget::canvas::Path;
use iced_native::Point;
use iced::widget::canvas::Stroke;

impl canvas::Program<Message> for ColoredRect {
    type State = ();

    fn draw(&self, _state: &(), _theme: &Theme, bounds: Rectangle, _cursor: Cursor) -> Vec<Geometry>{
        // // We prepare a new `Frame`
        // let mut frame = Frame::new(bounds.size());

        // // let rec1 = canvas::Path::rectangle(frame., iced::Size::new(bounds.width, bounds.height));
        // // let rec1 = canvas::Path::rectangle(iced::Point::new(bounds.x, bounds.y), iced::Size::new(bounds.width, bounds.height));
        // // frame.fill(&rec1, self.color);
        // frame.fill_rectangle(iced::Point::new(bounds.x, bounds.y), iced::Size::new(bounds.width, bounds.height), self.color);

        // vec![frame.into_geometry()]

        let mut frame = Frame::new(bounds.size());
        // frame.stroke(
        //     &Path::rectangle(
        //         Point {
        //             x: bounds.width / 10.,
        //             y: bounds.height / 10.,
        //         },
        //         Size {
        //             width: 4. * bounds.width / 5.,
        //             height: 4. * bounds.height / 5.,
        //         },
        //     ),
        //     Stroke::default(),
        // );
        frame.fill(&Path::rectangle(
                Point {
                    x: bounds.width / 10.,
                    y: bounds.height / 10.,
                },
                Size {
                    width: 4. * bounds.width / 5.,
                    height: 4. * bounds.height / 5.,
                },
            ),
            self.color,
        );

        vec![frame.into_geometry()]
    }
}

impl PaneType {
    fn content<'a>(&self, data: &'a LocalData, size: Size) -> Element<'a, Message> {
        match *self {
            PaneType::Selection => {
                text("Select pane type").size(16).into()
            }
            PaneType::Cpu => {
                let mut content = column![
                    text("CPU").size(24),
                ]
                .width(Length::Fill)
                // .height(Length::Fill)
                .spacing(0)
                .align_items(Alignment::Center);

                let padding = 10;

                let min_width = 150.0 + (padding as f32);
                let width = size.width - (padding as f32);
                let items_per_row = std::cmp::min(
                    std::cmp::max((width / min_width).trunc() as usize, 1usize),
                    data.cpu_charts.len()
                );
                let width_per_item = (width / (items_per_row as f32)) as u16 - padding;
                let height_per_item = (width_per_item * 2) / 3;
                // let max_width = 450;


                let mut i: usize = 0;
                let iter = data.cpu_charts.chunks(items_per_row);
                for cpu_charts in iter {
                    let mut row = row(vec![])
                        .spacing(padding)
                        .padding(padding)
                        .width(Length::Fill)
                        .height(Length::Units(height_per_item + 10))
                        .align_items(Alignment::Center);

                    for cpu_chart in cpu_charts {
                        row = row.push(container(cpu_chart.view(i))
                            .padding(0)
                            .width(Length::Units(width_per_item))
                            .height(Length::Units(height_per_item + 10))
                        );
                        i += 1;
                    }

                    content = content.push(row);
                }

                content.into()
                // content.into::<Element<Message, iced::Renderer>>()
                // std::convert::Into::<Element<Message, iced::Renderer>>::into(content).explain(Color::BLACK)
            }
            PaneType::Memory => {
                let mut content = column![
                    text("Memory").size(24),
                    row(vec![
                        canvas(ColoredRect { color: MEM_USED_COLOR })
                            .width(Length::Units(20))
                            .height(Length::Units(20))
                            .into(),
                        text("Used").size(16).into(),
                        canvas(ColoredRect { color: MEM_BUFF_COLOR })
                            .width(Length::Units(20))
                            .height(Length::Units(20))
                            .into(),
                        text("Buffered").size(16).into(),
                        canvas(ColoredRect { color: MEM_FREE_COLOR })
                            .width(Length::Units(20))
                            .height(Length::Units(20))
                            .into(),
                        text("Free").size(16).into(),
                    ]).align_items(Alignment::Center)
                ]
                .width(Length::Fill)
                .spacing(10)
                .align_items(Alignment::Center);

                if let Some(memory_chart) = &data.memory_chart {
                    content = content.push(container(memory_chart.view())
                    .padding(0)
                    .width(Length::Fill)
                    .height(Length::Units(150))
                    );
                }
                
                content.into()
            }
            PaneType::Disks => {
                // let c = canvas(ColoredRect { color: USED_COLOR })
                //     .width(Length::Units(50))
                //     .height(Length::Units(50));
                // return std::convert::Into::<Element<Message, iced::Renderer>>::into(c).explain(Color::BLACK);
                
                let mut content = column![
                    text("Partitions").size(24),
                    row(vec![
                        canvas(ColoredRect { color: USED_COLOR })
                            .width(Length::Units(20))
                            .height(Length::Units(20))
                            .into(),
                        text("Used").size(16).into(),
                        canvas(ColoredRect { color: FREE_COLOR })
                            .width(Length::Units(20))
                            .height(Length::Units(20))
                            .into(),
                        text("Free").size(16).into(),
                    ]).align_items(Alignment::Center)
                ]
                .width(Length::Fill)
                .spacing(0)
                .align_items(Alignment::Center);

                let padding = 10;

                let min_width = 150.0 + (padding as f32);
                let width = size.width - (padding as f32);
                let items_per_row = std::cmp::min(
                    std::cmp::max((width / min_width).trunc() as usize, 1usize),
                    data.disk_charts.len()
                );
                let width_per_item = (width / (items_per_row as f32)) as u16 - padding;
                let height_per_item = width_per_item;
                // let max_width = 450;

                let mut i = 0;
                for disk_charts in data.disk_charts.chunks(items_per_row) {
                    let mut row = row(vec![])
                        .spacing(5)
                        .padding(5)
                        .width(Length::Fill)
                        .height(Length::Units(height_per_item - 10))
                        .align_items(Alignment::Center);

                    for disk_chart in disk_charts {
                        row = row.push(container(disk_chart.view(i))
                            .padding(0)
                            .width(Length::Units(width_per_item))
                            .height(Length::Units(height_per_item - 10))
                        );
                        i += 1;
                    }
                    content = content.push(row);
                }

                content.into()
                // std::convert::Into::<Element<Message, iced::Renderer>>::into(content).explain(Color::BLACK)
            }
            PaneType::Info => {
                let mut content = column![
                    text("System Info").size(24),
                ]
                .width(Length::Fill)
                .spacing(5)
                .align_items(Alignment::Center);

                let mut info_content = column![]
                .width(Length::Fill)
                .spacing(0)
                .align_items(Alignment::Start);

                for line in &data.current_data_copy.extra_infos {
                    info_content = info_content.push(text(line).size(16));
                }
                content = content.push(info_content);

                content.into()
            }
            PaneType::Tasks => {
                let mut content = column![
                    text("Tasks").size(24),
                ]
                .width(Length::Fill)
                .spacing(0)
                .align_items(Alignment::Center);


                if let Some(tasks_chart) = &data.tasks_chart {
                    content = content.push(canvas(tasks_chart)
                        .width(Length::Fill)
                        .height(Length::Fill)
                    );
                }
                // let mut pid = column![].width(Length::Units(50)).spacing(2).align_items(Alignment::Start);
                // let mut name = column![].width(Length::Units(200)).spacing(2).align_items(Alignment::Start);
                // let mut status = column![].width(Length::Units(70)).spacing(2).align_items(Alignment::Start);
                // let mut user = column![].width(Length::Units(70)).spacing(2).align_items(Alignment::Start);
                // let mut cpu = column![].width(Length::Units(40)).spacing(2).align_items(Alignment::End);
                // let mut memory = column![].width(Length::Units(45)).spacing(2).align_items(Alignment::End);
                // let mut cmd = column![].width(Length::Fill).spacing(2).align_items(Alignment::Start);

                // pid = pid.push(text("PID").size(16));
                // name = name.push(text("Name").size(16));
                // status = status.push(text("Status").size(16));
                // user = user.push(text("User").size(16));
                // cpu = cpu.push(text("CPU%").size(16));
                // memory = memory.push(text("MEM%").size(16));
                // cmd = cmd.push(text("Command").size(16));

                // for task in &data.current_data_copy.process_list {
                //     pid = pid.push(text(format!("{}", task.pid)).size(16));
                //     name = name.push(text(format!("{}", task.name)).size(16));
                //     status = status.push(text(format!("{}", task.status)).size(16));
                //     user = user.push(text(format!("{}", task.user)).size(16));
                //     cpu = cpu.push(text(format!("{}", task.cpu)).size(16));
                //     memory = memory.push(text(format!("{}", task.memory)).size(16));
                //     cmd = cmd.push(text(format!("{}", task.cmd)).size(16).height(Length::Units(16)));
                // }

                // let content = row![
                //     pid,
                //     name,
                //     status,
                //     user,
                //     cpu,
                //     memory,
                //     cmd,
                // ]
                // .width(Length::Fill)
                // .spacing(5)
                // .align_items(Alignment::Start);
                
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
                write!(f, "Partitions")
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

fn view_content<>(
    _pane: pane_grid::Pane,
    _total_panes: usize,
    _is_pinned: bool,
    size: Size,
    pane_type: PaneType,
    local_data: &'_ LocalData,
) -> Element<'_, Message> {
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
    if pane_type == PaneType::Tasks {
        container(pane_type.content(local_data,size))
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(5)
            .center_x()
            .center_y()
            .into()
    } else {
        container(scrollable(pane_type.content(local_data, size)))
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(5)
            .center_x()
            .center_y()
            .into()
    }
    
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
                .height(Length::Shrink)
                .spacing(0)
                .padding(0)
                .push(text(format!("Core {}", idx)))
                .push(
                    ChartWidget::new(self).height(Length::Fill),
                )
                .align_items(Alignment::Center),
        )
        .width(Length::Fill)
        .height(Length::Shrink)
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
            .y_label_area_size(24)
            .margin(5)
            .build_cartesian_2d(1..self.max_points, 0f64..100.0)
            .expect("failed to build chart");

        chart
            .configure_mesh()
            .bold_line_style(plotters::style::colors::BLUE.mix(0.1))
            .light_line_style(plotters::style::colors::BLUE.mix(0.05))
            .axis_style(ShapeStyle::from(plotters::style::colors::BLUE.mix(0.45)).stroke_width(1))
            .y_labels(10)
            .y_label_style(
                ("sans-serif", 12)
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
    tech: String,
    capacity: String,
}

impl DiskUsageChart {
    fn new() -> Self {
        Self {
            cache: Cache::new(),
            data_points: (0.0, 100.0),
            tech: String::from(""),
            capacity: String::from(""),
        }
    }

    fn set_data(&mut self, value: (f64, f64), tech: String, cap: String) {
        self.data_points = value;
        self.tech = tech;
        self.capacity = cap;
        self.cache.clear();
    }

    fn view(&self, _idx: usize) -> Element<Message> {
        // container(
        //     column(Vec::new())
        //         .width(Length::Fill)
        //         .height(Length::Fill)
        //         .spacing(5)
        //         .push(text(format!("Disk {}", idx)))
        //         .push(
        //             ChartWidget::new(self).height(Length::Fill),
        //         ),
        // )
        // .width(Length::Fill)
        // .height(Length::Fill)
        // .align_x(alignment::Horizontal::Center)
        // .align_y(alignment::Vertical::Center)
        // .into()

        container(
            column(Vec::new())
                .width(Length::Fill)
                .height(Length::Shrink)
                .spacing(0)
                .padding(0)
                .push(text(format!("Partition {} ({})", self.tech, self.capacity)))
                .push(
                    ChartWidget::new(self).height(Length::Fill),
                )
                .align_items(Alignment::Center),
        )
        .width(Length::Fill)
        .height(Length::Shrink)
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

    fn build_chart<DB: DrawingBackend>(&self, _state: &Self::State, _chart: ChartBuilder<DB>) {}

    fn draw_chart<DB: DrawingBackend>(&self, _state: &Self::State, root: plotters_iced::DrawingArea<DB, plotters::coord::Shift>) {
        use plotters::prelude::*;
        
        const USED_COLOR: RGBColor = RGBColor(255, 222, 153);
        const FREE_COLOR: RGBColor = RGBColor(153, 222, 255);

        // let mut chart = chart
        //     .x_label_area_size(0)
        //     .y_label_area_size(0)
        //     .margin(5)
        //     .build_cartesian_2d(0..300, 0..300)
        //     .expect("failed to build chart");
            
        // chart
        //     .configure_mesh()
        //     .disable_x_mesh()
        //     .disable_y_mesh()
        //     .draw()
        //     .expect("failed to draw chart mesh");

        // let area = chart.plotting_area();
        let area = root;
        let dims = area.dim_in_pixel();
        let center = (dims.0 as i32 / 2, dims.1 as i32 / 2);
        let radius = (dims.1 / 2) as f64;
        let sizes = vec![self.data_points.1, self.data_points.0];
        let colors = vec![FREE_COLOR, USED_COLOR];
        let labels = vec!["", ""];

        let mut pie = Pie::new(&center, &radius, &sizes, &colors, &labels);
        pie.start_angle(-90.0);
        // pie.label_style((("sans-serif", 16).into_font()).color(&(BLACK)));
        pie.percentages((("sans-serif", radius * 0.32).into_font()).color(&BLACK));
        area.draw(&pie)
            .expect("failed to draw pie graph");

    }
}

struct MemoryUsageChart {
    cache: Cache,
    memory_points: (f64, f64),
    swap_points: (f64, f64),
    tech: String,
    capacity: String,
    swap_capacity: String,
}

impl MemoryUsageChart {
    fn new() -> Self {
        Self {
            cache: Cache::new(),
            memory_points: (0.0, 100.0),
            swap_points: (0.0, 100.0),
            tech: String::from(""),
            capacity: String::from(""),
            swap_capacity: String::from(""),
        }
    }

    fn set_data(&mut self, memory_value: (f64, f64), swap_value: (f64, f64), tech: String, cap: String, swap_cap: String) {
        self.memory_points = memory_value;
        self.swap_points = swap_value;
        self.tech = tech;
        self.capacity = cap;
        self.swap_capacity = swap_cap;
        self.cache.clear();
    }

    fn view(&self) -> Element<Message> {
        container(
            column(Vec::new())
                .width(Length::Fill)
                .height(Length::Shrink)
                .spacing(0)
                .padding(0)
                .push(text(format!("Memory ({}, {}) - Swap ({})", self.tech, self.capacity, self.swap_capacity)))
                .push(
                    ChartWidget::new(self).height(Length::Fill),
                )
                // .push(text(format!("Swap ({})", self.swap_capacity)))
                // .push(
                //     ChartWidget::new(self).height(Length::Fill),
                // )
                .align_items(Alignment::Center),
        )
        .width(Length::Fill)
        .height(Length::Shrink)
        .align_x(alignment::Horizontal::Center)
        .align_y(alignment::Vertical::Center)
        .into()
    }
}

impl Chart<Message> for MemoryUsageChart {
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

        const USED_COLOR: RGBColor = RGBColor(175, 175, 175);
        const BUFF_COLOR: RGBColor = RGBColor(175, 175, 255);
        const FREE_COLOR: RGBColor = RGBColor(0, 175, 255);

        let mut chart = chart
            .x_label_area_size(0)
            .y_label_area_size(36)
            .margin(5)
            .build_cartesian_2d(0f64..100.0, (1i32..2).into_segmented())
            .expect("failed to build chart");

        chart
            .configure_mesh()
            // .bold_line_style(plotters::style::colors::BLUE.mix(0.1))
            // .light_line_style(plotters::style::colors::BLUE.mix(0.05))
            // .axis_style(ShapeStyle::from(plotters::style::colors::BLUE.mix(0.45)).stroke_width(1))
            .y_labels(10)
            .y_label_style(
                ("sans-serif", 14)
                    .into_font()
                    .color(&plotters::style::colors::BLACK.mix(0.65))
                    .transform(FontTransform::Rotate90),
            )
            .y_label_formatter(&|y| if let SegmentValue::CenterOf(yy) = y { if *yy == 1 { String::from("SWAP") } else { String::from("RAM") } } else { String::from("Error") })
            .draw()
            .expect("failed to draw chart mesh");
        
        let data = [
            (0.0, self.swap_points.0, 1, USED_COLOR),
            (self.swap_points.0, self.swap_points.0 + self.swap_points.1, 1, BUFF_COLOR),
            (self.swap_points.0 + self.swap_points.1, 100.0, 1, FREE_COLOR),
            (0.0, self.memory_points.0, 2, USED_COLOR),
            (self.memory_points.0, self.memory_points.0 + self.memory_points.1, 2, BUFF_COLOR),
            (self.memory_points.0 + self.memory_points.1, 100.0, 2, FREE_COLOR),
        ];

        chart
            .draw_series(data.into_iter().map(|(start, usage, which, color)| {
                let y0 = SegmentValue::Exact(which);
                let y1 = SegmentValue::Exact(which + 1);
                let mut bar = Rectangle::new([(start, y0), (usage, y1)], color.filled());
                bar.set_margin(5, 5, 0, 0);
                bar
            }))
            .expect("failed to draw chart data");
    }
}


// struct ProcessInfo {
//     pid: usize,
//     name: String,
//     status: String,
//     user: String,
//     cpu: f64,
//     memory: f64,
//     cmd: String,
// }

mod tasks {
    use crate::*;
    use iced::widget::canvas::{event::{self, Event}};
    
    pub struct TasksListChart {
        pub process_info: Vec<ProcessInfo>,
        pub separators: Vec<f32>,
        pub item_sort: ItemSort,
        pub rev: bool,
    }

    #[derive(PartialEq, Eq)]
    pub enum ItemSort {
        Pid,
        Nice,
        Status,
        User,
        Cpu,
        Memory,
        Cmd,
    }

    impl ItemSort {
        pub fn by(u: usize) -> Self {
            match u {
                0 => ItemSort::Pid,
                1 => ItemSort::Nice,
                2 => ItemSort::Status,
                3 => ItemSort::User,
                4 => ItemSort::Cpu,
                5 => ItemSort::Memory,
                6 => ItemSort::Cmd,
                _ => ItemSort::Nice,
            }
        }
    }
    
    impl TasksListChart {
        pub fn new() -> Self {
            TasksListChart {
                process_info: vec![],
                separators: (1..7).into_iter().map(|i| (i as f32) / 12.0).collect(),
                // separators: vec![9., 17., 25., 37., 44., 51.],
                item_sort: ItemSort::Memory,
                rev: true,
            }
        }
    
        pub fn set_data(&mut self, process_info: &[ProcessInfo]) {
            self.process_info = process_info.to_owned();
            self.sort();
        }
        
        pub fn sort_by(&mut self, u: usize) {
            let new_sort = ItemSort::by(u);
            if self.item_sort == new_sort {
                self.rev = !self.rev;
            } else {
                self.rev = match new_sort {
                    ItemSort::Pid => false,
                    ItemSort::Nice => false,
                    ItemSort::Status => true,
                    ItemSort::User => false,
                    ItemSort::Cpu => true,
                    ItemSort::Memory => true,
                    ItemSort::Cmd => false,
                }
            }
            self.item_sort = new_sort;
            self.sort();
        }

        pub fn sort(&mut self) {
            self.process_info.sort_unstable_by(|a, b| {
                match self.item_sort {
                    ItemSort::Pid => { a.pid.cmp(&b.pid) }
                    ItemSort::Nice => { a.nice.cmp(&b.nice) }
                    ItemSort::Status => { a.status.cmp(&b.status) }
                    ItemSort::User => { a.user.cmp(&b.user) }
                    ItemSort::Cpu => { a.cpu.partial_cmp(&b.cpu).unwrap() }
                    ItemSort::Memory => { a.memory.partial_cmp(&b.memory).unwrap() }
                    ItemSort::Cmd => { a.cmd.cmp(&b.cmd) }
                }
            });
            if self.rev {
                self.process_info.reverse();
            }
        }
    }
    
    #[derive(Debug, Clone, Copy)]
    pub enum Pending {
        One {
            selected: usize,
            diff: f32,
        },
        // Two { from: Point, to: Point },
    }
    
    impl canvas::Program<Message> for TasksListChart {
        type State = (Option<Pending>, mouse::Interaction);
    
        fn update(
            &self,
            state: &mut Self::State,
            event: Event,
            bounds: Rectangle,
            cursor: Cursor,
        ) -> (event::Status, Option<Message>) {
            use std::iter::once;
            // if let Some(Pending::One { selected, diff }) = state.0 {
            //     if let Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) = event {
            //         state.0 = None;
            //     } else if let Event::Mouse(mouse::Event::CursorMoved { position }) = event {
            //         let c = Cursor::Available(position);
            //         if let Some(position) = c.position_from(bounds.position()) {

            //         }
            //     }
            // }
            let cursor_position =
                if let Some(position) = cursor.position_from(bounds.position()) {
                    position
                } else {
                    return (event::Status::Ignored, None);
                };
    
    
            let padding = 5.;
            let start = padding;
            let width = bounds.width - (2. * padding);
            let _height = bounds.height - (2. * padding);

            let line_height = 18.;
    
            match event {
                Event::Mouse(mouse_event) => {
                    let message = match mouse_event {
                        mouse::Event::ButtonPressed(mouse::Button::Left) => {
                            match state.0 {
                                None => {
                                    
                                    let selected = self.separators.iter().enumerate().find(|(_i, &sep)| {
                                        f32::abs(cursor_position.x - (start + sep*width)) <= 2.5
                                        && cursor_position.y >= start
                                        && cursor_position.y <= start + line_height
                                    });
                                    
                                    let iter = once(&0.).chain(self.separators.iter()).chain(once(&100.));
                                    let mut iter2 = iter.clone();
                                    iter2.next();
                                    let iter = iter.zip(iter2);
                                    let over = iter.enumerate().find(|(_i, (&sep1, &sep2))| {
                                        cursor_position.x > (start + sep1*width) + 2.5
                                        && cursor_position.x < (start + sep2*width) - 2.5
                                        && cursor_position.y >= start
                                        && cursor_position.y <= start + line_height
                                    });
    
                                    
                                    if let Some((selected, &sep)) = selected {
                                        state.0 = Some(Pending::One {
                                            selected,
                                            diff: cursor_position.x - (start + sep*width),
                                        });
                                        state.1 = mouse::Interaction::ResizingHorizontally;

                                        None
                                    } else if let Some((selected, _)) = over {
                                        state.1 = mouse::Interaction::Pointer;
                                        Some(Message::SortTasks(selected))
                                    } else {
                                        state.1 = mouse::Interaction::Idle;
                                        None
                                    }
                                }
                                _ => {
                                    None
                                }
                            }
                        }
                        mouse::Event::ButtonReleased(mouse::Button::Left) => {
                            match state.0 {
                                Some(Pending::One { .. }) => {
                                    state.0 = None;
    
                                    None
                                }
                                _ => {
                                    None
                                }
                            }
                        }
                        mouse::Event::CursorMoved { position } => {
                            let pos_x = if let Some(position) = Cursor::Available(position).position_from(bounds.position()) {
                                position.x
                            } else {
                                cursor_position.x
                            };
                            match state.0 {
                                Some(Pending::One { selected, diff }) => {
                                    let pos_x = pos_x - diff;
                                    let prev = if selected == 0 {
                                        None
                                    } else {
                                        Some(selected - 1)
                                    };
                                    let next = if selected == 5 {
                                        None
                                    } else {
                                        Some(selected + 1)
                                    };
                                    
                                    let mut ok = true;
                                    
    
                                    if let Some(prev) = prev {
                                        let sep = self.separators[prev];
                                        if pos_x <= start + sep*width + 5. {
                                            ok = false;
                                        }
                                    } else if pos_x <= start + 5.{
                                        ok = false;
                                    }
    
                                    if let Some(next) = next {
                                        let sep = self.separators[next];
                                        if pos_x >= start + sep*width - 5. {
                                            ok = false;
                                        }
                                    } else if pos_x >= start + width - 5. {
                                        ok = false;
                                    }
    
                                    if ok {
                                        Some(Message::DraggedTask(selected, (pos_x - start) / width))
                                    } else {
                                        None
                                    }
                                }
                                None => {

                                    if self.separators.iter().any(|&sep| {
                                        f32::abs(pos_x - (start + sep*width)) <= 2.5
                                        && cursor_position.y >= start
                                        && cursor_position.y <= start + line_height
                                    }) {
                                        state.1 = mouse::Interaction::ResizingHorizontally;
                                    } else if cursor_position.x > start + 2.5
                                    && cursor_position.x < start + width - 2.5
                                    && cursor_position.y >= start
                                    && cursor_position.y <= start + line_height {
                                        state.1 = mouse::Interaction::Pointer;
                                    } else {
                                        state.1 = mouse::Interaction::Idle;
                                    }

                                    None
                                }
                            }
                        }
                        _ => None,
                    };
    
                    (event::Status::Captured, message)
                }
                _ => (event::Status::Ignored, None),
            }
        }

        fn mouse_interaction(
            &self,
            state: &Self::State,
            _bounds: Rectangle<f32>,
            _cursor: Cursor
        ) -> mouse::Interaction {
            state.1
        }
    
        fn draw(
            &self,
            _state: &Self::State,
            _theme: &Theme,
            bounds: Rectangle,
            _cursor: Cursor
        ) -> Vec<Geometry>{
            let mut frame = Frame::new(bounds.size());
    
            let padding = 5.;
            let start = padding;
            let width = bounds.width - (2. * padding);
            let height = bounds.height - (2. * padding);

            let line_height = 18.;
    
            frame.fill(&Path::rectangle(
                    Point {
                        x: padding,
                        y: padding,
                    },
                    Size {
                        width,
                        height,
                    },
                ),
                Color::WHITE,
            );
    
            for sep in &self.separators {
                frame.stroke(&Path::line(
                        Point {
                            x: f32::trunc(start + sep*width),
                            y: f32::trunc(start),
                        },
                        Point {
                            x: f32::trunc(start + sep*width),
                            y: f32::trunc(start + height),
                        },
                    ),
                    Stroke::default().with_width(1.),
                );
            }
            frame.stroke(&Path::line(
                    Point {
                        x: f32::trunc(start),
                        y: f32::trunc(start + line_height),
                    },
                    Point {
                        x: f32::trunc(start + width),
                        y: f32::trunc(start + line_height),
                    },
                ),
                Stroke::default().with_width(1.),
            );

            let write: Vec<_> = std::iter::once(&0.).chain(self.separators.iter()).enumerate().map(|(i, &sep)| {
                let sp2 = *self.separators.get(i).unwrap_or(&1.);
                let max_str_size = std::cmp::max(((sp2 - sep)*width/6.0) as usize, 2) - 2;
                move |y, mut str: String| {
                    if str.len() > max_str_size {
                        str.truncate(max_str_size);
                        str.push('.');
                        str.push('.');
                    }
                    canvas::Text {
                        content: str,
                        position: Point::new(start + sep*width + 2., y),
                        ..Default::default()
                    }
                }
            }).collect();
            
            frame.fill_text(write[0](start, String::from("PID")));
            frame.fill_text(write[1](start, String::from("Nice")));
            frame.fill_text(write[2](start, String::from("Status")));
            frame.fill_text(write[3](start, String::from("User")));
            frame.fill_text(write[4](start, String::from("CPU%")));
            frame.fill_text(write[5](start, String::from("MEM%")));
            frame.fill_text(write[6](start, String::from("Command")));
            for (i, info) in self.process_info.iter().enumerate() {
                let y = start + ((i + 1) as f32) * line_height;
                if y > start + height - line_height {
                    break;
                }
                frame.fill_text(write[0](y, format!("{}", info.pid)));
                frame.fill_text(write[1](y, info.nice.to_string()));
                frame.fill_text(write[2](y, info.status.to_string()));
                frame.fill_text(write[3](y, info.user.to_string()));
                frame.fill_text(write[4](y, format!("{}", info.cpu)));
                frame.fill_text(write[5](y, format!("{}", info.memory)));
                frame.fill_text(write[6](y, info.cmd.to_string()));
            }
    
            vec![frame.into_geometry()]
        }
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