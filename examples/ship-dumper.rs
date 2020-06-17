// `error_chain!` can recurse deeply
#![recursion_limit = "1024"]

// Import the macro. Don't forget to add `error-chain` in your
// `Cargo.toml`!
#[macro_use]
extern crate error_chain;

use futures_channel::mpsc::{unbounded};
use futures_util::{SinkExt, StreamExt, pin_mut, future};
use crate::errors::{Result};
use std::env;
use eosio_shipper::shipper_types::{ShipRequests, ShipResultsEx, GetStatusRequestV0, GetBlocksRequestV0, SignedBlock};
use eosio_shipper::get_sink_stream;
use std::cmp::min;

mod errors {
    error_chain! {
    foreign_links {
            LIBEOSIOAPI(libabieos_sys::errors::Error);
            STDIO(std::io::Error);
        }
    }
}

fn get_args() -> Result<(String,u32)> {
    let args: Vec<String> = env::args().collect();
    let host = {
        if args.len() > 1 {
            &args[1]
        } else {
            "ws://127.0.0.1:9999"
            //  "https://api.testnet.eos.io"
        }
    };
    let start_block: u32 = if args.len() > 2 {
        args[2].parse::<u32>().unwrap()
    } else {
        0
    };

    Ok((host.parse().unwrap(),start_block))
}

#[tokio::main]
async fn main() {
    env_logger::init();
    match get_args() {
        Err(e) => {
            eprintln!("{:#?}", e);
        }
        Ok((host,start_block)) => {
            let (mut req_s, req_r) = unbounded::<ShipRequests>();
            let (res_s, mut res_r) = unbounded::<ShipResultsEx>();

            let ws = async {
                get_sink_stream(&host, req_r, res_s).await;
            };
            let dumper = async {
                req_s.send(ShipRequests::get_status_request_v0(GetStatusRequestV0 {})).await;
                let mut last_block: u32 = 0;
                let mut current: u32 = start_block;

                /*
                there are two ways to retrieve blocks.
                you can either get a batch where the window (max_messages_in_flight) is equal to
                the total, and just re-request a new batch when you're done (or close too) this
                batch

                *OR*

                you can set as smaller window size and use get_blocks_ack_request_v0 to request more
                messages from the request
                 */

                let mut last_fetched: u32 = 0;
                loop {
                    //   println!("Current= {} -> {}", current, last_fetched);
                    let sr: ShipResultsEx = res_r.next().await.unwrap();
                    match sr {
                        ShipResultsEx::Status(st) => {
                            last_block = st.chain_state_end_block;
                            last_fetched = min(current + 1 + 15, last_block);
                            println!("Chain - {} -> {}", st.chain_id, last_block);
                            req_s.send(ShipRequests::get_blocks_request_v0(GetBlocksRequestV0 {
                                start_block_num: current + 1,
                                end_block_num: last_fetched,
                                max_messages_in_flight: 15,
                                have_positions: vec![],
                                irreversible_only: false,
                                fetch_block: true,
                                fetch_traces: true,
                                fetch_deltas: true,
                            })).await;
                        }
                        ShipResultsEx::BlockResult(blo) => {
                            match blo.this_block {
                                Some(bp) => {
                                    current = bp.block_num;
                                    match blo.block {
                                        Some(b) => {
                                            match b {
                                                SignedBlock::signed_block_v0(b0) => {
                                                    println!("v0 - {} {} {} {} {} ", current, last_block,
                                                             b0.signed_header.header.producer,
                                                             b0.signed_header.header.timestamp,
                                                             b0.signed_header.producer_signature)
                                                }
                                                SignedBlock::signed_block_v1(b1) => {
                                                    println!("v1 - {} {} {} {} {} ", current, last_block,
                                                             b1.signed_header.header.producer,
                                                             b1.signed_header.header.timestamp,
                                                             b1.signed_header.producer_signature)
                                                }
                                            }
                                        }
                                        None => {
                                            println!("{} block empty?", current);
                                            req_s.send(ShipRequests::get_status_request_v0(GetStatusRequestV0 {})).await;
                                        }
                                    }
                                    if !blo.traces.is_empty() {
                                        println!("\t-{} #Trace", blo.traces.len())
                                    }
                                    if !blo.deltas.is_empty() {
                                        println!("\t-{} #Delta", blo.deltas.len())
                                    }
                                    if (current + 1) >= last_block {
                                        println!("{} reached end {}", current, last_block);
                                        req_s.send(ShipRequests::get_status_request_v0(GetStatusRequestV0 {})).await;
                                    } else {
                                        if (current + 1) >= last_fetched {
                                            last_fetched = min(current + 1 + 15, last_block);
                                            req_s.send(ShipRequests::get_blocks_request_v0(GetBlocksRequestV0 {
                                                start_block_num: current + 1,
                                                end_block_num: last_fetched,
                                                max_messages_in_flight: 15,
                                                have_positions: vec![],
                                                irreversible_only: false,
                                                fetch_block: true,
                                                fetch_traces: false,
                                                fetch_deltas: false,
                                            })).await;
                                        }
                                    }
                                }
                                None => {
                                    println!("{} {} empty", current, last_block);
                                    req_s.send(ShipRequests::get_status_request_v0(GetStatusRequestV0 {})).await;
                                }
                            }
                        }
                    }
                }
                //     req_s.send(ShipRequests::quit).await;
                //     res_r.close();
                //     ()
            };
            pin_mut!(ws,dumper);
            future::join(ws, dumper).await;
        }
    }

    println!("Hello, world!");
}
