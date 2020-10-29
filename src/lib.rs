use futures_channel::mpsc::{UnboundedReceiver, UnboundedSender};
use futures_util::{future, pin_mut, SinkExt, StreamExt};
//use log::*;
//use std::io::prelude::*;
use rust_embed::RustEmbed;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;
use url::Url;
// `error_chain!` can recurse deeply
//#![recursion_limit = "1024"]
//
use errors::{Error, ErrorKind, Result};
#[macro_use]
extern crate lazy_static;
pub mod errors;
pub mod shipper_types;

use crate::shipper_types::{ShipRequests, ShipResultsEx};
use libabieos_sys::{AbiFiles, ABIEOS};

#[derive(RustEmbed)]
#[folder = "resources/"]
pub struct ShipAbiFiles;

pub const EOSIO_SYSTEM: &str = "eosio";

pub async fn get_sink_stream(
    server_url: &str,
    mut in_tx: UnboundedReceiver<ShipRequests>,
    out_rx: UnboundedSender<ShipResultsEx>,
) -> Result<()> {
    let r = connect_async(Url::parse(server_url).expect("Can't connect to server")).await?;
    let socket = r.0;
    let abi_f = String::from_utf8(AbiFiles::get("abi.abi.json").unwrap().as_ref().to_vec())?;
    let abi: ABIEOS = ABIEOS::new_with_abi(EOSIO_SYSTEM, &abi_f)?;
    let (mut sink, mut stream) = socket.split();
    match stream.next().await {
        Some(msg) => {
            let msg_text = msg
                .map_err(|e| {
                    abi.destroy();
                    Error::with_chain(e, "get_sink_stream fail")
                })?
                .into_text()
                .map_err(|e| {
                    abi.destroy();
                    Error::with_chain(e, "get_sink_stream into_text")
                })?;
            let shipper_abi = ABIEOS::new_with_abi(EOSIO_SYSTEM, &msg_text).map_err(|e| {
                abi.destroy();
                Error::with_chain(e, "parsing shipper abi")
            })?;

            let out_loop = async {
                loop {
                    let data = stream
                        .next()
                        .await
                        .unwrap()
                        .expect("get_status_request_v0 Response error")
                        .into_data();

                    let r = ShipResultsEx::from_bin(&shipper_abi, &data).unwrap();

                    out_rx.unbounded_send(r).expect("Didn't send");
                }
            };
            let in_loop = async {
                loop {
                    let data: ShipRequests = in_tx.next().await.unwrap();

                    match data {
                        ShipRequests::get_status_request_v0(r) => {
                            let req = r.to_bin(&shipper_abi).unwrap();
                            let msg = Message::Binary(req);
                            sink.send(msg).await.expect("Didn't send");
                        }
                        ShipRequests::get_blocks_request_v0(br) => {
                            let req = br.to_bin(&shipper_abi).unwrap();
                            let msg = Message::Binary(req);
                            sink.send(msg).await.expect("Didn't send");
                        }
                        ShipRequests::get_blocks_ack_request_v0(ar) => {
                            let req = ar.to_bin(&shipper_abi).unwrap();
                            let msg = Message::Binary(req);
                            sink.send(msg).await.expect("Didn't send");
                        }
                        ShipRequests::quit => {
                            eprintln!("QUIT");
                            &sink.close();
                            break;
                        }
                    }
                }
            };
            pin_mut!(in_loop, out_loop);
            future::join(in_loop, out_loop).await;

            Ok(())
        }
        None => {
            abi.destroy();
            Err(ErrorKind::ExpectedABI.into())
        }
    }
}
