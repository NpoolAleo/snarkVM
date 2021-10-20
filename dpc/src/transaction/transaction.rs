// Copyright (C) 2019-2021 Aleo Systems Inc.
// This file is part of the snarkVM library.

// The snarkVM library is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// The snarkVM library is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with the snarkVM library. If not, see <https://www.gnu.org/licenses/>.

use crate::{
    record::*,
    Address,
    AleoAmount,
    Event,
    LocalCommitments,
    Network,
    Request,
    Transition,
    ViewKey,
    VirtualMachine,
};
use snarkvm_algorithms::CRH;
use snarkvm_utilities::{has_duplicates, to_bytes_le, FromBytes, ToBytes};

use anyhow::{anyhow, Result};
use rand::{CryptoRng, Rng};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashSet,
    hash::{Hash, Hasher},
    io::{Read, Result as IoResult, Write},
};

#[derive(Derivative, Serialize, Deserialize)]
#[derivative(
    Clone(bound = "N: Network"),
    Debug(bound = "N: Network"),
    PartialEq(bound = "N: Network"),
    Eq(bound = "N: Network")
)]
pub struct Transaction<N: Network> {
    /// The ID of this transaction.
    transaction_id: N::TransactionID,
    /// The network ID.
    network_id: u16,
    /// The ID of the inner circuit used to execute this transaction.
    inner_circuit_id: N::InnerCircuitID,
    /// The state transition.
    transitions: Vec<Transition<N>>,
    /// The events emitted from this transaction.
    events: Vec<Event<N>>,
}

impl<N: Network> Transaction<N> {
    /// Initializes a new transaction from a request.
    #[inline]
    pub fn new<R: Rng + CryptoRng>(request: &Request<N>, rng: &mut R) -> Result<Self> {
        VirtualMachine::<N>::new()?.execute(request, rng)?.finalize()
    }

    /// Initializes a new coinbase transaction.
    #[inline]
    pub fn new_coinbase<R: Rng + CryptoRng>(recipient: Address<N>, amount: AleoAmount, rng: &mut R) -> Result<Self> {
        let request = Request::new_coinbase(recipient, amount, rng)?;
        VirtualMachine::<N>::new()?.execute(&request, rng)?.finalize()
    }

    /// Initializes an instance of `Transaction` from the given inputs.
    #[inline]
    pub fn from(
        network_id: u16,
        inner_circuit_id: N::InnerCircuitID,
        transitions: Vec<Transition<N>>,
        events: Vec<Event<N>>,
    ) -> Result<Self> {
        let transaction_id = Self::compute_transaction_id(&transitions)?;

        let transaction = Self {
            transaction_id,
            network_id,
            inner_circuit_id,
            transitions,
            events,
        };

        match transaction.is_valid() {
            true => Ok(transaction),
            false => Err(anyhow!("Failed to initialize a transaction")),
        }
    }

