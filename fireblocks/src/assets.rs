use super::FbArgs;

#[derive(Clone, Debug)]
pub struct Assets {
    ids: Vec<String>,
    test_mode: bool,
}

impl Assets {
    pub fn new(args: FbArgs) -> Self {
        let ids = args.fireblocks_supported_asset_ids;
        let test_mode = args.fireblocks_test_mode;

        let ids = if test_mode {
            ids.iter().map(|id| format!("{id}_TEST")).collect()
        } else {
            ids
        };

        Self { ids, test_mode }
    }

    pub fn ids(&self) -> &Vec<String> {
        &self.ids
    }

    pub fn test_mode(&self) -> bool {
        self.test_mode
    }
}
