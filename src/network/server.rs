use mio::{Events, Poll, Ready, PollOpt, Token};
use mio::tcp::{TcpListener, TcpStream};
use std::net::{SocketAddr};
use std::collections::{HashMap};
use std::thread;
use std::io::{self, Read, Write};
use super::peer::{PeerContext};
use super::MSG_BUF_SIZE;
use super::message::{Message, ApiMessage, ConnectResult, ConnectHandle, TaskRequest};
use std::sync::mpsc::{self, TryRecvError};
use mio_extras::channel::{self, Receiver};
use log::{info};

// refer to https://sergey-melnychuk.github.io/2019/08/01/rust-mio-tcp-server/ 
// for context
const LISTENER: Token = Token(0);
const CONTROL: Token = Token(1);
const NETWORK_TOKEN: usize = 0;
const LOCAL_TOKEN: usize = 1;

const EVENT_CAP: usize = 1024;

//fn is_double_crnl(window: &[u8]) -> bool {
//    window.len() >= 4 &&
//        (window[0] == '\r' as u8) &&
//        (window[1] == '\n' as u8) &&
//        (window[2] == '\r' as u8) &&
//        (window[3] == '\n' as u8)
//}

pub struct Context {
    poll: mio::Poll,
    peers: HashMap<Token, PeerContext>,
    token_counter: usize,
    task_sender: mpsc::Sender<TaskRequest>,
    response_receiver: HashMap<Token, channel::Receiver<Message>>,
    api_receiver: channel::Receiver<ApiMessage>,
    local_addr: SocketAddr,
}

impl Context {
    pub fn new(
        task_sender: mpsc::Sender<TaskRequest>, 
        api_receiver: channel::Receiver<ApiMessage>,
        addr: &str 
    ) -> Context {
        let addr: SocketAddr = addr.parse().unwrap();
        Context{
            poll: Poll::new().unwrap(),
            peers: HashMap::new(),
            token_counter: 2, // 0, 1 token are reserved
            task_sender: task_sender,
            response_receiver: HashMap::new(),
            api_receiver: api_receiver,
            local_addr: addr,
        }
    }
    
    // start a server, spawn a process
    pub fn start(mut self) {
        let _handler = thread::spawn(move || {
            self.listen(); 
        });
        println!("listener started");
    }

    // register tcp in the event loop 
    pub fn register_peer(&mut self, socket: TcpStream) -> io::Result<Token> {
        let network_token = Token(self.token_counter);
        self.token_counter += 1;
        
        self.poll.register(
            &socket, 
            network_token.clone(),
            Ready::readable(),
            PollOpt::edge()
        ).unwrap();

        let (peer_context, event_rx) = PeerContext::new(socket);
        let local_token = Token(self.token_counter);
        self.token_counter += 1;

        self.peers.insert(network_token, peer_context);

        self.poll.register(
            &event_rx,
            local_token,
            Ready::readable(),
            PollOpt::edge()
        ).unwrap();
        println!("local token {}", local_token.0);
        self.response_receiver.insert(local_token, event_rx);

        Ok(network_token)
    }

    // create tcp stream for each peer
    pub fn connect(&mut self, connect_handle: ConnectHandle) -> io::Result<()> {
        let addr: SocketAddr = connect_handle.dest_addr.parse().unwrap();
        let stream = TcpStream::connect(&addr).unwrap();
        let network_token = self.register_peer(stream).unwrap();
        let mut peer = self.peers.get_mut(&network_token).unwrap();
        peer.connect_handle = Some(connect_handle);
        peer.send(Message::Ping("hello".to_string()));
        Ok(())
    }

