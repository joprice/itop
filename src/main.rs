mod event;
use crate::event::{Event, Events};
use itertools::Itertools;
use std::io;
use std::time::Duration;
use sysinfo::{ProcessExt, ProcessorExt, SystemExt};
use termion::event::Key;
use termion::input::MouseTerminal;
use termion::raw::IntoRawMode;
use termion::screen::AlternateScreen;
use tui::backend::{Backend, TermionBackend};
use tui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use tui::style::{Color, Modifier, Style};
use tui::widgets::{
    Block, Borders, Paragraph, RenderDirection, Row, Sparkline, Table, Text, Widget,
};
use tui::{Frame, Terminal};

struct ProcessMeta<'a> {
    name: &'a str,
    cpu_usage: f32,
    memory: u64,
    count: usize,
}

fn get_processes(system: &sysinfo::System) -> Vec<ProcessMeta> {
    let mut processes = system
        .get_process_list()
        .values()
        .map(|process| (process.name(), process))
        .into_group_map()
        .into_iter()
        .map(|(name, group)| {
            let mut cpu_usage = 0f32;
            let mut memory = 0u64;
            for process in &group {
                cpu_usage += process.cpu_usage();
                memory += process.memory();
            }
            ProcessMeta {
                name,
                cpu_usage,
                memory,
                count: group.len(),
            }
        })
        .collect::<Vec<_>>();
    processes.sort_by_key(|p| std::cmp::Reverse((p.cpu_usage * 100f32) as u32));
    processes
        .into_iter()
        //take enough for a reasonably large screen size
        .take(100)
        .collect()
}

fn draw_processes(mut f: &mut Frame<impl Backend>, app: &App, parent: Rect) {
    static HEADERS: [&str; 4] = [" Command", "CPU %", "Count", "Memory %"];
    let process_list = Block::default()
        .title(" Process List ")
        .border_style(Style::default().fg(Color::Cyan))
        .borders(Borders::ALL);
    let processes = app
        .processes
        .iter()
        .map(|item: &Vec<String>| Row::Data(item.iter()));
    Table::new(HEADERS.iter(), processes)
        .header_style(Style::default().modifier(Modifier::BOLD))
        .widths(&[
            Constraint::Percentage(50),
            Constraint::Percentage(12),
            Constraint::Percentage(12),
            Constraint::Percentage(26),
        ])
        .block(process_list)
        .render(&mut f, parent);
}

fn draw_memory(mut f: &mut Frame<impl Backend>, app: &App, parent: Rect) {
    let memory = Block::default()
        .title(" Memory Usage ")
        .border_style(Style::default().fg(Color::Cyan))
        .borders(Borders::ALL);

    Sparkline::default()
        .direction(RenderDirection::RTL)
        .data(&app.memory)
        .max(100)
        .block(memory)
        .render(&mut f, parent);
}

fn draw_cpu(mut f: &mut Frame<impl Backend>, app: &App, parent: Rect) {
    let cpu = Block::default()
        .title(" CPU Usage ")
        .border_style(Style::default().fg(Color::Cyan))
        .borders(Borders::ALL);

    Sparkline::default()
        .direction(RenderDirection::RTL)
        .data(&app.cpu)
        .style(Style::default().fg(Color::Red))
        .max(100)
        .block(cpu)
        .render(&mut f, parent);
}

fn draw_header(mut f: &mut Frame<impl Backend>, app: &App, parent: Rect) {
    let top = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(
            [
                Constraint::Percentage(40),
                Constraint::Percentage(30),
                Constraint::Percentage(30),
            ]
            .as_ref(),
        )
        .split(parent);
    let mut title = vec![Text::styled(
        &app.title,
        Style::default().fg(Color::Blue).modifier(Modifier::BOLD),
    )];
    if let Some(hostname) = &app.hostname {
        title.push(Text::raw(format!(" for {}", hostname)));
    }
    Paragraph::new(title.iter()).render(&mut f, top[0]);

    if let Ok(load) = sys_info::loadavg() {
        let load = format!(
            "Load Average: {:.2} {:.2} {:.2}",
            load.one, load.five, load.fifteen
        );
        Block::default().title(&load).render(&mut f, top[1]);
    }

    let date = chrono::Local::now();
    let time = date.format("%H:%M:%S").to_string();

    Paragraph::new([Text::raw(time)].iter())
        .alignment(Alignment::Right)
        .render(&mut f, top[2]);
}

