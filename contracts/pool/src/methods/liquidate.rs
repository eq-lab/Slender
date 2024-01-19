use common::{FixedI128, PERCENTAGE_FACTOR};
use debt_token_interface::DebtTokenClient;
use pool_interface::types::error::Error;
use s_token_interface::STokenClient;
use soroban_sdk::{assert_with_error, token, Address, Env};

use crate::methods::utils::recalculate_reserve_data::recalculate_reserve_data;
use crate::types::calc_account_data_cache::CalcAccountDataCache;
use crate::types::price_provider::PriceProvider;
use crate::types::user_configurator::UserConfigurator;
use crate::{
    add_stoken_underlying_balance, read_initial_health, read_token_balance,
    read_token_total_supply, write_token_balance, write_token_total_supply,
};

use super::account_position::calc_account_data;
use super::utils::validation::require_not_paused;

pub fn liquidate(
    env: &Env,
    liquidator: &Address,
    who: &Address,
    receive_stoken: bool,
) -> Result<(), Error> {
    // TODO: add user_configurator changes
    // TODO: and liquidator_configurator changes
    // TODO: go through the errors and set the valid ones

    liquidator.require_auth();

    require_not_paused(env);

    let mut user_configurator = UserConfigurator::new(env, who, false);
    let user_config = user_configurator.user_config()?;
    let mut price_provider = PriceProvider::new(env)?;

    let account_data = calc_account_data(
        env,
        who,
        &CalcAccountDataCache::none(),
        user_config,
        &mut price_provider,
        true,
    )?;

    assert_with_error!(env, !account_data.is_good_position(), Error::GoodPosition);

    let liquidation_collat = account_data
        .liquidation_collat
        .ok_or(Error::LiquidateMathError)?;

    let liquidation_debt = account_data
        .liquidation_debt
        .ok_or(Error::LiquidateMathError)?;

    assert_with_error!(
        env,
        liquidation_collat.len() > 0 && liquidation_debt.len() > 0,
        Error::LiquidateMathError
    );

    // let (covered_debt, liquidated_collateral) = do_liquidate(
    //     env,
    //     liquidator,
    //     who,
    //     &mut user_configurator,
    //     &liquidation,
    //     receive_stoken,
    //     &mut price_provider,
    // )?;

    let mut total_debt_after_in_base = account_data.debt;
    let mut total_collat_disc_after_in_base = account_data.discounted_collateral;
    let mut total_debt_to_cover_in_base = 0i128;

    let initial_health = read_initial_health(env)?;
    let zero_percent = FixedI128::from_inner(0);
    let initial_health =
        FixedI128::from_percentage(initial_health).ok_or(Error::CalcAccountDataMathError)?;
    let hundred_percent =
        FixedI128::from_percentage(PERCENTAGE_FACTOR).ok_or(Error::CalcAccountDataMathError)?;
    let npv_percent = FixedI128::from_rational(account_data.npv, total_collat_disc_after_in_base)
        .ok_or(Error::CalcAccountDataMathError)?;

    let liq_bonus = npv_percent.min(zero_percent).abs().min(hundred_percent);

    let total_debt_liq_bonus = hundred_percent
        .checked_sub(liq_bonus)
        .ok_or(Error::CalcAccountDataMathError)?;

    for collat in liquidation_collat {
        let discount = FixedI128::from_percentage(collat.reserve.configuration.discount)
            .ok_or(Error::CalcAccountDataMathError)?;

        let safe_collat_in_base = hundred_percent
            .checked_sub(initial_health)
            .unwrap()
            .mul_int(total_collat_disc_after_in_base)
            .ok_or(Error::CalcAccountDataMathError)?
            .checked_sub(total_debt_after_in_base)
            .ok_or(Error::CalcAccountDataMathError)?;

        let safe_discount_level = discount
            .checked_mul(initial_health)
            .ok_or(Error::CalcAccountDataMathError)?;

        let safe_discount = discount
            .checked_add(liq_bonus)
            .ok_or(Error::CalcAccountDataMathError)?
            .checked_sub(hundred_percent)
            .ok_or(Error::CalcAccountDataMathError)?
            .checked_sub(safe_discount_level)
            .ok_or(Error::CalcAccountDataMathError)?;

        let liq_comp_amount =
            price_provider.convert_from_base(&collat.asset, safe_collat_in_base)?;

        let liq_comp_amount = safe_discount
            .recip_mul_int(liq_comp_amount)
            .ok_or(Error::CalcAccountDataMathError)?;

        let liq_max_comp_amount = liq_comp_amount
            .is_negative()
            .then(|| collat.comp_balance)
            .unwrap_or_else(|| collat.comp_balance.min(liq_comp_amount));

        let total_sub_comp_amount = discount
            .mul_int(liq_max_comp_amount)
            .ok_or(Error::CalcAccountDataMathError)?;

        let total_sub_amount_in_base =
            price_provider.convert_to_base(&collat.asset, total_sub_comp_amount)?;

        let debt_comp_amount = total_debt_liq_bonus
            .mul_int(liq_max_comp_amount)
            .ok_or(Error::CalcAccountDataMathError)?;

        let debt_in_base = price_provider.convert_to_base(&collat.asset, debt_comp_amount)?;

        total_debt_after_in_base = total_debt_after_in_base
            .checked_sub(debt_in_base)
            .ok_or(Error::CalcAccountDataMathError)?;

        total_collat_disc_after_in_base = total_collat_disc_after_in_base
            .checked_sub(total_sub_amount_in_base)
            .ok_or(Error::CalcAccountDataMathError)?;

        let npv_after = total_collat_disc_after_in_base
            .checked_sub(total_debt_after_in_base)
            .ok_or(Error::CalcAccountDataMathError)?;

        let s_token = STokenClient::new(env, &collat.reserve.s_token_address);

        let mut s_token_supply = read_token_total_supply(env, &collat.reserve.s_token_address);
        let debt_token_supply = read_token_total_supply(env, &collat.reserve.debt_token_address);

        let liq_lp_amount = FixedI128::from_inner(collat.coeff)
            .recip_mul_int(liq_max_comp_amount)
            .ok_or(Error::LiquidateMathError)?;

        if receive_stoken {
            let mut liquidator_configurator = UserConfigurator::new(env, liquidator, true);
            let liquidator_config = liquidator_configurator.user_config()?;

            assert_with_error!(
                env,
                !liquidator_config.is_borrowing(env, collat.reserve.get_id()),
                Error::MustNotHaveDebt
            );

            let liquidator_collat_before = read_token_balance(env, &s_token.address, liquidator);

            let liquidator_collat_after = liquidator_collat_before
                .checked_add(liq_lp_amount)
                .ok_or(Error::MathOverflowError)?;

            s_token.transfer_on_liquidation(who, liquidator, &s_token_amount);
            write_token_balance(env, &s_token.address, liquidator, liquidator_collat_after)?;

            let use_as_collat = liquidator_collat_before == 0;

            liquidator_configurator
                .deposit(reserve.get_id(), &asset, use_as_collat)?
                .write();
        } else {
            let amount_to_sub = liq_lp_amount
                .checked_neg()
                .ok_or(Error::LiquidateMathError)?;
            s_token_supply = s_token_supply
                .checked_sub(liq_lp_amount)
                .ok_or(Error::MathOverflowError)?;

            s_token.burn(who, &liq_lp_amount, &liq_max_comp_amount, liquidator);
            add_stoken_underlying_balance(env, &s_token.address, amount_to_sub)?;
        }

        write_token_total_supply(env, &collat.reserve.s_token_address, s_token_supply)?;

        recalculate_reserve_data(
            env,
            &collat.asset,
            &collat.reserve,
            s_token_supply,
            debt_token_supply,
        )?;

        total_debt_to_cover_in_base += debt_in_base;

        if npv_after.is_positive() {
            break;
        }
    }

    for debt in liquidation_debt {
        if total_debt_to_cover_in_base.eq(&0) {
            break;
        }

        let compounded_debt_in_base =
            price_provider.convert_to_base(&debt.asset, debt.compounded_debt)?;

        let (debt_amount_to_burn, underlying_amount_to_transfer) =
            if total_debt_to_cover_in_base >= compounded_debt_in_base {
                total_debt_to_cover_in_base -= compounded_debt_in_base;

                user_configurator.repay(debt.reserve.get_id(), true)?;

                (debt.debt_token_balance, debt.compounded_debt)
            } else {
                let compounded_debt =
                    price_provider.convert_from_base(&debt.asset, total_debt_to_cover_in_base)?;

                let debt_to_burn = FixedI128::from_inner(debt.debt_coeff)
                    .recip_mul_int(compounded_debt)
                    .ok_or(Error::LiquidateMathError)?;

                total_debt_to_cover_in_base = 0;

                (debt_to_burn, compounded_debt)
            };

        let underlying_asset = token::Client::new(env, &debt.asset);
        let debt_token = DebtTokenClient::new(env, &debt.reserve.debt_token_address);

        underlying_asset.transfer(
            liquidator,
            &debt.reserve.s_token_address,
            &underlying_amount_to_transfer,
        );

        debt_token.burn(who, &debt_amount_to_burn);

        let mut debt_token_supply = read_token_total_supply(env, &debt.reserve.debt_token_address);
        let s_token_supply = read_token_total_supply(env, &debt.reserve.s_token_address);

        debt_token_supply = debt_token_supply
            .checked_sub(debt_amount_to_burn)
            .ok_or(Error::MathOverflowError)?;

        add_stoken_underlying_balance(
            env,
            &debt.reserve.s_token_address,
            underlying_amount_to_transfer,
        )?;
        write_token_total_supply(env, &debt.reserve.debt_token_address, debt_token_supply)?;
        write_token_balance(
            env,
            &debt_token.address,
            who,
            debt.debt_token_balance - debt_amount_to_burn,
        )?;

        recalculate_reserve_data(
            env,
            &debt.asset,
            &debt.reserve,
            s_token_supply,
            debt_token_supply,
        )?;
    }

    user_configurator.write();

    // event::liquidation(env, who, covered_debt, liquidated_collateral);

    Ok(())
}