    // polling events
    pub fn listen(&mut self) {
        let listener = TcpListener::bind(&self.local_addr).unwrap(); 
       
        self.poll.register(&listener, 
            LISTENER,
            Ready::readable(),
            PollOpt::edge()
        ).unwrap();

        self.poll.register(&self.api_receiver, 
            CONTROL,
            Ready::readable(),
            PollOpt::edge()
        ).unwrap();

        let mut events = Events::with_capacity(EVENT_CAP);
        let mut buf = [0; MSG_BUF_SIZE];
        loop {
            self.poll.poll(&mut events, None).unwrap(); 
            for event in &events {
                match event.token() {
                    LISTENER => {
                        loop {
                            match listener.accept() { 
                                Ok((socket, socket_addr)) => {
                                    self.register_peer(socket).unwrap();
                                    //println!("tcp registered {}", socket_addr);
                                },
                                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                                    //println!("Tcp listener would block");
                                    break; 
                                },
                                e => panic!("err={:?}", e),
                            }
                        }
                    },
                    CONTROL => {
                        loop {
                            let api_msg = self.api_receiver.try_recv();
                            match api_msg { 
                                Ok(msg) => {
                                    match msg {
                                        ApiMessage::ServerConnect(connect_handle) => {
                                            //println!("api connects to {}", connect_handle.dest_addr);
                                            self.connect(connect_handle);
                                        },
                                        ApiMessage::ServerBroadcast(network_message) => {
                                            println!("Server broadcast message to {} peers", self.peers.len());
                                            for (token, peer) in self.peers.iter() {
                                                if peer.addr != self.local_addr {
                                                    peer.send(network_message.clone()); 
                                                }
                                            }
                                        },
                                        ApiMessage::MinedBlock(block) => {
                                            let new_block_msg = Message::NewBlock(block.clone());
                                            for (token, peer) in self.peers.iter() {
                                                if peer.addr != self.local_addr {
                                                    peer.send(new_block_msg.clone()); 
                                                }
                                            }
                                            // send to local
                                            let performer_task = TaskRequest{
                                                peer: None, 
                                                msg: new_block_msg
                                            };
                                            self.task_sender.send(performer_task).expect("send request to worker");
                                        },
                                        ApiMessage::CreatedTransaction(transaction) => {
                                            // send to local
                                            let performer_task = TaskRequest{
                                                peer: None, 
                                                msg: Message::NewTransaction(transaction),
                                            };
                                            self.task_sender.send(performer_task).expect("send request to worker");
                                        },
                                        _ => println!("ApiMessage not implemented yet"),
                                    }
                                    break;
                                },
                                e => println!("api receiver Err {:?}", e),
                            }
                        }
                    },
                    token if event.readiness().is_readable() => {
                        let token_type: usize = token.0 % 2;
                        match token_type {
                            NETWORK_TOKEN => { 
                                loop {
                                    let mut peer = self.peers.get_mut(&token).unwrap(); 
                                    let read = peer.stream.read(&mut buf);
                                    match read {
                                        Ok(0) => {
                                            break;
                                        }
                                        Ok(len) => {
                                            if !peer.is_connected {
                                                match peer.connect_handle {
                                                    Some(ref handler) => {
                                                        handler.result_sender.send(ConnectResult::Success).unwrap();
                                                        peer.is_connected = true;
                                                    },
                                                    _ => (),
                                                }
                                            }
                                            peer.insert(&buf, len);
                                            // send task request to performer
                                            let performer_task = TaskRequest{
                                                peer: Some(peer.peer_handle.clone()), 
                                                msg: peer.request.clone()
                                            };
                                            //println!("send task to performer");
                                            self.task_sender.send(performer_task).expect("send request to worker");
                                        },
                                        Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                                            break; 
                                        },
                                        e => {
                                            //self.peers.remove(&token).unwrap();   
                                            println!("Connect fail. could not connect to {:?}", peer.addr);
                                            match peer.connect_handle {
                                                Some(ref handler) => handler.result_sender.send(ConnectResult::Fail).unwrap(),
                                                _ => (),
                                            }
                                            
                                        },
                                    }
                                }
                            },
                            LOCAL_TOKEN => {
                                let peer_token = Token(token.0 - 1);
                                let peer = self.peers.get(&peer_token).unwrap(); 
                                //println!("before reregister write");
                                self.poll.reregister(
                                        &peer.stream,
                                        peer_token,
                                        Ready::writable(),
                                        PollOpt::edge() | PollOpt::oneshot()
                                ).unwrap();
                            },
                            _ => unreachable!(),
                        }
                    },
                    token if event.readiness().is_writable() => {
                        let peer = self.peers.get_mut(&token).unwrap(); 
                        let peer_token = Token(token.0 + 1);
                        let receiver = self.response_receiver.get(&peer_token).expect("response_receiver empty");
                        loop {
                            let response_msg = receiver.try_recv();
                            match response_msg {
                                Ok(msg) => {
                                    let mut peer_stream = &peer.stream;
                                    let encoded_msg = bincode::serialize(&msg).expect("unable to encode msg");
                                    //println!("write message of size {}, {:?}", encoded_msg.len(), encoded_msg);
                                    //let decoded_msg: Message = bincode::deserialize(&encoded_msg).expect("unable to encode msg");
                                    //println!("{:?}", decoded_msg);
                                    peer_stream.write_all(encoded_msg.as_slice()).unwrap();
                                    self.poll.reregister(
                                        peer_stream,
                                        token,
                                        Ready::readable(),
                                        PollOpt::edge()
                                    ).expect("unable to reregister");  
                                    break;
                                },
                                Err(e) => println!("write try receive fails "),
                            }
                        }
                        
                    },
                    _ => unreachable!(),
                }
            }
        }
    }
}


