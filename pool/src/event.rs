use soroban_sdk::{symbol_short, Address, Env, Symbol};

pub(crate) fn reserve_used_as_collateral_enabled(e: &Env, who: Address, asset: Address) {
    let topics = (Symbol::new(e, "reserve_used_as_coll_enabled"), who);
    e.events().publish(topics, asset);
}

pub(crate) fn reserve_used_as_collateral_disabled(e: &Env, who: Address, asset: Address) {
    let topics = (Symbol::new(e, "reserve_used_as_coll_disabled"), who);
    e.events().publish(topics, asset);
}

pub(crate) fn deposit(e: &Env, who: Address, asset: Address, amount: i128) {
    let topics = (symbol_short!("deposit"), who);
    e.events().publish(topics, (asset, amount));
}

pub(crate) fn withdraw(e: &Env, who: Address, asset: Address, to: Address, amount: i128) {
    let topics = (symbol_short!("withdraw"), who);
    e.events().publish(topics, (to, asset, amount));
}
