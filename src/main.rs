#[macro_use]
extern crate clap;
extern crate rand;
mod network;
mod api;
mod crypto;
mod primitive;
mod miner;
mod db;
mod blockchain;

use std::{thread, time};
use std::sync::mpsc::{self};
use mio_extras::channel::{self};
use clap::{Arg, App, SubCommand};
// clap follow example  https://github.com/clap-rs/clap/blob/master/examples/01c_quick_example.rs
use std::fs::File;
use std::io::{BufRead, BufReader};
use network::message::{ApiMessage, ConnectResult, ConnectHandle};
use miner::miner::{Manager};
use miner::mempool::{Mempool};
use network::message::{Message};
use network::performer::{Performer};
use db::utxoDb::{UtxoDb};
use db::blockDb::{BlockDb};
use blockchain::blockchain::{BlockChain};
use std::sync::{Arc, Mutex};
use api::apiServer::ApiServer;
use api::transactionGenerator::{TransactionGenerator};
use primitive::block::{Transaction};
use rand::Rng;


fn main() {
    let matches = clap_app!(myapp =>
        (version: "0.0")
        (author: "Bowen Xue.<bx3@uw.edu>")
        (about: "simple blockchain network")
        (@arg neighbor: -n --neighbor +takes_value "Sets ip to connect to")
        (@arg ip: -i --ip  +takes_value "Sets local ip address to use")
//        (@arg api_port: -a --api  +takes_value "Sets local api address to use")
    )
    .get_matches();

    let listen_port: String = matches.value_of("ip").expect("missing ip address").to_string();
 //   let api_port: String = matches.value_of("api_port").expect("missing ip address").to_string();
    let neighbor_path = matches.value_of("neighbor").expect("missing neighbor file");
    //println!("ip {}", ip);
    //println!("neighbor: {}", neighbor_path);
    //
    let api_port: String = "127.0.0.1:40002".to_string();
    let utxo_db = Arc::new(Mutex::new(UtxoDb::new())); 
    let block_db = Arc::new(Mutex::new(BlockDb::new()));
    let blockchain = Arc::new(Mutex::new(BlockChain::new()));

    let (task_sender, task_receiver) = mpsc::channel();
    let (server_api_sender, server_api_receiver) = channel::channel();

    let (miner_manager, miner_control_sender) = Manager::new(
        server_api_sender.clone(), 
    );

    miner_manager.start();

    let mempool = Arc::new(Mutex::new(Mempool::new(miner_control_sender.clone(), blockchain.clone())));

    let mut performer = Performer::new(
        task_receiver, 
        mempool, 
        blockchain.clone(), 
        block_db.clone()
    );
    performer.start();

    let mut server = network::server::Context::new(task_sender, server_api_receiver, &listen_port);
    server.start();

    let api_server = ApiServer::new(api_port, miner_control_sender.clone());

    // connect to peer
    let f = File::open(neighbor_path).expect("Unable to open file");
    let f = BufReader::new(f);

    let mut neighbors: Vec<String> = vec![];
    for line in f.lines() {
        let line = line.expect("Unable to read line");
        neighbors.push(line.to_string());
    }
    let mut num_connected = 0;

    let sleep_time = time::Duration::from_millis(2000);
    thread::sleep(sleep_time);

    for neighbor in neighbors.iter() {
        let (sender, receiver) = mpsc::channel();
        let connect_handle = ConnectHandle {
            result_sender: sender,
            dest_addr: neighbor.clone(),
        };

        loop {
            server_api_sender.send(ApiMessage::ServerConnect(connect_handle.clone()));           
            match receiver.recv() {
                Ok(result) => {
                    match result {
                        ConnectResult::Success => {
                            println!("connect success");
                            break;
                        },
                        ConnectResult::Fail => {
                            println!("ConnectResult::Fail {:?}", neighbor);
                        },
                    } 
                },
                Err(e) => println!("receive error {:?}", e),
            }
            let sleep_time = time::Duration::from_millis(500);
            thread::sleep(sleep_time);
        }
    }
   
    println!("start generate transaction");

    let mut tx_gen = TransactionGenerator::new();
    println!("start periodically send ping message");
    loop {
        //if true {
            // periodically sends Ping to one of neighbor
        //    let ping_msg: String = format!("from {}", listen_port);
        //    let api_message = ApiMessage::ServerBroadcast(
        //        Message::Ping(ping_msg)
        //    );
        //    server_api_sender.send(api_message);  
        //    let sleep_time = time::Duration::from_millis(500);
        //    thread::sleep(sleep_time);
        //}
        
        if true {
            let mut transactions: Vec<Transaction> = tx_gen.generate_trans(1); 

            for tx in transactions.iter() {
                server_api_sender.send(ApiMessage::CreatedTransaction(tx.clone()));
                //let p2p_message = Message::NewTransaction(tx.clone());
                //let transactions_message = ApiMessage::ServerBroadcast(p2p_message);
                //println!("send broadcast message");
                //server_api_sender.send(transactions_message);
            }

            let num = rand::thread_rng().gen_range(0, 50);
            let sleep_time = time::Duration::from_millis(num);
            thread::sleep(sleep_time);
        }
    }

    thread::park();
}