// fn do_liquidate(
//     env: &Env,
//     liquidator: &Address,
//     who: &Address,
//     user_configurator: &mut UserConfigurator,
//     liquidation_data: &LiquidationData,
//     receive_stoken: bool,
//     price_provider: &mut PriceProvider,
// ) -> Result<(i128, i128), Error> {
//     let mut remaining_debt_in_base = liquidation_data.debt_to_cover_in_base;

//     let LiquidationCollateral {
//         asset,
//         reserve_data: reserve,
//         s_token_balance: who_s_token_balance,
//         collat_coeff: coll_coeff_fixed,
//         is_last_collat: is_last_collateral,
//         compounded_collat: who_compounded_collat,
//     } = liquidation_data.collat_to_receive.clone().unwrap();

//     let coll_coeff = FixedI128::from_inner(coll_coeff_fixed);

//     let who_compounded_balance_in_base =
//         price_provider.convert_to_base(&asset, who_compounded_collat)?;

//     let liq_bonus = FixedI128::from_percentage(reserve.configuration.liq_bonus)
//         .ok_or(Error::LiquidateMathError)?;

//     let debt_to_cover_with_penalty_in_base = liq_bonus
//         .mul_int(remaining_debt_in_base)
//         .ok_or(Error::LiquidateMathError)?;

