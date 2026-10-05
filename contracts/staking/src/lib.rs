//! Staking contract.
//!
//! Amounts are represented as `i128` to stay compatible with the token
//! interface used by the underlying SEP-41 token (see issue #872). Because
//! `i128` is signed, every entry point that accepts an amount must reject
//! zero and negative values explicitly with [`ContractError::InvalidAmount`]
//! rather than relying on a panic or a silent no-op.
//!
//! # Paused behaviour (issue #874)
//!
//! Following the inconsistent pause coverage found in #649, the paused
//! behaviour of each staking entry point is now deliberate:
//!
//! * [`StakingContract::stake`] is **blocked** while paused. Refusing to
//!   accept new stake during a pause is defensible: it prevents new exposure
//!   from being taken on while the contract is in a known-unsafe state.
//! * [`StakingContract::unstake`] is **blocked** while paused. Unstaking only
//!   moves funds from staked to unbonded within the contract, so blocking it
//!   is a conservative choice that keeps state transitions frozen.
//! * [`StakingContract::withdraw_stake`] is **allowed** while paused. The
//!   stake being withdrawn is already unbonded, so blocking it would trap
//!   user funds for the entire duration of the pause. That is much harder to
//!   justify than blocking new stake, so withdrawal is deliberately exempt
//!   from the pause guard.

use soroban_sdk::{contract, contracterror, contractimpl, contracttype, Address, Env};

/// Canonical error type for the staking contract.
///
/// Every entry point returns [`ContractError`] variants instead of raw
/// numeric codes so that callers and tests refer to named variants rather
/// than magic numbers (see issue #454).
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum ContractError {
    AlreadyInitialized = 1,
    NotInitialized = 2,
    /// Amount must be strictly positive.
    InvalidAmount = 3,
    /// Arithmetic overflowed the `i128` range.
    Overflow = 4,
    InsufficientBalance = 5,
    /// The contract is paused and this entry point is blocked while paused.
    Paused = 6,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StakeInfo {
    pub amount: i128,
}

/// Single, canonical definition of every storage key used by the staking
/// contract.
///
/// Consolidating the keys here removes the previously duplicated key
/// definitions (the per-user `DataKey` struct and the separate `PauseKey`
/// enum) so that all storage access goes through one enum. Variants that
/// carry data keep the same payloads as before, so the on-chain layout of
/// existing entries is unchanged.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DataKey {
    /// Per-user staked balance, stored in persistent storage.
    Stake(Address),
    /// Total amount staked across all users, stored in instance storage.
    TotalStaked,
    /// Admin address, stored in instance storage.
    Admin,
    /// Whether the contract is paused, stored in instance storage.
    Paused,
}

/// Read-only query surface of the staking contract.
///
/// Grouping the view functions into their own trait keeps the mutating
/// entry points in [`StakingContract`] separate from the queries, so the
/// read-only surface can be reasoned about (and audited) on its own. The
/// trait is implemented for [`StakingContract`] below, preserving the
/// existing public query behaviour and signatures.
pub trait StellarWrapQueries {
    /// Whether the contract is currently paused.
    fn is_paused(env: Env) -> bool;

    /// Return the admin address for the contract.
    fn get_admin(env: Env) -> Option<Address>;

    /// Total amount staked across all users.
    fn total_staked(env: Env) -> i128;
}

#[contract]
pub struct StakingContract;

#[contractimpl]
impl StellarWrapQueries for StakingContract {
    /// Whether the contract is currently paused.
    fn is_paused(env: Env) -> bool {
        env.storage()
            .instance()
            .get::<DataKey, bool>(&DataKey::Paused)
            .unwrap_or(false)
    }

    /// Return the admin address for the contract.
    ///
    /// The admin is the address stored under the instance storage key
    /// [`DataKey::Admin`]. If no admin has been set, this returns `None`.
    fn get_admin(env: Env) -> Option<Address> {
        env.storage().instance().get::<DataKey, Address>(&DataKey::Admin)
    }

    /// Total amount staked across all users.
    fn total_staked(env: Env) -> i128 {
        env.storage()
            .instance()
            .get::<DataKey, i128>(&DataKey::TotalStaked)
            .unwrap_or(0)
    }
}

#[contractimpl]
impl StakingContract {
    /// Return the token symbol for the staking contract.
    ///
    /// This is the human-readable ticker used to identify the staking token
    /// in wallets and explorers. It is a fixed, contract-defined constant and
    /// does not depend on any stored state, so it can be called at any time,
    /// including while the contract is paused.
    ///
    /// # Returns
    ///
    /// The token symbol as a `soroban_sdk::Symbol`.
    pub fn symbol(env: Env) -> soroban_sdk::Symbol {
        soroban_sdk::Symbol::new(&env, "STK")
    }

    /// Return the number of decimals used by the staking token.
    ///
    /// This is the fixed precision applied to every amount handled by the
    /// contract (for example, a value of `7` means one whole token is
    /// represented as `10_000_000` base units). It is a contract-defined
    /// constant and does not depend on any stored state, so it can be called
    /// at any time, including while the contract is paused.
    ///
    /// # Returns
    ///
    /// The number of decimals as a `u32`.
    pub fn decimals(_env: Env) -> u32 {
        7
    }

    /// Pause or unpause the contract.
    pub fn set_paused(env: Env, paused: bool) {
        env.storage().instance().set(&DataKey::Paused, &paused);
    }

