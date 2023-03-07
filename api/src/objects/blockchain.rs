use std::collections::HashMap;

use lazy_static::lazy_static;

use crate::entities::wallets::AssetType;

lazy_static! {
    pub static ref BLOCKCHAIN_ASSET_IDS: HashMap<Blockchain, Vec<AssetType>> = {
        let mut m = HashMap::new();
        m.insert(Blockchain::Solana, vec![
            AssetType::Solana,
            AssetType::SolanaTest,
        ]);
        m.insert(Blockchain::Polygon, vec![
            AssetType::MaticTest,
            AssetType::Matic,
        ]);
        m.insert(Blockchain::Ethereum, vec![
            AssetType::Eth,
            AssetType::EthTest,
        ]);
        m
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Blockchain {
    Solana,
    Polygon,
    Ethereum,
}
