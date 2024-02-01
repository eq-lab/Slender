use soroban_sdk::{contracttype, Address};

#[contracttype]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReserveType {
    /// Fungible reserve for wich created sToken and debtToken
    Fungible(Address, Address),
    /// RWA reserve
    RWA,
}