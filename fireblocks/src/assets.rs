use super::FbArgs;

pub const SOL: &str = "SOL";
pub const MATIC: &str = "MATIC";
pub const ETH: &str = "ETH";
pub const SOL_TEST: &str = "SOL_TEST";
pub const MATIC_TEST: &str = "MATIC_POLYGON_MUMBAI";
pub const ETH_TEST: &str = "ETH_TEST";

#[derive(Clone, Debug)]
pub struct Assets {
    ids: Vec<String>,
    test_mode: bool,
}

impl Assets {
    #[must_use]
    pub fn new(args: FbArgs) -> Self {
        let ids = args.fireblocks_supported_asset_ids;
        let test_mode = args.fireblocks_test_mode;

        Self { ids, test_mode }
    }

    #[must_use]
    pub fn ids(&self) -> Vec<String> {
        // TODO: adjust in the future to compute the ids based on the test_mode once during initialization of the struct
        self.ids.iter().map(|id| self.id(id)).collect()
    }

    #[must_use]
    pub fn test_mode(&self) -> bool {
        self.test_mode
    }

    #[must_use]
    pub fn id(&self, id: &str) -> String {
        match (self.test_mode, id) {
            (true, MATIC) => MATIC_TEST.to_string(),
            (true, _) => format!("{id}_TEST"),
            (false, _) => id.to_string(),
        }
    }
}
