use std::{
    collections::{HashMap, VecDeque},
    time::{Duration, Instant},
};

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
    pub vcdu_packets: VecDeque<(Instant, HashMap<u8, usize>)>,
    //vcdu_packets: HashMap<u8, usize>,
    apid: HashMap<u16, usize>,
}

impl Stats {
    pub fn new() -> Stats {
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
