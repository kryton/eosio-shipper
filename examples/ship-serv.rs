// `error_chain!` can recurse deeply
#![recursion_limit = "1024"]

// Import the macro. Don't forget to add `error-chain` in your
// `Cargo.toml`!
#[macro_use]
extern crate error_chain;
#[macro_use]
extern crate log;

use eosio_shipper::errors::{Error, Result};
use futures_util::{SinkExt, StreamExt};
use std::env;
use std::net::SocketAddr;
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::accept_async;
use tokio_tungstenite::tungstenite::Message;
//use tokio_tungstenite::tungstenite;
use eosio_shipper::shipper_types::{
    BlockHeader, BlockPosition, GetBlocksResultV1, GetStatusResponseV0, ShipRequests, ShipResults,
    SignedBlock, SignedBlockHeader, SignedBlockV1,
};
use eosio_shipper::{ShipAbiFiles, EOSIO_SYSTEM};
use libabieos_sys::{AbiFiles, ABIEOS};

fn handle_status_response() -> String {
    let gsr = GetStatusResponseV0 {
        head: BlockPosition {
            block_num: 10,
            block_id: gen_block_id(10),
        },
        last_irreversible: BlockPosition {
            block_num: 8,
            block_id: gen_block_id(8),
        },
        trace_begin_block: 0,
        trace_end_block: 2,
        chain_state_begin_block: 0,
        chain_state_end_block: 2,
        chain_id: Some(
            "00a7a47738ccf44cd09f38a24aed9d95c0d650d29dd23670ffaa75c483c92b44".to_string(),
        ),
    };
    let x = ShipResults::get_status_result_v0(gsr);
    let json = serde_json::to_string(&x);
    return json.unwrap();
}

//
fn gen_block_id(block_num: u32) -> String {
    let res = format!(
        "{:0>56}{:0>8x}",
        "00a7a475a5fce4a49cc43d7131e1a86efeeac498703e38319aad0759", block_num
    );

    assert_eq!(64, res.len());
    res
}

fn gen_block(
    shipper_abi: &ABIEOS,
    contract_name: &str,
    block_num: u32,
    end_block: u32,
    send_block: bool,
    send_delta: bool,
    send_trace: bool,
) -> String {
    let trace_hex = shipper_abi
        .json_to_hex(contract_name, "transaction_trace[]", "[]")
        .unwrap();
    let delta_hex = shipper_abi
        .json_to_hex(contract_name, "table_delta[]", "[]")
        .unwrap();
    let signed_block = SignedBlockV1 {
        signed_header: SignedBlockHeader {
            header: BlockHeader {
                timestamp: "2018-06-01T12:00:00.000".to_string(),
                producer: "ship_serv".to_string(),
                confirmed: 0,
                previous: if block_num == 1 {
                    "0000000000000000000000000000000000000000000000000000000000000000".to_string()
                } else {
                    gen_block_id(block_num)
                },
                transaction_mroot:
                    "0000000000000000000000000000000000000000000000000000000000000000".to_string(),
                action_mroot: "747d103e24c96deb1beebc13eb31f7c2188126946c8677dfd1691af9f9c03ab1"
                    .to_string(),
                schedule_version: 0,
                new_producers: None,
                header_extensions: vec![],
            },
            producer_signature:
                "SIG_K1_111111111111111111111111111111111111111111111111111111111111111116uk5ne"
                    .to_string(),
        },
        prune_state: 0,
        transactions: vec![],
        block_extensions: vec![],
    };
    let mut gbr = GetBlocksResultV1 {
        head: BlockPosition {
            block_num,
            block_id: gen_block_id(end_block),
        },
        last_irreversible: BlockPosition {
            block_num,
            block_id: gen_block_id(end_block),
        },
        this_block: Some(BlockPosition {
            block_num,
            block_id: gen_block_id(block_num),
        }),
        prev_block: if block_num == 1 {
            None
        } else {
            Some(BlockPosition {
                block_num: block_num.checked_sub(1).unwrap(),
                block_id: gen_block_id(block_num.checked_sub(1).unwrap()),
            })
        },
        block: Some(SignedBlock::signed_block_v1(signed_block)),
        traces: Some(trace_hex),
        deltas: Some(delta_hex),
    };
    let x = ShipResults::get_blocks_result_v1(gbr);
    let json = serde_json::to_string(&x);
    return json.unwrap();
}

