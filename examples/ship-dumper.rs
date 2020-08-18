// `error_chain!` can recurse deeply
#![recursion_limit = "1024"]

// Import the macro. Don't forget to add `error-chain` in your
// `Cargo.toml`!
#[macro_use]
extern crate error_chain;
#[macro_use]
extern crate log;

use crate::errors::Result;
use eosio_shipper::shipper_types::{
    ContractIndex128, ContractIndex256, ContractIndex64, ContractIndexDouble,
    ContractIndexLongDouble, ContractRow, ContractTable, GetBlocksRequestV0, GetBlocksResultV0Ex,
    GetStatusRequestV0, ShipRequests, ShipResultsEx, SignedBlock, TableRowTypes,
    TransactionVariantV0,
};
use eosio_shipper::shipper_types::{TransactionReceiptV0, TransactionReceiptV1};
use eosio_shipper::{get_sink_stream, EOSIO_SYSTEM};
use futures_channel::mpsc::unbounded;
use futures_util::{future, pin_mut, SinkExt, StreamExt};
use libabieos_sys::ABIEOS;
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

fn get_args() -> Result<(String, i64, String)> {
    let args: Vec<String> = env::args().collect();
    let host = {
        if args.len() > 1 {
            &args[1]
        } else {
            "ws://127.0.0.1:9999"
            //  "https://api.testnet.eos.io"
        }
    };
    let start_block: i64 = if args.len() > 2 {
        args[2].parse::<i64>().unwrap()
    } else {
        0
    };
    let arg = {
        if args.len() > 3 {
            &args[3]
        } else {
            "P"
        }
    };

    Ok((host.parse().unwrap(), start_block, arg.parse().unwrap()))
}

fn handle_performance(mut file: &File, current: u32, block: &GetBlocksResultV0Ex) {
    match &block.block {
        Some(b) => match b {
            SignedBlock::signed_block_v0(b0) => {
                debug!(
                    "v0 - {}{} {} {} ",
                    current,
                    b0.signed_header.header.producer,
                    b0.signed_header.header.timestamp,
                    b0.signed_header.producer_signature,
                );
                if b0.signed_header.header.confirmed > 0 {
                    info!("Got One {}", current);
                }
                let it = b0.transactions.iter().zip(block.transactions.iter());
                for (tr, trans) in it {
                    let tran_desc = match trans {
                        Some(t) => t
                            .actions
                            .iter()
                            .map(|f| format!("{}:{}", f.account, f.name))
                            .collect::<Vec<_>>()
                            .join("|"),
                        None => "".to_string(),
                    };
                    match &tr.trx {
                        TransactionVariantV0::transaction_id(tid) => {
                            file.write_all(
                                format!(
                                    "USAGE,T,{},{},{},{}\n",
                                    current,
                                    tid.transaction_id,
                                    tr.header.cpu_usage_us,
                                    tr.header.net_usage_words
                                )
                                .as_bytes(),
                            )
                            .unwrap();
                        }
                        TransactionVariantV0::packed_transaction_v0(ptrx) => match trans {
                            Some(t) => file
                                .write_all(
                                    format!(
                                        "USAGE:P0,{},{},{},{},{}\n",
                                        current,
                                        tran_desc,
                                        t.actions.len(),
                                        tr.header.cpu_usage_us,
                                        tr.header.net_usage_words
                                    )
                                    .as_bytes(),
                                )
                                .unwrap(),
                            None => file
                                .write_all(
                                    format!(
                                        "USAGE:P0,{},-None-,0,{},{}\n",
                                        current, tr.header.cpu_usage_us, tr.header.net_usage_words
                                    )
                                    .as_bytes(),
                                )
                                .unwrap(),
                        },
                        TransactionVariantV0::packed_transaction(ptrx) => match trans {
                            Some(t) => file
                                .write_all(
                                    format!(
                                        "USAGE:P,{},{},{},{},{}\n",
                                        current,
                                        tran_desc,
                                        t.actions.len(),
                                        tr.header.cpu_usage_us,
                                        tr.header.net_usage_words
                                    )
                                    .as_bytes(),
                                )
                                .unwrap(),
                            None => file
                                .write_all(
                                    format!(
                                        "USAGE:P,{},-None-,0,{},{}\n",
                                        current, tr.header.cpu_usage_us, tr.header.net_usage_words
                                    )
                                    .as_bytes(),
                                )
                                .unwrap(),
                        },
                    }
                }
            }
            SignedBlock::signed_block_v1(b1) => debug!(
                "v1 - {} {} {} {} ",
                current,
                b1.signed_header.header.producer,
                b1.signed_header.header.timestamp,
                b1.signed_header.producer_signature
            ),
        },
        None => debug!("empty?"),
    }
}

