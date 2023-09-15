#![cfg(test)]
extern crate std;

use pool_interface::types::collateral_params_input::CollateralParamsInput;
use pool_interface::types::flash_loan_asset::FlashLoanAsset;
use pool_interface::types::init_reserve_input::InitReserveInput;
use pool_interface::types::ir_params::IRParams;
use pool_interface::LendingPoolClient;
use price_feed_interface::PriceFeedClient;
use soroban_sdk::testutils::{Address as _, Ledger};
use soroban_sdk::{vec, Address, Bytes, Env, IntoVal, Symbol, Val, Vec};
use std::fs::OpenOptions;
use std::io::prelude::*;

use crate::LendingPool;

use super::set_as_collateral::init_with_debt;
use super::sut::{
    create_pool_contract, create_price_feed_contract, create_s_token_contract,
    create_token_contract, fill_pool, fill_pool_three, init_pool, DAY,
};
use super::upgrade::{debt_token_v2, pool_v2, s_token_v2};

const CPU_LIMIT: u64 = 100_000_000;

#[test]
fn account_position() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, true);
    let (_, borrower, _, _) = fill_pool_three(&env, &sut);

    measure_budget(&env, nameof(account_position), || {
        sut.pool.account_position(&borrower);
    });
}

#[test]
fn borrow() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, true);
    let (_, borrower, debt_config) = fill_pool(&env, &sut, false);
    let token_address = debt_config.token.address.clone();

    measure_budget(&env, nameof(borrow), || {
        sut.pool.borrow(&borrower, &token_address, &20_000_000);
    });
}

#[test]
fn collat_coeff() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, true);
    let (_, _, _, debt_config) = fill_pool_three(&env, &sut);
    let debt_token = debt_config.token.address.clone();

    measure_budget(&env, nameof(collat_coeff), || {
        sut.pool.collat_coeff(&debt_token);
    });
}

#[test]
fn configure_as_collateral() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, true);
    let asset_address = sut.token().address.clone();
    let decimals = sut.s_token().decimals();
    let params = CollateralParamsInput {
        liq_bonus: 11_000,
        liq_cap: 100_000_000 * 10_i128.pow(decimals),
        util_cap: 9_000,
        discount: 6_000,
    };

    measure_budget(&env, nameof(configure_as_collateral), || {
        sut.pool
            .configure_as_collateral(&asset_address.clone(), &params.clone());
    });
}

#[test]
fn debt_coeff() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, true);
    let (_, _, _, debt_config) = fill_pool_three(&env, &sut);
    let debt_token = debt_config.token.address.clone();

    measure_budget(&env, nameof(debt_coeff), || {
        sut.pool.debt_coeff(&debt_token);
    });
}

#[test]
fn deposit() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, true);

    let user = Address::random(&env);
    let token_address = sut.token().address.clone();

    sut.token_admin().mint(&user, &10_000_000_000);

    measure_budget(&env, nameof(deposit), || {
        sut.pool.deposit(&user, &token_address, &5_000_000_000)
    });
}

#[test]
fn enable_borrowing_on_reserve() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, true);
    let asset = sut.token().address.clone();

    measure_budget(&env, nameof(enable_borrowing_on_reserve), || {
        sut.pool.enable_borrowing_on_reserve(&asset, &true);
    });
}

#[test]
fn get_reserve() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, true);
    let asset = sut.token().address.clone();

    measure_budget(&env, nameof(get_reserve), || {
        sut.pool.get_reserve(&asset);
    });
}

#[test]
fn init_reserve() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::random(&env);
    let token_admin = Address::random(&env);

    let (underlying_token, _) = create_token_contract(&env, &token_admin);
    let (debt_token, _) = create_token_contract(&env, &token_admin);

    let pool = create_pool_contract(&env, &admin, false);
    let s_token = create_s_token_contract(&env, &pool.address, &underlying_token.address);
    assert!(pool.get_reserve(&underlying_token.address).is_none());

    let init_reserve_input = InitReserveInput {
        s_token_address: s_token.address.clone(),
        debt_token_address: debt_token.address.clone(),
        // decimals: 9,
    };

    measure_budget(&env, nameof(init_reserve), || {
        pool.init_reserve(
            &underlying_token.address.clone(),
            // &false,
            &init_reserve_input.clone(),
        );
    });
}