    /// Returns `true` if the transaction is well-formed, meaning it contains
    /// the correct network ID, unique serial numbers, unique commitments,
    /// correct ciphertext IDs, and a valid proof.
    #[inline]
    pub fn is_valid(&self) -> bool {
        // Returns `false` if the network ID is incorrect.
        if self.network_id != N::NETWORK_ID {
            eprintln!("Transaction contains an incorrect network ID");
            return false;
        }

        // Ensure the number of events is less than `N::NUM_EVENTS`.
        if self.events.len() > N::NUM_EVENTS {
            eprintln!("Transaction contains an invalid number of events");
            return false;
        }

        // Ensure the number of transitions is between 1 and 128 (inclusive).
        let num_transitions = self.transitions.len();
        if num_transitions < 1 || num_transitions > 128 {
            eprintln!("Transaction contains invalid number of transitions");
            return false;
        }

        // Returns `false` if the number of serial numbers in the transaction is incorrect.
        if self.serial_numbers().len() != num_transitions * N::NUM_INPUT_RECORDS {
            eprintln!("Transaction contains incorrect number of serial numbers");
            return false;
        }

        // Returns `false` if there are duplicate serial numbers in the transaction.
        if has_duplicates(self.serial_numbers()) {
            eprintln!("Transaction contains duplicate serial numbers");
            return false;
        }

        // Returns `false` if the number of commitments in the transaction is incorrect.
        if self.commitments().len() != num_transitions * N::NUM_OUTPUT_RECORDS {
            eprintln!("Transaction contains incorrect number of commitments");
            return false;
        }

        // Returns `false` if there are duplicate commitments numbers in the transaction.
        if has_duplicates(self.commitments()) {
            eprintln!("Transaction contains duplicate commitments");
            return false;
        }

        // Returns `false` if the number of record ciphertexts in the transaction is incorrect.
        if self.ciphertexts().len() != num_transitions * N::NUM_OUTPUT_RECORDS {
            eprintln!("Transaction contains incorrect number of record ciphertexts");
            return false;
        }

        // Returns `false` if there are duplicate ciphertexts in the transition.
        if has_duplicates(self.ciphertexts()) {
            eprintln!("Transaction contains duplicate ciphertexts");
            return false;
        }

        // Returns `false` if the transition is invalid.
        if !self.transitions[0].verify(*N::inner_circuit_id(), Default::default()) {
            eprintln!("Transaction contains an invalid transition");
            return false;
        }

        // Returns `false` if any transition is invalid.
        if num_transitions > 1 {
            // Initialize a local commitments tree.
            let mut local_commitments_tree = match LocalCommitments::<N>::new() {
                Ok(local_commitments_tree) => local_commitments_tree,
                Err(error) => {
                    eprintln!("Transaction failed to initialize a local commitments tree: {}", error);
                    return false;
                }
            };

            for window in self.transitions.windows(2) {
                if let [previous_transition, current_transition] = window {
                    // Update the local commitments tree.
                    if let Err(error) = local_commitments_tree.add(previous_transition.commitments()) {
                        eprintln!("Transaction failed to update local commitments tree: {}", error);
                        return false;
                    }

                    // Returns `false` if the transition is invalid.
                    if !current_transition.verify(*N::inner_circuit_id(), local_commitments_tree.root()) {
                        eprintln!("Transaction contains an invalid transition");
                        return false;
                    }
                }
            }

            // Returns `false` if the size of the local commitments tree does not match the number of transitions.
            if local_commitments_tree.len() != (num_transitions - 1) * N::NUM_INPUT_RECORDS {
                eprintln!("Transaction contains invalid local commitments tree state");
                return false;
            }
        }

        true
    }

    /// Returns the transaction ID.
    #[inline]
    pub fn transaction_id(&self) -> N::TransactionID {
        self.transaction_id
    }

    /// Returns the network ID.
    #[inline]
    pub fn network_id(&self) -> u16 {
        self.network_id
    }

    /// Returns the inner circuit ID.
    #[inline]
    pub fn inner_circuit_id(&self) -> N::InnerCircuitID {
        self.inner_circuit_id
    }

    /// Returns the block hashes used to execute the transitions.
    #[inline]
    pub fn block_hashes(&self) -> HashSet<N::BlockHash> {
        self.transitions.iter().map(Transition::block_hash).collect()
    }

    /// Returns the serial numbers.
    #[inline]
    pub fn serial_numbers(&self) -> Vec<N::SerialNumber> {
        self.transitions
            .iter()
            .flat_map(Transition::serial_numbers)
            .cloned()
            .collect()
    }

    /// Returns the commitments.
    #[inline]
    pub fn commitments(&self) -> Vec<N::Commitment> {
        self.transitions
            .iter()
            .flat_map(Transition::commitments)
            .cloned()
            .collect()
    }

    /// Returns the output record ciphertexts.
    #[inline]
    pub fn ciphertexts(&self) -> Vec<RecordCiphertext<N>> {
        self.transitions
            .iter()
            .flat_map(Transition::ciphertexts)
            .cloned()
            .collect()
    }

    /// Returns the value balance.
    #[inline]
    pub fn value_balance(&self) -> AleoAmount {
        self.transitions
            .iter()
            .map(Transition::value_balance)
            .fold(AleoAmount::ZERO, |a, b| a.add(*b))
    }

    /// Returns a reference to the state transitions.
    #[inline]
    pub fn transitions(&self) -> &Vec<Transition<N>> {
        &self.transitions
    }

