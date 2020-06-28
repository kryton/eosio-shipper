// `error_chain!` can recurse deeply
#![recursion_limit = "1024"]

// Import the macro. Don't forget to add `error-chain` in your
// `Cargo.toml`!
#[macro_use]
extern crate error_chain;
#[macro_use]
extern crate log;

use crate::errors::Result;
use eosio_shipper::get_sink_stream;
use eosio_shipper::shipper_types::{
    ContractIndex128, ContractIndex256, ContractIndex64, ContractIndexDouble,
    ContractIndexLongDouble, ContractRow, ContractTable, GetBlocksRequestV0, GetStatusRequestV0,
    ShipRequests, ShipResultsEx, SignedBlock, TableRowTypes,
};
use futures_channel::mpsc::unbounded;
use futures_util::{future, pin_mut, SinkExt, StreamExt};
use std::cmp::min;
use std::env;
use std::fs::File;
use std::io::Write;

mod errors {
    error_chain! {
    foreign_links {
            LIBEOSIOAPI(libabieos_sys::errors::Error);
            STDIO(std::io::Error);
        }
    }
}

fn get_args() -> Result<(String, u32)> {
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

    Ok((host.parse().unwrap(), start_block))
}

#[tokio::main]
async fn main() {
    env_logger::init();
    let fetch_block = false;
    let fetch_traces = false;
    let fetch_deltas = true;
    match get_args() {
        Err(e) => {
            eprintln!("{:#?}", e);
        }
        Ok((host, start_block)) => {
            let (mut req_s, req_r) = unbounded::<ShipRequests>();
            let (res_s, mut res_r) = unbounded::<ShipResultsEx>();

            let ws = async {
                get_sink_stream(&host, req_r, res_s).await;
            };
            let dumper = async {
                let mut delta_file = File::create("deltas.txt").unwrap();
                req_s
                    .send(ShipRequests::get_status_request_v0(GetStatusRequestV0 {}))
                    .await;
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
                            last_fetched = min(current + 1 + 150, last_block);
                            info!(
                                "Chain - {} -> {}",
                                st.chain_id.unwrap_or(String::from("?NONE?")),
                                last_block
                            );
                            req_s
                                .send(ShipRequests::get_blocks_request_v0(GetBlocksRequestV0 {
                                    start_block_num: current + 1,
                                    end_block_num: last_fetched,
                                    max_messages_in_flight: 150,
                                    have_positions: vec![],
                                    irreversible_only: false,
                                    fetch_block,
                                    fetch_traces,
                                    fetch_deltas,
                                }))
                                .await;
                        }
                        ShipResultsEx::BlockResult(blo) => match blo.this_block {
                            Some(bp) => {
                                current = bp.block_num;
                                match blo.block {
                                    Some(b) => match b {
                                        SignedBlock::signed_block_v0(b0) => debug!(
                                            "v0 - {} {} {} {} {} ",
                                            current,
                                            last_block,
                                            b0.signed_header.header.producer,
                                            b0.signed_header.header.timestamp,
                                            b0.signed_header.producer_signature
                                        ),
                                        SignedBlock::signed_block_v1(b1) => debug!(
                                            "v1 - {} {} {} {} {} ",
                                            current,
                                            last_block,
                                            b1.signed_header.header.producer,
                                            b1.signed_header.header.timestamp,
                                            b1.signed_header.producer_signature
                                        ),
                                    },
                                    None => debug!("empty?"),
                                }
                                if !blo.traces.is_empty() {
                                    info!("\t-{} #Trace", blo.traces.len())
                                }
                                if !blo.deltas.is_empty() {
                                    for delta in blo.deltas {
                                        if delta.name == "contract_row" {
                                            for row in delta.rows {
                                                match row.data {
                                                    TableRowTypes::contract_row(cr) => match cr {
                                                        ContractRow::contract_row_v0(cr0) => {
                                                            delta_file.write_all(
                                                            format!(
                                                                "{},ROW,{},{},{},{},{},{},{}\n",
                                                                current, row.present, cr0.code, cr0.payer, cr0.scope, cr0.table, cr0.primary_key, cr0.value
                                                            ).as_bytes()).unwrap()
                                                        }
                                                        _ => {}
                                                    },
                                                    TableRowTypes::contract_table(ct) => match ct {
                                                        ContractTable::contract_table_v0(ct0) => {
                                                            delta_file.write_all(
                                                                format!(
                                                                "{},TABLE,{},{},{},{},{}\n",
                                                                current, row.present, ct0.code, ct0.payer, ct0.scope, ct0.table
                                                            ).as_bytes()).unwrap()
                                                        },
                                                        _ => {}
                                                    },
                                                    TableRowTypes::contract_index64(ci) => match ci {
                                                        ContractIndex64::contract_index64_v0(ci0) => {
                                                            delta_file.write_all(
                                                                format!(
                                                                "{},INDEX64,{},{},{},{},{},{},{}\n",
                                                                current, row.present, ci0.code, ci0.payer, ci0.scope, ci0.table, ci0.primary_key, ci0.secondary_key
                                                            ).as_bytes()).unwrap()
                                                        },
                                                        _ => {}
                                                    },
                                                    TableRowTypes::contract_index128(ci) => match ci {
                                                        ContractIndex128::contract_index128_v0(ci0) => {
                                                            delta_file.write_all(
                                                                format!(
                                                                "{},INDEX128,{},{},{},{},{},{},{}\n",
                                                                current, row.present, ci0.code, ci0.payer, ci0.scope, ci0.table, ci0.primary_key, ci0.secondary_key
                                                            ).as_bytes()).unwrap()
                                                        },
                                                        _ => {}
                                                    },
                                                    TableRowTypes::contract_index256(ci) => match ci {
                                                        ContractIndex256::contract_index256_v0(ci0) => {
                                                            delta_file.write_all(
                                                                format!(
                                                                "{},INDEX256,{},{},{},{},{},{},{}\n",
                                                                current, row.present, ci0.code, ci0.payer, ci0.scope, ci0.table, ci0.primary_key, ci0.secondary_key
                                                            ).as_bytes()).unwrap()
                                                        },
                                                        _ => {}
                                                    },
                                                    TableRowTypes::contract_index_double(ci) => match ci {
                                                        ContractIndexDouble::contract_index_double_v0(ci0) => {
                                                            delta_file.write_all(
                                                                format!(
                                                                "{},INDEXDBL,{},{},{},{},{},{},{}\n",
                                                                current, row.present, ci0.code, ci0.payer, ci0.scope, ci0.table, ci0.primary_key, ci0.secondary_key
                                                            ).as_bytes()).unwrap()
                                                        },
                                                        _ => {}
                                                    },
                                                    TableRowTypes::contract_index_long_double(ci) => match ci {
                                                        ContractIndexLongDouble::contract_index_long_double_v0(ci0) => {
                                                            delta_file.write_all(
                                                                format!(
                                                                "{},INDEXLONGDBL,{},{},{},{},{},{},{}\n",
                                                                current, row.present, ci0.code, ci0.payer, ci0.scope, ci0.table, ci0.primary_key, ci0.secondary_key
                                                            ).as_bytes()).unwrap()
                                                        },
                                                        _ => {}
                                                    },
                                                    _ => {}
                                                }
                                            }
                                        }
                                    }
                                }
                                if (current + 1) >= last_block {
                                    debug!("{} reached end {}", current, last_block);

                                    req_s
                                        .send(ShipRequests::get_status_request_v0(
                                            GetStatusRequestV0 {},
                                        ))
                                        .await;
                                    delta_file.sync_data();
                                } else {
                                    if (current + 1) >= last_fetched {
                                        last_fetched = min(current + 1 + 150, last_block);
                                        debug!("GBR-{}->{} {}", current, last_fetched, last_block);
                                        req_s
                                            .send(ShipRequests::get_blocks_request_v0(
                                                GetBlocksRequestV0 {
                                                    start_block_num: current + 1,
                                                    end_block_num: last_fetched,
                                                    max_messages_in_flight: 150,
                                                    have_positions: vec![],
                                                    irreversible_only: false,
                                                    fetch_block,
                                                    fetch_traces,
                                                    fetch_deltas,
                                                },
                                            ))
                                            .await;
                                        delta_file.sync_data();
                                    }
                                }
                            }
                            None => {
                                error!("{} {} empty", current, last_block);
                                req_s
                                    .send(ShipRequests::get_status_request_v0(
                                        GetStatusRequestV0 {},
                                    ))
                                    .await;
                                delta_file.sync_all();
                            }
                        },
                    }
                }
                //     req_s.send(ShipRequests::quit).await;
                //     res_r.close();
                //     ()
            };
            pin_mut!(ws, dumper);
            future::join(ws, dumper).await;
        }
    }

    println!("Hello, world!");
}
