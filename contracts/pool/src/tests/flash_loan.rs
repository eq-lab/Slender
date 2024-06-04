use crate::tests::sut::{fill_pool, init_pool};
use common::FixedI128;
use pool_interface::types::flash_loan_asset::FlashLoanAsset;
use pool_interface::types::pool_config::PoolConfig;
use soroban_sdk::testutils::Events;
use soroban_sdk::{vec, Bytes, Env, IntoVal, Symbol, Val, Vec};

#[test]
#[should_panic(expected = "HostError: Error(Contract, #310)")]
fn should_fail_when_receiver_receive_returns_false() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, false);
    let (_, borrower, _) = fill_pool(&env, &sut, false);

    let _: Val = env.invoke_contract(
        &sut.flash_loan_receiver.address,
        &Symbol::new(&env, "initialize"),
        vec![&env, sut.pool.address.into_val(&env), true.into_val(&env)],
    );

    let loan_assets = Vec::from_array(
        &env,
        [FlashLoanAsset {
            asset: sut.reserves[0].token.address.clone(),
            amount: 1000000,
            borrow: false,
        }],
    );

    sut.pool.flash_loan(
        &borrower,
        &sut.flash_loan_receiver.address,
        &loan_assets,
        &Bytes::new(&env),
    );
}

#[test]
fn should_require_borrower_to_pay_fee() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, false);
    let (_, borrower, _) = fill_pool(&env, &sut, false);

    let _: Val = env.invoke_contract(
        &sut.flash_loan_receiver.address,
        &Symbol::new(&env, "initialize"),
        vec![&env, sut.pool.address.into_val(&env), false.into_val(&env)],
    );

    let loan_assets = Vec::from_array(
        &env,
        [
            FlashLoanAsset {
                asset: sut.reserves[0].token.address.clone(),
                amount: 10_000,
                borrow: false,
            },
            FlashLoanAsset {
                asset: sut.reserves[1].token.address.clone(),
                amount: 2_000_000,
                borrow: false,
            },
            FlashLoanAsset {
                asset: sut.reserves[2].token.address.clone(),
                amount: 3_000_000,
                borrow: false,
            },
        ],
    );

    let treasury_asset_0_before = sut.pool.protocol_fee(&sut.reserves[0].token.address);
    let treasury_asset_1_before = sut.pool.protocol_fee(&sut.reserves[1].token.address);
    let treasury_asset_2_before = sut.pool.protocol_fee(&sut.reserves[2].token.address);

    let s_token_undetlying_asset_0_before = sut.reserves[0]
        .token
        .balance(&sut.reserves[0].s_token().address);
    let s_token_undetlying_asset_1_before = sut.reserves[0]
        .token
        .balance(&sut.reserves[1].s_token().address);
    let s_token_undetlying_asset_2_before = sut.reserves[0]
        .token
        .balance(&sut.reserves[2].s_token().address);

    sut.pool.flash_loan(
        &borrower,
        &sut.flash_loan_receiver.address,
        &loan_assets,
        &Bytes::new(&env),
    );

    let treasury_asset_0_after = sut.pool.protocol_fee(&sut.reserves[0].token.address);
    let treasury_asset_1_after = sut.pool.protocol_fee(&sut.reserves[1].token.address);
    let treasury_asset_2_after = sut.pool.protocol_fee(&sut.reserves[2].token.address);

    let s_token_undetlying_asset_0_after = sut.reserves[0]
        .token
        .balance(&sut.reserves[0].s_token().address);
    let s_token_undetlying_asset_1_after = sut.reserves[0]
        .token
        .balance(&sut.reserves[1].s_token().address);
    let s_token_undetlying_asset_2_after = sut.reserves[0]
        .token
        .balance(&sut.reserves[2].s_token().address);

    assert_eq!(treasury_asset_0_before, 0);
    assert_eq!(treasury_asset_1_before, 0);
    assert_eq!(treasury_asset_2_before, 0);
    assert_eq!(s_token_undetlying_asset_0_before, 2_000_000);
    assert_eq!(s_token_undetlying_asset_1_before, 0);
    assert_eq!(s_token_undetlying_asset_2_before, 0);

    assert_eq!(treasury_asset_0_after, 5);
    assert_eq!(treasury_asset_1_after, 1000);
    assert_eq!(treasury_asset_2_after, 1500);
    assert_eq!(s_token_undetlying_asset_0_after, 2_000_000);
    assert_eq!(s_token_undetlying_asset_1_after, 0);
    assert_eq!(s_token_undetlying_asset_2_after, 0);
}

