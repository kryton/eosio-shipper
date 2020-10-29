//use serde::de::{SeqAccess, Visitor};
use std::collections::HashSet;

use serde::ser::SerializeTuple;
use serde::{Deserialize, Serialize, Serializer};
// source from work done by @lucas3fonseca and @leordev
// plan is to move to their work once it is public
use crate::errors::Result;
use chrono::{DateTime, Utc};
use flate2::read::ZlibDecoder;
use libabieos_sys::{eosio_datetime_format, hex_to_bin, ABIEOS};
use log::*;
use std::fmt;
use std::io::prelude::*;

lazy_static! {
    static ref ROWTYPES: HashSet<String> = vec![
        String::from("account"),
        String::from("account_metadata"),
        String::from("code"),
        String::from("contract_table"),
        String::from("contract_row"),
        String::from("contract_index64"),
        String::from("contract_index128"),
        String::from("contract_index256"),
        String::from("contract_index_double"),
        String::from("contract_index_long_double"),
        // key_value
        // global_property
        // generated_transaction
        // protocol_state
        // permission
        // permission_link
        String::from("resource_limits"),
        String::from("resource_usage"),
        String::from("resource_limits_state"),
        String::from("resource_limits_config"),

    ] .into_iter().collect();
}

//#[derive( Deserialize)]
pub struct Checksum256 {
    pub value: [u8; 32],
}

impl Checksum256 {
    pub fn to_string(&self) -> String {
        let mut hex_string = String::from("");
        for v in &self.value {
            let hex = format!("{:02x}", v);
            hex_string += hex.as_str();
        }
        hex_string
    }
}

impl fmt::Display for Checksum256 {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.to_string())
    }
}

impl fmt::Debug for Checksum256 {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.to_string())
    }
}

#[allow(non_camel_case_types)]
#[derive(Debug, Deserialize)]
pub enum ShipRequests {
    get_status_request_v0(GetStatusRequestV0),
    get_blocks_request_v0(GetBlocksRequestV0),
    get_blocks_ack_request_v0(GetBlocksACKRequestV0),
    quit,
}

impl ShipRequests {
    pub fn from_bin(shipper_abi: &ABIEOS, bin: &[u8]) -> Result<ShipRequests> {
        let mut s: String = String::from("");
        for b in bin {
            let hex = format!("{:02x}", b);
            s += hex.as_str();
        }
        let json = shipper_abi.hex_to_json("eosio", "request", s.as_bytes())?;

        let sr: ShipRequests = serde_json::from_str(&json)?;
        Ok(sr)
    }
}

