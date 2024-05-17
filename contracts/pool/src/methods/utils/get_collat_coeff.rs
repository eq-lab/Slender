use common::FixedI128;
use pool_interface::types::{error::Error, reserve_data::ReserveData};
use soroban_sdk::Env;

use super::rate::get_actual_lender_accrued_rate;

/// Returns collateral coefficient
/// collateral_coeff = [underlying_balance + lender_ar * total_debt_token]/total_stoken
pub fn get_collat_coeff(
    env: &Env,
    reserve: &ReserveData,
    s_token_supply: i128,
    s_token_underlying_balance: i128,
    debt_token_supply: i128,
) -> Result<FixedI128, Error> {
    if s_token_supply == 0 {
        return Ok(FixedI128::ONE);
    } //@audit can s_token_supply be zero while total_debt is not? 
    //@audit can s_token_supply be negative? 

    let collat_ar = get_actual_lender_accrued_rate(env, reserve)?; //@audit shouldn't this be named lender_ar? 
    
    FixedI128::from_rational( //@audit we are rounding toward zero i.e., ROUNDING DOWN
        s_token_underlying_balance
            .checked_add(
                collat_ar
                    .mul_int(debt_token_supply)
                    .ok_or(Error::CollateralCoeffMathError)?,
            )
            .ok_or(Error::CollateralCoeffMathError)?,
        s_token_supply,
    )
    .ok_or(Error::CollateralCoeffMathError)
}