#[test]
fn should_borrow_if_borrowing_specified_on_asset() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, false);
    let (_, borrower, _) = fill_pool(&env, &sut, false);

    let _: Val = env.invoke_contract(
        &sut.flash_loan_receiver.address,
        &Symbol::new(&env, "initialize"),
        vec![&env, sut.pool.address.into_val(&env), false.into_val(&env)],
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
                borrow: false,
            },
            FlashLoanAsset {
                asset: sut.reserves[2].token.address.clone(),
                amount: 3000000,
                borrow: true,
            },
        ],
    );

    let borrower_debt_before = sut.reserves[2].debt_token().balance(&borrower);

    sut.pool.flash_loan(
        &borrower,
        &sut.flash_loan_receiver.address,
        &loan_assets,
        &Bytes::new(&env),
    );

    let borrower_debt_after = sut.reserves[2].debt_token().balance(&borrower);

    assert_eq!(borrower_debt_before, 0);
    assert_eq!(borrower_debt_after, 3000001);
}

#[test]
fn should_emit_events() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, false);
    let (_, borrower, _) = fill_pool(&env, &sut, false);

    let _: Val = env.invoke_contract(
        &sut.flash_loan_receiver.address,
        &Symbol::new(&env, "initialize"),
        vec![&env, sut.pool.address.into_val(&env), false.into_val(&env)],
    );

    let loan_assets = Vec::from_array(
        &env,
        [FlashLoanAsset {
            asset: sut.reserves[0].token.address.clone(),
            amount: 1000000,
            borrow: false,
        }],
    );

    sut.pool.flash_loan(
        &borrower,
        &sut.flash_loan_receiver.address,
        &loan_assets,
        &Bytes::new(&env),
    );

    let events = env.events().all().pop_back_unchecked();

    assert_eq!(
        vec![&env, events],
        vec![
            &env,
            (
                sut.pool.address.clone(),
                (
                    Symbol::new(&env, "flash_loan"),
                    &borrower,
                    &sut.flash_loan_receiver.address,
                    &sut.reserves[0].token.address
                )
                    .into_val(&env),
                (1000000i128, 500i128).into_val(&env)
            ),
        ]
    );
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #205)")]
fn rwa_fail_when_exceed_assets_limit() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, false);
    let (_, borrower, _) = fill_pool(&env, &sut, false);

    sut.pool.set_pool_configuration(
        &sut.pool_admin,
        &PoolConfig {
            base_asset_address: sut.reserves[0].token.address.clone(),
            base_asset_decimals: sut.reserves[0].token.decimals(),
            flash_loan_fee: 5,
            initial_health: 0,
            timestamp_window: 20,
            user_assets_limit: 1,
            min_collat_amount: 0,
            min_debt_amount: 0,
            liquidation_protocol_fee: 0,
        },
    );

    let _: Val = env.invoke_contract(
        &sut.flash_loan_receiver.address,
        &Symbol::new(&env, "initialize"),
        vec![&env, sut.pool.address.into_val(&env), false.into_val(&env)],
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
                borrow: false,
            },
            FlashLoanAsset {
                asset: sut.reserves[2].token.address.clone(),
                amount: 3000000,
                borrow: true,
            },
        ],
    );

    sut.pool.flash_loan(
        &borrower,
        &sut.flash_loan_receiver.address,
        &loan_assets,
        &Bytes::new(&env),
    );
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #208)")]
fn should_fail_when_debt_lt_min_position_amount() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, false);
    let (_, borrower, _) = fill_pool(&env, &sut, false);

    sut.pool.set_pool_configuration(
        &sut.pool_admin,
        &PoolConfig {
            base_asset_address: sut.reserves[0].token.address.clone(),
            base_asset_decimals: sut.reserves[0].token.decimals(),
            flash_loan_fee: 5,
            initial_health: 0,
            timestamp_window: 20,
            user_assets_limit: 2,
            min_collat_amount: 0,
            min_debt_amount: 4_000_000,
            liquidation_protocol_fee: 0,
        },
    );

    let _: Val = env.invoke_contract(
        &sut.flash_loan_receiver.address,
        &Symbol::new(&env, "initialize"),
        vec![&env, sut.pool.address.into_val(&env), false.into_val(&env)],
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
                borrow: false,
            },
            FlashLoanAsset {
                asset: sut.reserves[2].token.address.clone(),
                amount: 3000000,
                borrow: true,
            },
        ],
    );

    sut.pool.flash_loan(
        &borrower,
        &sut.flash_loan_receiver.address,
        &loan_assets,
        &Bytes::new(&env),
    );
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #6)")]
fn should_fail_if_borrow_in_grace_period() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, false);
    let (_, borrower, _) = fill_pool(&env, &sut, false);

    let _: Val = env.invoke_contract(
        &sut.flash_loan_receiver.address,
        &Symbol::new(&env, "initialize"),
        vec![&env, sut.pool.address.into_val(&env), false.into_val(&env)],
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
                borrow: false,
            },
            FlashLoanAsset {
                asset: sut.reserves[2].token.address.clone(),
                amount: 3000000,
                borrow: true,
            },
        ],
    );

    sut.pool.set_pause(&sut.pool_admin, &true);
    sut.pool.set_pause(&sut.pool_admin, &false);

    sut.pool.flash_loan(
        &borrower,
        &sut.flash_loan_receiver.address,
        &loan_assets,
        &Bytes::new(&env),
    );
}

