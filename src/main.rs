#![allow(unused_imports)]

use nanomsg::{Protocol, Socket};

use std::io;
use termion::event::Key;
use termion::raw::IntoRawMode;
use tui::backend::TermionBackend;
use tui::layout::{Constraint, Direction, Layout};
use tui::widgets::{BarChart, Block, Borders, Paragraph, Text, Widget};
use tui::Terminal;

use crossbeam_channel::select;
use crossbeam_channel::unbounded;
use std::collections::HashMap;
use std::io::{Read, Write};
use std::time::{Duration, Instant};

mod ui;
use ui::{App, Stat, Stats};

mod lrit;
use lrit::*;

mod crc;
use crc::calc_crc;

pub mod handlers;

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
            let _ = writeln!(file, "======");
            let _ = writeln!(file, "Panic!");
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

    let target: String = std::env::args()
        .nth(1)
        .unwrap_or("tcp://127.0.0.1:5004".to_owned());

    let stdout = io::stdout().into_raw_mode()?;
    let backend = TermionBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.clear();

    // channels for messaging
    let (s, log_receiver) = unbounded();
    let logger = ui::AppLogger::new(s);
    log::set_boxed_logger(Box::new(logger));
    log::set_max_level(log::LevelFilter::Debug);

    let mut app = ui::App::new();

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

    let mut handlers: Vec<Box<handlers::Handler>> = Vec::new();
    handlers.push(Box::new(handlers::TextHandler::new()));
    handlers.push(Box::new(handlers::ImageHandler::new()));
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
                    for mut handler in &mut handlers {
                        handler.handle(&lrit)
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
