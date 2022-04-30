// Copyright (C) 2019-2022 Aleo Systems Inc.
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

use crate::Program;
use snarkvm_circuits::{algorithms::Pedersen64 as Pedersen64Gadget, Hash, Literal, ToBits};

const SETUP_MESSAGE: &str = "Pedersen64";

#[derive(Clone)]
pub struct Pedersen64<P: Program>(Pedersen64Gadget<P::Environment>);

impl<P: Program> Default for Pedersen64<P> {
    fn default() -> Self {
        Self::new(SETUP_MESSAGE)
    }
}

impl<P: Program> Pedersen64<P> {
    /// Creates a new instance of `Pedersen64`.
    pub fn new(message: &str) -> Self {
        Self(Pedersen64Gadget::setup(message))
    }

    /// Hash the given `value`. Halts if given a value that exceeds the input size.
    pub fn hash(&self, value: Literal<P::Environment>) -> Literal<P::Environment> {
        match value {
            Literal::Boolean(a) => Literal::Field(self.0.hash(&a.to_bits_le())),
            Literal::I8(a) => Literal::Field(self.0.hash(&a.to_bits_le())),
            Literal::I16(a) => Literal::Field(self.0.hash(&a.to_bits_le())),
            Literal::I32(a) => Literal::Field(self.0.hash(&a.to_bits_le())),
            Literal::I64(a) => Literal::Field(self.0.hash(&a.to_bits_le())),
            Literal::U8(a) => Literal::Field(self.0.hash(&a.to_bits_le())),
            Literal::U16(a) => Literal::Field(self.0.hash(&a.to_bits_le())),
            Literal::U32(a) => Literal::Field(self.0.hash(&a.to_bits_le())),
            Literal::U64(a) => Literal::Field(self.0.hash(&a.to_bits_le())),
            _ => P::halt("Invalid input size for Pedersen64 hash"),
        }
    }
}
