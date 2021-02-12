use anyhow::{Context, Result};

use std::io::Write;
use std::iter::FromIterator;
use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpStream};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use bitcoin::consensus::encode;
use bitcoin::network::stream_reader::StreamReader;
use bitcoin::network::{
    address, constants,
    message::{self, NetworkMessage},
    message_blockdata::{GetHeadersMessage, Inventory},
    message_network,
};
use bitcoin::secp256k1;
use bitcoin::secp256k1::rand::Rng;
use bitcoin::{Block, BlockHash, Network};

use crate::chain::{Chain, NewHeader};

struct Connection {
    stream: TcpStream,
    reader: StreamReader<TcpStream>,
    network: Network,
}

impl Connection {
    pub fn connect(network: Network, address: SocketAddr) -> Result<Self> {
        let stream = TcpStream::connect(address).context("p2p failed to connect")?;
        let reader = StreamReader::new(
            stream.try_clone().context("stream failed to clone")?,
            /*buffer_size*/ Some(1 << 20),
        );
        let mut conn = Self {
            stream,
            reader,
            network,
        };
        conn.send(build_version_message())?;
        if let NetworkMessage::GetHeaders(_) = conn.recv()? {
            conn.send(NetworkMessage::Headers(vec![]))?;
        }
        Ok(conn)
    }

    fn send(&mut self, msg: NetworkMessage) -> Result<()> {
        let raw_msg = message::RawNetworkMessage {
            magic: self.network.magic(),
            payload: msg,
        };
        self.stream
            .write_all(encode::serialize(&raw_msg).as_slice())
            .context("p2p failed to send")
    }

    fn recv(&mut self) -> Result<NetworkMessage> {
        loop {
            let raw_msg: message::RawNetworkMessage =
                self.reader.read_next().context("p2p failed to recv")?;

            match raw_msg.payload {
                NetworkMessage::Version(version) => {
                    trace!("peer version: {:?}", version);
                    self.send(NetworkMessage::Verack)?;
                }
                NetworkMessage::Ping(nonce) => {
                    self.send(NetworkMessage::Pong(nonce))?;
                }
                NetworkMessage::Verack | NetworkMessage::Alert(_) | NetworkMessage::Addr(_) => {}
                NetworkMessage::Inv(inv) => {
                    trace!("inv: {:?}", inv);
                }
                payload => return Ok(payload),
            };
        }
    }
}

pub struct Client {
    conn: Mutex<Connection>,
}

impl Client {
    pub fn connect(network: Network, address: SocketAddr) -> Result<Self> {
        let conn = Mutex::new(Connection::connect(network, address)?);
        Ok(Self { conn })
    }

    pub(crate) fn get_new_headers(&self, chain: &Chain) -> Result<Vec<NewHeader>> {
        let mut conn = self.conn.lock().unwrap();

        let msg = GetHeadersMessage::new(chain.locator(), BlockHash::default());
        conn.send(NetworkMessage::GetHeaders(msg))?;
        let headers = match conn.recv()? {
            NetworkMessage::Headers(headers) => headers,
            msg => bail!("unexpected {:?}", msg),
        };

        debug!("got {} new headers", headers.len());
        let prev_blockhash = match headers.first().map(|h| h.prev_blockhash) {
            None => return Ok(vec![]),
            Some(prev_blockhash) => prev_blockhash,
        };
        let new_heights = match chain.get_block_height(&prev_blockhash) {
            Some(last_height) => (last_height + 1)..,
            None => bail!("missing prev_blockhash: {}", prev_blockhash),
        };
        Ok(headers
            .into_iter()
            .zip(new_heights)
            .map(NewHeader::from)
            .collect())
    }

    pub(crate) fn for_blocks<B, F>(&self, blockhashes: B, mut func: F) -> Result<()>
    where
        B: IntoIterator<Item = BlockHash>,
        F: FnMut(BlockHash, Block),
    {
        let mut conn = self.conn.lock().unwrap();

        let blockhashes = Vec::from_iter(blockhashes);
        if blockhashes.is_empty() {
            return Ok(());
        }
        let inv = blockhashes
            .iter()
            .map(|h| Inventory::WitnessBlock(*h))
            .collect();
        debug!("loading {} blocks", blockhashes.len());
        conn.send(NetworkMessage::GetData(inv))?;
        for hash in blockhashes {
            match conn.recv()? {
                NetworkMessage::Block(block) => {
                    assert_eq!(block.block_hash(), hash, "got unexpected block");
                    func(hash, block);
                }
                msg => bail!("unexpected {:?}", msg),
            };
        }
        Ok(())
    }
}

fn build_version_message() -> NetworkMessage {
    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), 0);
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time error")
        .as_secs() as i64;

    let services = constants::ServiceFlags::NETWORK | constants::ServiceFlags::WITNESS;

    NetworkMessage::Version(message_network::VersionMessage {
        version: constants::PROTOCOL_VERSION,
        services,
        timestamp,
        receiver: address::Address::new(&addr, services),
        sender: address::Address::new(&addr, services),
        nonce: secp256k1::rand::thread_rng().gen(),
        user_agent: String::from("electrs"),
        start_height: 0,
        relay: false,
    })
}
