use std::sync::{Arc, RwLock, Mutex};
use std::sync::mpsc::SyncSender;
use sv::util::Hash256;
use sv::messages::{Tx, Message, INV_VECT_TX, InvVect, Inv};
use sv::peer::{PeerDisconnected, PeerMessage};
use sv::util::rx::Observer;
use crate::net::tx_bus::TxEvent;
use crate::net::addr_bus::{AddrEvent};
use lru_cache::LruCache;
use std::collections::HashMap;

pub struct MessageHandler {
    addr_bus: SyncSender<AddrEvent>,
    tx_bus: SyncSender<TxEvent>,

    out_cache: Arc<RwLock<HashMap<Hash256, Tx>>>,
    waiting_cache: Arc<Mutex<LruCache<Hash256, ()>>>,
}

impl MessageHandler {
    pub fn new(
        addr_bus: SyncSender<AddrEvent>,
        tx_bus: SyncSender<TxEvent>
    ) -> Arc<MessageHandler> {
        return Arc::from(MessageHandler{
            addr_bus,
            tx_bus,
            out_cache: Arc::new(RwLock::new(HashMap::new())),
            waiting_cache: Arc::new(Mutex::new(LruCache::new(1000)))
        })
    }

    pub fn send(self: Arc<Self>, tx: Tx) {
        self.out_cache
            .write()
            .unwrap()
            .insert(tx.hash(), tx);
    }
}

impl Observer<PeerMessage> for MessageHandler {
    fn next(&self, event: &PeerMessage) {
        match &event.message {
            Message::Addr(addr) => {
                self.addr_bus
                    .send(AddrEvent::AddrMessage(addr.clone()))
                    .unwrap();
            }
            Message::Tx(tx) => {
                println!("Got tx!");
                self.tx_bus
                    .send(TxEvent::RawTx(tx.clone()))
                    .unwrap();
            }
            Message::Inv(inv) => {
                let mut waiting = self.waiting_cache.lock().unwrap();

                let inv_vects = inv.objects.iter()
                    .filter_map(|x| if x.obj_type == INV_VECT_TX
                        && !waiting.contains_key(&x.hash) {
                        waiting.insert(x.clone().hash, ());
                        Some(x.clone())
                    } else {
                        None
                    })
                    .collect::<Vec<InvVect>>();

                event.peer.send(&Message::GetData(Inv {
                    objects: inv_vects
                }));
            }
            Message::GetData(inv) => {
                for object in &inv.objects {
                    match self.out_cache.read().unwrap().get(&object.hash) {
                        Some(tx) => {
                            event.peer.send(&Message::Tx(tx.clone()));
                        },
                        None => {}
                    }
                }
            },
            _ => {}
        }
    }
}


