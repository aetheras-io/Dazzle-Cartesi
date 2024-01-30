use ethers_core::types::Address;
use serde::Serialize;
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize)]
pub struct IngameWalletManager {
    wallet_map: HashMap<Address, Address>,
}

#[derive(Debug, Clone, Serialize)]
pub struct IngameWalletManagerState {
    pub wallet_map: HashMap<String, String>,
}

impl IngameWalletManager {
    pub fn new() -> Self {
        IngameWalletManager {
            wallet_map: HashMap::<Address, Address>::new(),
        }
    }

    // pub fn get_ingame_wallet(&self, metamask_wallet_address: &Address) -> Option<&Address> {
    //     self.wallet_map.get(metamask_wallet_address)
    // }

    //#TODO: When to delete mapping?

    pub fn get_current_state(&self) -> IngameWalletManagerState {
        IngameWalletManagerState {
            wallet_map: self
                .wallet_map
                .iter()
                .map(|(metamask_wallet, ingame_wallet)| {
                    (
                        format!("{:#x}", metamask_wallet),
                        format!("{:#x}", ingame_wallet),
                    )
                })
                .collect(),
        }
    }

    pub fn set_ingame_wallet(
        &mut self,
        metamask_wallet_address: &Address,
        ingame_wallet_address: Address,
    ) {
        self.wallet_map
            .insert(*metamask_wallet_address, ingame_wallet_address);
    }

    pub fn is_ingame_wallet_attached(&self, ingame_wallet_address: &Address) -> bool {
        self.wallet_map
            .values()
            .any(|&addr| addr.eq(ingame_wallet_address))
    }
}
