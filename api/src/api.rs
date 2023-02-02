use fireblocks::objects::vault::{
    CreateVaultAssetResponse, CreateVaultWallet, QueryVaultAccounts, VaultAccount,
    VaultAccountsPagedResponse, VaultAsset,
};
use hub_core::{prelude::*, uuid::Uuid};
use poem::{web::Data, Result};
use poem_openapi::{
    param::{Header, Path},
    payload::Json,
    Object, OpenApi,
};
use sea_orm::{prelude::*, Set};

use crate::{
    entities::{prelude::*, treasuries, wallets},
    AppState,
};

pub struct TreasuryApi;

#[OpenApi]
impl TreasuryApi {
    #[oai(path = "/treasuries/{treasury}/wallets", method = "post")]
    async fn create_treasury_wallet(
        &self,
        state: Data<&AppState>,
        #[oai(name = "X-USER-ID")] user_id: Header<Uuid>,
        treasury: Path<Uuid>,
        req: Json<CreateTreasuryWalletRequest>,
    ) -> Result<Json<CreateVaultAssetResponse>> {
        let Header(user_id) = user_id;
        let Data(state) = state;
        let Path(treasury_id) = treasury;

        let conn = state.connection.get();
        let fireblocks = state.fireblocks.clone();

        // AssetID would be enum for polygon/solana
        // Reterive assets endpoint

        // insert treasury to get the treasury id

        let treasury = Treasuries::find_by_id(treasury_id)
            .one(conn)
            .await
            .context("failed to load treasury record from db")?
            .context("treasury not found in db")?;

        let vault_asset = fireblocks
            .create_vault_wallet(
                treasury.vault_id.clone(),
                req.asset_id.clone(),
                CreateVaultWallet {
                    eos_account_name: None,
                },
            )
            .await?;

        let v = vault_asset.clone();

        let active_model = wallets::ActiveModel {
            treasury_id: Set(treasury_id),
            asset_id: Set(vault_asset.id),
            address: Set(vault_asset.address),
            legacy_address: Set(vault_asset.legacy_address),
            tag: Set(vault_asset.tag),
            created_by: Set(user_id),
            ..Default::default()
        };
        active_model
            .insert(conn)
            .await
            .context("failed to insert treasury wallets")?;

        Ok(Json(v))
    }

    #[oai(path = "/vaults", method = "post")]
    async fn list_vaults(
        &self,
        state: Data<&AppState>,
        req: Json<ListVaultsRequest>,
    ) -> Result<Json<VaultAccountsPagedResponse>> {
        let fireblocks = state.fireblocks.clone();

        let vaults = fireblocks
            .get_vaults(QueryVaultAccounts {
                name_prefix: None,
                name_suffix: None,
                min_amount_threshold: None,
                asset_id: req.asset_id,
                order_by: req.order_by.unwrap_or_default().to_string(),
                limit: req.limit.unwrap_or(500),
                before: None,
                after: None,
                max_bip44_address_index_used: 966,
                max_bip44_change_address_index_used: 20,
            })
            .await
            .context("failed to get vaults")?;

        Ok(Json(vaults))
    }

    #[oai(path = "/vaults/{vault}", method = "get")]
    async fn get_vault(
        &self,
        state: Data<&AppState>,
        vault: Path<String>,
    ) -> Result<Json<VaultAccount>> {
        let fireblocks = state.fireblocks.clone();
        let Path(vault_id) = vault;
        let vault = fireblocks.get_vault(vault_id).await?;

        Ok(Json(vault))
    }

    #[oai(path = "/assets", method = "get")]
    async fn list_vault_assets(&self, state: Data<&AppState>) -> Result<Json<Vec<VaultAsset>>> {
        let fireblocks = state.fireblocks.clone();

        let vault = fireblocks.vault_assets().await?;

        Ok(Json(vault))
    }

    #[oai(path = "/treasuries/{treasury}", method = "get")]
    async fn get_treasury(
        &self,
        state: Data<&AppState>,
        treasury: Path<Uuid>,
    ) -> Result<Json<TreasuryResponse>> {
        let db = state.connection.get();
        let fireblocks = state.fireblocks.clone();
        let Path(treasury) = treasury;

        let t = treasuries::Entity::find_by_id(treasury)
            .one(db)
            .await
            .context("failed to load project treasuries")?
            .context("project treasury not found in db")?;

        let db_wallets = wallets::Entity::find()
            .filter(wallets::Column::TreasuryId.eq(t.id))
            .all(db)
            .await
            .context("failed to get wallets from db")?;

        let mut wallets: Vec<FireblockWallet> = Vec::new();

        for w in db_wallets {
            let asset = fireblocks
                .vault_asset(t.vault_id.clone(), w.asset_id.clone())
                .await?;

            wallets.push(FireblockWallet {
                wallet: w,
                balance: asset,
            });
        }

        let res = TreasuryResponse {
            treasury: t,
            wallets,
        };
        Ok(Json(res))
    }
}

#[derive(poem_openapi::Enum, Debug, Copy, Clone, Eq, PartialEq)]
pub enum OrderBy {
    Asc,
    Desc,
}

impl fmt::Display for OrderBy {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

impl Default for OrderBy {
    fn default() -> Self {
        Self::Desc
    }
}

#[derive(Object)]
pub struct ListVaultsRequest {
    asset_id: Option<u64>,
    limit: Option<u64>,
    order_by: Option<OrderBy>,
}

#[derive(Object, Debug)]
struct TreasuryResponse {
    treasury: treasuries::Model,
    wallets: Vec<FireblockWallet>,
}

#[derive(Object, Debug, Clone)]
struct FireblockWallet {
    #[oai(flatten = true)]
    wallet: wallets::Model,
    balance: VaultAsset,
}

#[derive(Object)]
pub struct CreateVaultRequest {
    project_id: String,
}

#[derive(Object, Clone)]
pub struct CreateTreasuryWalletRequest {
    asset_id: String,
}