async fn accept_connection(peer: SocketAddr, stream: TcpStream) -> Result<()> {
    //let abi_f = AbiFiles::get("abi.abi.json").unwrap();
    //let abi_js = String::from_utf8(abi_f.as_ref().to_vec())?;
    //let abi = ABIEOS::new_with_abi(EOSIO_SYSTEM, &abi_js).unwrap();

    //
    let ship_abi_f = ShipAbiFiles::get("shipper.abi.json").unwrap();
    let ship_abi_js: String = String::from_utf8(ship_abi_f.as_ref().to_vec()).unwrap();

    let shipper_abi = ABIEOS::new_with_abi(EOSIO_SYSTEM, &ship_abi_js)?;
    info!("New WS Stream connection: {}", peer);

    let mut ws_stream = accept_async(stream).await.expect("Failed to accept");

    info!("New WebSocket connection: {}", peer);
    let msg: Message = Message::Text(ship_abi_js.clone());
    ws_stream.send(msg).await?;
    let mut current_block: u32 = 0;
    let mut end_block: u32 = 0;
    let mut window_end: u32 = 0;
    let mut window_size: u32 = 0;
    let mut send_block = false;
    let mut send_delta = false;
    let mut send_trace = false;
    loop {
        let msg = ws_stream
            .next()
            .await
            .unwrap()
            .expect("get_status_request_v0 Response error")
            .into_data();

        let sr = ShipRequests::from_bin(&shipper_abi, &msg)?;

        info!("SR-{:?}", sr);
        match sr {
            ShipRequests::get_status_request_v0(_) => {
                let json = handle_status_response();
                debug!("{}", json);
                let bin = shipper_abi
                    .json_to_bin(EOSIO_SYSTEM, "result", &json)
                    .unwrap();
                let msg = Message::Binary(bin);
                ws_stream.send(msg).await?;
            }
            ShipRequests::get_blocks_request_v0(br) => {
                end_block = br.end_block_num;
                current_block = br.start_block_num;
                window_size = br.max_messages_in_flight;
                window_end = if window_size == u32::MAX {
                    window_size
                } else {
                    br.start_block_num + window_size
                };
                send_block = br.fetch_block;
                send_delta = br.fetch_deltas;
                send_trace = br.fetch_traces;
                while current_block < window_end {
                    let json = gen_block(
                        &shipper_abi,
                        EOSIO_SYSTEM,
                        end_block,
                        current_block,
                        send_block,
                        send_trace,
                        send_delta,
                    );
                    let bin = shipper_abi
                        .json_to_bin(EOSIO_SYSTEM, "result", &json)
                        .unwrap();
                    let msg = Message::Binary(bin);
                    info!("Sent Block {}", current_block);
                    ws_stream.send(msg).await?;

                    current_block += 1;
                }
                info!("{:?}", br);
            }
            ShipRequests::get_blocks_ack_request_v0(ar) => {
                info!("{:?}", ar);
            }
            ShipRequests::quit => {
                break;
            }
        };

        //   ws_stream.send(Message::Text(String::from("Thank you")));
    }
    shipper_abi.destroy();
    Ok(())
}

fn get_args() -> Result<(String, String)> {
    let args: Vec<String> = env::args().collect();
    let listen_ip_port = {
        if args.len() > 1 {
            &args[1]
        } else {
            "0.0.0.0:9999"
            //  "https://api.testnet.eos.io"
        }
    };
    let input_file = {
        if args.len() > 2 {
            &args[2]
        } else {
            "input_file.txt"
        }
    };

    Ok((listen_ip_port.parse().unwrap(), input_file.parse().unwrap()))
}

#[tokio::main]
async fn main() {
    env_logger::init();

    match get_args() {
        Err(e) => {
            eprintln!("{:#?}", e);
        }
        Ok((listen_port, input_file)) => {
            let mut listener = TcpListener::bind(&listen_port).await.expect("Can't listen");

            info!("Listening on: {}", listen_port);
            loop {
                let (socket, _) = listener.accept().await.unwrap();
                // process_socket(socket).await;
                let peer = socket
                    .peer_addr()
                    .expect("connected streams should have a peer address");
                info!("Got one {}", peer);

                let r = accept_connection(peer, socket).await;
            }
            /*
            while let Ok((stream, _)) = listener.accept().await {
                let peer = stream
                    .peer_addr()
                    .expect("connected streams should have a peer address");
                info!("Peer address: {}", peer);

                tokio::spawn(accept_connection(peer, stream));
            }

             */
        }
    }

    println!("Hello, world!");
}
