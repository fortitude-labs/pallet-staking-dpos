# Direct-DPOS Staking Module

The Staking module is used to manage funds at stake by network maintainers.

## Overview

The Staking module is the means by which a set of network maintainers (known as _authorities_ in
some contexts and _validators_ in others) are chosen based upon those who voluntarily place
funds under deposit.

### Terminology

- Staking: The process of locking up funds for some time in order to become a rewarded maintainer of the network.
- Validating: The process of running a node to actively maintain the network, either by
  producing blocks or guaranteeing finality of the chain.
- Delegating: The process of placing staked funds behind one or more validators 
- Stash account: The account holding an owner's funds used for staking.

### Goals

The staking system Direct-DPoS is designed to make the following possible:

- Stake funds that are controlled by a cold wallet.

### Scenarios

#### Staking

Almost any interaction with the Staking module requires a process of _**bonding**_ (also known
as being a _staker_). To become *bonded*, a fund-holding account known as the _stash account_,
which holds some or all of the funds that become frozen in place as part of the staking process,
used.

An account can become bonded using the `bond` call.


#### Validating

A **validator** takes the role of either validating blocks or ensuring their finality,
maintaining the veracity of the network. A validator should avoid both any sort of malicious
misbehavior and going offline. Bonded accounts that state interest in being a validator do NOT
get immediately chosen as a validator. Instead, they are declared as a _candidate_ and they
_might_ get elected at the _next period_ as a validator. The result of the election is determined
by delegators and their votes.

An account can become a validator candidate via the `bond`.

#### Delegation

A **delegator** does not take any _direct_ role in maintaining the network, instead, it votes on
a set of validators  to be elected. Once nterest in delegation is stated by an account, it
takes effect at the next election round. 
An account can become a delegator via the `vote` call.


### Session managing

The module implement the trait `SessionManager`. Which is the only API to query new validator
set and allowing these validator set to be rewarded once their era is ended.

## Interface

### Dispatchable Functions

The dispatchable functions of the Staking module enable the steps needed for entities to accept
and change their role, alongside some helper functions to get/set the metadata of the module.

### Public Functions

The Staking module contains many public storage items and (im)mutable functions.


### Election Algorithm

The current election algorithm select the validator with more stake.


## GenesisConfig

There's no genesis config. The module will start to propose winners when the minimum number of candidates is reached.

## Related Modules

- [Balances](https://docs.rs/pallet-balances/latest/pallet_balances/): Used to manage values at stake.
- [Session](https://docs.rs/pallet-session/latest/pallet_session/): Used to manage sessions. 

License: MIT
