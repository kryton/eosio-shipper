// `error_chain!` can recurse deeply
#![recursion_limit = "1024"]

// Import the macro. Don't forget to add `error-chain` in your
// `Cargo.toml`!
#[macro_use]
extern crate error_chain;

use futures_channel::mpsc::{ unbounded};
use futures_util::{SinkExt, StreamExt, pin_mut, future};
use crate::errors::{ Result};
use std::env;
use eosio_shipper::shipper_types::{ShipRequests, ShipResultsEx, GetStatusRequestV0, GetBlocksRequestV0};
use eosio_shipper::get_sink_stream;

mod errors {
    error_chain! {
    foreign_links {
            LIBEOSIOAPI(libabieos_sys::errors::Error);
            STDIO(std::io::Error);
        }
    }
}

fn get_args() -> Result<String> {
    let args: Vec<String> = env::args().collect();
    let host = {
        if args.len() > 1 {
            &args[1]
        } else {
            "ws://127.0.0.1:9999"
            //  "https://api.testnet.eos.io"
        }
    };

    Ok(host.parse().unwrap())
}

#[tokio::main]
async fn main() {
    env_logger::init();
    match get_args() {
        Err(e) => {
            eprintln!("{:#?}", e);
        }
        Ok(host) => {
            let (mut req_s,  req_r) = unbounded::<ShipRequests>();
            let ( res_s, mut res_r) = unbounded::<ShipResultsEx>();

            let ws = async {
                get_sink_stream(&host,req_r,res_s).await;
            };
            let dumper = async {
                req_s.send(ShipRequests::get_status_request_v0(GetStatusRequestV0{})).await;
                let mut blocks = 1;

                let sr:ShipResultsEx = res_r.next().await.unwrap();
                match sr {
                    ShipResultsEx::Status(s) => {
                        println!("Chain - {}", s.chain_id)
                    }
                    _ => {}
                }
                /*
                there are two ways to retrieve blocks.
                you can either get a batch where the window (max_messages_in_flight) is equal to
                the total, and just re-request a new batch when you're done (or close too) this
                batch

                *OR*

                you can set as smaller window size and use get_blocks_ack_request_v0 to request more
                messages from the request
                 */
                req_s.send(ShipRequests::get_blocks_request_v0(GetBlocksRequestV0 {
                    start_block_num: blocks,
                    end_block_num:  blocks + 15,
                    max_messages_in_flight: 15,
                    have_positions: vec![],
                    irreversible_only: false,
                    fetch_block: true,
                    fetch_traces: false,
                    fetch_deltas: false
                })).await;
                loop {
                    let sr: ShipResultsEx = res_r.next().await.unwrap();
                    match sr {
                        ShipResultsEx::Status(_st) => {
                            println!("{}?", blocks)
                        },
                        ShipResultsEx::BlockResult(blo) => {
                            let current:u32 = blo.this_block.unwrap().block_num ;
                            match blo.block {
                                Some(b) => {
                                    println!("{} {} {} {} {}", blocks, b.producer, b.timestamp, current, b.producer_signature)
                                },
                                None => {
                                    println!("{} -?- -?- ", blocks)
                                }
                            }
                            if current >= blocks +10 {
                                blocks += 15;
                                req_s.send(ShipRequests::get_blocks_request_v0(GetBlocksRequestV0 {
                                    start_block_num: blocks,
                                    end_block_num:  blocks + 15,
                                    max_messages_in_flight: 15,
                                    have_positions: vec![],
                                    irreversible_only: false,
                                    fetch_block: true,
                                    fetch_traces: false,
                                    fetch_deltas: false
                                })).await;
                            }
                        },
                    }
                }
           //     req_s.send(ShipRequests::quit).await;
           //     res_r.close();
           //     ()
            };
            pin_mut!(ws,dumper);
            future::join(ws, dumper).await;
        },
    }

    println!("Hello, world!");
}