fn handle_delta(mut delta_file: &File, current: u32, block: &GetBlocksResultV0Ex) {
    if block.deltas.is_empty() {
        return;
    }
    for delta in &block.deltas {
        if delta.name == "contract_row" {
            for row in &delta.rows {
                match &row.data {
                    TableRowTypes::contract_row(cr) => match cr {
                        ContractRow::contract_row_v0(cr0) => delta_file
                            .write_all(
                                format!(
                                    "{},ROW,{},{},{},{},{},{},{}\n",
                                    current,
                                    row.present,
                                    cr0.code,
                                    cr0.payer,
                                    cr0.scope,
                                    cr0.table,
                                    cr0.primary_key,
                                    cr0.value
                                )
                                .as_bytes(),
                            )
                            .unwrap(), //_ => {}
                    },
                    TableRowTypes::contract_table(ct) => match ct {
                        ContractTable::contract_table_v0(ct0) => delta_file
                            .write_all(
                                format!(
                                    "{},TABLE,{},{},{},{},{}\n",
                                    current, row.present, ct0.code, ct0.payer, ct0.scope, ct0.table
                                )
                                .as_bytes(),
                            )
                            .unwrap(), //_ => {}
                    },
                    TableRowTypes::contract_index64(ci) => match ci {
                        ContractIndex64::contract_index64_v0(ci0) => delta_file
                            .write_all(
                                format!(
                                    "{},INDEX64,{},{},{},{},{},{},{}\n",
                                    current,
                                    row.present,
                                    ci0.code,
                                    ci0.payer,
                                    ci0.scope,
                                    ci0.table,
                                    ci0.primary_key,
                                    ci0.secondary_key
                                )
                                .as_bytes(),
                            )
                            .unwrap(), //_ => {}
                    },
                    TableRowTypes::contract_index128(ci) => match ci {
                        ContractIndex128::contract_index128_v0(ci0) => delta_file
                            .write_all(
                                format!(
                                    "{},INDEX128,{},{},{},{},{},{},{}\n",
                                    current,
                                    row.present,
                                    ci0.code,
                                    ci0.payer,
                                    ci0.scope,
                                    ci0.table,
                                    ci0.primary_key,
                                    ci0.secondary_key
                                )
                                .as_bytes(),
                            )
                            .unwrap(), //_ => {}
                    },
                    TableRowTypes::contract_index256(ci) => match ci {
                        ContractIndex256::contract_index256_v0(ci0) => delta_file
                            .write_all(
                                format!(
                                    "{},INDEX256,{},{},{},{},{},{},{}\n",
                                    current,
                                    row.present,
                                    ci0.code,
                                    ci0.payer,
                                    ci0.scope,
                                    ci0.table,
                                    ci0.primary_key,
                                    ci0.secondary_key
                                )
                                .as_bytes(),
                            )
                            .unwrap(), //_ => {}
                    },
                    TableRowTypes::contract_index_double(ci) => match ci {
                        ContractIndexDouble::contract_index_double_v0(ci0) => delta_file
                            .write_all(
                                format!(
                                    "{},INDEXDBL,{},{},{},{},{},{},{}\n",
                                    current,
                                    row.present,
                                    ci0.code,
                                    ci0.payer,
                                    ci0.scope,
                                    ci0.table,
                                    ci0.primary_key,
                                    ci0.secondary_key
                                )
                                .as_bytes(),
                            )
                            .unwrap(), //_ => {}
                    },
                    TableRowTypes::contract_index_long_double(ci) => match ci {
                        ContractIndexLongDouble::contract_index_long_double_v0(ci0) => delta_file
                            .write_all(
                                format!(
                                    "{},INDEXLONGDBL,{},{},{},{},{},{},{}\n",
                                    current,
                                    row.present,
                                    ci0.code,
                                    ci0.payer,
                                    ci0.scope,
                                    ci0.table,
                                    ci0.primary_key,
                                    ci0.secondary_key
                                )
                                .as_bytes(),
                            )
                            .unwrap(), //_ => {}
                    },
                    _ => {}
                }
            }
        }
    }
}

