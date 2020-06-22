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
use async_std::prelude::*;
use async_std::{io, task};
use libp2p_bitswap::{Bitswap, BitswapEvent};
use libipld_core::cid::Cid;
use libipld_core::cid::Codec;
use libipld_core::multihash::Sha2_256;
use libp2p::mdns::service::{MdnsPacket, MdnsService};
use std::{task::{Context, Poll}};

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
    let (peer1_id, trans) = mk_transport();
    let mut swarm1 = Swarm::new(trans, Bitswap::new(), peer1_id.clone());
    let mut multi_addr = Swarm::listeners(&swarm1);
    let (peer2_id, trans) = mk_transport();
    let mut swarm2 = Swarm::new(trans, Bitswap::new(), peer2_id.clone());

    let (mut tx, mut rx) = mpsc::channel::<Multiaddr>(1);
    Swarm::listen_on(&mut swarm1, "/ip4/127.0.0.1/tcp/0".parse().unwrap()).unwrap();


    let Block {
        cid: cid_orig,
        data: data_orig,
    } = new_block(b"Hey bro");
    let cid = cid_orig.clone();


    let peer1 = async move {
        while let Some(_) = swarm1.next().now_or_never() {}

        for l in Swarm::listeners(&swarm1) {
            tx.send(l.clone()).await.unwrap();
        }
        let mut service = MdnsService::new();

        loop {
            let (srv, packet) = service.next().await;
            match swarm1.next().await {
                BitswapEvent::ReceivedWant(peer_id, cid, _) => {
                    println!("P1: Recived Want from {}", peer_id);
                    if &cid == &cid_orig {
                        swarm1.send_block(&peer_id, cid_orig.clone(), data_orig.clone());
                        println!("P1: Sending Block to peer {}", peer_id);
                    }
                }
                _ => {}
            }
            match packet {
                MdnsPacket::Response(response) => {
                    for peer in response.discovered_peers() {
                        for addr in peer.addresses() {
                            Swarm::dial_addr(&mut swarm1, addr.clone());
                        }
                    }
                    service = srv
                }
            }
        }
        
    };


    let peer2 = async move {

        swarm2.want_block(cid, 1000);

        loop {
            match swarm2.next().await {
                BitswapEvent::ReceivedBlock(peer_id, cid, data) => {
                    println!("P2: Recieved Block {} from peer {:?}", cid.clone(), peer_id);

                },
                _ => {}
            }
        }
    };

    let mut stdin = io::BufReader::new(io::stdin()).lines();

    task::block_on(future::poll_fn(move |cx: &mut Context| {
        loop {
            match stdin.try_poll_next_unpin(cx)? {
                Poll::Ready(Some(line)) => swarm.publish(&topic, line.as_bytes()),
                Poll::Ready(None) => panic!("Stdin closed"),
                Poll::Pending => break,
            };
        }
    }))







    //future::select(Box::pin(peer1), Box::pin(peer2)).await.factor_first().0;

}
