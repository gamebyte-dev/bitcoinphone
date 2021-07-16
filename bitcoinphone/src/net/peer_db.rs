use std::fs::{File, OpenOptions};
use std::net::{IpAddr, Ipv6Addr, Ipv4Addr};
use std::io::{BufReader, BufRead, Write, Lines};
use std::str::FromStr;
use sv::network::Network;
use std::sync::mpsc::{Sender, Receiver, SyncSender, TryRecvError, sync_channel};
use std::sync::{mpsc, Arc, Mutex};
use std::{time, thread};
use pickledb::{PickleDb, PickleDbDumpPolicy, SerializationMethod, PickleDbIterator, PickleDbIteratorItem};
use std::ops::{SubAssign, AddAssign};
use std::time::Duration;
use std::collections::HashSet;
use crate::net::tx_bus::TxEvent;
use crate::net::addr_bus::{AddrOp, AddressAction};
use crate::util::traits::Spawnable;

pub const MAX_PEERS: usize = 1000;
pub type IpTuple = (IpAddr, u16);
pub type SafePeerDB = Arc<Mutex<PeerDB>>;

pub const PEERS_FILE: &'static str = "peers.txt";
pub const GOOD_PEER: &'static str = "good";
pub const BAD_PEER: &'static str = "bad";

pub struct PeerDB {
    db: PickleDb,
    seen_map: HashSet<String>
}

impl PeerDB {
    pub fn new() -> SafePeerDB {
        let db = Self::open_create(PEERS_FILE);

        let mut peer_db = PeerDB{
            db,
            seen_map: HashSet::new()
        };

        peer_db.seed();

        return Arc::from(Mutex::new(peer_db));
    }

    fn seed(&mut self) {
        for (ip, port) in Network::Mainnet.seed_iter() {
            self.add(ip, port);
        }
        self.db.dump();
    }

    pub fn remove(&mut self, ip: IpAddr, port: u16) {
        let key = format!("{}:{}", ip.to_string(), port.to_string());
        self.db.set(&key, &BAD_PEER.to_string());
    }

    pub fn get(&mut self) -> Option<IpTuple> {
        for (index, item) in self.db.iter().enumerate() {
            if self.seen_map.contains(&item.get_key().to_string()) {
                continue;
            }

            if &item.get_value::<String>().unwrap() == BAD_PEER {
                continue;
            } else {
                self.seen_map.insert(item.get_key().to_string());
                return Some(Self::parse_key(item.get_key()))
            }
        }
        return None;
    }

    pub fn parse_key(key: &str) -> IpTuple {
        let mut key = key.split(":").collect::<Vec<&str>>();
        let port = u16::from_str(key.pop().expect("Bad key, no port!"))
            .expect("could not parse port");
        let ip = key.join(":");

        return (Self::parse_ip(ip), port);
    }

    pub fn add(&mut self, ip: IpAddr, port: u16) {
        let key = &format!("{}:{}", ip.to_string(), port.to_string())[..];

        if self.db.exists(key) {
            return;
        }

        self.db.set(key, &GOOD_PEER.to_string());
    }

    fn open_create(file_name: &str) -> PickleDb {
        return match PickleDb::load(file_name, PickleDbDumpPolicy::DumpUponRequest, SerializationMethod::Yaml) {
            Ok(db) => db,
            Err(_) => PickleDb::new(file_name, PickleDbDumpPolicy::DumpUponRequest, SerializationMethod::Yaml)
        };
    }

    fn parse_ip(ip: String) -> IpAddr {
        return Ipv6Addr::from_str(&ip)
            .map(|ip_result| IpAddr::from(ip_result))
            .unwrap_or_else(|_err| IpAddr::from(
                Ipv4Addr::from_str(&ip)
                    .expect(&format!("Unable to parse IP!! {}", ip))));
    }
}


impl Spawnable<AddrOp> for Arc<Mutex<PeerDB>> {
    fn spawn_gateway(&self) -> SyncSender<AddrOp> {
        let (tx, rx): (SyncSender<AddrOp>, Receiver<AddrOp>) = sync_channel(100);
        let peer_db = self.clone();

        thread::spawn(move || loop {
            match rx.recv() {
                Ok(AddrOp(addresses, action)) => {
                    let mut this = peer_db
                        .lock()
                        .unwrap();

                    match action {
                        AddressAction::Add => {
                            addresses.into_iter().for_each(|(ip, port)| {
                                this.add(ip, port);
                            });
                        },
                        AddressAction::Remove => {
                            addresses.into_iter().for_each(|(ip, port)| {
                                this.remove(ip, port);
                            });
                        }
                    }

                    this.db.dump();
                },
                Err(e) => panic!("Got error in address command!")
            }
        });

        return tx;
    }
}