use futures::future::join;
use futures::channel::mpsc;
use futures::prelude::*;
use libp2p::core::muxing::StreamMuxerBox;
use libp2p::core::transport::boxed::Boxed;
use libp2p::core::transport::upgrade::Version;
use libp2p::identity::Keypair;
use libp2p::secio::SecioConfig;
use libp2p::tcp::TcpConfig;
use libp2p::yamux::Config as YamuxConfig;
use libp2p::{PeerId, Swarm, Transport, Multiaddr};
use std::io::{Error, ErrorKind};
use std::time::Duration;
use async_std::{io, task};
use libp2p_bitswap::{Bitswap, BitswapEvent};
use libipld_core::cid::Cid;
use libipld_core::cid::Codec;
use libipld_core::multihash::Sha2_256;
use std::{task::{Context, Poll}};
use std::fs::File;
use std::io::BufReader;
use std::io::prelude::*;

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


fn main() {
    let (peer1_id, trans) = mk_transport();
    let mut swarm1 = Swarm::new(trans, Bitswap::new(), peer1_id.clone());

    Swarm::listen_on(&mut swarm1, "/ip4/0.0.0.0/tcp/62891".parse().unwrap()).unwrap();


    let Block {
        cid: cid_orig,
        data: data_orig,
    } = new_block(b"Hey bro");
    let cid = cid_orig.clone();
    
    if let Some(to_dial) = std::env::args().nth(1) {
        let dialing = to_dial.clone();
        match to_dial.parse() {
            Ok(to_dial) => match libp2p::Swarm::dial_addr(&mut swarm1, to_dial) {
                Ok(_) => println!("Dialed {:?}", dialing),
                Err(e) => println!("Dial {:?} failed: {:?}", dialing, e),
            },
            Err(err) => println!("Failed to parse address to dial: {:?}", err),
        }
    }


    let mut stdin = io::BufReader::new(io::stdin()).lines();
    let mut listening = false;

    task::block_on(future::poll_fn(move |cx: &mut Context| {


        loop {
            match swarm1.poll_next_unpin(cx) {
                Poll::Ready(Some(bitswap_event)) => match bitswap_event {
                    BitswapEvent::ReceivedWant(peer_id, cid, _) => {
                        println!("P1: Recived Want from {}", peer_id);
                        swarm1.send_block(&peer_id, cid_orig.clone(), data_orig.clone());
                        println!("P1: Sending Block to peer {}", peer_id);
                        
                    },
                    BitswapEvent::ReceivedBlock(peer_id, cid, data) => {
                        println!("P1: Recived Block from {}", peer_id);
                        println!("P1: Cid {}", cid);
                    },
                    BitswapEvent::ReceivedCancel(peer_id, cid) => {
                        println!("P1: Recived Cancel {} from {}", cid, peer_id);
                    }
                },
                Poll::Ready(None) | Poll::Pending => break,
                _ => {}
            }
        }

        if !listening {
            for addr in libp2p::Swarm::listeners(&swarm1) {
                println!("Listening on {:?}", addr);
                listening = true;
            }
        }

        Poll::Pending
        
    }))







    //future::select(Box::pin(peer1), Box::pin(peer2)).await.factor_first().0;

}
