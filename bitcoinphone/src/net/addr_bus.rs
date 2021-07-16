use std::net::IpAddr;
use sv::messages::Addr;
use std::sync::mpsc::{SyncSender, sync_channel};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

pub(crate) type IpTuple = (IpAddr, u16);

pub enum AddressAction {
    Add,
    Remove
}

pub struct AddrOp(pub Vec<IpTuple>, pub AddressAction);

pub enum AddrEvent{
    Op(AddrOp),
    AddrMessage(Addr)
}

pub struct AddrBus {
    peer_db_sender: SyncSender<AddrOp>
}

impl AddrBus {
    pub fn new(peer_db_sender: SyncSender<AddrOp>) -> SyncSender<AddrEvent> {
        let bus = Arc::from(AddrBus {
            peer_db_sender
        });

        return bus.spawn();
    }

    fn spawn(self: Arc<AddrBus>) -> SyncSender<AddrEvent> {
        let (sender, receiver) = sync_channel(100);
        std::thread::spawn(move || loop {
            let this = self.clone();
            match receiver.recv() {
                Ok(AddrEvent::Op(operation)) => this.peer_db_sender.send(operation).unwrap(),
                Ok(AddrEvent::AddrMessage(addr)) => {
                    let ip_tuples = this.clone().process_addrs(addr);
                    this.peer_db_sender
                        .send(AddrOp(ip_tuples, AddressAction::Add))
                        .unwrap();
                }
                _ => {
                    panic!("Invalid message passed to AddrBus!");
                }
            }
        });

        return sender;
    }

    pub fn process_addrs(self: Arc<AddrBus>, addr: Addr) -> Vec<IpTuple> {
        let two_hours_ago = SystemTime::now()
            .duration_since(UNIX_EPOCH).unwrap().as_secs() - 7200;
        let mut ip_tuples = vec![];

        for addr in &addr.addrs {
            if addr.last_connected_time > (two_hours_ago as u32) {
                continue;
            }
            ip_tuples.push((IpAddr::from(addr.addr.ip), addr.addr.port));
        }

        return ip_tuples;
    }
}