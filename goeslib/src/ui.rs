use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::collections::VecDeque;
use std::time::{Duration, Instant};

use crossbeam_channel::Sender;
use goeslib::lrit::{self, VirtualChannel, LRIT};
use goeslib::stats::{Stat, Stats};
use std::io;
use termion::raw::IntoRawMode;
use tui::backend::Backend;
use tui::layout::{Constraint, Corner, Direction, Layout, Rect};
use tui::widgets::{BarChart, Block, Borders, List, Paragraph, Text, Widget};
use tui::Frame;
use tui::Terminal;

const MIN_DRAW_INTERVAL: Duration = Duration::from_millis(100);

pub struct App {
    pub stats: Stats,
    messages: Vec<String>,
    last_draw: Instant,
    vcs: HashMap<u8, VirtualChannel>,
}

pub struct AppLogger {
    app_channel: Sender<String>,
}

impl AppLogger {
    pub fn new(chan: Sender<String>) -> AppLogger {
        AppLogger { app_channel: chan }
    }
}

impl log::Log for AppLogger {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        true
    }

    fn log(&self, record: &log::Record) {
        if !record.target().starts_with("goes_dht") && record.level() >= log::Level::Debug {
            return;
        }
        let _ = self
            .app_channel
            .send(format!("{} {} {}", record.target(), record.level(), record.args()));
    }

    fn flush(&self) {}
}

impl App {
    pub fn new() -> App {
        App {
            stats: Stats::new(),
            messages: Vec::new(),
            last_draw: Instant::now(),
            vcs: HashMap::new(),
        }
    }

    /// Process an incoming VCDU packet, and return any completed LRIT files (if any)
    pub fn process(&mut self, vcdu: lrit::VCDU) -> Vec<LRIT> {
        let id = vcdu.VCID();
        self.record(Stat::Packet);
        self.record(Stat::VCDUPacket(id));
        if vcdu.is_fill() {
            return Vec::new();
        }
        // Each VCDU needs to be processed by the corresponding VirtualChannel
        let vc = self
            .vcs
            .entry(id)
            .or_insert_with(|| VirtualChannel::new(id, vcdu.counter()));
        vc.process_vcdu(vcdu, &mut self.stats)
    }

    pub fn record(&mut self, stat: Stat) {
        self.stats.record(stat);
    }

    pub fn info(&mut self, msg: impl ToString) {
        self.messages.push(msg.to_string());

        self.trim_messages();
    }

    pub fn clear_msg(&mut self) {
        self.messages.clear();
    }

    fn trim_messages(&mut self) {
        // keep only the most recent messages
        let len = self.messages.len();
        if len > 200 {
            self.messages = self.messages.split_off(len - 200);
        }
    }

    pub fn draw<B: Backend>(&mut self, terminal: &mut Terminal<B>) {
        if self.last_draw.elapsed() <= MIN_DRAW_INTERVAL {
            return;
        }
        terminal.draw(|mut f| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Percentage(10), Constraint::Length(10), Constraint::Min(20)].as_ref())
                .split(f.size());

            self.draw_stats(&mut f, chunks[1]);
            self.draw_messages(&mut f, chunks[2]);
        });
        self.last_draw = Instant::now();
    }

    fn draw_stats<B>(&mut self, f: &mut Frame<B>, area: Rect)
    where
        B: Backend,
    {
        let dursec = 10;
        let duration = Duration::from_secs(dursec);

        let mut total_map = HashMap::new();
        for (inst, map) in &self.stats.vcdu_packets {
            if inst.elapsed() > duration {
                continue;
            }
            for (id, count) in map {
                *total_map.entry(id).or_insert(0) += count;
            }
        }

        let mut sorted = total_map.into_iter().collect::<Vec<_>>();
        sorted.sort_by_key(|(&k, _)| k);
        let d: Vec<(String, u64)> = sorted
            .into_iter()
            .map(|(k, v)| (format!("VC{:02}", k), (v as u64 / dursec) as u64))
            .collect();
        let d: Vec<(&str, u64)> = d.iter().map(|(a, b)| (a.as_ref(), *b)).collect();

        BarChart::default()
            .data(&d)
            .bar_width(4)
            .bar_gap(1)
            .max(60)
            .block(Block::default().borders(Borders::ALL).title("VCDU receive rates (pps)"))
            .render(f, area);
    }

    fn draw_messages<B>(&self, f: &mut Frame<B>, area: Rect)
    where
        B: Backend,
    {
        // 1 message, hight 5, skip max(-4, 0) skip 0
        // 6 messages, height 5, skip max(1, 0) skip 1
        let h = (area.height - 2) as usize;
        let to_skip = if self.messages.len() > h {
            (self.messages.len() - h) as usize
        } else {
            0
        };

        let msg: Vec<_> = self
            .messages
            .iter()
            .skip(to_skip)
            .map(|m| {
                Text::raw({
                    let mut s = m.clone();
                    s.push('\n');
                    s
                })
            })
            .collect();

        Paragraph::new(msg.iter().map(|m| m))
            .wrap(true)
            .block(Block::default().borders(Borders::ALL).title("Messages"))
            .render(f, area);
    }
}