struct App {
    memory: slice_deque::SliceDeque<u64>,
    cpu: slice_deque::SliceDeque<u64>,
    processes: Vec<Vec<String>>,
    system: sysinfo::System,
    title: String,
    hostname: Option<String>,
}

const BUFFER_CAPACITY: usize = 1000;

impl App {
    fn update(&mut self, processes: bool) {
        if processes {
            self.update_processes();
        }
        self.update_memory();
        self.update_cpu();
    }

    fn update_cpu(&mut self) {
        self.system.refresh_cpu();
        let processors = self.system.get_processor_list();
        let total: f32 = processors.iter().map(|p| p.get_cpu_usage()).sum();
        let cpu_percentage = (total / (processors.len() as f32) * 100f32) as u64;
        self.cpu.push_front(cpu_percentage);
        if self.cpu.len() > BUFFER_CAPACITY {
            self.cpu.pop_back();
        }
    }

    fn update_memory(&mut self) {
        self.system.refresh_memory();
        let used = self.system.get_used_memory() as f64;
        let total = self.system.get_total_memory() as f64;
        let memory_percentage = (used / total * 100f64) as u64;
        self.memory.push_front(memory_percentage);
        if self.memory.len() > BUFFER_CAPACITY {
            self.memory.pop_back();
        }
    }

    fn update_processes(&mut self) {
        self.system.refresh_processes();
        let processes = get_processes(&self.system);
        let total_memory = self.system.get_total_memory();
        std::mem::replace(
            &mut self.processes,
            processes
                .into_iter()
                .map(
                    |ProcessMeta {
                         name,
                         cpu_usage,
                         memory,
                         count,
                     }| {
                        vec![
                            format!(" {}", name),
                            format!("{:.2}", cpu_usage),
                            format!("{}", count),
                            format!("{:.2}", (memory as f64 / total_memory as f64) * 100f64),
                        ]
                    },
                )
                .collect::<Vec<_>>(),
        );
    }
}

fn main() -> Result<(), failure::Error> {
    let stdout = io::stdout().into_raw_mode()?;
    let stdout = MouseTerminal::from(stdout);
    let stdout = AlternateScreen::from(stdout);
    let backend = TermionBackend::new(stdout);
    let events = Events::with_config(event::Config {
        exit_key: Key::Char('q'),
        tick_rate: Duration::from_millis(300),
    });
    let mut terminal = Terminal::new(backend)?;
    let mut app = App {
        memory: slice_deque::SliceDeque::new(),
        cpu: slice_deque::SliceDeque::new(),
        processes: vec![],
        system: sysinfo::System::new(),
        hostname: sys_info::hostname().ok(),
        title: "itop".to_owned(),
    };

    let mut i = 0;
    loop {
        terminal.draw(|mut f| {
            let outer = Layout::default()
                .direction(Direction::Vertical)
                .constraints(
                    [
                        Constraint::Percentage(5),
                        Constraint::Percentage(47),
                        Constraint::Percentage(47),
                    ]
                    .as_ref(),
                )
                .split(f.size());

            draw_header(&mut f, &app, outer[0]);
            draw_cpu(&mut f, &app, outer[1]);

            let bottom = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
                .split(outer[2]);

            draw_memory(&mut f, &app, bottom[0]);
            draw_processes(&mut f, &app, bottom[1]);
        })?;
        match events.next()? {
            Event::Input(_) => {
                break;
            }
            Event::Tick => {
                // refreshing processes is expensive, so do it less frequently
                let update_processes = i % 8 == 0;
                app.update(update_processes);
                i += 1;
            }
        }
    }

    Ok(())
}
