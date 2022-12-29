use async_graphql::SimpleObject;
use serde::{Deserialize, Serialize};

/// Paged
/// Query parameters
/// https://docs.fireblocks.com/api/?javascript#list-vault-accounts-paged
#[derive(Debug, Serialize, Deserialize, SimpleObject)]
#[serde(rename_all = "camelCase")]
pub struct QueryVaultAccounts {
    name_prefix: Option<String>,
    name_suffix: Option<String>,
    min_amount_threshold: Option<u64>,
    asset_id: Option<u64>,
    order_by: String,
    limit: u64,
    before: Option<String>,
    after: Option<String>,
    max_bip44_address_index_used: u64,
    max_bip44_change_address_index_used: u64,
}

#[derive(Debug, Serialize, Deserialize, SimpleObject)]
#[serde(rename_all = "camelCase")]
pub struct Paging {
    before: String,
    after: String,
}

/// https://docs.fireblocks.com/api/?javascript#vaultaccountspagedresponse
#[derive(Debug, Serialize, Deserialize, SimpleObject)]
#[serde(rename_all = "camelCase")]
pub struct VaultAccountsPagedResponse {
    accounts: Vec<VaultAccount>,
    paging: Paging,
    #[serde(rename = "previousUrl")]
    previous_url: String,
    #[serde(rename = "nextUrl")]
    next_url: String,
}

/// Query Response
/// https://docs.fireblocks.com/api/?javascript#vaultaccount
#[derive(Debug, Serialize, Deserialize, SimpleObject)]
#[serde(rename_all = "camelCase")]
pub struct VaultAccount {
    id: String,
    name: String,
    hidden_on_ui: bool,
    customer_ref_id: String,
    auto_fuel: bool,
    assets: Vec<VaultAsset>,
}

/// https://docs.fireblocks.com/api/?javascript#createvaultassetresponse
#[derive(Debug, Serialize, Deserialize, SimpleObject)]
#[serde(rename_all = "camelCase")]
pub struct VaultAsset {
    id: String,
    total: String,
    balance: String,
    available: String,
    pending: String,
    staked: String,
    frozen: String,
    locked_amount: String,
    #[serde(rename = "maxBip44AddressIndexUsed")]
    max_bip44_address_index_used: u64,
    #[serde(rename = "maxBip44ChangeAddressIndexUsed")]
    max_bip44_change_address_index_used: u64,
    total_staked_cpu: String,
    total_staked_network: String,
    self_staked_cpu: String,
    self_staked_network: String,
    pending_refund_cpu: String,
    pending_refund_network: String,
    block_height: String,
    block_hash: String,
}

/// Query parameters
/// https://docs.fireblocks.com/api/?javascript#create-a-new-vault-account
#[derive(Debug, Serialize, Deserialize, SimpleObject)]
#[serde(rename_all = "camelCase")]
pub struct CreateVault {
    name: String,
    hidden_on_ui: Option<String>,
    customer_ref_id: Option<String>,
    auto_fuel: Option<String>,
}
