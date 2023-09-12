use common::FixedI128;
use debt_token_interface::DebtTokenClient;
use flash_loan_receiver_interface::{Asset as ReceiverAsset, FlashLoanReceiverClient};
use pool_interface::types::error::Error;
use pool_interface::types::flash_loan_asset::FlashLoanAsset;
use s_token_interface::STokenClient;
use soroban_sdk::{assert_with_error, token, vec, Address, Bytes, Env, Vec};

use crate::event;
use crate::storage::{read_flash_loan_fee, read_reserve, read_treasury};

use super::borrow::do_borrow;
use super::utils::recalculate_reserve_data::recalculate_reserve_data;
use super::utils::validation::{
    require_active_reserve, require_borrowing_enabled, require_not_paused, require_positive_amount,
};

pub fn flash_loan(
    env: &Env,
    who: &Address,
    receiver: &Address,
    loan_assets: &Vec<FlashLoanAsset>,
    params: &Bytes,
) -> Result<(), Error> {
    who.require_auth();
    require_not_paused(env);

    let fee =
        FixedI128::from_percentage(read_flash_loan_fee(env)).ok_or(Error::MathOverflowError)?;

    let loan_asset_len = loan_assets.len();
    assert_with_error!(env, loan_asset_len > 0, Error::MustBePositive);

    let mut receiver_assets = vec![env];
    let mut reserves = vec![env];

    for i in 0..loan_asset_len {
        let loan_asset = loan_assets.get_unchecked(i);

        require_positive_amount(env, loan_asset.amount);

        let reserve = read_reserve(env, &loan_asset.asset)?;
        require_active_reserve(env, &reserve);
        require_borrowing_enabled(env, &reserve);

        let s_token = STokenClient::new(env, &reserve.s_token_address);
        s_token.transfer_underlying_to(receiver, &loan_asset.amount);

        reserves.push_back(reserve);
        receiver_assets.push_back(ReceiverAsset {
            asset: loan_asset.asset,
            amount: loan_asset.amount,
            premium: fee
                .mul_int(loan_asset.amount)
                .ok_or(Error::MathOverflowError)?,
        });
    }

    let loan_receiver = FlashLoanReceiverClient::new(env, receiver);
    let loan_received = loan_receiver.receive(&receiver_assets, params);
    assert_with_error!(env, loan_received, Error::FlashLoanReceiverError);

    let treasury = read_treasury(env);

    for i in 0..loan_asset_len {
        let loan_asset = loan_assets.get_unchecked(i);
        let received_asset = receiver_assets.get_unchecked(i);
        let reserve = reserves.get_unchecked(i);

        if !loan_asset.borrow {
            let amount_with_premium = received_asset
                .amount
                .checked_add(received_asset.premium)
                .ok_or(Error::MathOverflowError)?;

            let underlying_asset = token::Client::new(env, &received_asset.asset);
            let s_token = STokenClient::new(env, &reserve.s_token_address);

            underlying_asset.transfer_from(
                &env.current_contract_address(),
                receiver,
                &reserve.s_token_address,
                &amount_with_premium,
            );
            s_token.transfer_underlying_to(&treasury, &received_asset.premium);
        } else {
            let s_token = STokenClient::new(env, &reserve.s_token_address);
            let debt_token = DebtTokenClient::new(env, &reserve.debt_token_address);
            let s_token_supply = s_token.total_supply();

            let debt_token_supply_after = do_borrow(
                env,
                who,
                &received_asset.asset,
                &reserve,
                s_token.balance(who),
                debt_token.balance(who),
                s_token_supply,
                debt_token.total_supply(),
                received_asset.amount,
            )?;

            recalculate_reserve_data(
                env,
                &received_asset.asset,
                &reserve,
                s_token_supply,
                debt_token_supply_after,
            )?;
        }

        event::flash_loan(
            env,
            who,
            receiver,
            &received_asset.asset,
            received_asset.amount,
            received_asset.premium,
        );
    }

    Ok(())
}