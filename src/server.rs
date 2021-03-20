use anyhow::{Context, Result};
use crossbeam_channel::{select, unbounded, Sender};
use rayon::prelude::*;
use serde_json::{de::from_str, Value};

use std::{
    collections::hash_map::HashMap,
    io::{BufRead, BufReader, Write},
    net::{Shutdown, TcpListener, TcpStream},
    thread,
};

use crate::{signals, Client, Config, Rpc};

fn spawn(f: impl 'static + Send + FnOnce() -> Result<()>) -> thread::JoinHandle<()> {
    let builder = thread::Builder::new();
    builder
        .spawn(|| {
            if let Err(e) = f() {
                error!("thread failed: {}", e);
            }
        })
        .expect("failed to spawn a thread")
}

struct Peer {
    client: Client,
    stream: TcpStream,
}

impl Peer {
    fn new(stream: TcpStream) -> Self {
        Self {
            client: Client::default(),
            stream,
        }
    }
}

pub fn run(config: Config, mut rpc: Rpc) -> Result<()> {
    let listener = TcpListener::bind(config.electrum_rpc_addr)?;
    info!("serving Electrum RPC on {}", listener.local_addr()?);

    let (server_tx, server_rx) = unbounded();
    spawn(|| accept_loop(listener, server_tx)); // detach accepting thread
    let signal_rx = signals::register();

    let mut peers = HashMap::<usize, Peer>::new();
    loop {
        select! {
            recv(signal_rx) -> sig => {
                match sig.context("signal channel disconnected")? {
                    signals::Signal::Exit => {
                        info!("stopping Electrum RPC server");
                        return Ok(())
                    }
                    signals::Signal::Trigger => (),
                }
            }
            recv(server_rx) -> event => {
                let event = event.context("server disconnected")?;
                let buffered_events = server_rx.iter().take(server_rx.len());
                for event in std::iter::once(event).chain(buffered_events) {
                    handle(&rpc, &mut peers, event);
                }
            }
            default(config.wait_duration) => (),
        }
        rpc.sync().context("rpc sync failed")?;
        peers
            .par_iter_mut()
            .map(|(peer_id, peer)| {
                let notifications = rpc.update_client(&mut peer.client)?;
                send(*peer_id, peer, &notifications)
            })
            .collect::<Result<_>>()?;
    }
}

struct Event {
    peer_id: usize,
    msg: Message,
}

enum Message {
    New(TcpStream),
    Request(String),
    Done,
}

fn handle(rpc: &Rpc, peers: &mut HashMap<usize, Peer>, event: Event) {
    match event.msg {
        Message::New(stream) => {
            debug!("{}: connected", event.peer_id);
            peers.insert(event.peer_id, Peer::new(stream));
        }
        Message::Request(line) => {
            let result = match peers.get_mut(&event.peer_id) {
                Some(peer) => handle_request(rpc, event.peer_id, peer, line),
                None => {
                    warn!("{}: unknown peer for {}", event.peer_id, line);
                    Ok(())
                }
            };
            if let Err(e) = result {
                error!("{}: {}", event.peer_id, e);
                let _ = peers
                    .remove(&event.peer_id)
                    .map(|peer| peer.stream.shutdown(Shutdown::Both));
            }
        }
        Message::Done => {
            debug!("{}: disconnected", event.peer_id);
            peers.remove(&event.peer_id);
        }
    }
}

fn handle_request(rpc: &Rpc, peer_id: usize, peer: &mut Peer, line: String) -> Result<()> {
    let request: Value = from_str(&line).with_context(|| format!("invalid request: {}", line))?;
    let response: Value = rpc
        .handle_request(&mut peer.client, request)
        .with_context(|| format!("failed to handle request: {}", line))?;
    send(peer_id, peer, &[response])
}

fn send(peer_id: usize, peer: &mut Peer, values: &[Value]) -> Result<()> {
    for value in values {
        let mut response = value.to_string();
        debug!("{}: send {}", peer_id, response);
        response += "\n";
        peer.stream
            .write_all(response.as_bytes())
            .with_context(|| format!("failed to send response: {}", response))?;
    }
    Ok(())
}

fn accept_loop(listener: TcpListener, server_tx: Sender<Event>) -> Result<()> {
    for (peer_id, conn) in listener.incoming().enumerate() {
        let stream = conn.context("failed to accept")?;
        let tx = server_tx.clone();
        spawn(move || {
            let result = recv_loop(peer_id, &stream, tx);
            let _ = stream.shutdown(Shutdown::Both);
            result
        });
    }
    Ok(())
}

fn recv_loop(peer_id: usize, stream: &TcpStream, server_tx: Sender<Event>) -> Result<()> {
    server_tx.send(Event {
        peer_id,
        msg: Message::New(stream.try_clone()?),
    })?;
    let reader = BufReader::new(stream);
    for line in reader.lines() {
        let line = line.with_context(|| format!("{}: recv failed", peer_id))?;
        debug!("{}: recv {}", peer_id, line);
        let msg = Message::Request(line);
        server_tx.send(Event { peer_id, msg })?;
    }
    server_tx.send(Event {
        peer_id,
        msg: Message::Done,
    })?;
    Ok(())
}
