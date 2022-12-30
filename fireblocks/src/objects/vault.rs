use async_graphql::SimpleObject;
use serde::{Deserialize, Serialize};

/// Paged
/// Query parameters
/// https://docs.fireblocks.com/api/?javascript#list-vault-accounts-paged
#[derive(Debug, Clone, Serialize, Deserialize, SimpleObject)]
#[serde(rename_all = "camelCase")]
pub struct QueryVaultAccounts {
    pub name_prefix: Option<String>,
    pub name_suffix: Option<String>,
    pub min_amount_threshold: Option<u64>,
    pub asset_id: Option<u64>,
    pub order_by: String,
    pub limit: u64,
    pub before: Option<String>,
    pub after: Option<String>,
    pub max_bip44_address_index_used: u64,
    pub max_bip44_change_address_index_used: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, SimpleObject)]
#[serde(rename_all = "camelCase")]
pub struct Paging {
    pub before: String,
    pub after: String,
}

/// https://docs.fireblocks.com/api/?javascript#vaultaccountspagedresponse
#[derive(Debug, Clone, Serialize, Deserialize, SimpleObject)]
#[serde(rename_all = "camelCase")]
pub struct VaultAccountsPagedResponse {
    pub accounts: Vec<VaultAccount>,
    pub paging: Paging,
    #[serde(rename = "previousUrl")]
    pub previous_url: String,
    #[serde(rename = "nextUrl")]
    pub next_url: String,
}

/// Query Response
/// https://docs.fireblocks.com/api/?javascript#vaultaccount
#[derive(Debug, Clone, Serialize, Deserialize, SimpleObject)]
#[serde(rename_all = "camelCase")]
pub struct VaultAccount {
    pub id: String,
    pub name: String,
    pub hidden_on_ui: bool,
    pub customer_ref_id: String,
    pub auto_fuel: bool,
    pub assets: Vec<VaultAsset>,
}

/// https://docs.fireblocks.com/api/?javascript#createvaultassetresponse
#[derive(Debug, Clone, Serialize, Deserialize, SimpleObject)]
#[serde(rename_all = "camelCase")]
pub struct VaultAsset {
    pub id: String,
    pub total: String,
    pub balance: String,
    pub available: String,
    pub pending: String,
    pub staked: String,
    pub frozen: String,
    pub locked_amount: String,
    #[serde(rename = "maxBip44AddressIndexUsed")]
    pub max_bip44_address_index_used: u64,
    #[serde(rename = "maxBip44ChangeAddressIndexUsed")]
    pub max_bip44_change_address_index_used: u64,
    pub total_staked_cpu: String,
    pub total_staked_network: String,
    pub self_staked_cpu: String,
    pub self_staked_network: String,
    pub pending_refund_cpu: String,
    pub pending_refund_network: String,
    pub block_height: String,
    pub block_hash: String,
}

/// Query parameters
/// https://docs.fireblocks.com/api/?javascript#create-a-new-vault-account
#[derive(Debug, Clone, Serialize, Deserialize, SimpleObject)]
#[serde(rename_all = "camelCase")]
pub struct CreateVault {
    pub name: String,
    pub hidden_on_ui: Option<String>,
    pub customer_ref_id: Option<String>,
    pub auto_fuel: Option<String>,
}

/// https://docs.fireblocks.com/api/?javascript#create-a-new-wallet-under-the-vault-account
#[derive(Debug, Clone, Serialize, Deserialize, SimpleObject)]
#[serde(rename_all = "camelCase")]
pub struct CreateVaultWallet {
    pub eos_account_name: Option<String>,
}

/// https://docs.fireblocks.com/api/?javascript#createvaultassetresponse
#[derive(Debug, Clone, Serialize, Deserialize, SimpleObject)]
#[serde(rename_all = "camelCase")]
pub struct CreateVaultAssetResponse {
    pub id: String,
    pub address: String,
    pub legacy_address: String,
    pub tag: String,
    pub eos_account_name: String,
}
