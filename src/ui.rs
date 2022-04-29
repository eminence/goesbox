use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::collections::VecDeque;
use std::time::{Duration, Instant};

use crossbeam_channel::Sender;
use std::io;
use termion::raw::IntoRawMode;
use tui::backend::Backend;
use tui::layout::{Constraint, Corner, Direction, Layout, Rect};
use tui::widgets::{BarChart, Block, Borders, List, Paragraph, Text, Widget};
use tui::Frame;
use tui::Terminal;

use crate::lrit::*;

const MIN_DRAW_INTERVAL: Duration = Duration::from_millis(100);

pub enum Stat {
    Packet,
    /// A packet for a specific vcid
    VCDUPacket(u8),
    Bytes(usize),
    /// A VCDU with vcid=63
    FillPacket,
    /// A packet full of TP_PDU data, but we had no previous header for it
    DiscardedDataPacket,

    /// A packet for a specific APID
    APID(u16),
}

pub struct Stats {
    time: Instant,
    packets: usize,
    bytes: usize,
    fills: usize,
    discards: usize,
    vcdu_packets: VecDeque<(Instant, HashMap<u8, usize>)>,
    //vcdu_packets: HashMap<u8, usize>,
    apid: HashMap<u16, usize>,
}

impl Stats {
    fn new() -> Stats {
        Stats {
            time: Instant::now(),
            packets: 0,
            bytes: 0,
            fills: 0,
            discards: 0,
            vcdu_packets: VecDeque::new(),
            apid: HashMap::new(),
        }
    }
    pub fn record(&mut self, stat: Stat) {
        match stat {
            Stat::Packet => self.packets += 1,
            Stat::Bytes(b) => self.bytes += b,
            Stat::FillPacket => self.fills += 1,
            Stat::DiscardedDataPacket => self.discards += 1,
            Stat::VCDUPacket(id) => {
                // if the first bucket in vcdu_packets is less than 1 second old, use it
                // else, push a new bucket on the front
                if let Some((inst, map)) = self.vcdu_packets.front_mut() {
                    if inst.elapsed() < Duration::from_secs(1) {
                        *map.entry(id).or_insert(0) += 1;
                        return;
                    }
                }

                self.vcdu_packets.push_front((Instant::now(), {
                    let mut map = HashMap::new();
                    map.insert(id, 1);
                    map
                }));
            }
            Stat::APID(id) => *self.apid.entry(id).or_insert(0) += 1,
        }
    }

    fn print(&self) {
        let secs = self.time.elapsed().as_millis() as f32 / 1000.0;
        println!("==============");
        println!("Total packets: {:0.2} pps", self.packets as f32 / secs);
        println!("Discards: {:0.2} pps", self.discards as f32 / secs);
        println!("VC stats:");
        //for (vcid, count) in self.vcdu_packets.iter() {
        //    println!("  VC {}: {:0.2} pps", vcid, *count as f32 / secs);
        //}
        //println!("APID stats:");
        //for (id, count) in self.apid.iter() {
        //    println!("  APID {}: {:0.2} pps", id, *count as f32 / secs);
        //}
    }

    fn reset(&mut self) {
        self.time = Instant::now();
        self.packets = 0;
        self.bytes = 0;
        self.fills = 0;
        self.discards = 0;
        //self.vcdu_packets = HashMap::new();
    }
}

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
        if record.level() >= log::Level::Debug {
            return;
        }
        self.app_channel.send(format!(
            "{} {} {}",
            record.target(),
            record.level(),
            record.args()
        ));
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

    pub fn process(&mut self, vcdu: crate::lrit::VCDU) -> Vec<LRIT> {
        let id = vcdu.VCID();
        self.record(Stat::Packet);
        self.record(Stat::VCDUPacket(id));
        if vcdu.is_fill() {
            return Vec::new();
        }
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
                .constraints(
                    [
                        Constraint::Percentage(10),
                        Constraint::Length(10),
                        Constraint::Min(20),
                    ]
                    .as_ref(),
                )
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
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("VCDU receive rates (pps)"),
            )
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