#[test]
fn ir_params() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, true);

    measure_budget(&env, nameof(ir_params), || {
        sut.pool.ir_params();
    });
}

#[test]
fn liquidate() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, true);
    let (_, borrower, liquidator, _) = fill_pool_three(&env, &sut);

    sut.pool.liquidate(&liquidator, &borrower, &true);

    measure_budget(&env, nameof(liquidate), || {
        sut.pool.ir_params();
    });
}

#[test]
fn paused() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, true);

    measure_budget(&env, nameof(paused), || {
        sut.pool.paused();
    });
}

#[test]
fn price_feed() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, true);

    measure_budget(&env, nameof(price_feed), || {
        sut.pool.price_feed(&sut.token().address);
    });
}

#[test]
fn repay() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, true);
    let (_, borrower, debt_config) = fill_pool(&env, &sut, true);
    let debt_token = &debt_config.token.address;

    env.ledger().with_mut(|li| li.timestamp = 2 * DAY);

    measure_budget(&env, nameof(repay), || {
        sut.pool.repay(&borrower, &debt_token.clone(), &i128::MAX);
    });
}

#[test]
fn set_as_collateral() {
    let env = Env::default();
    env.mock_all_auths();
    let (sut, user, (_, _), (collat_token, _)) = init_with_debt(&env);

    sut.reserves[2].token_admin.mint(&user, &2_000_000_000);
    sut.pool
        .deposit(&user, &sut.reserves[2].token_admin.address, &2_000_000_000);

    measure_budget(&env, nameof(set_as_collateral), || {
        sut.pool.set_as_collateral(&user, &collat_token, &false);
    });
}

#[test]
fn set_decimals() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, true);

    measure_budget(&env, nameof(set_as_collateral), || {
        sut.pool.set_decimals(&sut.token().address, &9);
    });
}

#[test]
fn set_base_asset() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, true);

    measure_budget(&env, nameof(set_as_collateral), || {
        sut.pool.set_base_asset(&sut.token().address, &true);
    });
}

#[test]
fn set_ir_params() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, true);

    let ir_params_input = IRParams {
        alpha: 144,
        initial_rate: 201,
        max_rate: 50_001,
        scaling_coeff: 9_001,
    };

    measure_budget(&env, nameof(set_ir_params), || {
        sut.pool.set_ir_params(&ir_params_input);
    });
}

#[test]
fn set_pause() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, true);

    measure_budget(&env, nameof(set_pause), || {
        sut.pool.set_pause(&true);
    });
}

#[test]
fn set_price_feed() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::random(&env);
    let asset_1 = Address::random(&env);
    let asset_2 = Address::random(&env);

    let pool = create_pool_contract(&env, &admin, false);
    let price_feed: PriceFeedClient<'_> = create_price_feed_contract(&env);
    let assets = vec![&env, asset_1.clone(), asset_2.clone()];

    measure_budget(&env, nameof(set_price_feed), || {
        pool.set_price_feed(&price_feed.address.clone(), &assets.clone());
    });
}

#[test]
fn set_reserve_status() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, true);
    let asset = sut.token().address.clone();

    measure_budget(&env, nameof(set_reserve_status), || {
        sut.pool.set_reserve_status(&asset, &true);
    });
}

#[test]
fn stoken_underlying_balance() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, true);
    let lender = Address::random(&env);

    sut.reserves[0].token_admin.mint(&lender, &2_000_000_000);
    sut.pool
        .deposit(&lender, &sut.reserves[0].token.address, &1_000_000_000);

    measure_budget(&env, nameof(stoken_underlying_balance), || {
        sut.pool
            .stoken_underlying_balance(&sut.reserves[0].s_token.address);
    });
}

#[test]
fn treasury() {
    let env = Env::default();
    env.mock_all_auths();

    let pool = LendingPoolClient::new(&env, &env.register_contract(None, LendingPool));
    let flash_loan_fee = 5;

    pool.initialize(
        &Address::random(&env),
        &Address::random(&env),
        &flash_loan_fee,
        &IRParams {
            alpha: 143,
            initial_rate: 200,
            max_rate: 50_000,
            scaling_coeff: 9_000,
        },
    );

    measure_budget(&env, nameof(treasury), || {
        pool.treasury();
    });
}

