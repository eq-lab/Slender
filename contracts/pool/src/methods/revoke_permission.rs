use pool_interface::types::{error::Error, permission::Permission};
use soroban_sdk::{Address, Env};

use crate::{read_permission_owners, write_permission_owners};

use super::utils::validation::require_permission;

pub fn revoke_permission(
    env: &Env,
    who: &Address,
    owner: &Address,
    permission: &Permission,
) -> Result<(), Error> {
    require_permission(env, who, &Permission::Permission)?;

    let mut permission_owners = read_permission_owners(env, permission);

    if permission == &Permission::Permission && permission_owners.len() == 1 {
        return Err(Error::NoPermissioned);
    }

    if let Ok(idx) = permission_owners.binary_search(owner) {
        permission_owners.remove(idx);
        write_permission_owners(env, &permission_owners, permission);
    }

    Ok(())
}