#[test]
fn should_not_fail_in_grace_period() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, false);
    let (_, borrower, _) = fill_pool(&env, &sut, false);

    let _: Val = env.invoke_contract(
        &sut.flash_loan_receiver.address,
        &Symbol::new(&env, "initialize"),
        vec![&env, sut.pool.address.into_val(&env), false.into_val(&env)],
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
                borrow: false,
            },
            FlashLoanAsset {
                asset: sut.reserves[2].token.address.clone(),
                amount: 3000000,
                borrow: false,
            },
        ],
    );

    sut.pool.set_pause(&sut.pool_admin, &true);
    sut.pool.set_pause(&sut.pool_admin, &false);

    sut.pool.flash_loan(
        &borrower,
        &sut.flash_loan_receiver.address,
        &loan_assets,
        &Bytes::new(&env),
    );
}

#[test]
fn flashloan_should_pay_protocol_fee_if_not_borrow() {
    let env = Env::default();
    env.mock_all_auths();

    let sut = init_pool(&env, false);
    let (_, borrower, _) = fill_pool(&env, &sut, false);

    let _: Val = env.invoke_contract(
        &sut.flash_loan_receiver.address,
        &Symbol::new(&env, "initialize"),
        vec![&env, sut.pool.address.into_val(&env), false.into_val(&env)],
    );

    let flash_loan_fee =
        FixedI128::from_percentage(sut.pool.pool_configuration().flash_loan_fee).unwrap();

    let protocol_fee_0_before = sut.pool.protocol_fee(&sut.reserves[0].token.address);
    let protocol_fee_1_before = sut.pool.protocol_fee(&sut.reserves[1].token.address);

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
                borrow: false,
            },
            FlashLoanAsset {
                asset: sut.reserves[2].token.address.clone(),
                amount: 3000000,
                borrow: true,
            },
        ],
    );

    sut.pool.flash_loan(
        &borrower,
        &sut.flash_loan_receiver.address,
        &loan_assets,
        &Bytes::new(&env),
    );

    let protocol_fee_0_after = sut.pool.protocol_fee(&sut.reserves[0].token.address);
    let protocol_fee_1_after = sut.pool.protocol_fee(&sut.reserves[1].token.address);

    assert!(protocol_fee_0_after > protocol_fee_0_before);
    assert!(protocol_fee_1_after > protocol_fee_1_before);

    assert_eq!(
        protocol_fee_0_after - protocol_fee_0_before,
        flash_loan_fee.mul_int(1000000).unwrap()
    );
    assert_eq!(
        protocol_fee_1_after - protocol_fee_1_before,
        flash_loan_fee.mul_int(2000000).unwrap()
    );
}
