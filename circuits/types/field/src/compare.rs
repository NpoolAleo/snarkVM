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

use super::*;

impl<E: Environment> Compare<Field<E>> for Field<E> {
    type Boolean = Boolean<E>;

    /// Returns `true` if `self` is less than `other`.
    fn is_less_than(&self, other: &Self) -> Self::Boolean {
        if self.is_constant() && other.is_constant() {
            Boolean::constant(self.eject_value() < other.eject_value())
        } else if self.is_constant() {
            // For advanced users: this implementation saves us from instantiating 253 constants for
            // the bits of `self`.
            let self_bits_le = self.eject_value().to_bits_le();

            let bit_pairs_le = self_bits_le.into_iter().zip_eq(other.to_bits_le());

            bit_pairs_le.fold(Boolean::constant(false), |rest_is_less, (self_bit, other_bit)| {
                if self_bit {
                    Boolean::ternary(&!&other_bit, &other_bit, &rest_is_less)
                } else {
                    Boolean::ternary(&other_bit, &other_bit, &rest_is_less)
                }
            })
        } else if other.is_constant() {
            // For advanced users: this implementation saves us from instantiating 253 constants for
            // the bits of `self`.
            let self_bits_le = self.to_bits_le();

            let bit_pairs_le = self_bits_le.iter().zip_eq(other.eject_value().to_bits_le());

            bit_pairs_le.fold(Boolean::constant(false), |rest_is_less, (self_bit, other_bit)| {
                if other_bit {
                    Boolean::ternary(&!self_bit, &!self_bit, &rest_is_less)
                } else {
                    Boolean::ternary(self_bit, &!self_bit, &rest_is_less)
                }
            })
        } else {
            // Initialize an iterator over `self` and `other` from MSB to LSB.
            let self_bits_le = self.to_bits_le();
            let other_bits_le = other.to_bits_le();
            let bits_le = self_bits_le.iter().zip_eq(&other_bits_le);

            bits_le.fold(Boolean::constant(false), |rest_is_less, (self_bit, other_bit)| {
                Boolean::ternary(&self_bit.bitxor(other_bit), other_bit, &rest_is_less)
            })
        }
    }

    /// Returns `true` if `self` is greater than `other`.
    fn is_greater_than(&self, other: &Self) -> Self::Boolean {
        other.is_less_than(self)
    }

    /// Returns `true` if `self` is less than or equal to `other`.
    fn is_less_than_or_equal(&self, other: &Self) -> Self::Boolean {
        other.is_greater_than_or_equal(self)
    }

    /// Returns `true` if `self` is greater than or equal to `other`.
    fn is_greater_than_or_equal(&self, other: &Self) -> Self::Boolean {
        !self.is_less_than(other)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use snarkvm_circuits_environment::Circuit;
    use snarkvm_utilities::{test_rng, UniformRand};

    const ITERATIONS: usize = 1000;

    #[rustfmt::skip]
    fn run_test(
        mode_a: Mode,
        mode_b: Mode,
        num_constants: usize,
        num_public: usize,
        num_private: usize,
        num_constraints: usize,
    ) {
        for _i in 0..ITERATIONS {
            let first: <Circuit as Environment>::BaseField = UniformRand::rand(&mut test_rng());
            let second: <Circuit as Environment>::BaseField = UniformRand::rand(&mut test_rng());

            // Check `is_less_than`.
            let a = Field::<Circuit>::new(mode_a, first);
            let b = Field::<Circuit>::new(mode_b, second);
            Circuit::scope(&format!("Less Than: {} {}", mode_a, mode_b), || {
                let candidate = a.is_less_than(&b);
                assert_eq!(first < second, candidate.eject_value());
                assert_scope!(num_constants, num_public, num_private, num_constraints);
            });
            Circuit::reset();

            // Check `is_less_than_or_equal`
            let a = Field::<Circuit>::new(mode_a, first);
            let b = Field::<Circuit>::new(mode_b, second);
            Circuit::scope(&format!("Less Than Or Equal: {} {}", mode_a, mode_b), || {
                let candidate = a.is_less_than_or_equal(&b);
                assert_eq!(first <= second, candidate.eject_value());
                assert_scope!(num_constants, num_public, num_private, num_constraints);
            });
            Circuit::reset();

            // Check `is_greater_than`
            let a = Field::<Circuit>::new(mode_a, first);
            let b = Field::<Circuit>::new(mode_b, second);
            Circuit::scope(&format!("Greater Than: {} {}", mode_a, mode_b), || {
                let candidate = a.is_greater_than(&b);
                assert_eq!(first > second, candidate.eject_value());
                assert_scope!(num_constants, num_public, num_private, num_constraints);
            });
            Circuit::reset();

            // Check `is_greater_than_or_equal`
            let a = Field::<Circuit>::new(mode_a, first);
            let b = Field::<Circuit>::new(mode_b, second);
            Circuit::scope(&format!("Greater Than Or Equal: {} {}", mode_a, mode_b), || {
                let candidate = a.is_greater_than_or_equal(&b);
                assert_eq!(first >= second, candidate.eject_value());
                assert_scope!(num_constants, num_public, num_private, num_constraints);
            });
            Circuit::reset();
        }
    }

    #[test]
    fn test_constant_compare_with_constant() {
        run_test(Mode::Constant, Mode::Constant, 0, 0, 0, 0);
    }

    #[test]
    fn test_constant_compare_with_public() {
        run_test(Mode::Constant, Mode::Public, 0, 0, 506, 507);
    }

    #[test]
    fn test_constant_compare_with_private() {
        run_test(Mode::Constant, Mode::Private, 0, 0, 506, 507);
    }

    #[test]
    fn test_public_compare_with_constant() {
        run_test(Mode::Constant, Mode::Public, 0, 0, 506, 507);
    }

    #[test]
    fn test_private_compare_with_constant() {
        run_test(Mode::Constant, Mode::Private, 0, 0, 506, 507);
    }

    #[test]
    fn test_public_compare_with_public() {
        run_test(Mode::Public, Mode::Public, 0, 0, 1012, 1014);
    }

    #[test]
    fn test_public_compare_with_private() {
        run_test(Mode::Public, Mode::Private, 0, 0, 1012, 1014);
    }

    #[test]
    fn test_private_compare_with_public() {
        run_test(Mode::Private, Mode::Public, 0, 0, 1012, 1014);
    }

    #[test]
    fn test_private_compare_with_private() {
        run_test(Mode::Private, Mode::Private, 0, 0, 1012, 1014);
    }
}