#[tokio::main]
async fn main() {
    env_logger::init();

    match get_args() {
        Err(e) => {
            eprintln!("{:#?}", e);
        }
        Ok((host, start_block, run_mode)) => {
            let (mut req_s, req_r) = unbounded::<ShipRequests>();
            let (res_s, mut res_r) = unbounded::<ShipResultsEx>();
            let fetch_block = run_mode.contains("P");
            let fetch_traces = run_mode.contains("T");
            let fetch_deltas = run_mode.contains("D");
            let ws = async {
                get_sink_stream(&host, req_r, res_s).await;
            };
            let dumper = async {
                let mut delta_file: Option<File> = None;
                let mut perf_file: Option<File> = None;
                if run_mode.contains("D") {
                    delta_file = Some(File::create("deltas.txt").unwrap());
                }
                if run_mode.contains("P") {
                    let mut f = File::create("perf.txt").unwrap();
                    f.write_all(format!("type,block#,first-action,#actions,cpu,net\n").as_bytes());
                    perf_file = Some(f);
                }

                req_s
                    .send(ShipRequests::get_status_request_v0(GetStatusRequestV0 {}))
                    .await;
                let mut last_block: u32 = 0;
                let mut direction;
                let mut current: u32;
                if start_block > 0 {
                    direction = true;
                    current = start_block as u32;
                } else {
                    direction = false;
                    current = 0;
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

                let mut last_fetched: u32 = 0;
                loop {
                    //   println!("Current= {} -> {}", current, last_fetched);
                    let sr: ShipResultsEx = res_r.next().await.unwrap();
                    match sr {
                        ShipResultsEx::Status(st) => {
                            last_block = st.chain_state_end_block;
                            info!(
                                "Chain - {} -> {}",
                                st.chain_id.unwrap_or(String::from("?NONE?")),
                                last_block
                            );
                            if direction {
                                last_fetched = min(current + 1 + 150, last_block);

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
                            } else {
                                direction = true;
                                current = (st.chain_state_end_block as i64 + start_block) as u32; // start_block would be negative here
                                last_fetched = min(current + 1 + 150, st.chain_state_end_block);

                                req_s
                                    .send(ShipRequests::get_blocks_request_v0(GetBlocksRequestV0 {
                                        start_block_num: current,
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
                        }
                        ShipResultsEx::BlockResult(blo) => match &blo.this_block {
                            Some(bp) => {
                                current = bp.block_num;

                                if !blo.traces.is_empty() {
                                    info!("\t-{} #Trace", blo.traces.len())
                                }
                                if run_mode.contains("P") {
                                    match &perf_file {
                                        Some(x) => handle_performance(&x, current, &blo),
                                        None => {}
                                    }
                                }
                                if run_mode.contains("D") {
                                    //  let x = delta_file.unwrap();
                                    match &delta_file {
                                        Some(x) => handle_delta(&x, current, &blo),
                                        None => {}
                                    }
                                }

                                if (current + 1) >= last_block {
                                    debug!("{} reached end {}", current, last_block);

                                    req_s
                                        .send(ShipRequests::get_status_request_v0(
                                            GetStatusRequestV0 {},
                                        ))
                                        .await;
                                // delta_file.sync_data();
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
                                        //   delta_file.sync_data();
                                    }
                                }
                            }
                            None => {
                                debug!("{} {} empty", current, last_block);
                                req_s
                                    .send(ShipRequests::get_status_request_v0(
                                        GetStatusRequestV0 {},
                                    ))
                                    .await;
                                // delta_file.sync_all();
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
