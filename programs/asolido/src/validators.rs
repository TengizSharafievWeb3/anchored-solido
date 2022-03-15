// SPDX-FileCopyrightText: 2021 Chorus One AG
// SPDX-License-Identifier: GPL-3.0

//! A type that stores a map (dictionary) from public key to some value `T`.

use anchor_lang::prelude::*;
use solana_program::pubkey::Pubkey;

use crate::error::LidoError;
use crate::state::{Validator, VALIDATOR_CONSTANT_SIZE};

/// An entry in `AccountMap`.
#[derive(Clone, Default, Debug, Eq, PartialEq, AnchorSerialize, AnchorDeserialize)]
pub struct PubkeyAndEntry {
    pub pubkey: Pubkey,
    pub entry: Validator,
}

/// A map from public key to `T`, implemented as a vector of key-value pairs.
#[derive(Clone, Default, Debug, Eq, PartialEq, AnchorSerialize, AnchorDeserialize)]
pub struct Validators {
    pub entries: Vec<PubkeyAndEntry>,
    pub maximum_entries: u32,
}

pub trait EntryConstantSize {
    const SIZE: usize;
}

impl Validators {
    /// Creates a new instance with the `maximum_entries` positions filled with the default value
    pub fn new_fill_default(maximum_entries: u32) -> Self {
        let entries = vec![
            PubkeyAndEntry {
                pubkey: Pubkey::default(),
                entry: Validator::default(),
            };
            maximum_entries as usize
        ];
        Validators {
            entries,
            maximum_entries,
        }
    }

    /// Creates a new empty instance
    pub fn new(maximum_entries: u32) -> Self {
        Validators {
            entries: Vec::new(),
            maximum_entries,
        }
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn add(&mut self, address: Pubkey, value: Validator) -> std::result::Result<(), LidoError> {
        if self.len() == self.maximum_entries as usize {
            return Err(LidoError::MaximumNumberOfAccountsExceeded);
        }
        if !self.entries.iter().any(|pe| pe.pubkey == address) {
            self.entries.push(PubkeyAndEntry {
                pubkey: address,
                entry: value,
            });
        } else {
            return Err(LidoError::DuplicatedEntry);
        }
        Ok(())
    }

    pub fn remove(&mut self, address: &Pubkey) -> Result<Validator> {
        let idx = self
            .entries
            .iter()
            .position(|pe| &pe.pubkey == address)
            .ok_or_else(|| error!(LidoError::InvalidAccountMember))?;
        Ok(self.entries.swap_remove(idx).entry)
    }

    pub fn get(&self, address: &Pubkey) -> std::result::Result<&PubkeyAndEntry, LidoError> {
        self.entries
            .iter()
            .find(|pe| &pe.pubkey == address)
            .ok_or(LidoError::InvalidAccountMember)
    }

    pub fn get_mut(
        &mut self,
        address: &Pubkey,
    ) -> std::result::Result<&mut PubkeyAndEntry, LidoError> {
        self.entries
            .iter_mut()
            .find(|pe| &pe.pubkey == address)
            .ok_or(LidoError::InvalidAccountMember)
    }

    /// Return how many bytes are needed to serialize an instance holding `max_entries`.
    pub fn required_bytes(max_entries: usize) -> usize {
        let key_size = std::mem::size_of::<Pubkey>();
        let value_size = VALIDATOR_CONSTANT_SIZE;
        let entry_size = key_size + value_size;

        // 8 bytes for the length and u32 field, then the entries themselves.
        8 + entry_size * max_entries as usize
    }

    /// Return how many entries could fit in a buffer of the given size.
    pub fn maximum_entries(buffer_size: usize) -> usize {
        let key_size = std::mem::size_of::<Pubkey>();
        let value_size = VALIDATOR_CONSTANT_SIZE;
        let entry_size = key_size + value_size;

        buffer_size.saturating_sub(8) / entry_size
    }

    /// Iterate just the values, not the keys.
    pub fn iter_entries(&self) -> IterEntries {
        IterEntries {
            iter: self.entries.iter(),
        }
    }

    /// Iterate just the values mutably, not the keys.
    pub fn iter_entries_mut(&mut self) -> IterEntriesMut {
        IterEntriesMut {
            iter: self.entries.iter_mut(),
        }
    }
}

pub struct IterEntries<'a> {
    iter: std::slice::Iter<'a, PubkeyAndEntry>,
}

impl<'a> std::iter::Iterator for IterEntries<'a> {
    type Item = &'a Validator;

    fn next(&mut self) -> Option<&'a Validator> {
        self.iter.next().map(|pubkey_entry| &pubkey_entry.entry)
    }
}

pub struct IterEntriesMut<'a> {
    iter: std::slice::IterMut<'a, PubkeyAndEntry>,
}

impl<'a> std::iter::Iterator for IterEntriesMut<'a> {
    type Item = &'a mut Validator;

    fn next(&mut self) -> Option<&'a mut Validator> {
        self.iter.next().map(|pubkey_entry| &mut pubkey_entry.entry)
    }
}
