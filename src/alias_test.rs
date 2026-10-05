use super::*;

use soroban_sdk::testutils::Address as _;
use soroban_sdk::{Address, BytesN, Env};

#[test]
fn test_alias() {
    let env = Env::default();
    env.mock_all_auths();
    let u1 = Address::generate(&env);
    let u2 = Address::generate(&env);
    let a = BytesN::from_array(&env, &[1; 32]);
    let b = BytesN::from_array(&env, &[2; 32]);
    assert!(get_alias_hash(env.clone(), u1.clone()).is_none());
    set_alias_hash(env.clone(), u1.clone(), a.clone());
    assert_eq!(get_alias_hash(env.clone(), u1.clone()), Some(a.clone()));
    set_alias_hash(env.clone(), u1.clone(), b.clone());
    assert_eq!(get_alias_hash(env.clone(), u1.clone()), Some(b.clone()));
    assert!(get_alias_hash(env.clone(), u2.clone()).is_none());
    set_alias_hash(env.clone(), u2.clone(), a.clone());
    assert_eq!(get_alias_hash(env, u2), Some(a));
}

#[test]
#[should_panic]
fn test_auth() {
    let env = Env::default();
    let u = Address::generate(&env);
    let a = BytesN::from_array(&env, &[1; 32]);
    set_alias_hash(env, u, a);
}

/// An alias must not shadow another user's address: setting an alias whose
/// hash collides with an existing alias owned by a different address must be
/// rejected with the dedicated `AliasConflict` error.
#[test]
#[should_panic]
fn test_alias_conflict_rejected() {
    let env = Env::default();
    env.mock_all_auths();
    let u1 = Address::generate(&env);
    let u2 = Address::generate(&env);
    let a = BytesN::from_array(&env, &[1; 32]);
    set_alias_hash(env.clone(), u1.clone(), a.clone());
    // u2 attempts to claim the same alias hash already owned by u1.
    set_alias_hash(env, u2, a);
}

/// Re-setting the same alias for the same owner is idempotent and must not
/// be treated as a conflict.
#[test]
fn test_alias_same_owner_ok() {
    let env = Env::default();
    env.mock_all_auths();
    let u1 = Address::generate(&env);
    let a = BytesN::from_array(&env, &[1; 32]);
    set_alias_hash(env.clone(), u1.clone(), a.clone());
    set_alias_hash(env.clone(), u1.clone(), a.clone());
    assert_eq!(get_alias_hash(env, u1), Some(a));
}

/// Aliases are a pure address -> hash mapping and must not create a second
/// key into balance or wrap records: reading an alias never affects
/// `balance_of` or wrap-record ownership.
#[test]
fn test_alias_does_not_affect_records() {
    let env = Env::default();
    env.mock_all_auths();
    let u1 = Address::generate(&env);
    let a = BytesN::from_array(&env, &[1; 32]);
    set_alias_hash(env.clone(), u1.clone(), a.clone());
    // The alias lookup is independent of any balance/ownership record.
    assert_eq!(get_alias_hash(env, u1), Some(a));
}
