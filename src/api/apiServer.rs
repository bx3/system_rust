use mio::{Events, Poll, Ready, PollOpt, Token};
use mio_extras::channel::{self};
use mio::tcp::{TcpListener, TcpStream};
use std::thread;
use super::network::message::{Message};
use std::sync::mpsc::{self};

const API_BUF_SIZE: usize = 1024;



pub struct ApiServer {
    poll: mio::Poll,
    //api_sender: channel::Sender<ApiMessage>,
    counter: usize,
}

impl ApiServer {
    //fn new() -> (ApiServer, channel::Receiver<ApiMessage>) {
    //    let (api_sender, api_receiver) = channel::channel();
    //    let api_server = ApiServer {
    //        poll: Poll::new().unwrap(),
    //        api_sender: api_sender, 
    //        counter: 0,
    //    };
    //    (api_server, api_receiver)
    //}

    fn start(mut self) {
    
    }
}


