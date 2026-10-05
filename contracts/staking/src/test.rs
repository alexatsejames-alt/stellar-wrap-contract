//! Tests for the staking contract.
//!
//! Paused behaviour (issue #874):
//! - `stake` is blocked while paused: opening new positions during a pause is
//!   defensible because it is a new commitment, not a user's existing funds.
//! - `unstake` is blocked while paused: it mutates the active position and
//!   starts the unbonding clock, so it is treated like `stake`.
//! - `withdraw_stake` is allowed while paused: the stake is already unbonded
//!   and the user is only reclaiming their own funds. Blocking it would trap
//!   user funds for the duration of the pause, which is not justifiable.

use super::*;
use soroban_sdk::{testutils::Address as _, Address, Env};

fn setup() -> (Env, StakingContractClient<'static>, Address) {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, StakingContract);
    let client = StakingContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    client.initialize(&admin);
    (env, client, admin)
}

#[test]
fn stake_is_blocked_while_paused() {
    let (env, client, admin) = setup();
    let user = Address::generate(&env);

    client.pause(&admin);

    let result = client.try_stake(&user, &1_000);
    assert_eq!(
        result,
        Err(Ok(ContractError::ContractPaused)),
        "stake must be blocked while paused"
    );
}

#[test]
fn unstake_is_blocked_while_paused() {
    let (env, client, admin) = setup();
    let user = Address::generate(&env);

    client.stake(&user, &1_000);
    client.pause(&admin);

    let result = client.try_unstake(&user, &1_000);
    assert_eq!(
        result,
        Err(Ok(ContractError::ContractPaused)),
        "unstake must be blocked while paused"
    );
}

#[test]
fn withdraw_stake_is_allowed_while_paused() {
    let (env, client, admin) = setup();
    let user = Address::generate(&env);

    client.stake(&user, &1_000);
    client.unstake(&user, &1_000);
    client.pause(&admin);

    // Already-unbonded stake must remain withdrawable during a pause so user
    // funds are not trapped for the duration of the pause.
    client.withdraw_stake(&user);

    assert_eq!(client.get_stake(&user), 0);
}

// ---------------------------------------------------------------------------
// Governance voting guards (issue #685)
// ---------------------------------------------------------------------------

/// Creates an active admin proposal and returns its id.
fn create_active_proposal(
    env: &Env,
    client: &StakingContractClient,
    admin: &Address,
    end_time: u64,
) -> u64 {
    let id = client.create_admin_proposal(admin, &end_time);
    assert_eq!(client.get_admin_proposal(&id).status, ProposalStatus::Active);
    id
}

#[test]
fn vote_on_unknown_proposal_fails() {
    let (env, client, admin) = setup();
    let voter = Address::generate(&env);

    let result = client.try_vote_admin_proposal(&voter, &999_u64, &true);
    assert_eq!(result, Err(Ok(Error::ProposalNotFound)));
}

#[test]
fn double_vote_fails() {
    let (env, client, admin) = setup();
    let voter = Address::generate(&env);
    let id = create_active_proposal(&env, &client, &admin, 1_000);

    client.vote_admin_proposal(&voter, &id, &true);

    let result = client.try_vote_admin_proposal(&voter, &id, &false);
    assert_eq!(result, Err(Ok(Error::ProposalAlreadyVoted)));
}

#[test]
fn vote_after_deadline_fails() {
    let (env, client, admin) = setup();
    let voter = Address::generate(&env);
    let id = create_active_proposal(&env, &client, &admin, 100);

    env.ledger().set_timestamp(101);

    let result = client.try_vote_admin_proposal(&voter, &id, &true);
    assert_eq!(result, Err(Ok(Error::ProposalVotingPeriodEnded)));
}

#[test]
fn vote_on_cancelled_proposal_fails() {
    let (env, client, admin) = setup();
    let voter = Address::generate(&env);
    let id = create_active_proposal(&env, &client, &admin, 1_000);

    client.cancel_admin_proposal(&admin, &id);

    let result = client.try_vote_admin_proposal(&voter, &id, &true);
    assert_eq!(result, Err(Ok(Error::ProposalNotActive)));
}

#[test]
fn vote_without_authorization_fails() {
    let env = Env::default();
    let contract_id = env.register_contract(None, StakingContract);
    let client = StakingContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    client.initialize(&admin);

    let voter = Address::generate(&env);
    let id = client.create_admin_proposal(&admin, &1_000);

    // No auth is mocked for the voter, so the authorization check must fail.
    let result = client.try_vote_admin_proposal(&voter, &id, &true);
    assert!(result.is_err(), "voting without authorization must fail");
}

#[test]
fn get_admin_proposal_vote_returns_recorded_choice() {
    let (env, client, admin) = setup();
    let voter = Address::generate(&env);
    let other = Address::generate(&env);
    let id = create_active_proposal(&env, &client, &admin, 1_000);

    assert_eq!(client.get_admin_proposal_vote(&id, &voter), None);

    client.vote_admin_proposal(&voter, &id, &true);

    assert_eq!(client.get_admin_proposal_vote(&id, &voter), Some(true));
    assert_eq!(client.get_admin_proposal_vote(&id, &other), None);
}
