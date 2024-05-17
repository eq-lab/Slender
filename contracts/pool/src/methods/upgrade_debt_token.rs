use debt_token_interface::DebtTokenClient;
use pool_interface::types::error::Error;
use soroban_sdk::{Address, BytesN, Env};

use crate::storage::read_reserve;

use super::utils::{get_fungible_lp_tokens::get_fungible_lp_tokens, validation::require_admin};

pub fn upgrade_debt_token(
    env: &Env,
    asset: &Address,
    new_wasm_hash: &BytesN<32>,
) -> Result<(), Error> {
    require_admin(env).unwrap(); //@audit why did we decide to panic here and not in upgrade where we propagate the error to the user? 

    let reserve = read_reserve(env, asset)?; //@audit note to self: we do not check if the reserve is active or if borrowing is enabled. Is that Ok?
    let (_, debt_token_address) = get_fungible_lp_tokens(&reserve)?;
    let debt_token = DebtTokenClient::new(env, debt_token_address);
    debt_token.upgrade(new_wasm_hash);

    Ok(())
}