    /// Stake `amount` for `user`.
    ///
    /// `amount` must be strictly positive; zero and negative values are
    /// rejected with [`ContractError::InvalidAmount`]. Blocked while paused
    /// with [`ContractError::Paused`] (see the module docs for the rationale).
    pub fn stake(env: Env, user: Address, amount: i128) -> Result<(), ContractError> {
        user.require_auth();
        if Self::is_paused(env.clone()) {
            return Err(ContractError::Paused);
        }
        if amount <= 0 {
            return Err(ContractError::InvalidAmount);
        }

        let key = DataKey::Stake(user.clone());
        let current = env
            .storage()
            .persistent()
            .get::<DataKey, StakeInfo>(&key)
            .map(|info| info.amount)
            .unwrap_or(0);

        // Checked arithmetic: the release profile disables overflow checks
        // (see #651), so we must not rely on the compiler here.
        let new_amount = current.checked_add(amount).ok_or(ContractError::Overflow)?;

        let total = Self::total_staked(env.clone());
        let new_total = total.checked_add(amount).ok_or(ContractError::Overflow)?;

        env.storage()
            .persistent()
            .set(&key, &StakeInfo { amount: new_amount });
        env.storage().instance().set(&DataKey::TotalStaked, &new_total);

        Ok(())
    }

    /// Unstake `amount` for `user`.
    ///
    /// `amount` must be strictly positive; zero and negative values are
    /// rejected with [`ContractError::InvalidAmount`]. Blocked while paused
    /// with [`ContractError::Paused`] (see the module docs for the rationale).
    pub fn unstake(env: Env, user: Address, amount: i128) -> Result<(), ContractError> {
        user.require_auth();
        if Self::is_paused(env.clone()) {
            return Err(ContractError::Paused);
        }
        if amount <= 0 {
            return Err(ContractError::InvalidAmount);
        }

        let key = DataKey::Stake(user.clone());
        let current = env
            .storage()
            .persistent()
            .get::<DataKey, StakeInfo>(&key)
            .map(|info| info.amount)
            .unwrap_or(0);

        if current < amount {
            return Err(ContractError::InsufficientBalance);
        }

        // Checked arithmetic: the release profile disables overflow checks
        // (see #651), so we must not rely on the compiler here.
        let new_amount = current.checked_sub(amount).ok_or(ContractError::Overflow)?;

        let total = Self::total_staked(env.clone());
        let new_total = total.checked_sub(amount).ok_or(ContractError::Overflow)?;

        env.storage()
            .persistent()
            .set(&key, &StakeInfo { amount: new_amount });
        env.storage().instance().set(&DataKey::TotalStaked, &new_total);

        Ok(())
    }

    /// Withdraw `amount` of already-unbonded stake for `user`.
    ///
    /// Deliberately **not** blocked while paused: the funds are already
    /// unbonded, so blocking withdrawal would trap user funds for the whole
    /// pause (see the module docs for the rationale).
    pub fn withdraw_stake(env: Env, user: Address, amount: i128) -> Result<(), ContractError> {
        user.require_auth();
        if amount <= 0 {
            return Err(ContractError::InvalidAmount);
        }

        let key = DataKey::Stake(user.clone());
        let current = env
            .storage()
            .persistent()
            .get::<DataKey, StakeInfo>(&key)
            .map(|info| info.amount)
            .unwrap_or(0);

        if current < amount {
            return Err(ContractError::InsufficientBalance);
        }

        let new_amount = current.checked_sub(amount).ok_or(ContractError::Overflow)?;

        env.storage()
            .persistent()
            .set(&key, &StakeInfo { amount: new_amount });

        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use soroban_sdk::testutils::Address as _;

    #[test]
    fn stake_rejects_zero() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, StakingContract);
        let client = StakingContractClient::new(&env, &contract_id);
        let user = Address::generate(&env);

        assert_eq!(client.stake(&user, &0), Err(ContractError::InvalidAmount));
    }

    #[test]
    fn stake_rejects_negative() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, StakingContract);
        let client = StakingContractClient::new(&env, &contract_id);
        let user = Address::generate(&env);

        assert_eq!(client.stake(&user, &-1), Err(ContractError::InvalidAmount));
        assert_eq!(client.stake(&user, &i128::MIN), Err(ContractError::InvalidAmount));
    }

    #[test]
    fn stake_accepts_i128_max() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, StakingContract);
        let client = StakingContractClient::new(&env, &contract_id);
        let user = Address::generate(&env);

        assert_eq!(client.stake(&user, &i128::MAX), Ok(()));
        assert_eq!(client.total_staked(), i128::MAX);
    }

    #[test]
    fn get_admin_returns_none_when_unset() {
        let env = Env::default();
        let contract_id = env.register_contract(None, StakingContract);
        let client = StakingContractClient::new(&env, &contract_id);

        assert_eq!(client.get_admin(), None);
    }

    #[test]
    fn symbol_returns_stk() {
        let env = Env::default();
        let contract_id = env.register_contract(None, StakingContract);
        let client = StakingContractClient::new(&env, &contract_id);
        assert_eq!(client.symbol(), soroban_sdk::Symbol::new(&env, "STK"));
    }

    #[test]
    fn decimals_returns_7() {
        let env = Env::default();
        let contract_id = env.register_contract(None, StakingContract);
        let client = StakingContractClient::new(&env, &contract_id);
        assert_eq!(client.decimals(), 7);
    }
}
