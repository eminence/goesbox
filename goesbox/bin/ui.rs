//! A text-based user interface for the goesbox.

use goeslib::lrit::{VirtualChannel, VCDU};
use goeslib::stats::{Stat, Stats};
use goeslib::{handlers, lrit};
use log::warn;
use nanomsg::{Protocol, Socket};
use tui::text::{Span, Spans};

use std::io;
use termion::event::Key;
use termion::raw::IntoRawMode;
use tui::backend::{Backend, TermionBackend};
use tui::layout::{Constraint, Direction, Layout, Rect};
use tui::widgets::{BarChart, Block, Borders, Paragraph, Widget, Wrap};
use tui::{Frame, Terminal};

use crossbeam_channel::unbounded;
use crossbeam_channel::{select, Sender};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::time::{Duration, Instant};

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
    pub fn process(&mut self, vcdu: lrit::VCDU) -> Vec<lrit::LRIT> {
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

        let widget = BarChart::default()
            .data(&d)
            .bar_width(4)
            .bar_gap(1)
            .max(60)
            .block(Block::default().borders(Borders::ALL).title("VCDU receive rates (pps)"));
        f.render_widget(widget, area)
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

        let msg: Vec<Spans> = self
            .messages
            .iter()
            .skip(to_skip)
            .map(|m| {
                Spans::from(vec![Span::raw({
                    let mut s = m.clone();
                    s.push('\n');
                    s
                })])
            })
            .collect();

        let widget = Paragraph::new(msg)
            .wrap(Wrap { trim: true })
            .block(Block::default().borders(Borders::ALL).title("Messages"));
        f.render_widget(widget, area);
    }
}

pub fn set_panic_handler() {
    let old_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        // log this panic to disk:
        if let Ok(mut file) = std::fs::OpenOptions::new()
            .write(true)
            .append(true)
            .create(true)
            .truncate(false)
            .open("panic.log")
        {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0);

            let _ = writeln!(file, "======");
            let _ = writeln!(file, "Panic! {}", now);
            let payload = info.payload();
            if let Some(m) = payload.downcast_ref::<&str>() {
                let _ = writeln!(file, "{}", m);
            } else if let Some(m) = payload.downcast_ref::<String>() {
                let _ = writeln!(file, "{}", m);
            } else {
                let _ = writeln!(file, "{:?}", payload);
            }

            if let Some(loc) = info.location() {
                let _ = writeln!(file, "Location: {}", loc);
            }
        }
        old_hook(info)
    }));
}

fn main() -> Result<(), io::Error> {
    set_panic_handler();

    let target: String = std::env::args().nth(1).unwrap_or("tcp://127.0.0.1:5004".to_owned());

    let stdout = io::stdout().into_raw_mode()?;
    let backend = TermionBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.clear();

    // channels for messaging
    let (s, log_receiver) = unbounded();
    let logger = AppLogger::new(s);
    log::set_boxed_logger(Box::new(logger));
    log::set_max_level(log::LevelFilter::Debug);

    let mut app = App::new();

    let mut sock = Socket::new(Protocol::Sub).expect("socket::new");
    sock.connect(&target).expect("sock.bind");
    sock.subscribe(b"").expect("sock.subscribe");
    log::info!("Connected and subscribed to {}", target);

    // all network receiving will happen in a new thread, and will send VCDU packets
    // to the main thread via a channel
    let (s, net) = unbounded();
    std::thread::spawn(move || {
        let mut buf = Vec::new();

        loop {
            buf.truncate(0);
            let num_bytes_read = sock.read_to_end(&mut buf).expect("sock.read");
            //println!("bytes read: {}", num_bytes_read);
            if num_bytes_read != 892 {
                eprintln!("Read a packet that wasn't 892 bytes!");
                return;
            }
            s.send(buf[..num_bytes_read].to_owned());
        }
    });

    // spawn a thread to handle keyboard input
    let (s, kbd) = unbounded();
    std::thread::spawn(move || {
        use termion::input::TermRead;
        let stdin = io::stdin();
        for evt in stdin.keys() {
            s.send(evt.unwrap());
        }
    });

    let mut handlers: Vec<Box<dyn handlers::Handler>> = Vec::new();
    handlers.push(Box::new(handlers::TextHandler::new()));
    handlers.push(Box::new(handlers::ImageHandler::new()));
    handlers.push(Box::new(handlers::DcsHandler::new()));
    handlers.push(Box::new(handlers::DebugHandler::new()));

    loop {
        select! {
            recv(kbd) -> msg => {
                let msg = msg.unwrap();
                if msg == Key::Esc || msg == Key::Char('q')  || msg == Key::Ctrl('c') {
                    break;
                } else if msg == Key::Char('c') {
                    app.clear_msg();
                    app.draw(&mut terminal);
                } else {
                    log::info!("got kbd {:?}", msg);
                }

            },
            recv(net) -> data => {
                let data = data.unwrap();
                let vcdu = VCDU::new(&data[..892]);

                for lrit in app.process(vcdu) {
                    for handler in &mut handlers {
                        match handler.handle(&lrit) {
                            Ok(()) => {},
                            Err(handlers::HandlerError::Skipped) => {},
                            Err(e) => {
                                warn!("Handler failed: {:?}", e);
                            }
                        }
                    }
                    let code = lrit.headers.primary.filetype_code ;
                    if code != 0 && code != 2 && code != 130 {
                        log::info!("{:?}", lrit.headers);
                    }
                }
                app.draw(&mut terminal);
            },
            recv(log_receiver) -> data => {
                let data = data.unwrap();
                app.info(data);
                app.draw(&mut terminal);
            },
            default(Duration::from_millis(100)) => {
                app.draw(&mut terminal);
            }

        };
    }

    //loop {

    //    app.record(ui::Stat::Packet);

    //    app.record(ui::Stat::VCDUPacket(vcdu.VCID()));

    //    if vcdu.is_fill() {
    //        continue;
    //    }
    //    let id = vcdu.VCID();
    //    //println!("VCID {}", id);
    //    let vc = vcs.entry(id).or_insert_with(|| VirtualChannel::new(id));
    //    vc.process_vcdu(vcdu, &mut app);
    //    //println!("VCDU: SCID={}", vcdu.SCID());
    //    //println!("      VCID={}", vcdu.VCID());
    //    //println!("      cntr={}", vcdu.counter());

    //    app.draw(&mut terminal);

    //}

    Ok(())
}