impl Serialize for ShipRequests {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut m = serializer.serialize_tuple(2)?;
        match self {
            ShipRequests::get_status_request_v0(k) => {
                m.serialize_element("get_status_request_v0")?;
                m.serialize_element(k)?;
            }
            ShipRequests::get_blocks_request_v0(k) => {
                m.serialize_element("get_blocks_request_v0")?;
                m.serialize_element(k)?;
            }
            ShipRequests::get_blocks_ack_request_v0(k) => {
                m.serialize_element("get_blocks_ack_request_v0")?;
                m.serialize_element(k)?;
            }
            ShipRequests::quit => {
                m.serialize_element("quit")?;
                m.serialize_element(&{})?;
            }
        }
        m.end()
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GetStatusRequestV0 {}

impl GetStatusRequestV0 {
    pub fn to_bin(&self, shipper_abi: &ABIEOS) -> Result<Vec<u8>> {
        let r: ShipRequests = ShipRequests::get_status_request_v0 {
            0: GetStatusRequestV0 {},
        };
        let _json = serde_json::to_string(&r)?;
        let json = "[\"get_status_request_v0\",{}]";
        let trx = shipper_abi.json_to_bin("eosio", "request", &json);

        Ok(trx?)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GetBlocksRequestV0 {
    pub start_block_num: u32,
    pub end_block_num: u32,
    pub max_messages_in_flight: u32,
    pub have_positions: Vec<BlockPosition>,
    pub irreversible_only: bool,
    pub fetch_block: bool,
    pub fetch_traces: bool,
    pub fetch_deltas: bool,
}

impl GetBlocksRequestV0 {
    pub fn to_bin(&self, shipper_abi: &ABIEOS) -> Result<Vec<u8>> {
        let _json = String::from(serde_json::to_string(&self)?);
        let json: String =
            String::from("[\"get_blocks_request_v0\",") + &_json + &String::from("]");
        let trx = shipper_abi.json_to_bin("eosio", "request", &json);
        Ok(trx?)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GetBlocksACKRequestV0 {
    pub num_messages: u32,
}

impl GetBlocksACKRequestV0 {
    pub fn to_bin(&self, shipper_abi: &ABIEOS) -> Result<Vec<u8>> {
        let _json = String::from(serde_json::to_string(&self)?);
        let json: String =
            String::from("[\"get_blocks_ack_request_v0\",") + &_json + &String::from("]");
        let trx = shipper_abi.json_to_bin("eosio", "request", &json);
        Ok(trx?)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BlockPosition {
    pub block_num: u32,
    pub block_id: String,
}

#[allow(non_camel_case_types)]
#[derive(Debug, Deserialize)]
//#[serde(untagged)]
pub enum ShipResults {
    get_status_result_v0(GetStatusResponseV0),
    get_blocks_result_v0(GetBlocksResultV0),
    get_blocks_result_v1(GetBlocksResultV1),
}

impl Serialize for ShipResults {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut m = serializer.serialize_tuple(2)?;
        match self {
            ShipResults::get_status_result_v0(k) => {
                m.serialize_element("get_status_result_v0")?;
                m.serialize_element(k)?;
            }
            ShipResults::get_blocks_result_v0(k) => {
                m.serialize_element("get_blocks_result_v0")?;
                m.serialize_element(k)?;
            }
            ShipResults::get_blocks_result_v1(k) => {
                m.serialize_element("get_blocks_result_v1")?;
                m.serialize_element(k)?;
            }
        }
        m.end()
    }
}

#[derive(Debug, Deserialize)]
pub enum ShipResultsEx {
    Status(GetStatusResponseV0),
    BlockResult(GetBlocksResultV0Ex),
}

impl ShipResultsEx {
    pub fn from_bin(shipper_abi: &ABIEOS, bin: &[u8]) -> Result<ShipResultsEx> {
        let mut s: String = String::from("");
        for b in bin {
            let hex = format!("{:02x}", b);
            s += hex.as_str();
        }
        let json = shipper_abi.hex_to_json("eosio", "result", s.as_bytes())?;
        debug!("{}", json);
        let sr: ShipResults = serde_json::from_str(&json)?;
        match sr {
            ShipResults::get_blocks_result_v0(br) => {
                let traces = match br.traces {
                    None => vec![],
                    Some(t) => ShipResultsEx::convert_traces(shipper_abi, &t.as_bytes()).unwrap(),
                };
                let deltas = match br.deltas {
                    None => vec![],
                    Some(t) => ShipResultsEx::convert_deltas(shipper_abi, &t.as_bytes()).unwrap(),
                };
                let (block, trans) = match br.block {
                    None => (None, vec![]),
                    Some(t) => {
                        let sb: SignedBlock =
                            ShipResultsEx::convert_block_v0(shipper_abi, &t.as_bytes()).unwrap();
                        let v_ot: Vec<Option<Transaction>> = sb.get_trx(shipper_abi);
                        (Some(sb), v_ot)
                    }
                };

                let br_ex = GetBlocksResultV0Ex {
                    head: br.head,
                    last_irreversible: br.last_irreversible,
                    this_block: br.this_block,
                    prev_block: br.prev_block,
                    block: block,
                    traces: traces,
                    deltas: deltas,
                    transactions: trans,
                };

                Ok(ShipResultsEx::BlockResult(br_ex))
            }
            ShipResults::get_blocks_result_v1(br) => {
                let traces = match br.traces {
                    None => vec![],
                    Some(t) => ShipResultsEx::convert_traces(shipper_abi, &t.as_bytes()).unwrap(),
                };
                let deltas = match br.deltas {
                    None => vec![],
                    Some(t) => ShipResultsEx::convert_deltas(shipper_abi, &t.as_bytes()).unwrap(),
                };
                let (block, trans) = match br.block {
                    None => (None, vec![]),
                    Some(t) => {
                        let v_ot: Vec<Option<Transaction>> = t.get_trx(shipper_abi);
                        (Some(t), v_ot)
                    }
                };

                let br_ex = GetBlocksResultV0Ex {
                    head: br.head,
                    last_irreversible: br.last_irreversible,
                    this_block: br.this_block,
                    prev_block: br.prev_block,
                    block: block,
                    traces: traces,
                    deltas: deltas,
                    transactions: trans,
                };

                Ok(ShipResultsEx::BlockResult(br_ex))
            }
            ShipResults::get_status_result_v0(sr) => Ok(ShipResultsEx::Status(sr)),
            //_ => Err("Invalid response to block response".into()),
        }
    }
    fn convert_traces(shipper_abi: &ABIEOS, trace_hex: &[u8]) -> Result<Vec<Traces>> {
        if trace_hex.len() == 0 {
            Ok(vec![])
        } else {
            let json = shipper_abi.hex_to_json("eosio", "transaction_trace[]", trace_hex)?;
            let trace_v: Vec<Traces> = serde_json::from_str(&json)?;
            Ok(trace_v)
        }
    }

    fn convert_deltas(shipper_abi: &ABIEOS, delta_hex: &[u8]) -> Result<Vec<TableDeltaEx>> {
        if delta_hex.len() == 0 {
            Ok(vec![])
        } else {
            let json = shipper_abi.hex_to_json("eosio", "table_delta[]", delta_hex)?;
            let deltas: Vec<TableDeltas> = serde_json::from_str(&json)?;
            let mut delta_ex: Vec<TableDeltaEx> = Vec::with_capacity(deltas.len());
            for delta in deltas {
                match delta {
                    TableDeltas::table_delta_v0(td0) => {
                        let name = td0.name;
                        let mut row_ex: Vec<TableRowEx> = Vec::with_capacity(td0.rows.len());
                        for row in td0.rows {
                            if ROWTYPES.contains(&name) {
                                // println!("{}",name);
                                let _json =
                                    shipper_abi.hex_to_json("eosio", &name, row.data.as_bytes())?;
                                let json = format!("{{\"{}\":{}}}", &name, _json);
                                let r: TableRowTypes = serde_json::from_str(&json)?;
                                row_ex.push(TableRowEx {
                                    present: row.present,
                                    data: r,
                                });
                            } else {
                                row_ex.push(TableRowEx {
                                    present: row.present,
                                    data: TableRowTypes::Other(row.data),
                                });
                            }
                        }
                        let td_ex = TableDeltaEx { name, rows: row_ex };
                        delta_ex.push(td_ex);
                    }
                }
            }
            Ok(delta_ex)
        }
    }

    // v0 only has a signed_block_v0 .. v1 contains a variant here
    fn convert_block_v0(shipper_abi: &ABIEOS, block_hex: &[u8]) -> Result<SignedBlock> {
        let json = shipper_abi.hex_to_json("eosio", "signed_block", block_hex)?;
        let signed_block: SignedBlockV0 = serde_json::from_str(&json)?;
        Ok(SignedBlock::signed_block_v0(signed_block))
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GetStatusResponseV0 {
    pub head: BlockPosition,
    pub last_irreversible: BlockPosition,
    pub trace_begin_block: u32,
    pub trace_end_block: u32,
    pub chain_state_begin_block: u32,
    pub chain_state_end_block: u32,
    pub chain_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GetBlocksResultV0 {
    pub head: BlockPosition,
    pub last_irreversible: BlockPosition,
    pub this_block: Option<BlockPosition>,
    pub prev_block: Option<BlockPosition>,
    pub block: Option<String>,
    pub traces: Option<String>,
    pub deltas: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GetBlocksResultV1 {
    pub head: BlockPosition,
    pub last_irreversible: BlockPosition,
    pub this_block: Option<BlockPosition>,
    pub prev_block: Option<BlockPosition>,
    pub block: Option<SignedBlock>,
    pub traces: Option<String>,
    pub deltas: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct GetBlocksResultV0Ex {
    pub head: BlockPosition,
    pub last_irreversible: BlockPosition,
    pub this_block: Option<BlockPosition>,
    pub prev_block: Option<BlockPosition>,
    pub block: Option<SignedBlock>,
    pub traces: Vec<Traces>,
    pub deltas: Vec<TableDeltaEx>,
    pub transactions: Vec<Option<Transaction>>,
}

#[allow(non_camel_case_types)]
#[derive(Debug, Deserialize)]
pub enum Traces {
    transaction_trace_v0(TransactionTraceV0),
}

impl Serialize for Traces {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut m = serializer.serialize_tuple(2)?;
        match self {
            Traces::transaction_trace_v0(k) => {
                m.serialize_element("get_status_result_v0")?;
                m.serialize_element(k)?;
            }
        }
        m.end()
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TransactionTraceV0 {
    pub id: String,
    pub status: u8,
    pub cpu_usage_us: u32,
    pub net_usage_words: u32,
    pub elapsed: String,
    pub net_usage: String,
    pub scheduled: bool,
    pub action_traces: Vec<ActionTraceVariant>,
    pub account_ram_delta: Option<AccountDelta>,
    pub except: Option<String>,
    pub error_code: Option<u64>,
    pub failed_dtrx_trace: Option<Box<Traces>>,
    pub partial: Option<PartialTransactionVariant>,
}

#[allow(non_camel_case_types)]
#[derive(Debug, Deserialize)]
pub enum PartialTransactionVariant {
    partial_transaction_v0(PartialTransactionV0),
    partial_transaction_v1(PartialTransactionV1),
}

impl Serialize for PartialTransactionVariant {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut m = serializer.serialize_tuple(2)?;
        match self {
            PartialTransactionVariant::partial_transaction_v0(k) => {
                m.serialize_element("partial_transaction_v0")?;
                m.serialize_element(k)?;
            }
            PartialTransactionVariant::partial_transaction_v1(k) => {
                m.serialize_element("partial_transaction_v1")?;
                m.serialize_element(k)?;
            }
        }
        m.end()
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PartialTransactionV0 {
    #[serde(with = "eosio_datetime_format")]
    pub expiration: DateTime<Utc>,
    pub ref_block_num: u16,
    pub ref_block_prefix: u32,
    pub max_net_usage_words: u32,
    pub max_cpu_usage_ms: u8,
    pub delay_sec: u32,
    pub transaction_extensions: Vec<Extension>,
    pub signatures: Vec<String>,
    //    pub context_free_data: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PartialTransactionV1 {
    #[serde(with = "eosio_datetime_format")]
    pub expiration: DateTime<Utc>,
    pub ref_block_num: u16,
    pub ref_block_prefix: u32,
    pub max_net_usage_words: u32,
    pub max_cpu_usage_ms: u8,
    pub delay_sec: u32,
    pub transaction_extensions: Vec<Extension>,
    pub prunable_data: Option<PrunableData>,
}

#[allow(non_camel_case_types)]
#[derive(Debug, Deserialize)]
pub enum ActionTraceVariant {
    action_trace_v0(ActionTraceV0),
    action_trace_v1(ActionTraceV1),
}

impl Serialize for ActionTraceVariant {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut m = serializer.serialize_tuple(2)?;
        match self {
            ActionTraceVariant::action_trace_v0(k) => {
                m.serialize_element("action_trace_v0")?;
                m.serialize_element(k)?;
            }
            ActionTraceVariant::action_trace_v1(k) => {
                m.serialize_element("action_trace_v1")?;
                m.serialize_element(k)?;
            }
        }
        m.end()
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ActionTraceV0 {
    pub action_ordinal: u32,
    pub creator_action_ordinal: u32,
    pub receipt: Option<ActionReceiptVariant>,
    pub receiver: String,
    pub act: Action,
    pub context_free: bool,
    pub elapsed: String,
    pub console: String,
    pub account_ram_deltas: Vec<AccountDelta>,
    pub except: Option<String>,
    pub error_code: Option<u64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ActionTraceV1 {
    pub error_code: Option<u64>,
    pub context_free: bool,
    pub elapsed: String,
    pub except: Option<String>,
    pub account_ram_deltas: Vec<AccountDelta>,
    pub console: String,
    // account_disk_deltas : Vec<>,
    pub action_ordinal: u32,
    pub return_value: String,
    pub creator_action_ordinal: u32,
    pub act: Action,
    pub receiver: String,
    pub receipt: Option<ActionReceiptVariant>,
}

#[allow(non_camel_case_types)]
#[derive(Debug, Deserialize)]
pub enum ActionReceiptVariant {
    action_receipt_v0(ActionReceiptV0),
}

impl Serialize for ActionReceiptVariant {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut m = serializer.serialize_tuple(2)?;
        match self {
            ActionReceiptVariant::action_receipt_v0(k) => {
                m.serialize_element("action_receipt_v0")?;
                m.serialize_element(k)?;
            }
        }
        m.end()
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ActionReceiptV0 {
    pub receiver: String,
    pub act_digest: String,
    pub global_sequence: String,
    pub recv_sequence: String,
    pub auth_sequence: Vec<AccountAuthSequence>,
    pub code_sequence: u32,
    pub abi_sequence: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AccountAuthSequence {
    pub account: String,
    pub sequence: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Action {
    pub account: String,
    pub name: String,
    pub authorization: Vec<PermissionLevel>,
    pub data: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AccountDelta {
    pub account: String,
    pub delta: String,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct PermissionLevel {
    pub actor: String,
    pub permission: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Extension {
    pub r#type: u16,
    pub data: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TableDeltaV0 {
    pub name: String,
    pub rows: Vec<TableRow>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TableRow {
    pub present: bool,
    pub data: String,
}

#[allow(non_camel_case_types)]
#[derive(Debug, Deserialize)]
pub enum TableDeltas {
    table_delta_v0(TableDeltaV0),
}

impl Serialize for TableDeltas {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut m = serializer.serialize_tuple(2)?;
        match self {
            TableDeltas::table_delta_v0(k) => {
                m.serialize_element("table_delta_v0")?;
                m.serialize_element(k)?;
            }
        }
        m.end()
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProducerKey {
    pub name: String,
    pub public_key: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProducerSchedule {
    pub version: u32,
    pub producer: Vec<ProducerKey>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TransactionHeader {
    #[serde(with = "eosio_datetime_format")]
    pub expiration: DateTime<Utc>,
    pub ref_block_num: u16,
    pub ref_block_prefix: u32,
    pub max_net_usage_words: u32,
    pub max_cpu_usage_ms: u8,
    pub delay_sec: u32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Transaction {
    #[serde(flatten)]
    pub header: TransactionHeader,
    pub context_free_actions: Vec<Action>,
    pub actions: Vec<Action>,
    pub transaction_extensions: Vec<Extension>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TransactionReceiptHeader {
    pub status: u8,
    pub cpu_usage_us: u32,
    pub net_usage_words: u32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TransactionReceiptV0 {
    #[serde(flatten)]
    pub header: TransactionReceiptHeader,
    pub trx: TransactionVariantV0,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TransactionReceiptV1 {
    #[serde(flatten)]
    pub header: TransactionReceiptHeader,
    pub trx: TransactionVariantV1,
}

#[allow(non_camel_case_types)]
#[derive(Debug, Deserialize, Clone)]
pub enum TransactionVariantV0 {
    transaction_id(TransactionID),
    packed_transaction(PackedTransactionV0),
    // EOSIO 2.0 uses this?
    packed_transaction_v0(PackedTransactionV0),
}

impl Serialize for TransactionVariantV0 {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut m = serializer.serialize_tuple(2)?;
        match self {
            TransactionVariantV0::transaction_id(k) => {
                m.serialize_element("transaction_id")?;
                m.serialize_element(k)?;
            }
            TransactionVariantV0::packed_transaction_v0(k) => {
                m.serialize_element("packed_transaction_v0")?;
                m.serialize_element(k)?;
            }
            TransactionVariantV0::packed_transaction(k) => {
                m.serialize_element("packed_transaction")?;
                m.serialize_element(k)?;
            }
        }
        m.end()
    }
}

#[allow(non_camel_case_types)]
#[derive(Debug, Deserialize, Clone)]
pub enum TransactionVariantV1 {
    transaction_id(TransactionID),
    packed_transaction_v1(PackedTransactionV1),
}

impl Serialize for TransactionVariantV1 {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut m = serializer.serialize_tuple(2)?;
        match self {
            TransactionVariantV1::transaction_id(k) => {
                m.serialize_element("transaction_id")?;
                m.serialize_element(k)?;
            }
            TransactionVariantV1::packed_transaction_v1(k) => {
                m.serialize_element("packed_transaction_v1")?;
                m.serialize_element(k)?;
            }
        }
        m.end()
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TransactionID {
    pub transaction_id: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PackedTransactionV0 {
    pub signatures: Vec<String>,
    pub compression: u8,
    pub packed_context_free_data: String,
    pub packed_trx: String,
}

impl PackedTransactionV0 {
    fn convert_trx(&self, shipper_abi: &ABIEOS) -> Option<Transaction> {
        if self.packed_trx.len() != 0 {
            match self.compression {
                0 => {
                    let json = shipper_abi
                        .hex_to_json("eosio", "transaction", self.packed_trx.as_bytes())
                        .unwrap();
                    let trace_v: Transaction = serde_json::from_str(&json).unwrap();
                    //self.transaction= Some(trace_v);
                    Some(trace_v)
                }
                1 => {
                    let bin_compressed_trx = hex_to_bin(&self.packed_trx);
                    let mut d = ZlibDecoder::new(bin_compressed_trx.as_slice());
                    let mut buffer = Vec::new();
                    d.read_to_end(&mut buffer).unwrap();
                    let json = shipper_abi
                        .bin_to_json("eosio", "transaction", &buffer)
                        .unwrap();
                    let trace_v: Transaction = serde_json::from_str(&json).unwrap();
                    Some(trace_v)
                }
                _ => {
                    error!(
                        "Invalid compression level of {}. Skipped (PackedTransactionV0)",
                        self.compression
                    );
                    None
                }
            }
        } else {
            None
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PackedTransactionV1 {
    pub compression: u8,
    pub prunable_data: PrunableData,
    pub packed_trx: String,
}

impl PackedTransactionV1 {
    fn convert_trx(&self, shipper_abi: &ABIEOS) -> Option<Transaction> {
        if self.packed_trx.len() != 0 {
            match self.compression {
                0 => {
                    let json = shipper_abi
                        .hex_to_json("eosio", "transaction", self.packed_trx.as_bytes())
                        .unwrap();
                    let trace_v: Transaction = serde_json::from_str(&json).unwrap();
                    Some(trace_v)
                }
                1 => {
                    let bin_compressed_trx = hex_to_bin(&self.packed_trx);
                    let mut d = ZlibDecoder::new(bin_compressed_trx.as_slice());
                    let mut buffer = Vec::new();
                    d.read_to_end(&mut buffer).unwrap();
                    let json = shipper_abi
                        .bin_to_json("eosio", "transaction", &buffer)
                        .unwrap();
                    let trace_v: Transaction = serde_json::from_str(&json).unwrap();
                    Some(trace_v)
                }
                _ => {
                    error!(
                        "Invalid compression level of {}. Skipped (PackedTransactionV1)",
                        self.compression
                    );
                    None
                }
            }
        } else {
            None
        }
    }
}

#[allow(non_camel_case_types)]
#[derive(Debug, Deserialize, Clone)]
pub enum PrunableData {
    prunable_data_full_legacy(PrunableDataFullLegacy),
    prunable_data_none(PrunableDataNone),
    prunable_data_partial(PrunableDataPartial),
    prunable_data_full(PrunableDataFull),
}

impl Serialize for PrunableData {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut m = serializer.serialize_tuple(2)?;
        match self {
            PrunableData::prunable_data_full_legacy(k) => {
                m.serialize_element("prunable_data_full_legacy")?;
                m.serialize_element(k)?;
            }
            PrunableData::prunable_data_none(k) => {
                m.serialize_element("prunable_data_none")?;
                m.serialize_element(k)?;
            }
            PrunableData::prunable_data_partial(k) => {
                m.serialize_element("prunable_data_partial")?;
                m.serialize_element(k)?;
            }
            PrunableData::prunable_data_full(k) => {
                m.serialize_element("prunable_data_full")?;
                m.serialize_element(k)?;
            }
        }
        m.end()
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PrunableDataFullLegacy {
    pub signatures: Vec<String>,
    pub packed_context_segments: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PrunableDataFull {
    pub signatures: Vec<String>,
    pub context_free_segments: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PrunableDataPartial {
    pub signatures: Vec<String>,
    pub context_free_segments: Vec<ContextFreeSegmentType>,
}

#[allow(non_camel_case_types)]
#[derive(Debug, Deserialize, Clone)]
pub enum ContextFreeSegmentType {
    signature(String),
    bytes(String),
}

impl Serialize for ContextFreeSegmentType {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut m = serializer.serialize_tuple(2)?;
        match self {
            ContextFreeSegmentType::signature(k) => {
                m.serialize_element("signature")?;
                m.serialize_element(k)?;
            }
            ContextFreeSegmentType::bytes(k) => {
                m.serialize_element("bytes")?;
                m.serialize_element(k)?;
            }
        }
        m.end()
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PrunableDataNone {
    pub prunable_digest: String,
}

#[allow(non_camel_case_types)]
#[derive(Debug, Deserialize)]
pub enum SignedBlock {
    signed_block_v0(SignedBlockV0),
    signed_block_v1(SignedBlockV1),
}

impl Serialize for SignedBlock {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut m = serializer.serialize_tuple(2)?;
        match self {
            SignedBlock::signed_block_v0(k) => {
                m.serialize_element("signed_block_v0")?;
                m.serialize_element(k)?;
            }
            SignedBlock::signed_block_v1(k) => {
                m.serialize_element("signed_block_v1")?;
                m.serialize_element(k)?;
            }
        }
        m.end()
    }
}

impl SignedBlock {
    pub fn get_trx(&self, shipper_abi: &ABIEOS) -> Vec<Option<Transaction>> {
        let mut vo_t: Vec<Option<Transaction>> = vec![];
        match self {
            SignedBlock::signed_block_v0(k) => {
                //  let mut kt = k.transactions.clone();
                for t in &k.transactions {
                    match &t.trx {
                        TransactionVariantV0::transaction_id(_) => {
                            vo_t.push(None);
                        }
                        TransactionVariantV0::packed_transaction(pt) => {
                            let transaction = pt.convert_trx(shipper_abi);
                            vo_t.push(transaction);
                        }
                        TransactionVariantV0::packed_transaction_v0(pt) => {
                            let transaction = pt.convert_trx(shipper_abi);
                            vo_t.push(transaction);
                        }
                    }
                }
            }
            SignedBlock::signed_block_v1(k) => {
                // let mut kt = k.transactions.clone();
                for t in &k.transactions {
                    match &t.trx {
                        TransactionVariantV1::transaction_id(_tt) => vo_t.push(None),
                        TransactionVariantV1::packed_transaction_v1(pt) => {
                            let transaction = pt.convert_trx(shipper_abi);
                            vo_t.push(transaction);
                        }
                    }
                }
            }
        }
        vo_t
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BlockHeader {
    pub timestamp: String,
    pub producer: String,
    pub confirmed: u16,
    pub previous: String,
    pub transaction_mroot: String,
    pub action_mroot: String,
    pub schedule_version: u32,
    pub new_producers: Option<ProducerSchedule>,
    pub header_extensions: Vec<Extension>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SignedBlockHeader {
    #[serde(flatten)]
    pub header: BlockHeader,
    pub producer_signature: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SignedBlockV0 {
    #[serde(flatten)]
    pub signed_header: SignedBlockHeader,
    pub transactions: Vec<TransactionReceiptV0>,
    pub block_extensions: Vec<Extension>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SignedBlockV1 {
    #[serde(flatten)]
    pub signed_header: SignedBlockHeader,
    pub prune_state: u8,
    pub transactions: Vec<TransactionReceiptV1>,
    pub block_extensions: Vec<Extension>,
}

#[derive(Debug, Deserialize)]
pub struct TableDeltaEx {
    pub name: String,
    pub rows: Vec<TableRowEx>,
}

#[derive(Debug, Deserialize)]
pub struct TableRowEx {
    pub present: bool,
    pub data: TableRowTypes,
}

#[allow(non_camel_case_types)]
#[derive(Debug, Deserialize)]
pub enum TableRowTypes {
    account(Account),
    account_metadata(AccountMetadata),
    code(Code),
    contract_table(ContractTable),
    contract_row(ContractRow),
    contract_index64(ContractIndex64),
    contract_index128(ContractIndex128),
    contract_index256(ContractIndex256),
    contract_index_double(ContractIndexDouble),
    // TODO float 128 it accepts the string.. but no idea next step
    contract_index_long_double(ContractIndexLongDouble),

    resource_limits(ResourceLimits),
    resource_usage(ResourceUsage),
    resource_limits_state(ResourceLimitsState),
    resource_limits_config(ResourceLimitsConfig),

    Other(String),
}

impl Serialize for TableRowTypes {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut m = serializer.serialize_tuple(2)?;
        match self {
            TableRowTypes::account(k) => {
                m.serialize_element("account")?;
                m.serialize_element(k)?;
            }
            TableRowTypes::account_metadata(k) => {
                m.serialize_element("account_metadata")?;
                m.serialize_element(k)?;
            }
            TableRowTypes::code(k) => {
                m.serialize_element("code")?;
                m.serialize_element(k)?;
            }
            TableRowTypes::contract_table(k) => {
                m.serialize_element("contract_table")?;
                m.serialize_element(k)?;
            }
            TableRowTypes::contract_row(k) => {
                m.serialize_element("contract_row")?;
                m.serialize_element(k)?;
            }
            TableRowTypes::contract_index64(k) => {
                m.serialize_element("contract_index64")?;
                m.serialize_element(k)?;
            }
            TableRowTypes::contract_index128(k) => {
                m.serialize_element("contract_index128")?;
                m.serialize_element(k)?;
            }
            TableRowTypes::contract_index256(k) => {
                m.serialize_element("contract_index256")?;
                m.serialize_element(k)?;
            }
            TableRowTypes::contract_index_double(k) => {
                m.serialize_element("contract_index_double")?;
                m.serialize_element(k)?;
            }
            TableRowTypes::contract_index_long_double(k) => {
                m.serialize_element("contract_index_long_double")?;
                m.serialize_element(k)?;
            }
            TableRowTypes::resource_limits(k) => {
                m.serialize_element("resource_limits")?;
                m.serialize_element(k)?;
            }
            TableRowTypes::resource_usage(k) => {
                m.serialize_element("resource_usage")?;
                m.serialize_element(k)?;
            }
            TableRowTypes::resource_limits_state(k) => {
                m.serialize_element("resource_limits_state")?;
                m.serialize_element(k)?;
            }
            TableRowTypes::resource_limits_config(k) => {
                m.serialize_element("resource_limits_config")?;
                m.serialize_element(k)?;
            }
            TableRowTypes::Other(_k) => {}
        }
        m.end()
    }
}

#[allow(non_camel_case_types)]
#[derive(Debug, Deserialize)]
pub enum ContractTable {
    contract_table_v0(ContractTableV0),
}

impl Serialize for ContractTable {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut m = serializer.serialize_tuple(2)?;
        match self {
            ContractTable::contract_table_v0(k) => {
                m.serialize_element("contract_table_v0")?;
                m.serialize_element(k)?;
            }
        }
        m.end()
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ContractTableV0 {
    pub code: String,
    pub scope: String,
    pub table: String,
    pub payer: String,
}

#[allow(non_camel_case_types)]
#[derive(Debug, Deserialize)]
pub enum ContractRow {
    contract_row_v0(ContractRowV0),
}

impl Serialize for ContractRow {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut m = serializer.serialize_tuple(2)?;
        match self {
            ContractRow::contract_row_v0(k) => {
                m.serialize_element("contract_row_v0")?;
                m.serialize_element(k)?;
            }
        }
        m.end()
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ContractRowV0 {
    pub code: String,
    pub scope: String,
    pub table: String,
    pub primary_key: String,
    pub payer: String,
    pub value: String,
}

#[allow(non_camel_case_types)]
#[derive(Debug, Deserialize)]
pub enum ContractIndex64 {
    contract_index64_v0(ContractIndex64V0),
}

impl Serialize for ContractIndex64 {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut m = serializer.serialize_tuple(2)?;
        match self {
            ContractIndex64::contract_index64_v0(k) => {
                m.serialize_element("contract_index64_v0")?;
                m.serialize_element(k)?;
            }
        }
        m.end()
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ContractIndex64V0 {
    pub code: String,
    pub scope: String,
    pub table: String,
    pub primary_key: String,
    pub payer: String,
    pub secondary_key: String,
}

#[allow(non_camel_case_types)]
#[derive(Debug, Deserialize)]
pub enum ContractIndex128 {
    contract_index128_v0(ContractIndex128V0),
}

impl Serialize for ContractIndex128 {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut m = serializer.serialize_tuple(2)?;
        match self {
            ContractIndex128::contract_index128_v0(k) => {
                m.serialize_element("contract_index128_v0")?;
                m.serialize_element(k)?;
            }
        }
        m.end()
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ContractIndex128V0 {
    pub code: String,
    pub scope: String,
    pub table: String,
    pub primary_key: String,
    pub payer: String,
    pub secondary_key: String,
}

#[allow(non_camel_case_types)]
#[derive(Debug, Deserialize)]
pub enum ContractIndex256 {
    contract_index256_v0(ContractIndex256V0),
}

impl Serialize for ContractIndex256 {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut m = serializer.serialize_tuple(2)?;
        match self {
            ContractIndex256::contract_index256_v0(k) => {
                m.serialize_element("contract_index256_v0")?;
                m.serialize_element(k)?;
            }
        }
        m.end()
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ContractIndex256V0 {
    pub code: String,
    pub scope: String,
    pub table: String,
    pub primary_key: String,
    pub payer: String,
    pub secondary_key: String,
}

#[allow(non_camel_case_types)]
#[derive(Debug, Deserialize)]
pub enum ContractIndexDouble {
    contract_index_double_v0(ContractIndexDoubleV0),
}

impl Serialize for ContractIndexDouble {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut m = serializer.serialize_tuple(2)?;
        match self {
            ContractIndexDouble::contract_index_double_v0(k) => {
                m.serialize_element("contract_index_double_v0")?;
                m.serialize_element(k)?;
            }
        }
        m.end()
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ContractIndexDoubleV0 {
    pub code: String,
    pub scope: String,
    pub table: String,
    pub primary_key: String,
    pub payer: String,
    pub secondary_key: String,
}

#[allow(non_camel_case_types)]
#[derive(Debug, Deserialize)]
pub enum ContractIndexLongDouble {
    contract_index_long_double_v0(ContractIndexLongDoubleV0),
}

impl Serialize for ContractIndexLongDouble {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut m = serializer.serialize_tuple(2)?;
        match self {
            ContractIndexLongDouble::contract_index_long_double_v0(k) => {
                m.serialize_element("contract_index_long_double_v0")?;
                m.serialize_element(k)?;
            }
        }
        m.end()
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ContractIndexLongDoubleV0 {
    pub code: String,
    pub scope: String,
    pub table: String,
    pub primary_key: String,
    pub payer: String,
    // TODO: float 128
    pub secondary_key: String,
}

#[allow(non_camel_case_types)]
#[derive(Debug, Deserialize)]
pub enum Code {
    code_v0(CodeV0),
}

impl Serialize for Code {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut m = serializer.serialize_tuple(2)?;
        match self {
            Code::code_v0(k) => {
                m.serialize_element("code_v0")?;
                m.serialize_element(k)?;
            }
        }
        m.end()
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CodeV0 {
    pub vm_type: u8,
    pub vm_version: u8,
    pub code_hash: String,
    pub code: String,
}

#[allow(non_camel_case_types)]
#[derive(Debug, Deserialize)]
pub enum AccountMetadata {
    account_metadata_v0(AccountMetadataV0),
}

impl Serialize for AccountMetadata {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut m = serializer.serialize_tuple(2)?;
        match self {
            AccountMetadata::account_metadata_v0(k) => {
                m.serialize_element("account_metadata_v0")?;
                m.serialize_element(k)?;
            }
        }
        m.end()
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CodeID {
    pub vm_type: u8,
    pub vm_version: u8,
    pub code_hash: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AccountMetadataV0 {
    pub name: String,
    pub privileged: bool,
    #[serde(with = "eosio_datetime_format")]
    pub last_code_update: DateTime<Utc>,
    pub code: Option<CodeID>,
}

#[allow(non_camel_case_types)]
#[derive(Debug, Deserialize)]
pub enum Account {
    account_v0(AccountV0),
}

impl Serialize for Account {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut m = serializer.serialize_tuple(2)?;
        match self {
            Account::account_v0(k) => {
                m.serialize_element("account_v0")?;
                m.serialize_element(k)?;
            }
        }
        m.end()
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AccountV0 {
    pub name: String,
    #[serde(with = "eosio_datetime_format")]
    pub creation_date: DateTime<Utc>,
    pub abi: String,
}

#[allow(non_camel_case_types)]
#[derive(Debug, Deserialize)]
pub enum ResourceUsage {
    resource_usage_v0(ResourceUsageV0),
}

impl Serialize for ResourceUsage {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut m = serializer.serialize_tuple(2)?;
        match self {
            ResourceUsage::resource_usage_v0(k) => {
                m.serialize_element("resource_usage_v0")?;
                m.serialize_element(k)?;
            }
        }
        m.end()
    }
}

#[allow(non_camel_case_types)]
#[derive(Debug, Deserialize)]
pub enum UsageAccumulator {
    usage_accumulator_v0(UsageAccumulatorV0),
}

impl Serialize for UsageAccumulator {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut m = serializer.serialize_tuple(2)?;
        match self {
            UsageAccumulator::usage_accumulator_v0(k) => {
                m.serialize_element("usage_accumulator_v0")?;
                m.serialize_element(k)?;
            }
        }
        m.end()
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UsageAccumulatorV0 {
    pub last_ordinal: u32,
    pub value_ex: String,
    // u64
    pub consumed: String, // u64
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ResourceUsageV0 {
    pub owner: String,
    pub net_usage: UsageAccumulator,
    pub cpu_usage: UsageAccumulator,
    pub ram_usage: String, //u64
}

#[allow(non_camel_case_types)]
#[derive(Debug, Deserialize)]
pub enum ResourceLimits {
    resource_limits_v0(ResourceLimitsV0),
}

impl Serialize for ResourceLimits {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut m = serializer.serialize_tuple(2)?;
        match self {
            ResourceLimits::resource_limits_v0(k) => {
                m.serialize_element("resource_limits_v0")?;
                m.serialize_element(k)?;
            }
        }
        m.end()
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ResourceLimitsV0 {
    pub owner: String,
    pub net_weight: String,
    //u64
    pub cpu_weight: String,
    //u64
    pub ram_bytes: String, //u64
}

#[allow(non_camel_case_types)]
#[derive(Debug, Deserialize)]
pub enum ResourceLimitsState {
    resource_limits_state_v0(ResourceLimitsStateV0),
}

impl Serialize for ResourceLimitsState {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut m = serializer.serialize_tuple(2)?;
        match self {
            ResourceLimitsState::resource_limits_state_v0(k) => {
                m.serialize_element("resource_limits_state_v0")?;
                m.serialize_element(k)?;
            }
        }
        m.end()
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ResourceLimitsStateV0 {
    pub average_block_net_usage: UsageAccumulator,
    pub average_block_cpu_usage: UsageAccumulator,
    //u64
    pub total_net_weight: String,
    //u64
    pub total_cpu_weight: String,
    //u64
    pub total_ram_bytes: String,
    //u64
    pub virtual_net_limit: String,
    //u64
    pub virtual_cpu_limit: String, //u64
}

#[allow(non_camel_case_types)]
#[derive(Debug, Deserialize)]
pub enum ResourceLimitsConfig {
    resource_limits_config_v0(ResourceLimitsConfigV0),
}

impl Serialize for ResourceLimitsConfig {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut m = serializer.serialize_tuple(2)?;
        match self {
            ResourceLimitsConfig::resource_limits_config_v0(k) => {
                m.serialize_element("resource_limits_config_v0")?;
                m.serialize_element(k)?;
            }
        }
        m.end()
    }
}

#[allow(non_camel_case_types)]
#[derive(Debug, Deserialize)]
pub enum ElasticLimitParameters {
    elastic_limit_parameters_v0(ElasticLimitParametersV0),
}

impl Serialize for ElasticLimitParameters {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut m = serializer.serialize_tuple(2)?;
        match self {
            ElasticLimitParameters::elastic_limit_parameters_v0(k) => {
                m.serialize_element("elastic_limit_parameters_v0")?;
                m.serialize_element(k)?;
            }
        }
        m.end()
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ElasticLimitParametersV0 {
    pub target: String,
    //u64
    pub max: String,
    //u64
    pub periods: u32,
    //u64
    pub max_multiplier: u32,
    //u64
    pub contract_rate: ResourceLimitsRatio,
    pub expand_rate: ResourceLimitsRatio,
}

#[allow(non_camel_case_types)]
#[derive(Debug, Deserialize)]
pub enum ResourceLimitsRatio {
    resource_limits_ratio_v0(ResourceLimitsRatioV0),
}

impl Serialize for ResourceLimitsRatio {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut m = serializer.serialize_tuple(2)?;
        match self {
            ResourceLimitsRatio::resource_limits_ratio_v0(k) => {
                m.serialize_element("resource_limits_ratio_v0")?;
                m.serialize_element(k)?;
            }
        }
        m.end()
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ResourceLimitsRatioV0 {
    pub numerator: String,
    //u64
    pub denominator: String, //u64
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ResourceLimitsConfigV0 {
    pub cpu_limit_parameters: ElasticLimitParameters,
    pub net_limit_parameters: ElasticLimitParameters,
    pub account_cpu_usage_average_window: u32,
    pub account_net_usage_average_window: u32,
}
