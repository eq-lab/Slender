use crate::*;
use soroban_sdk::testutils::Address as _;

#[test]
fn shoould_return_treasury_address() {
    let env = Env::default();
    env.mock_all_auths();

    let pool = LendingPoolClient::new(&env, &env.register_contract(None, LendingPool));

    let admin = Address::random(&env);
    let treasury = Address::random(&env);

    pool.initialize(
        &admin,
        &treasury,
        &IRParams {
            alpha: 143,
            initial_rate: 200,
            max_rate: 50_000,
            scaling_coeff: 9_000,
        },
    );

    assert_eq!(pool.treasury(), treasury);
}