//     let (debt_to_cover_in_base, withdraw_amount_in_base) =
//         if debt_to_cover_with_penalty_in_base > who_compounded_balance_in_base {
//             // take all available collateral and decrease covered debt by bonus
//             let debt_to_cover_in_base =
//                 (FixedI128::from_inner(2 * FixedI128::DENOMINATOR - liq_bonus.into_inner()))
//                     .mul_int(who_compounded_balance_in_base)
//                     .ok_or(Error::LiquidateMathError)?;
//             (debt_to_cover_in_base, who_compounded_balance_in_base)
//         } else {
//             // take collateral with bonus and cover all debt
//             (remaining_debt_in_base, debt_to_cover_with_penalty_in_base)
//         };

//     let (s_token_amount, underlying_amount) =
//         if withdraw_amount_in_base != who_compounded_balance_in_base {
//             let underlying_amount =
//                 price_provider.convert_from_base(&asset, withdraw_amount_in_base)?;
//             let s_token_amount = coll_coeff
//                 .recip_mul_int(underlying_amount)
//                 .ok_or(Error::LiquidateMathError)?;
//             (s_token_amount, underlying_amount)
//         } else {
//             (who_s_token_balance, who_compounded_collat)
//         };

//     let s_token = STokenClient::new(env, &reserve.s_token_address);
//     let mut s_token_supply = read_token_total_supply(env, &reserve.s_token_address);
//     let debt_token_supply = read_token_total_supply(env, &reserve.debt_token_address);

//     if receive_stoken {
//         let mut liquidator_configurator = UserConfigurator::new(env, liquidator, true);
//         let liquidator_config = liquidator_configurator.user_config()?;

//         assert_with_error!(
//             env,
//             !liquidator_config.is_borrowing(env, reserve.get_id()),
//             Error::MustNotHaveDebt
//         );

