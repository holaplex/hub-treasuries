use hub_core::serde_with::skip_serializing_none;
use serde::{Deserialize, Serialize};

/// <https://docs.fireblocks.com/api/?javascript#create-a-new-transaction>
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[skip_serializing_none]
pub struct CreateTransaction {
    pub asset_id: String,
    pub source: TransferPeerPath,
    pub destination: Option<DestinationTransferPeerPath>,
    pub destinations: Option<Vec<TransactionRequestDestination>>,
    pub amount: String,
    pub treat_as_gross_amount: Option<bool>,
    pub note: Option<String>,
    pub operation: TransactionOperation,
    pub customer_ref_id: Option<String>,
    pub extra_parameters: Option<ExtraParameters>,
}

/// <https://docs.fireblocks.com/api/?javascript#transactionoperation>
#[allow(non_camel_case_types)]
#[derive(Copy, Clone, Eq, PartialEq, Serialize, Deserialize, Debug, poem_openapi::Enum)]
#[allow(clippy::upper_case_acronyms)]
pub enum TransactionOperation {
    TRANSFER,
    RAW,
    CONTRACT_CALL,
    MINT,
    BURN,
    SUPPLY_TO_COMPOUND,
    REDEEM_FROM_COMPOUND,
}
/// <https://docs.fireblocks.com/api/?javascript#transferpeerpath>
#[derive(Debug, Serialize, Deserialize, poem_openapi::Object)]
#[serde(rename_all = "camelCase")]
pub struct TransferPeerPath {
    #[serde(rename = "type")]
    pub peer_type: String,
    pub id: String,
}

/// <https://docs.fireblocks.com/api/?javascript#destinationtransferpeerpath>
#[derive(Debug, Serialize, Deserialize, poem_openapi::Object)]
#[serde(rename_all = "camelCase")]
#[skip_serializing_none]
pub struct DestinationTransferPeerPath {
    #[serde(rename = "type")]
    pub peer_type: String,
    pub id: Option<String>,
    pub one_time_address: Option<OneTimeAddress>,
}

/// <https://docs.fireblocks.com/api/?javascript#transactionrequestdestination>
#[derive(Debug, Serialize, Deserialize, poem_openapi::Object)]
#[serde(rename_all = "camelCase")]
#[skip_serializing_none]
pub struct TransactionRequestDestination {
    pub amount: String,
    pub destination: DestinationTransferPeerPath,
}

/// <https://docs.fireblocks.com/api/?javascript#onetimeaddress>

#[derive(Debug, Serialize, Deserialize, poem_openapi::Object)]
#[serde(rename_all = "camelCase")]
#[skip_serializing_none]
pub struct OneTimeAddress {
    pub address: String,
    pub tag: Option<String>,
}

/// <https://docs.fireblocks.com/api/?javascript#transactionstatus>
#[allow(non_camel_case_types)]
#[allow(clippy::upper_case_acronyms)]
#[derive(Copy, Clone, Eq, PartialEq, Serialize, Deserialize, Debug, poem_openapi::Enum)]
pub enum TransactionStatus {
    SUBMITTED,
    QUEUED,
    PENDING_AUTHORIZATION,
    PENDING_SIGNATURE,
    BROADCASTING,
    PENDING_3RD_PARTY_MANUAL_APPROVAL,
    PENDING_3RD_PARTY,
    CONFIRMING,
    PARTIALLY_COMPLETED,
    PENDING_AML_SCREENING,
    COMPLETED,
    CANCELLED,
    REJECTED,
    BLOCKED,
    FAILED,
    PENDING,
}

/// <https://docs.fireblocks.com/api/?javascript#createtransactionresponse>
#[derive(Debug, Clone, Serialize, Deserialize, poem_openapi::Object)]
#[serde(rename_all = "camelCase")]
pub struct CreateTransactionResponse {
    pub id: String,
    pub status: TransactionStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ExtraParameters {
    Content(String),
    RawMessageData(RawMessageData),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RawMessageData {
    pub messages: Vec<UnsignedMessage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UnsignedMessage {
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransactionDetails {
    pub id: String,
    pub asset_id: String,
    pub tx_hash: String,
    pub status: TransactionStatus,
    pub sub_status: String,
    pub signed_messages: Vec<SignedMessageResponse>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SignedMessageResponse {
    pub content: String,
    pub algorithm: String,
    pub derivation_path: Vec<usize>,
    pub signature: SignatureResponse,
    pub public_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SignatureResponse {
    pub full_sig: String,
    pub r: Option<String>,
    pub s: Option<String>,
    pub v: Option<u64>,
}
