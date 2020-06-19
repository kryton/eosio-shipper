# EOSIO Shipper
## What is it?

An framework to allow Rust services to communicate with [EOSIO](https://github.com/EOSIO/eos) node via the [State History Plugin (SHiP)](https://developers.eos.io/manuals/eos/latest/nodeos/plugins/state_history_plugin/index)

The SHiP endpoint allows you to monitor
* blocks being produced
* table deltas
* traces

## What is it not?

This is not an API to allow you to write contracts in rust.  (see [eosio-rust](https://github.com/sagan-software/eosio-rust) for that), 

This is not the HTTP API  (see [eosio-client-api](https://crates.io/crates/eosio-client-api) for that)

## How to connect to nodeos.
This code interacts via the Websocket API specified when starting nodeos.
```
--plugin eosio::state_history_plugin \
--trace-history \
--chain-state-history \
--state-history-endpoint=127.0.0.1:9999 
```
In this example the endpoint would be ws://127.0.0.1:9999/

I don't know of any public/open SHiP endpoints.

The library is designed to run in a seperate thread, and uses [futures_channel::mpsc::unbounded](https://docs.rs/futures-channel-preview/0.3.0-alpha.19/futures_channel/mpsc/fn.unbounded.html) to communicate requests/responses to the SHiP endpoint itself.

The example program [ship-dumper](/examples/ship-dumper.rs) provides a simple example of how to use this tool.  (feedback welcome)

## Status

 _early_ stages.

## Build notes

* This should work with the current release of EOSIO (2.0.x), although I actually use the _develop_ branch for development. 

# SHiP protocol (rough notes)
SHiP uses websockets to communicate. 
* It seems to only process one command at a time. so don't just fire off multiple commands and expect it to work.
* Upon connection Nodeos sends the ABI file required to parse the messages.
* Once you have this, you can start sending out commands. As of writing this there are 3
  * Status - gives you the chain ID, and info on how many blocks have been written
  * Get Blocks - fetch a group of blocks, N at a time (window parameter). you have the option of fetching blocks, traces, and table deltas
  * Ack Blocks - Nodeos will only send N blocks.. you need to send a ACK to keep progressing through your fetch
* The response you get back is slightly different depending on what version you are running against. The demo program just sends multiple Get Blocks until it hits the end, and then gets a new status to see where the end is.. 

if you request blocks past the end, it will return 'None'.. 
