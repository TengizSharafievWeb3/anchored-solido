// SPDX-FileCopyrightText: 2021 Chorus One AG
// SPDX-License-Identifier: GPL-3.0

//! A type that stores a map (dictionary) from public key to some value `T`.

use anchor_lang::prelude::*;
use solana_program::pubkey::Pubkey;

use crate::error::LidoError;

/// A map from public key to `T`, implemented as a vector of key-value pairs.
#[derive(Clone, Default, Debug, Eq, PartialEq, AnchorSerialize, AnchorDeserialize)]
pub struct Maintainers {
    pub entries: Vec<Pubkey>,
    pub maximum_entries: u32,
}

impl Maintainers {
    /// Creates a new instance with the `maximum_entries` positions filled with the default value
    pub fn new_fill_default(maximum_entries: u32) -> Self {
        Maintainers {
            entries: vec![Pubkey::default(); maximum_entries as usize],
            maximum_entries,
        }
    }

    /// Creates a new empty instance
    pub fn new(maximum_entries: u32) -> Self {
        Maintainers {
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

    pub fn add(&mut self, address: Pubkey) -> std::result::Result<(), LidoError> {
        if self.len() == self.maximum_entries as usize {
            return Err(LidoError::MaximumNumberOfAccountsExceeded);
        }
        if !self.entries.iter().any(|pe| *pe == address) {
            self.entries.push(address);
        } else {
            return Err(LidoError::DuplicatedEntry);
        }
        Ok(())
    }

    pub fn remove(&mut self, address: &Pubkey) -> std::result::Result<(), LidoError> {
        let idx = self
            .entries
            .iter()
            .position(|pe| pe == address)
            .ok_or(LidoError::InvalidAccountMember)?;
        self.entries.swap_remove(idx);
        Ok(())
    }

    pub fn get(&self, address: &Pubkey) -> std::result::Result<&Pubkey, LidoError> {
        self.entries
            .iter()
            .find(|pe| *pe == address)
            .ok_or(LidoError::InvalidAccountMember)
    }

    pub fn get_mut(&mut self, address: &Pubkey) -> std::result::Result<&mut Pubkey, LidoError> {
        self.entries
            .iter_mut()
            .find(|pe| *pe == address)
            .ok_or(LidoError::InvalidAccountMember)
    }

    /// Return how many bytes are needed to serialize an instance holding `max_entries`.
    pub fn required_bytes(max_entries: usize) -> usize {
        let entry_size = std::mem::size_of::<Pubkey>();

        // 8 bytes for the length and u32 field, then the entries themselves.
        8 + entry_size * max_entries as usize
    }

    /// Return how many entries could fit in a buffer of the given size.
    pub fn maximum_entries(buffer_size: usize) -> usize {
        let entry_size = std::mem::size_of::<Pubkey>();

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
    iter: std::slice::Iter<'a, Pubkey>,
}

impl<'a> std::iter::Iterator for IterEntries<'a> {
    type Item = &'a Pubkey;

    fn next(&mut self) -> Option<&'a Pubkey> {
        self.iter.next()
    }
}

pub struct IterEntriesMut<'a> {
    iter: std::slice::IterMut<'a, Pubkey>,
}

impl<'a> std::iter::Iterator for IterEntriesMut<'a> {
    type Item = &'a mut Pubkey;

    fn next(&mut self) -> Option<&'a mut Pubkey> {
        self.iter.next()
    }
}