    /// Returns the transition IDs.
    #[inline]
    pub fn transition_ids(&self) -> Vec<N::TransitionID> {
        self.transitions.iter().map(Transition::transition_id).collect()
    }

    /// Returns a reference to the events.
    #[inline]
    pub fn events(&self) -> &Vec<Event<N>> {
        &self.events
    }

    /// Returns the ciphertext IDs.
    #[inline]
    pub fn to_ciphertext_ids(&self) -> Result<Vec<N::CiphertextID>> {
        self.transitions
            .iter()
            .flat_map(Transition::to_ciphertext_ids)
            .collect::<Result<Vec<_>>>()
    }

    /// Returns records from the transaction belonging to the given account view key.
    #[inline]
    pub fn to_decrypted_records(&self, account_view_key: &ViewKey<N>) -> Vec<Record<N>> {
        self.transitions
            .iter()
            .flat_map(Transition::ciphertexts)
            .filter_map(|c| c.decrypt(account_view_key).ok())
            .filter(|record| !record.is_dummy())
            .collect()
    }

    /// Transaction ID := Hash(transition IDs)
    #[inline]
    pub(crate) fn compute_transaction_id(transitions: &Vec<Transition<N>>) -> Result<N::TransactionID> {
        Ok(N::transaction_id_crh().hash(&to_bytes_le![
            transitions.iter().map(Transition::transition_id).collect::<Vec<_>>()
        ]?)?)
    }
}

impl<N: Network> FromBytes for Transaction<N> {
    #[inline]
    fn read_le<R: Read>(mut reader: R) -> IoResult<Self> {
        let network_id: u16 = FromBytes::read_le(&mut reader)?;
        let inner_circuit_id = FromBytes::read_le(&mut reader)?;

        let num_transitions: u16 = FromBytes::read_le(&mut reader)?;
        let mut transitions = Vec::with_capacity(num_transitions as usize);
        for _ in 0..num_transitions {
            transitions.push(FromBytes::read_le(&mut reader)?);
        }

        let num_events: u16 = FromBytes::read_le(&mut reader)?;
        let mut events = Vec::with_capacity(num_events as usize);
        for _ in 0..num_events {
            events.push(FromBytes::read_le(&mut reader)?);
        }

        Ok(Self::from(network_id, inner_circuit_id, transitions, events).expect("Failed to deserialize a transaction"))
    }
}

impl<N: Network> ToBytes for Transaction<N> {
    #[inline]
    fn write_le<W: Write>(&self, mut writer: W) -> IoResult<()> {
        self.network_id.write_le(&mut writer)?;
        self.inner_circuit_id.write_le(&mut writer)?;
        (self.transitions.len() as u16).write_le(&mut writer)?;
        self.transitions.write_le(&mut writer)?;
        (self.events.len() as u16).write_le(&mut writer)?;
        self.events.write_le(&mut writer)
    }
}

impl<N: Network> Hash for Transaction<N> {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.transaction_id().hash(state);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{testnet2::Testnet2, Account, AccountScheme};
    use snarkvm_utilities::UniformRand;

    use rand::thread_rng;

    #[test]
    fn test_decrypt_records() {
        let rng = &mut thread_rng();
        let account = Account::<Testnet2>::new(rng);

        // Craft the expected coinbase record.
        let expected_record = Record::new_output(
            account.address(),
            1234,
            Default::default(),
            *Testnet2::noop_program_id(),
            UniformRand::rand(rng),
            rng,
        )
        .unwrap();

        // Craft a transaction with 1 coinbase record.
        let transaction = Transaction::new_coinbase(account.address(), AleoAmount(1234), rng).unwrap();
        let decrypted_records = transaction.to_decrypted_records(&account.view_key());
        assert_eq!(decrypted_records.len(), 1); // Excludes dummy records upon decryption.

        let candidate_record = decrypted_records.first().unwrap();
        assert_eq!(expected_record.owner(), candidate_record.owner());
        assert_eq!(expected_record.value(), candidate_record.value());
        assert_eq!(expected_record.payload(), candidate_record.payload());
        assert_eq!(expected_record.program_id(), candidate_record.program_id());
    }
}