use domain::cartesi::{AdvanceMetadata, VoucherMeta};
use domain::game_core::{DinderError, ServerError};
use ethers_core::types::{Address, U256};
use serde::Serialize;
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize)]
pub struct BalanceManagerState {
    pub balance_map: HashMap<String, String>,
    pub voucher_meta_map: HashMap<String, Vec<VoucherMeta>>,
}

#[derive(Debug, Clone, Serialize)]
pub struct BalanceManager {
    balance_map: HashMap<Address, U256>,
    voucher_meta_map: HashMap<Address, Vec<VoucherMeta>>,
}

impl BalanceManager {
    pub fn new() -> Self {
        BalanceManager {
            balance_map: HashMap::<Address, U256>::new(),
            voucher_meta_map: HashMap::<Address, Vec<VoucherMeta>>::new(),
        }
    }

    pub fn get_balance(&self, address: &Address) -> Option<&U256> {
        self.balance_map.get(address)
    }

    pub fn deposit(&mut self, address: &Address, amount: U256) -> U256 {
        let new_balance = self
            .get_balance(address)
            .map_or(amount, |current| current.saturating_add(amount));

        self.balance_map.insert(*address, new_balance);
        new_balance
    }

    #[allow(dead_code)]
    pub fn withdraw(&mut self, address: &Address, amount: U256) -> Result<U256, DinderError> {
        match self.get_balance(address) {
            Some(current) => match current < &amount {
                true => Err(ServerError::InsufficientBalance(
                    current.to_string(),
                    amount.to_string(),
                )
                .into()),
                _ => {
                    let new_balance = current.saturating_sub(amount);
                    self.balance_map.insert(*address, new_balance);
                    Ok(new_balance)
                }
            },
            None => {
                Err(ServerError::InsufficientBalance("0".to_string(), amount.to_string()).into())
            }
        }
    }

    #[allow(dead_code)]
    pub fn update_voucher_meta(
        &mut self,
        address: &Address,
        amount: String,
        meta: AdvanceMetadata,
    ) {
        let new_meta = VoucherMeta {
            timestamp: meta.timestamp,
            input_index: meta.input_index.to_string(),
            amount,
        };

        //#NOTE: if using map_or here, it will complain about moving issue of new_meta
        let updated_metas = match self.voucher_meta_map.get(address) {
            Some(metas) => {
                let mut new_vec = metas.to_vec();
                new_vec.push(new_meta);
                new_vec
            }
            None => vec![new_meta],
        };

        self.voucher_meta_map.insert(*address, updated_metas);
    }

    #[allow(dead_code)]
    pub fn get_voucher_meta(&self, address: &Address) -> Option<&Vec<VoucherMeta>> {
        self.voucher_meta_map.get(address)
    }

    //#TODO: When to delete mapping?

    pub fn get_current_state(&self) -> BalanceManagerState {
        BalanceManagerState {
            balance_map: self
                .balance_map
                .iter()
                .map(|(address, balance)| (format!("{:#x}", address), balance.to_string()))
                .collect(),
            voucher_meta_map: self
                .voucher_meta_map
                .iter()
                .map(|(address, voucher_list)| (format!("{:#x}", address), voucher_list.clone()))
                .collect(),
        }
    }
}