#[test]
fn user_configuration() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, true);
    let (_, borrower, _) = fill_pool(&env, &sut, false);

    measure_budget(&env, nameof(user_configuration), || {
        sut.pool.user_configuration(&borrower);
    });
}

#[test]
fn withdraw() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, true);
    let (_, borrower, _) = fill_pool(&env, &sut, false);

    measure_budget(&env, nameof(withdraw), || {
        sut.pool
            .withdraw(&borrower, &sut.token().address, &10_000, &borrower);
    });
}

#[test]
fn flash_loan_fee() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, true);

    measure_budget(&env, nameof(flash_loan_fee), || {
        sut.pool.flash_loan_fee();
    });
}

#[test]
fn set_flash_loan_fee() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, true);

    measure_budget(&env, nameof(set_flash_loan_fee), || {
        sut.pool.set_flash_loan_fee(&15);
    });
}

#[test]
fn flash_loan() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, false);
    let (_, borrower, _) = fill_pool(&env, &sut, false);

    let _: Val = env.invoke_contract(
        &sut.flash_loan_receiver.address,
        &Symbol::new(&env, "initialize"),
        vec![
            &env,
            borrower.into_val(&env),
            sut.pool.address.into_val(&env),
            false.into_val(&env),
        ],
    );

    let loan_assets = Vec::from_array(
        &env,
        [
            FlashLoanAsset {
                asset: sut.reserves[0].token.address.clone(),
                amount: 1000000,
                borrow: false,
            },
            FlashLoanAsset {
                asset: sut.reserves[1].token.address.clone(),
                amount: 2000000,
                borrow: true,
            },
            FlashLoanAsset {
                asset: sut.reserves[2].token.address.clone(),
                amount: 3000000,
                borrow: true,
            },
        ],
    );

    measure_budget(&env, nameof(flash_loan), || {
        sut.pool.flash_loan(
            &borrower,
            &sut.flash_loan_receiver.address,
            &loan_assets,
            &Bytes::new(&env),
        );
    });
}

#[test]
fn upgrade() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, true);
    let pool_v2_wasm = env.deployer().upload_contract_wasm(pool_v2::WASM);

    measure_budget(&env, nameof(upgrade), || {
        sut.pool.upgrade(&pool_v2_wasm);
    });
}

#[test]
fn upgrade_s_token() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, true);
    let asset = sut.reserves[0].token.address.clone();

    let s_token_v2_wasm = env.deployer().upload_contract_wasm(s_token_v2::WASM);

    measure_budget(&env, nameof(upgrade_s_token), || {
        sut.pool.upgrade_s_token(&asset, &s_token_v2_wasm);
    });
}

#[test]
fn upgrade_debt_token() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, true);
    let debt_token_v2_wasm = env.deployer().upload_contract_wasm(debt_token_v2::WASM);
    let asset = sut.reserves[0].token.address.clone();

    measure_budget(&env, nameof(upgrade_debt_token), || {
        sut.pool.upgrade_debt_token(&asset, &debt_token_v2_wasm);
    });
}

fn measure_budget(env: &Env, function: &str, callback: impl FnOnce()) {
    let cpu_before = env.budget().cpu_instruction_cost();
    // TODO: bug in v0.9.2 (returns CPU cost)
    let memory_before = env.budget().memory_bytes_cost();

    callback();

    let cpu_after = env.budget().cpu_instruction_cost();
    let memory_after = env.budget().memory_bytes_cost();

    let cpu = cpu_after - cpu_before;
    let memory = memory_after - memory_before;

    let budget = &[
        std::format!("['{}'] = {{\n", function),
        std::format!("    \"cpu_cost\": {},\n", cpu),
        std::format!("    \"memory_cost\": {},\n", memory),
        std::format!("    \"cpu_limit_exceeded\": {},\n", cpu > CPU_LIMIT),
        std::format!("}}"),
    ]
    .concat();

    let mut file = OpenOptions::new()
        .create(true)
        .write(true)
        .append(true)
        .open("src/tests/snapshots/budget_utilization.snap")
        .unwrap();
    let result = writeln!(file, "{}", budget);

    if let Err(e) = result {
        panic!("Failed to write budget consumption: {}", e);
    }
}

fn nameof<F>(_: F) -> &'static str
where
    F: Fn(),
{
    std::any::type_name::<F>()
}