//         let liquidator_collat_before = read_token_balance(env, &s_token.address, liquidator);

//         let liquidator_collat_after = liquidator_collat_before
//             .checked_add(s_token_amount)
//             .ok_or(Error::MathOverflowError)?;

//         s_token.transfer_on_liquidation(who, liquidator, &s_token_amount);
//         write_token_balance(env, &s_token.address, liquidator, liquidator_collat_after)?;

//         let use_as_collat = liquidator_collat_before == 0;

//         liquidator_configurator
//             .deposit(reserve.get_id(), &asset, use_as_collat)?
//             .write();
//     } else {
//         let amount_to_sub = underlying_amount
//             .checked_neg()
//             .ok_or(Error::MathOverflowError)?;
//         s_token_supply = s_token_supply
//             .checked_sub(s_token_amount)
//             .ok_or(Error::MathOverflowError)?;

//         s_token.burn(who, &s_token_amount, &underlying_amount, liquidator);
//         add_stoken_underlying_balance(env, &s_token.address, amount_to_sub)?;
//     }

//     // no overflow as withdraw_amount_in_base guaranteed less or equal to to_cover_in_base
//     remaining_debt_in_base -= debt_to_cover_in_base;

//     let is_withdraw = who_s_token_balance == s_token_amount;
//     user_configurator.withdraw(reserve.get_id(), &asset, is_withdraw)?;

//     write_token_total_supply(env, &reserve.s_token_address, s_token_supply)?;
//     let who_collat_after = who_s_token_balance
//         .checked_sub(s_token_amount)
//         .ok_or(Error::MathOverflowError)?;
//     write_token_balance(env, &s_token.address, who, who_collat_after)?;

//     recalculate_reserve_data(env, &asset, &reserve, s_token_supply, debt_token_supply)?;

//     assert_with_error!(
//         env,
//         !is_last_collateral || remaining_debt_in_base == 0,
//         Error::NotEnoughCollateral
//     );

//     let LiquidationDebt {
//         asset,
//         reserve_data,
//         debt_token_balance: who_debt_token_balance,
//         debt_coeff,
//         compounded_debt: who_compounded_debt,
//     } = liquidation_data.debt_to_cover.clone().unwrap();

//     let fully_repayed = remaining_debt_in_base == 0;

//     let (debt_amount_to_burn, underlying_amount_to_transfer) = if fully_repayed {
//         (who_debt_token_balance, who_compounded_debt)
//     } else {
//         // no overflow as remaining_debt_with_penalty always less then total_debt_with_penalty_in_base
//         let compounded_debt_to_cover =
//             price_provider.convert_from_base(&asset, debt_to_cover_in_base)?;
//         let debt_to_burn = FixedI128::from_inner(debt_coeff)
//             .recip_mul_int(compounded_debt_to_cover)
//             .ok_or(Error::LiquidateMathError)?;
//         (debt_to_burn, compounded_debt_to_cover)
//     };

//     let underlying_asset = token::Client::new(env, &asset);
//     let debt_token = DebtTokenClient::new(env, &reserve_data.debt_token_address);

//     underlying_asset.transfer(
//         liquidator,
//         &reserve_data.s_token_address,
//         &underlying_amount_to_transfer,
//     );
//     debt_token.burn(who, &debt_amount_to_burn);
//     user_configurator.repay(reserve_data.get_id(), fully_repayed)?;

//     let mut debt_token_supply = read_token_total_supply(env, &reserve_data.debt_token_address);
//     let s_token_supply = read_token_total_supply(env, &reserve_data.s_token_address);

//     debt_token_supply = debt_token_supply
//         .checked_sub(debt_amount_to_burn)
//         .ok_or(Error::MathOverflowError)?;

//     add_stoken_underlying_balance(
//         env,
//         &reserve_data.s_token_address,
//         underlying_amount_to_transfer,
//     )?;
//     write_token_total_supply(env, &reserve_data.debt_token_address, debt_token_supply)?;
//     write_token_balance(
//         env,
//         &debt_token.address,
//         who,
//         who_debt_token_balance - debt_amount_to_burn,
//     )?;

//     recalculate_reserve_data(
//         env,
//         &asset,
//         &reserve_data,
//         s_token_supply,
//         debt_token_supply,
//     )?;

//     user_configurator.write();

//     Ok((debt_to_cover_in_base, withdraw_amount_in_base))
// }
