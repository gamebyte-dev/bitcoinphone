use sv::peer::{Peer, SVPeerFilter};
use std::net::{IpAddr, Ipv6Addr};
use sv::network::Network;
use sv::messages::{Version, NodeAddr, Message, FilterAdd, FilterLoad};
use std::sync::Arc;
use sv::util::BloomFilter;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use crate::net::peerman::message_handler::MessageHandler;
use lazy_static::lazy_static;
use sv::util::rx::Observable;
use std::iter::Filter;

lazy_static!{
    static ref PEER_VERSION: Version = Version{
        version: 70015,
        services: 0,
        timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as i64,
        recv_addr: NodeAddr{
            services: 0,
            ip: Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 1),
            port: 8333,
        },
        tx_addr: NodeAddr{
            services: 0,
            ip: Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 1),
            port: 8333,
        },
        nonce: 0,
        user_agent: "/BitcoinPhone/".into(),
        start_height: 0,
        relay: false,
    };
}
pub struct PPeer{
    pub(crate) sv_peer: Arc<Peer>,
}

impl PPeer {
    pub(crate) fn new(ip: IpAddr, port: u16, handler: Arc<MessageHandler>, bloom_filter: BloomFilter) -> Option<Self> {
        let sv_peer = Peer::connect(
            ip,
            port,
            Network::Mainnet,
            PEER_VERSION.clone(),
            Arc::from(SVPeerFilter{
                min_start_height: 0
            })
        );

        sv_peer.connected_event()
            .poll_timeout(Duration::new(2,0));

        if !sv_peer.connected() {
            return None;
        }
        println!("Connected to peer {}:{}!", ip, port);
        sv_peer.send(&Message::FilterLoad(FilterLoad{
            bloom_filter,
            flags: 0
        }));

        sv_peer
            .messages()
            .subscribe(&handler.clone());

        sv_peer
            .send(&Message::GetAddr);

        return Some(PPeer{
            sv_peer
        });
    }

    pub fn get_id(&self) -> String {
        return Self::get_id_from_peer(self.sv_peer.clone());
    }

    pub fn get_id_from_peer(peer: Arc<Peer>) -> String {
        return format!("{}:{}", peer.ip.to_string(), peer.port.to_string());
    }

    pub fn send(&self, msg: &Message) {
        self.sv_peer.send(msg)
            .unwrap();
    }
}
