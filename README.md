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

The aim is to provide a callback function
* which will be called when a new block is received


## Status

 _early_ stages.


## Build notes

* This should work with the current release of EOSIO (2.0.x), although I actually use the _develop_ branch for development. 
