use fireblocks::objects::vault::{
    CreateVault, CreateVaultAssetResponse, CreateVaultWallet, QueryVaultAccounts, VaultAccount,
    VaultAccountsPagedResponse, VaultAsset,
};
use hub_core::{prelude::*, uuid::Uuid};
use poem::{web::Data, Result};
use poem_openapi::{param::Path, payload::Json, Object, OpenApi};
use sea_orm::{prelude::*, Set};

use crate::{
    entities::{prelude::*, project_treasuries, treasuries, wallets},
    AppState, UserID,
};

pub struct OrgsApi;

#[OpenApi]
impl OrgsApi {
    #[oai(path = "/treasury/create", method = "post")]
    async fn create_vault(
        &self,
        state: Data<&AppState>,
        user_id: UserID,
        project_id: Json<String>,
    ) -> Result<Json<VaultAccount>> {
        let UserID(id) = user_id;
        let Data(state) = state;
        let conn = state.connection.get();
        let fireblocks = state.fireblocks.clone();

        let user_id = id.context("X-USER-ID header not found")?;

        let create_vault = CreateVault {
            name: project_id.to_string(),
            hidden_on_ui: None,
            customer_ref_id: Some(user_id.to_string()),
            auto_fuel: Some(false),
        };

        let vault = fireblocks.create_vault(create_vault).await?;

        let treasury = treasuries::ActiveModel {
            vault_id: Set(vault.id.clone()),
            ..Default::default()
        };

        let treasury: treasuries::Model = treasury
            .clone()
            .insert(conn)
            .await
            .context("failed to get treasury record from db")?;

        let project_treasuries_active_model = project_treasuries::ActiveModel {
            project_id: Set(
                Uuid::parse_str(&project_id).context("failed to parse project id to Uuid")?
            ),
            treasury_id: Set(treasury.id),
            ..Default::default()
        };

        project_treasuries_active_model
            .insert(conn)
            .await
            .context("failed to insert project treasuries")?;

        Ok(Json(vault))
    }

    #[oai(path = "/treasury/wallet/create", method = "post")]
    async fn create_treasury_wallet(
        &self,
        state: Data<&AppState>,
        user_id: UserID,
        treasury_id: String,
        asset_id: String,
    ) -> Result<Json<CreateVaultAssetResponse>> {
        let UserID(id) = user_id;
        let Data(state) = state;
        let conn = state.connection.get();
        let fireblocks = state.fireblocks.clone();

        let user_id = id.context("X-USER-ID header not found")?;

        // AssetID would be enum for polygon/solana
        // Reterive assets endpoint

        // insert treasury to get the treasury id

        let treasury_id =
            Uuid::from_str(&treasury_id).context("failed to parse treasury_id to Uuid")?;

        let treasury = Treasuries::find_by_id(treasury_id)
            .one(conn)
            .await
            .context("failed to load treasury record from db")?
            .context("treasury not found in db")?;

        let vault_asset = fireblocks
            .create_vault_wallet(treasury.vault_id.clone(), asset_id, CreateVaultWallet {
                eos_account_name: None,
            })
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

    #[oai(path = "/vaults", method = "get")]
    async fn list_vaults(
        &self,
        state: Data<&AppState>,
        // req: Json<ListVaultsRequest>,
    ) -> Result<Json<VaultAccountsPagedResponse>> {
        let fireblocks = state.fireblocks.clone();

        let vaults = fireblocks
            .get_vaults(QueryVaultAccounts {
                name_prefix: None,
                name_suffix: None,
                min_amount_threshold: None,
                asset_id: None,
                order_by: OrderBy::default().to_string(),
                limit: 500,
                before: None,
                after: None,
                max_bip44_address_index_used: 966,
                max_bip44_change_address_index_used: 20,
            })
            .await
            .context("failed to get vaults")?;

        Ok(Json(vaults))
    }

    #[oai(path = "/vault/:vault_id", method = "get")]
    async fn get_vault(
        &self,
        state: Data<&AppState>,
        vault_id: Path<String>,
    ) -> Result<Json<VaultAccount>> {
        let fireblocks = state.fireblocks.clone();

        let vault = fireblocks.get_vault(vault_id.0).await?;

        Ok(Json(vault))
    }

    #[oai(path = "/vault/assets", method = "get")]
    async fn list_vault_assets(&self, state: Data<&AppState>) -> Result<Json<Vec<VaultAsset>>> {
        let fireblocks = state.fireblocks.clone();

        let vault = fireblocks.vault_assets().await?;

        debug!("{:?}", vault);

        Ok(Json(vault))
    }

    #[oai(path = "/treasury/:project_id", method = "get")]
    async fn project(
        &self,
        state: Data<&AppState>,
        project_id: Path<String>,
    ) -> Result<Json<ProjectTreasuryResponse>> {
        let db = state.connection.get();
        let fireblocks = state.fireblocks.clone();

        let pt = project_treasuries::Entity::find()
            .filter(project_treasuries::Column::ProjectId.eq(project_id.0))
            .one(db)
            .await
            .context("failed to load project treasuries")?
            .context("project treasury not found")?;

        let t = treasuries::Entity::find_by_id(pt.treasury_id)
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

        let res = ProjectTreasuryResponse {
            project_treasury: pt,
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
struct ProjectTreasuryResponse {
    #[oai(flatten = true)]
    project_treasury: project_treasuries::Model,
    treasury: treasuries::Model,
    wallets: Vec<FireblockWallet>,
}

#[derive(Object, Debug, Clone)]
struct FireblockWallet {
    #[oai(flatten = true)]
    wallet: wallets::Model,
    balance: VaultAsset,
}
