use std::str::FromStr;
use async_std::{io, task};
use futures::channel::mpsc;
use futures::future::join;
use futures::prelude::*;
use libipld_core::cid::Cid;
use libipld_core::cid::Codec;
use libipld_core::multihash::Sha2_256;
use libp2p::core::muxing::StreamMuxerBox;
use libp2p::core::transport::boxed::Boxed;
use libp2p::core::transport::upgrade::Version;
use libp2p::identity::Keypair;
use libp2p::kad::record::store::{Error as RecordError, MemoryStore};
use libp2p::kad::record::Key;
use libp2p::kad::{
    BootstrapError, BootstrapOk, GetProvidersOk, Kademlia, KademliaEvent, QueryId, QueryResult,
};
use libp2p::mdns::service::{MdnsPacket, MdnsService};
use libp2p::secio::SecioConfig;
use libp2p::tcp::TcpConfig;
use libp2p::yamux::Config as YamuxConfig;
use libp2p::{Multiaddr, PeerId, Swarm, Transport};
use libp2p_bitswap::{Bitswap, BitswapEvent};
use std::io::{Error, ErrorKind};
use std::task::{Context, Poll};
use std::time::Duration;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Block {
    pub cid: Cid,
    pub data: Box<[u8]>,
}

impl Block {
    pub fn new(data: Box<[u8]>, cid: Cid) -> Self {
        Self { cid, data }
    }

    pub fn cid(&self) -> &Cid {
        &self.cid
    }

    pub fn data(&self) -> &[u8] {
        &self.data
    }
}

fn new_block(bytes: &[u8]) -> Block {
    let digest = Sha2_256::digest(bytes);
    let cid = Cid::new_v1(Codec::Raw, digest);
    Block::new(bytes.to_vec().into_boxed_slice(), cid)
}

fn mk_transport() -> (PeerId, Boxed<(PeerId, StreamMuxerBox), Error>) {
    let key = Keypair::generate_ed25519();
    let peer_id = key.public().into_peer_id();
    let transport = TcpConfig::new()
        .nodelay(true)
        .upgrade(Version::V1)
        .authenticate(SecioConfig::new(key))
        .multiplex(YamuxConfig::default())
        .timeout(Duration::from_secs(20))
        .map(|(peer_id, muxer), _| (peer_id, StreamMuxerBox::new(muxer)))
        .map_err(|err| Error::new(ErrorKind::Other, err))
        .boxed();
    (peer_id, transport)
}

#[async_std::main]
async fn main() {
    let (peer2_id, trans) = mk_transport();
    let bitswap = Bitswap::new();
    let mut swarm2 = Swarm::new(trans, bitswap, peer2_id.clone());

    Swarm::listen_on(&mut swarm2, "/ip4/127.0.0.1/tcp/0".parse().unwrap()).unwrap();

    let Block {
        cid: cid_orig,
        data: data_orig,
    } = new_block(b"Hey bro");
    let cid = cid_orig.clone();

    if let Some(to_dial) = std::env::args().nth(1) {
        let dialing = to_dial.clone();
        match to_dial.parse() {
            Ok(to_dial) => match libp2p::Swarm::dial_addr(&mut swarm2, to_dial) {
                Ok(_) => println!("Dialed {:?}", dialing),
                Err(e) => println!("Dial {:?} failed: {:?}", dialing, e),
            },
            Err(err) => println!("Failed to parse address to dial: {:?}", err),
        }
    }

    let mut cid1 = Cid::from_str("bafybeifx7yeb55armcsxwwitkymga5xf53dxiarykms3ygqic223w5sk3m").unwrap();

    let mut stdin = io::BufReader::new(io::stdin()).lines();
    let mut listening = false;
    swarm2.want_block(cid1.clone(), 100);

    task::block_on(future::poll_fn(move |cx: &mut Context| {
        loop {
            match swarm2.poll_next_unpin(cx) {
                Poll::Ready(Some(bitswap_event)) => match bitswap_event {
                    BitswapEvent::ReceivedWant(peer_id, cid, _) => {
                        println!("P1: Recived Want from {}", peer_id);
                        swarm2.send_block(&peer_id, cid_orig.clone(), data_orig.clone());
                        println!("P1: Sending Block to peer {}", peer_id);
                    }
                    BitswapEvent::ReceivedBlock(peer_id, cid, data) => {
                        println!("P1: Recived Block from {}", peer_id);
                        println!("P1: Cid {}", cid);
                        println!("P2: {:?}", String::from_utf8(data.to_vec()).unwrap());
                        swarm2.cancel_block(&cid);
                    }
                    BitswapEvent::ReceivedCancel(peer_id, cid) => {
                        println!("P1: Recived Cancel {} from {}", cid, peer_id);
                    }
                },
                Poll::Ready(None) | Poll::Pending => break,
                _ => {}
            }
        }

        if !listening {
            for addr in libp2p::Swarm::listeners(&swarm2) {
                println!("Listening on {:?}", addr);
                listening = true;
            }
        }

        Poll::Pending
    }))

    //future::select(Box::pin(peer1), Box::pin(peer2)).await.factor_first().0;
}
