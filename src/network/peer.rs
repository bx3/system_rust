use std::net::{TcpListener, SocketAddr};
use mio::net::TcpStream;
use std::sync::mpsc::{self, Sender, Receiver, channel};
use super::message::{Message};
use mio_extras::channel::{self};
use std::collections::{VecDeque};
use super::message::{ApiMessage, ConnectResult, ConnectHandle, PeerHandle};
use log::{warn, info};
use super::MSG_BUF_SIZE;


pub struct PeerContext {
    pub addr: SocketAddr,
    pub stream: mio::net::TcpStream,
    pub peer_handle: PeerHandle,
    pub request: Message,
    pub connect_handle: Option<ConnectHandle>,
    pub is_connected: bool,
}

impl PeerContext {
    pub fn new(
        stream: mio::net::TcpStream
    ) -> (PeerContext, channel::Receiver<Message>) {
        let (sender, receiver) = channel::channel();
        let peer_context = PeerContext {
            addr: stream.local_addr().unwrap(),
            stream: stream,
            peer_handle: PeerHandle{response_sender: sender}, 
            request: Message::Ping("Default".to_string()),
            connect_handle: None,   
            is_connected: false,
        };
        (peer_context, receiver)
    }

    pub fn insert(&mut self, request: &[u8], len: usize) -> bool {
        if len == 0 {
            warn!("current request is empty"); 
            return false;
        }
        let mut request_with_size: Vec<u8> = vec![0; len];
        request_with_size.copy_from_slice(&request[0..len]);
        //println!("request_with_size {:?}", request_with_size);
        let decoded_msg: Message = bincode::deserialize(&request_with_size).expect("unable to decode msg");
        self.request = decoded_msg;
        true
    }

    pub fn send(&self, msg: Message) {
        self.peer_handle.response_sender.send(msg); 
    }
}
