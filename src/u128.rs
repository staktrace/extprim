//! Unsigned 128-bit integer.

use std::fmt;
use std::u64;
use std::str::FromStr;
use std::ops::*;
use std::cmp::{PartialOrd, Ord, Ordering};
use std::convert::From;
use std::num::ParseIntError;

use rand::{Rand, Rng};
use num_traits::*;

use i128::i128;
use compiler_rt::{udiv128, umod128, udivmod128};
use error;
use traits::{Wrapping, ToExtraPrimitive};

//{{{ Structure

/// Number of bits an unsigned 128-bit number occupies.
pub const BITS: usize = 128;

/// Number of bytes an unsigned 128-bit number occupies.
pub const BYTES: usize = 16;

/// The smallest unsigned 128-bit integer (0).
pub const MIN: u128 = u128 { lo: 0, hi: 0 };

/// The largest unsigned 128-bit integer
/// (`340_282_366_920_938_463_463_374_607_431_768_211_455`).
pub const MAX: u128 = u128 { lo: u64::MAX, hi: u64::MAX };

/// The constant 0.
pub const ZERO: u128 = MIN;

/// The constant 1.
pub const ONE: u128 = u128 { lo: 1, hi: 0 };

/// An unsigned 128-bit number.
#[derive(Default, Copy, Clone, Hash, PartialEq, Eq)]
#[repr(C)]
#[allow(non_camel_case_types)]
pub struct u128 {
    // TODO these two fields are public because `const fn` are not yet stable.

    /// The lower 64-bit of the number.
    #[doc(hidden)]
    pub lo: u64,

    /// The higher 64-bit of the number.
    #[doc(hidden)]
    pub hi: u64,
}

impl u128 {
    /// Constructs a new 128-bit integer from a 64-bit integer.

    pub fn new(lo: u64) -> u128 {
        u128 { lo: lo, hi: 0 }
    }

    /// Constructs a new 128-bit integer from the high-64-bit and low-64-bit
    /// parts.
    ///
    /// ```
    /// use extprim::u128::u128;
    /// let number = u128::from_parts(6692605942, 14083847773837265618);
    /// assert_eq!(format!("{}", number), "123456789012345678901234567890");
    /// ```

    pub fn from_parts(hi: u64, lo: u64) -> u128 {
        u128 { lo: lo, hi: hi }
    }

    /// Fetch the lower-64-bit of the number.
    pub fn low64(self) -> u64 {
        self.lo
    }

    /// Fetch the higher-64-bit of the number.
    pub fn high64(self) -> u64 {
        self.hi
    }

    /// Convert this number to signed without checking.
    pub fn as_i128(self) -> i128 {
        i128::from_parts(self.hi as i64, self.lo)
    }
}

//}}}

//{{{ Rand

impl Rand for u128 {
    fn rand<R: Rng>(rng: &mut R) -> u128 {
        let (lo, hi) = rng.gen();
        u128::from_parts(lo, hi)
    }
}

//}}}

//{{{ Add, Sub

impl u128 {
    /// Wrapping (modular) addition. Computes `self + other`, wrapping around at
    /// the boundary of the type.
    pub fn wrapping_add(self, other: u128) -> u128 {
        let (lo, carry) = self.lo.overflowing_add(other.lo);
        let hi = self.hi.wrapping_add(other.hi);
        let hi = hi.wrapping_add(if carry { 1 } else { 0 });
        u128::from_parts(hi, lo)
    }

    /// Wrapping (modular) subtraction. Computes `self - other`, wrapping around
    /// at the boundary of the type.
    pub fn wrapping_sub(self, other: u128) -> u128 {
        let (lo, borrow) = self.lo.overflowing_sub(other.lo);
        let hi = self.hi.wrapping_sub(other.hi);
        let hi = hi.wrapping_sub(if borrow { 1 } else { 0 });
        u128::from_parts(hi, lo)
    }

    /// Calculates `self + other`.
    ///
    /// Returns a tuple of the addition along with a boolean indicating whether
    /// an arithmetic overflow would occur. If an overflow would have occurred
    /// then the wrapped value is returned.
    pub fn overflowing_add(self, other: u128) -> (u128, bool) {
        let (lo, lo_carry) = self.lo.overflowing_add(other.lo);
        let (hi, hi_carry_1) = self.hi.overflowing_add(if lo_carry { 1 } else { 0 });
        let (hi, hi_carry_2) = hi.overflowing_add(other.hi);
        (u128::from_parts(hi, lo), hi_carry_1 || hi_carry_2)
    }

    /// Calculates `self - other`.
    ///
    /// Returns a tuple of the subtraction along with a boolean indicating
    /// whether an arithmetic overflow would occur. If an overflow would have
    /// occurred then the wrapped value is returned.
    pub fn overflowing_sub(self, other: u128) -> (u128, bool) {
        let (lo, lo_borrow) = self.lo.overflowing_sub(other.lo);
        let (hi, hi_borrow_1) = self.hi.overflowing_sub(if lo_borrow { 1 } else { 0 });
        let (hi, hi_borrow_2) = hi.overflowing_sub(other.hi);
        (u128::from_parts(hi, lo), hi_borrow_1 || hi_borrow_2)
    }

    /// Saturating integer addition.
    ///
    /// Computes `self + other`, saturating at the numeric bounds instead of
    /// overflowing.
    pub fn saturating_add(self, other: u128) -> u128 {
        self.checked_add(other).unwrap_or(MAX)
    }

    /// Saturating integer subtraction.
    ///
    /// Computes `self - other`, saturating at the numeric bounds instead of
    /// overflowing.
    pub fn saturating_sub(self, other: u128) -> u128 {
        if self <= other {
            ZERO
        } else {
            self.wrapping_sub(other)
        }
    }

    /// Computes `!self + 1`, i.e. the negation of itself when viewed as a
    /// signed integer.
    pub fn wrapping_neg(self) -> u128 {
        ONE.wrapping_add(!self)
    }
}

forward_symmetric!(Add(add, checked_add, wrapping_add, overflowing_add) for u128);
forward_symmetric!(Sub(sub, checked_sub, wrapping_sub, overflowing_sub) for u128);
forward_assign!(AddAssign(add_assign, add) for u128);
forward_assign!(SubAssign(sub_assign, sub) for u128);

impl Neg for Wrapping<u128> {
    type Output = Self;
    fn neg(self) -> Self {
        Wrapping(self.0.wrapping_neg())
    }
}

impl CheckedAdd for u128 {
    fn checked_add(&self, other: &Self) -> Option<Self> {
        Self::checked_add(*self, *other)
    }
}

impl CheckedSub for u128 {
    fn checked_sub(&self, other: &Self) -> Option<Self> {
        Self::checked_sub(*self, *other)
    }
}

impl Saturating for u128 {
    fn saturating_add(self, other: Self) -> Self {
        Self::saturating_add(self, other)
    }

    fn saturating_sub(self, other: Self) -> Self {
        Self::saturating_add(self, other)
    }
}

#[cfg(test)]
mod add_sub_tests {
    use u128::{u128, ZERO, ONE, MAX};

    #[test]
    fn test_add() {
        assert_eq!(u128::from_parts(23, 12) + u128::from_parts(78, 45),
                    u128::from_parts(101, 57));
        assert_eq!(u128::from_parts(0x4968eca20d32da8d, 0xaf40c0e1a806fa23) +
                    u128::from_parts(0x71b6119ef76e4fe3, 0x0f58496c74669747),
                    u128::from_parts(0xbb1efe4104a12a70, 0xbe990a4e1c6d916a));
        assert_eq!(u128::from_parts(1, 0xffffffff_ffffffff) + u128::from_parts(0, 1),
                    u128::from_parts(2, 0));
    }

    #[test]
    fn test_wrapping_overflowing_add() {
        let a = u128::from_parts(0xfeae4b298df991ae, 0x21b6c7c3766908a7);
        let b = u128::from_parts(0x08a45eef16781327, 0xff1049ddf49ff8a8);
        let c = u128::from_parts(0x0752aa18a471a4d6, 0x20c711a16b09014f);
        assert_eq!(a.wrapping_add(b), c);
        assert_eq!(a.overflowing_add(b), (c, true));
        assert_eq!(a.checked_add(b), None);
        assert_eq!(a.saturating_add(b), MAX);

        assert_eq!(ONE.wrapping_add(ONE), u128::new(2));
        assert_eq!(ONE.overflowing_add(ONE), (u128::new(2), false));
        assert_eq!(ONE.checked_add(ONE), Some(u128::new(2)));
        assert_eq!(ONE.saturating_add(ONE), u128::new(2));

        assert_eq!(MAX.wrapping_add(ONE), ZERO);
        assert_eq!(MAX.overflowing_add(ONE), (ZERO, true));
        assert_eq!(MAX.checked_add(ONE), None);
        assert_eq!(MAX.saturating_add(ONE), MAX);
    }

    #[test]
    #[should_panic(expected="arithmetic operation overflowed")]
    fn test_add_overflow_without_carry() {
        u128::from_parts(0x80000000_00000000, 0) + u128::from_parts(0x80000000_00000000, 0);
    }

    #[test]
    #[should_panic(expected="arithmetic operation overflowed")]
    fn test_add_overflow_with_carry() {
        MAX + ONE;
    }

    #[test]
    fn test_sub() {
        assert_eq!(u128::from_parts(78, 45) - u128::from_parts(23, 12),
                    u128::from_parts(55, 33));
        assert_eq!(u128::from_parts(0xfeae4b298df991ae, 0x21b6c7c3766908a7) -
                    u128::from_parts(0x08a45eef16781327, 0xff1049ddf49ff8a8),
                    u128::from_parts(0xf609ec3a77817e86, 0x22a67de581c90fff));
    }

    #[test]
    fn test_wrapping_overflowing_sub() {
        let a = u128::from_parts(3565142335064920496, 15687467940602204387);
        let b = u128::from_parts(4442421226426414073, 17275749316209243331);
        let c = u128::from_parts(17569465182348058038, 16858462698102512672);
        let neg_c = c.wrapping_neg();

        assert_eq!(a.wrapping_sub(b), c);
        assert_eq!(a.overflowing_sub(b), (c, true));
        assert_eq!(a.checked_sub(b), None);
        assert_eq!(a.saturating_sub(b), ZERO);

        assert_eq!(b.wrapping_sub(a), neg_c);
        assert_eq!(b.overflowing_sub(a), (neg_c, false));
        assert_eq!(b.checked_sub(a), Some(neg_c));
        assert_eq!(b.saturating_sub(a), neg_c);

        assert_eq!(ONE.wrapping_sub(ONE), ZERO);
        assert_eq!(ONE.overflowing_sub(ONE), (ZERO, false));
        assert_eq!(ONE.checked_sub(ONE), Some(ZERO));
        assert_eq!(ONE.saturating_sub(ONE), ZERO);

        assert_eq!(ZERO.wrapping_sub(ONE), MAX);
        assert_eq!(ZERO.overflowing_sub(ONE), (MAX, true));
        assert_eq!(ZERO.checked_sub(ONE), None);
        assert_eq!(ZERO.saturating_sub(ONE), ZERO);
    }

    #[test]
    #[should_panic(expected="arithmetic operation overflowed")]
    fn test_sub_overflow() {
        ZERO - ONE;
    }
}

//}}}

//{{{ PartialOrd, Ord

impl PartialOrd for u128 {
    fn partial_cmp(&self, other: &u128) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for u128 {
    fn cmp(&self, other: &u128) -> Ordering {
        (self.hi, self.lo).cmp(&(other.hi, other.lo))
    }
}

#[cfg(test)]
mod cmp_tests {
    use u128::{u128, MAX, ZERO, ONE};

    #[test]
    fn test_ord() {
        let a = ZERO;
        let b = ONE;
        let c = u128::from_parts(1, 0);
        let d = MAX;

        assert!(a < b);
        assert!(a <= b);
        assert!(c > b);
        assert!(c == c);
        assert!(d != c);
        assert!(d >= c);
    }
}

//}}}

//{{{ Not, BitAnd, BitOr, BitXor

impl Not for u128 {
    type Output = Self;
    fn not(self) -> Self {
        u128 { lo: !self.lo, hi: !self.hi }
    }
}

impl BitAnd for u128 {
    type Output = Self;
    fn bitand(self, other: Self) -> Self {
        u128 { lo: self.lo & other.lo, hi: self.hi & other.hi }
    }
}

impl BitOr for u128 {
    type Output = Self;
    fn bitor(self, other: Self) -> Self {
        u128 { lo: self.lo | other.lo, hi: self.hi | other.hi }
    }
}

impl BitXor for u128 {
    type Output = Self;
    fn bitxor(self, other: Self) -> Self {
        u128 { lo: self.lo ^ other.lo, hi: self.hi ^ other.hi }
    }
}

impl Not for Wrapping<u128> {
    type Output = Self;
    fn not(self) -> Self {
        Wrapping(!self.0)
    }
}

impl BitAnd for Wrapping<u128> {
    type Output = Self;
    fn bitand(self, other: Self) -> Self {
        Wrapping(self.0 & other.0)
    }
}

impl BitOr for Wrapping<u128> {
    type Output = Self;
    fn bitor(self, other: Self) -> Self {
        Wrapping(self.0 | other.0)
    }
}

impl BitXor for Wrapping<u128> {
    type Output = Self;
    fn bitxor(self, other: Self) -> Self {
        Wrapping(self.0 ^ other.0)
    }
}

forward_assign!(BitAndAssign(bitand_assign, bitand) for u128);
forward_assign!(BitOrAssign(bitor_assign, bitor) for u128);
forward_assign!(BitXorAssign(bitxor_assign, bitxor) for u128);

#[cfg(test)]
mod bitwise_tests {
    use u128::u128;

    #[test]
    fn test_not() {
        assert_eq!(u128::from_parts(0x491d3b2d80d706a6, 0x1eb41c5d2ad1a379),
                    !u128::from_parts(0xb6e2c4d27f28f959, 0xe14be3a2d52e5c86));
    }

    #[test]
    fn test_bit_and() {
        assert_eq!(u128::from_parts(0x8aff8559dc82aa91, 0x8bbf525fb0c5cd79) &
                    u128::from_parts(0x8dcecc950badb6f1, 0xb26ab6ca714bce40),
                    u128::from_parts(0x88ce84110880a291, 0x822a124a3041cc40));
    }

    #[test]
    fn test_bit_or() {
        assert_eq!(u128::from_parts(0xe3b7e0ae1e8f8beb, 0x5c76dd080dd43e30) |
                    u128::from_parts(0x35591b16599e2ece, 0x2e2957ca426d7b07),
                    u128::from_parts(0xf7fffbbe5f9fafef, 0x7e7fdfca4ffd7f37));
    }

    #[test]
    fn test_bit_xor() {
        assert_eq!(u128::from_parts(0x50b17617e8f6ee49, 0x1b06f037a9187c71) ^
                    u128::from_parts(0x206f313ea29823bd, 0x66e0bc7aa198785a),
                    u128::from_parts(0x70de47294a6ecdf4, 0x7de64c4d0880042b));
    }
}

//}}}

//{{{ Shl, Shr

impl u128 {
    /// Panic-free bitwise shift-left; yields `self << (shift % 128)`.
    pub fn wrapping_shl(self, shift: u32) ->  u128 {
        let lo = self.lo;
        let hi = self.hi;

        let (lo, hi) = if (shift & 64) != 0 {
            (0, lo.wrapping_shl(shift & 63))
        } else {
            let new_lo = lo.wrapping_shl(shift);
            let mut new_hi = hi.wrapping_shl(shift);
            if (shift & 127) != 0 {
                new_hi |= lo.wrapping_shr(64u32.wrapping_sub(shift));
            }
            (new_lo, new_hi)
        };

        u128::from_parts(hi, lo)
    }

    /// Panic-free bitwsie shift-right; yields `self >> (shift % 128)`.
    pub fn wrapping_shr(self, shift: u32) -> u128 {
        let lo = self.lo;
        let hi = self.hi;

        let (hi, lo) = if (shift & 64) != 0 {
            (0, hi.wrapping_shr(shift & 63))
        } else {
            let new_hi = hi.wrapping_shr(shift);
            let mut new_lo = lo.wrapping_shr(shift);
            if (shift & 127) != 0 {
                new_lo |= hi.wrapping_shl(64u32.wrapping_sub(shift));
            }
            (new_hi, new_lo)
        };

        u128::from_parts(hi, lo)
    }

    pub fn overflowing_shl(self, other: u32) -> (u128, bool) {
        (self.wrapping_shl(other), other >= 128)
    }

    pub fn overflowing_shr(self, other: u32) -> (u128, bool) {
        (self.wrapping_shr(other), other >= 128)
    }
}

forward_shift!(Shl(shl, checked_shl, wrapping_shl, overflowing_shl) for u128);
forward_shift!(Shr(shr, checked_shr, wrapping_shr, overflowing_shr) for u128);
forward_assign!(ShlAssign<u8|u16|u32|u64|usize|i8|i16|i32|i64|isize>(shl_assign, shl) for u128);
forward_assign!(ShrAssign<u8|u16|u32|u64|usize|i8|i16|i32|i64|isize>(shr_assign, shr) for u128);

#[cfg(test)]
mod shift_tests {
    use u128::u128;

    #[test]
    fn test_shl() {
        assert_eq!(u128::from_parts(0x1e5c7801b0e575f7, 0x53f09dac5b28f152) << 0,
                    u128::from_parts(0x1e5c7801b0e575f7, 0x53f09dac5b28f152));
        assert_eq!(u128::from_parts(0x1e5c7801b0e575f7, 0x53f09dac5b28f152) << 1,
                    u128::from_parts(0x3cb8f00361caebee, 0xa7e13b58b651e2a4));
        assert_eq!(u128::from_parts(0x1e5c7801b0e575f7, 0x53f09dac5b28f152) << 64,
                    u128::from_parts(0x53f09dac5b28f152, 0x0));
        assert_eq!(u128::from_parts(0x1e5c7801b0e575f7, 0x53f09dac5b28f152) << 120,
                    u128::from_parts(0x5200000000000000, 0x0));

        assert_eq!(u128::from_parts(0xf8025363ddcd51d8, 0x509d78e4a3008bcd) << 0,
                    u128::from_parts(0xf8025363ddcd51d8, 0x509d78e4a3008bcd));
        assert_eq!(u128::from_parts(0xf8025363ddcd51d8, 0x509d78e4a3008bcd) << 1,
                    u128::from_parts(0xf004a6c7bb9aa3b0, 0xa13af1c94601179a));
        assert_eq!(u128::from_parts(0xf8025363ddcd51d8, 0x509d78e4a3008bcd) << 64,
                    u128::from_parts(0x509d78e4a3008bcd, 0x0));
        assert_eq!(u128::from_parts(0xf8025363ddcd51d8, 0x509d78e4a3008bcd) << 120,
                    u128::from_parts(0xcd00000000000000, 0x0));
    }

    #[test]
    fn test_shr() {
        assert_eq!(u128::from_parts(0x1e5c7801b0e575f7, 0x53f09dac5b28f152) >> 0,
                    u128::from_parts(0x1e5c7801b0e575f7, 0x53f09dac5b28f152));
        assert_eq!(u128::from_parts(0x1e5c7801b0e575f7, 0x53f09dac5b28f152) >> 1,
                    u128::from_parts(0xf2e3c00d872bafb, 0xa9f84ed62d9478a9));
        assert_eq!(u128::from_parts(0x1e5c7801b0e575f7, 0x53f09dac5b28f152) >> 64,
                    u128::from_parts(0x0, 0x1e5c7801b0e575f7));
        assert_eq!(u128::from_parts(0x1e5c7801b0e575f7, 0x53f09dac5b28f152) >> 120,
                    u128::from_parts(0x0, 0x1e));

        assert_eq!(u128::from_parts(0xf8025363ddcd51d8, 0x509d78e4a3008bcd) >> 0,
                    u128::from_parts(0xf8025363ddcd51d8, 0x509d78e4a3008bcd));
        assert_eq!(u128::from_parts(0xf8025363ddcd51d8, 0x509d78e4a3008bcd) >> 1,
                    u128::from_parts(0x7c0129b1eee6a8ec, 0x284ebc72518045e6));
        assert_eq!(u128::from_parts(0xf8025363ddcd51d8, 0x509d78e4a3008bcd) >> 64,
                    u128::from_parts(0x0, 0xf8025363ddcd51d8));
        assert_eq!(u128::from_parts(0xf8025363ddcd51d8, 0x509d78e4a3008bcd) >> 120,
                    u128::from_parts(0x0, 0xf8));
    }

    #[test]
    #[should_panic(expected="shift operation overflowed")]
    fn test_shl_overflow() {
        u128::from_parts(0x1e5c7801b0e575f7, 0x53f09dac5b28f152) << 128;
    }

    #[test]
    #[should_panic(expected="shift operation overflowed")]
    fn test_shr_overflow() {
        u128::from_parts(0x1e5c7801b0e575f7, 0x53f09dac5b28f152) >> 256;
    }
}

#[cfg(all(test, extprim_channel="unstable"))]
mod shift_bench {
    use u128::u128;
    use test::{Bencher, black_box};

    // randomize shift range to avoid possible branch prediction effect.
    const BENCH_SHIFTS: &'static [u32] = &[
        77, 45, 57, 7, 34, 75, 38, 89, 89, 66, 16, 111, 66, 123, 14, 80, 94, 43,
        46, 86, 121, 31, 123, 33, 23, 57, 50, 28, 26, 46, 8, 88, 74, 55, 108,
        127, 1, 70, 73, 2, 1, 45, 36, 96, 124, 124, 91, 63, 25, 94, 8, 68, 41,
        127, 107, 10, 111, 98, 97, 72, 78, 10, 125, 17, 62, 3, 65, 67, 13, 41,
        68, 109, 23, 100, 98, 16, 78, 13, 0, 63, 107, 64, 13, 23, 69, 73, 2, 38,
        16, 9, 124, 120, 39, 119, 3, 15, 25, 11, 84, 102, 69, 58, 39, 116, 66,
        87, 111, 17, 11, 29, 35, 123, 23, 38, 43, 85, 32, 7, 34, 84, 27, 35,
        122, 64, 33, 83, 78, 105, 31, 5, 58, 25, 21, 34, 15, 94, 10, 23, 48, 89,
        23, 99, 110, 105, 32, 7, 116, 31, 10, 14, 22, 84, 40, 57, 7, 35, 8, 95,
        121, 66, 95, 103, 26, 62, 24, 36, 48, 58, 122, 66, 37, 56, 35, 87, 36,
        41, 75, 37, 25, 40, 60, 39, 94, 18, 33, 113, 34, 66, 34, 34, 88, 95, 81,
        115, 10, 67, 33, 34, 23, 53, 10, 119, 54, 107, 37, 17, 85, 42, 83, 85,
        102, 104, 94, 24, 97, 104, 93, 9, 95, 75, 41, 112, 64, 63, 72, 3, 26,
        65, 103, 88, 121, 105, 98, 82, 89, 30, 37, 64, 68, 41, 93, 57, 105, 100,
        108, 102, 44, 17, 61, 72, 33, 126, 73, 105, 0, 119, 97, 28, 9, 101, 44,
    ];

    #[bench]
    fn bench_shl(bencher: &mut Bencher) {
        let number = u128::from_parts(9741918172058430398, 3937562729638942691);
        bencher.iter(|| {
            for i in BENCH_SHIFTS {
                black_box(number.wrapping_shl(*i));
            }
        });
    }

    #[bench]
    fn bench_shr(bencher: &mut Bencher) {
        let number = u128::from_parts(9741918172058430398, 3937562729638942691);
        bencher.iter(|| {
            for i in BENCH_SHIFTS {
                black_box(number.wrapping_shr(*i));
            }
        });
    }

}

//}}}

//{{{ Mul

/// Computes the product of two unsigned 64-bit integers. Returns a 128-bit
/// integer.
#[cfg(not(all(target_arch="x86_64", extprim_channel="unstable")))]
fn u64_long_mul(left: u64, right: u64) -> u128 {
    let a = left >> 32;
    let b = left & 0xffffffff;
    let c = right >> 32;
    let d = right & 0xffffffff;

    let lo = b.wrapping_mul(d);
    let (mid, carry) = a.wrapping_mul(d).overflowing_add(b.wrapping_mul(c));
    let hi = a.wrapping_mul(c).wrapping_add(if carry { 1 << 32 } else { 0 });

    u128::from_parts(hi, lo).wrapping_add(u128::from_parts(mid >> 32, mid << 32))
}

#[cfg(all(target_arch="x86_64", extprim_channel="unstable"))]
fn u64_long_mul(left: u64, right: u64) -> u128 {
    unsafe {
        let mut result: u128 = ::std::mem::uninitialized();
        asm!("
            movq $2, %rax
            mulq $3
            movq %rax, $0
            movq %rdx, $1
        "
        : "=r"(result.lo), "=r"(result.hi)
        : "r"(left), "r"(right)
        : "rax", "rdx");
        result
    }
}

impl u128 {
    /// Wrapping (modular) multiplication. Computes `self * other`, wrapping
    /// around at the boundary of the type.
    pub fn wrapping_mul(self, other: u128) -> u128 {
        let a = self.hi;
        let b = self.lo;
        let c = other.hi;
        let d = other.lo;
        let mut low = u64_long_mul(b, d);
        let ad = a.wrapping_mul(d);
        let bc = b.wrapping_mul(c);
        low.hi = low.hi.wrapping_add(ad).wrapping_add(bc);
        low
    }

    /// Calculates the multiplication of `self` and `other`.
    ///
    /// Returns a tuple of the multiplication along with a boolean indicating
    /// whether an arithmetic overflow would occur. If an overflow would have
    /// occurred then the wrapped value is returned.
    pub fn overflowing_mul(self, other: u128) -> (u128, bool) {
        let a = self.hi;
        let b = self.lo;
        let c = other.hi;
        let d = other.lo;

        let (hi, hi_overflow_mul) = match (a, c) {
            (a, 0) => a.overflowing_mul(d),
            (0, c) => c.overflowing_mul(b),
            (a, c) => (a.wrapping_mul(d).wrapping_add(c.wrapping_mul(b)), true),
        };

        let mut low = u64_long_mul(b, d);
        let (hi, hi_overflow_add) = low.hi.overflowing_add(hi);
        low.hi = hi;

        (low, hi_overflow_mul || hi_overflow_add)
    }

    /// Saturating integer multiplication. Computes `self * other`, saturating
    /// at the numeric bounds instead of overflowing.
    pub fn saturating_mul(self, other: u128) -> u128 {
        self.checked_mul(other).unwrap_or(MAX)
    }

    /// Wrapping (modular) multiplication. Computes `self * other`, wrapping
    /// around at the boundary of the type.
    pub fn wrapping_mul_64(self, other: u64) -> u128 {
        let mut low = u64_long_mul(self.lo, other);
        low.hi = low.hi.wrapping_add(self.hi.wrapping_mul(other));
        low
    }

    pub fn overflowing_mul_64(self, other: u64) -> (u128, bool) {
        let mut low = u64_long_mul(self.lo, other);
        let (hi, hi_overflow_mul) = self.hi.overflowing_mul(other);
        let (hi, hi_overflow_add) = low.hi.overflowing_add(hi);
        low.hi = hi;
        (low, hi_overflow_mul || hi_overflow_add)
    }

    pub fn saturating_mul_64(self, other: u64) -> u128 {
        self.checked_mul_64(other).unwrap_or(MAX)
    }
}

forward_symmetric!(Mul(mul, checked_mul, wrapping_mul, overflowing_mul) for u128);
forward_symmetric!(Mul<u64>(mul, checked_mul_64, wrapping_mul_64, overflowing_mul_64) for u128);
forward_assign!(MulAssign(mul_assign, mul) for u128);
forward_assign!(MulAssign<u64>(mul_assign, mul) for u128);

impl Mul<u128> for u64 {
    type Output = u128;

    fn mul(self, other: u128) -> u128 {
        other * self
    }
}

impl Mul<Wrapping<u128>> for Wrapping<u64> {
    type Output = Wrapping<u128>;

    fn mul(self, other: Wrapping<u128>) -> Wrapping<u128> {
        other * self
    }
}

impl CheckedMul for u128 {
    fn checked_mul(&self, other: &Self) -> Option<Self> {
        Self::checked_mul(*self, *other)
    }
}

#[cfg(test)]
mod mul_tests {
    use std::u64;
    use u128::{u128, u64_long_mul, ZERO, ONE, MAX};

    #[test]
    fn test_u64_long_mul() {
        assert_eq!(u128::from_parts(0xaaa4d56f5b2f577, 0x916fb81166049cc3),
                    u64_long_mul(6263979403966582069, 2263184174907185431));
        assert_eq!(u128::new(10), u64_long_mul(2, 5));
        assert_eq!(u128::from_parts(0xfffffffffffffffe, 1),
                    u64_long_mul(u64::MAX, u64::MAX));
    }

    #[test]
    fn test_mul() {
        assert_eq!(u128::new(6263979403966582069) * u128::new(2263184174907185431),
                    u128::from_parts(0xaaa4d56f5b2f577, 0x916fb81166049cc3));
        assert_eq!(u128::from_parts(47984616521, 3126587552720577884) * u128::new(323057793),
                    u128::from_parts(15501804311280354074, 13195922651658531676));
        assert_eq!(ONE * ONE, ONE);
        assert_eq!(ZERO * ONE, ZERO);
        assert_eq!(ZERO * ZERO, ZERO);
        assert_eq!(MAX * ONE, MAX);
        assert_eq!(MAX * ZERO, ZERO);
    }

    #[test]
    #[should_panic(expected="arithmetic operation overflowed")]
    fn test_mul_overflow_10_10() {
        u128::from_parts(1, 0) * u128::from_parts(1, 0);
    }

    #[test]
    #[should_panic(expected="arithmetic operation overflowed")]
    fn test_mul_overflow_80_80() {
        u128::from_parts(0x80000000_00000000, 0) * u128::from_parts(0x80000000_00000000, 0);
    }

    #[test]
    #[should_panic(expected="arithmetic operation overflowed")]
    fn test_mul_overflow_max_max() {
        MAX * MAX;
    }

    #[test]
    #[should_panic(expected="arithmetic operation overflowed")]
    fn test_mul_overflow_max_2() {
        MAX * u128::new(2);
    }

    #[test]
    fn test_mul_64() {
        assert_eq!(u128::new(6263979403966582069) * 2263184174907185431u64,
                    u128::from_parts(0xaaa4d56f5b2f577, 0x916fb81166049cc3));
        assert_eq!(u128::from_parts(47984616521, 3126587552720577884) * 323057793u64,
                    u128::from_parts(15501804311280354074, 13195922651658531676));
    }

    #[test]
    #[should_panic(expected="arithmetic operation overflowed")]
    fn test_mul_64_overflow_max_2() {
        MAX * 2u64;
    }


    #[test]
    fn test_wrapping_overflowing_mul() {
        let a = u128::from_parts(0xc1b27561c3e63bad, 0x53b0ad364ee597dc);
        let b = u128::from_parts(0x50f5d9a72dd704f3, 0x5ecee7a38577df37);
        let c = u128::from_parts(0xf5651b2427082a5e, 0x0052af17d5e04444);
        assert_eq!(a.wrapping_mul(b), c);
        assert_eq!(a.overflowing_mul(b), (c, true));
        assert_eq!(a.checked_mul(b), None);
        assert_eq!(a.saturating_mul(b), MAX);

        let a = u128::from_parts(15266745824950921091, 7823906177946456204);
        let b = u128::new(8527117857836076447);
        let c = u128::from_parts(15018621813448572278, 1743241062838166260);
        assert_eq!(a.wrapping_mul(b), c);
        assert_eq!(a.overflowing_mul(b), (c, true));
        assert_eq!(a.checked_mul(b), None);
        assert_eq!(a.saturating_mul(b), MAX);

        assert_eq!(b.wrapping_mul(a), c);
        assert_eq!(b.overflowing_mul(a), (c, true));
        assert_eq!(b.checked_mul(a), None);
        assert_eq!(b.saturating_mul(a), MAX);

        assert_eq!(ONE.wrapping_mul(ONE), ONE);
        assert_eq!(ONE.overflowing_mul(ONE), (ONE, false));
        assert_eq!(ONE.checked_mul(ONE), Some(ONE));
        assert_eq!(ONE.saturating_mul(ONE), ONE);

        assert_eq!(MAX.wrapping_mul(ONE), MAX);
        assert_eq!(MAX.overflowing_mul(ONE), (MAX, false));
        assert_eq!(MAX.checked_mul(ONE), Some(MAX));
        assert_eq!(MAX.saturating_mul(ONE), MAX);
    }
}

#[cfg(all(test, extprim_channel="unstable"))]
mod mul_bench {
    use u128::{u128, u64_long_mul};
    use test::{Bencher, black_box};

    const BENCH_LONG_MUL: &'static [u64] = &[
        11738324199100816218, 3625949024665125869, 11861868675607089770, 0,
        6847039601565126724, 5990205122755364850, 9702538440775714705, 1,
        10515012783906113246, 124429589608972701, 16050761648142104263, 2,
        5351676941151834955, 6016449362915734881, 2914632825655711546, 65536,
        7683069626972557929, 2782994233456154833, 4294967296, 281474976710656,
    ];

    const BENCH_MUL: &'static [u128] = &[
        u128 { lo: 13698662153774026983, hi: 11359772643830585857 },
        u128 { lo: 2369395906159065085, hi: 9392107235602601877 },
        u128 { lo: 1316137604845241724, hi: 3387495557620150388 },
        u128 { lo: 4377298216549927656, hi: 4459898349916441418 },
        u128 { lo: 0, hi: 1 },
        u128 { lo: 4002933201893849592, hi: 16874811549516268507 },
        u128 { lo: 13499130554936672837, hi: 7450290244389993204 },
        u128 { lo: 14853481505607028172, hi: 9904715859096779071 },
        u128 { lo: 5904460318883801886, hi: 1039448585925084376 },
        u128 { lo: 2, hi: 0 },
        u128 { lo: 16506484467360819289, hi: 14931546970023365577 },
        u128 { lo: 14721531095705410753, hi: 14699503783417097848 },
        u128 { lo: 10292416800274947511, hi: 14856377574170601940 },
        u128 { lo: 17255829222190888162, hi: 11606899158687852303 },
        u128 { lo: 11087763062048402971, hi: 14746910067372570493 },
        u128 { lo: 4294967296, hi: 4294967296 },
        u128 { lo: 11389837759419328446, hi: 14469025657316200340 },
        u128 { lo: 18363409626181059962, hi: 2420940920506351250 },
        u128 { lo: 18224881384786225007, hi: 15381587162621094041 },
        u128 { lo: 1727909608960628680, hi: 8964631137233999389 }
    ];

    #[bench]
    fn bench_u64_long_mul(bencher: &mut Bencher) {
        bencher.iter(|| {
            for a in BENCH_LONG_MUL {
                for b in BENCH_LONG_MUL.iter() {
                    black_box(u64_long_mul(*a, *b));
                }
            }
        });
    }

    #[bench]
    fn bench_mul(bencher: &mut Bencher) {
        bencher.iter(|| {
            for a in BENCH_MUL {
                for b in BENCH_MUL {
                    black_box(a.wrapping_mul(*b));
                }
            }
        });
    }

    #[bench]
    fn bench_mul_64(bencher: &mut Bencher) {
        bencher.iter(|| {
            for a in BENCH_MUL {
                for b in BENCH_MUL {
                    black_box(a.wrapping_mul_64(b.lo));
                }
            }
        });
    }
}

//}}}

//{{{ Div, Rem

impl u128 {
    /// Wrapping (modular) division. Computes `self / other`. Wrapped division
    /// on unsigned types is just normal division. There's no way wrapping could
    /// ever happen. This function exists, so that all operations are accounted
    /// for in the wrapping operations.
    pub fn wrapping_div(self, other: u128) -> u128 {
        self.checked_div(other)
            .unwrap_or_else(|| panic!("attempted to divide by zero"))
    }

    /// Wrapping (modular) remainder. Computes `self % other`. Wrapped remainder
    /// calculation on unsigned types is just the regular remainder calculation.
    /// There's no way wrapping could ever happen. This function exists, so that
    /// all operations are accounted for in the wrapping operations.
    pub fn wrapping_rem(self, other: u128) -> u128 {
        self.checked_rem(other)
            .unwrap_or_else(|| panic!("attempted remainder with a divisor of zero"))
    }

    /// Calculates the divisor when `self` is divided by `other`.
    ///
    /// Returns a tuple of the divisor along with a boolean indicating whether
    /// an arithmetic overflow would occur. Note that for unsigned integers
    /// overflow never occurs, so the second value is always `false`.
    pub fn overflowing_div(self, other: u128) -> (u128, bool) {
        (self.wrapping_div(other), false)
    }

    /// Calculates the remainder when `self` is divided by `other`.
    ///
    /// Returns a tuple of the remainder along with a boolean indicating whether
    /// an arithmetic overflow would occur. Note that for unsigned integers
    /// overflow never occurs, so the second value is always `false`.
    pub fn overflowing_rem(self, other: u128) -> (u128, bool) {
        (self.wrapping_rem(other), false)
    }

    /// Checked integer division. Computes `self / other`, returning `None` if
    /// `other == 0`.
    pub fn checked_div(self, other: u128) -> Option<u128> {
        if other == ZERO {
            None
        } else {
            Some(udiv128(self, other))
        }
    }

    /// Checked integer remainder. Computes `self % other`, returning `None` if
    /// `other == 0`.
    pub fn checked_rem(self, other: u128) -> Option<u128> {
        if other == ZERO {
            None
        } else {
            Some(umod128(self, other))
        }
    }
}

impl Div for u128 {
    type Output = Self;
    fn div(self, other: Self) -> Self {
        self.wrapping_div(other)
    }
}

impl Rem for u128 {
    type Output = Self;
    fn rem(self, other: Self) -> Self {
        self.wrapping_rem(other)
    }
}

impl Div for Wrapping<u128> {
    type Output = Self;
    fn div(self, other: Self) -> Self {
        Wrapping(self.0.wrapping_div(other.0))
    }
}

impl Rem for Wrapping<u128> {
    type Output = Self;
    fn rem(self, other: Self) -> Self {
        Wrapping(self.0.wrapping_rem(other.0))
    }
}

forward_assign!(DivAssign(div_assign, div) for u128);
forward_assign!(RemAssign(rem_assign, rem) for u128);

impl CheckedDiv for u128 {
    fn checked_div(&self, other: &Self) -> Option<Self> {
        Self::checked_div(*self, *other)
    }
}

/// Computes the divisor and remainder simultaneously. Returns `(a/b, a%b)`.
///
/// Panics if the denominator is zero. Unlike the primitive types, calling this
/// is likely faster than calling `a/b` and `a%b` separately.
pub fn div_rem(numerator: u128, denominator: u128) -> (u128, u128) {
    if denominator == ZERO {
        panic!("attempted to divide by zero");
    } else {
        udivmod128(numerator, denominator)
    }
}

#[cfg(test)]
mod div_rem_tests {
    use u128::{u128, ONE, ZERO, div_rem};

    #[test]
    fn test_div() {
        assert_eq!(u128::from_parts(9071183389512669386, 9598842501673620991) /
                    u128::new(6108228772930395530),
                    u128::from_parts(1, 8948071126007945734));
        assert_eq!(u128::from_parts(3450248868015763521, 12952733755616785885) /
                    u128::new(10250320568692650382),
                    u128::new(6209157794858157762));
        assert_eq!(u128::from_parts(10328265298226767242, 6197012475834382470) /
                    u128::from_parts(3051664430350890703, 4511783754636171344),
                    u128::new(3));
    }

    #[test]
    #[should_panic(expected="attempted to divide by zero")]
    fn test_div_by_zero() {
        ONE / ZERO;
    }

    #[test]
    fn test_rem() {
        assert_eq!(u128::from_parts(9071183389512669386, 9598842501673620991) %
                    u128::new(6108228772930395530),
                    u128::new(5166992697756803267));
        assert_eq!(u128::from_parts(3450248868015763521, 12952733755616785885) %
                    u128::new(10250320568692650382),
                    u128::new(5507621082750620737));
        assert_eq!(u128::from_parts(10328265298226767242, 6197012475834382470) %
                    u128::from_parts(3051664430350890703, 4511783754636171344),
                    u128::from_parts(1173272007174095132, 11108405285635420054));
    }

    #[test]
    #[should_panic(expected="attempted remainder with a divisor of zero")]
    fn test_rem_by_zero() {
        ONE % ZERO;
    }

    #[test]
    fn test_div_rem() {
        assert_eq!(div_rem(u128::from_parts(10328265298226767242, 6197012475834382470),
                            u128::from_parts(3051664430350890703, 4511783754636171344)),
                    (u128::new(3),
                        u128::from_parts(1173272007174095132, 11108405285635420054)));
    }
}

//}}}

//{{{ Casting

impl ToPrimitive for u128 {
    fn to_i64(&self) -> Option<i64> {
        if self.hi != 0 {
            None
        } else {
            self.lo.to_i64()
        }
    }

    fn to_u64(&self) -> Option<u64> {
        if self.hi != 0 {
            None
        } else {
            Some(self.lo)
        }
    }
}

impl FromPrimitive for u128 {
    fn from_u64(n: u64) -> Option<u128> {
        Some(u128::new(n))
    }

    fn from_i64(n: i64) -> Option<u128> {
        n.to_u64().map(u128::new)
    }
}

impl ToExtraPrimitive for u128 {
    fn to_u128(&self) -> Option<u128> {
        Some(*self)
    }

    fn to_i128(&self) -> Option<i128> {
        if self.hi >= 0x8000_0000_0000_0000 {
            None
        } else {
            Some(i128(*self))
        }
    }
}

impl From<u8> for u128 {
    fn from(arg: u8) -> Self {
        u128::new(arg as u64)
    }
}

impl From<u16> for u128 {
    fn from(arg: u16) -> Self {
        u128::new(arg as u64)
    }
}

impl From<u32> for u128 {
    fn from(arg: u32) -> Self {
        u128::new(arg as u64)
    }
}

impl From<u64> for u128 {
    fn from(arg: u64) -> Self {
        u128::new(arg)
    }
}

//}}}

//{{{ Constants

impl Bounded for u128 {
    fn min_value() -> Self {
        MIN
    }

    fn max_value() -> Self {
        MAX
    }
}

impl Zero for u128 {
    fn zero() -> Self {
        ZERO
    }

    fn is_zero(&self) -> bool {
        *self == ZERO
    }
}

impl One for u128 {
    fn one() -> Self {
        ONE
    }
}

//}}}

//{{{ PrimInt

impl PrimInt for u128 {
    fn count_ones(self) -> u32 {
        self.lo.count_ones() + self.hi.count_ones()
    }

    fn count_zeros(self) -> u32 {
        self.lo.count_zeros() + self.hi.count_zeros()
    }

    fn leading_zeros(self) -> u32 {
        if self.hi == 0 {
            64 + self.lo.leading_zeros()
        } else {
            self.hi.leading_zeros()
        }
    }

    fn trailing_zeros(self) -> u32 {
        if self.lo == 0 {
            64 + self.hi.trailing_zeros()
        } else {
            self.lo.trailing_zeros()
        }
    }

    fn rotate_left(self, shift: u32) -> Self {
        let rotated = match shift & 63 {
            0 => self,
            n => u128 {
                lo: self.lo << n | self.hi >> 64u32.wrapping_sub(n),
                hi: self.hi << n | self.lo >> 64u32.wrapping_sub(n),
            },
        };
        if shift & 64 == 0 {
            rotated
        } else {
            u128 { lo: rotated.hi, hi: rotated.lo }
        }
    }

    fn rotate_right(self, shift: u32) -> Self {
        self.rotate_left(128u32.wrapping_sub(shift))
    }

    fn swap_bytes(self) -> Self {
        u128 { lo: self.hi.swap_bytes(), hi: self.lo.swap_bytes() }
    }

    fn signed_shl(self, shift: u32) -> Self {
        self << (shift as usize)
    }

    fn signed_shr(self, shift: u32) -> Self {
        (i128(self) >> (shift as usize)).0
    }

    fn unsigned_shl(self, shift: u32) -> Self {
        self << (shift as usize)
    }

    fn unsigned_shr(self, shift: u32) -> Self {
        self >> (shift as usize)
    }

    fn from_be(x: Self) -> Self {
        if cfg!(target_endian="big") {
            x
        } else {
            x.swap_bytes()
        }
    }

    fn from_le(x: Self) -> Self {
        if cfg!(target_endian="little") {
            x
        } else {
            x.swap_bytes()
        }
    }

    fn to_be(self) -> Self {
        PrimInt::from_be(self)
    }

    fn to_le(self) -> Self {
        PrimInt::from_le(self)
    }

    fn pow(self, mut exp: u32) -> Self {
        let mut base = self;
        let mut acc = ONE;

        while exp > 1 {
            if (exp & 1) == 1 {
                acc *= base;
            }
            exp /= 2;
            base *= base;
        }

        if exp == 1 {
            acc *= base;
        }
        acc
    }
}

impl Unsigned for u128 {
}

#[cfg(test)]
mod prim_int_tests {
    use std::u64;
    use num_traits::PrimInt;
    use u128::{u128, MAX, ZERO, ONE};

    #[test]
    fn test_rotate() {
        assert_eq!(u128::from_parts(0x1e5c7801b0e575f7, 0x53f09dac5b28f152).rotate_right(0),
                    u128::from_parts(0x1e5c7801b0e575f7, 0x53f09dac5b28f152));
        assert_eq!(u128::from_parts(0x1e5c7801b0e575f7, 0x53f09dac5b28f152).rotate_right(1),
                    u128::from_parts(0xf2e3c00d872bafb, 0xa9f84ed62d9478a9));
        assert_eq!(u128::from_parts(0x1e5c7801b0e575f7, 0x53f09dac5b28f152).rotate_right(3),
                    u128::from_parts(0x43cb8f00361caebe, 0xea7e13b58b651e2a));
        assert_eq!(u128::from_parts(0x1e5c7801b0e575f7, 0x53f09dac5b28f152).rotate_right(64),
                    u128::from_parts(0x53f09dac5b28f152, 0x1e5c7801b0e575f7));
        assert_eq!(u128::from_parts(0x1e5c7801b0e575f7, 0x53f09dac5b28f152).rotate_right(120),
                    u128::from_parts(0x5c7801b0e575f753, 0xf09dac5b28f1521e));
    }

    #[test]
    fn test_swap_bytes() {
        assert_eq!(u128::from_parts(0xf0d6891695897d01, 0xb6e2f3a4b065e277).swap_bytes(),
                    u128::from_parts(0x77e265b0a4f3e2b6, 0x017d89951689d6f0));
    }

    #[test]
    fn test_leading_zeros() {
        assert_eq!(u128::from_parts(1, 0).leading_zeros(), 63);
        assert_eq!(u128::from_parts(1, 4).leading_zeros(), 63);
        assert_eq!(u128::from_parts(0, 1).leading_zeros(), 127);
        assert_eq!(u128::from_parts(0, 0).leading_zeros(), 128);
    }

    #[test]
    fn test_trailing_zeros() {
        assert_eq!(u128::from_parts(1, 0).trailing_zeros(), 64);
        assert_eq!(u128::from_parts(3, 0).trailing_zeros(), 64);
        assert_eq!(u128::from_parts(1, 4).trailing_zeros(), 2);
        assert_eq!(u128::from_parts(0, 1).trailing_zeros(), 0);
        assert_eq!(u128::from_parts(0, 0).trailing_zeros(), 128);
    }

    #[test]
    fn test_checked_add() {
        assert_eq!(Some(u128::from_parts(u64::MAX, 0)),
                    u128::from_parts(u64::MAX-1, u64::MAX)
                        .checked_add(u128::new(1)));
        assert_eq!(Some(u128::from_parts(u64::MAX, 0)), u128::new(1)
                        .checked_add(u128::from_parts(u64::MAX-1, u64::MAX)));
        assert_eq!(None, u128::from_parts(u64::MAX, 1)
                        .checked_add(u128::from_parts(u64::MAX, 2)));
        assert_eq!(None, MAX.checked_add(u128::new(1)));
    }

    #[test]
    fn test_checked_sub() {
        assert_eq!(None, ZERO.checked_sub(ONE));
        assert_eq!(None, ZERO.checked_sub(MAX));
        assert_eq!(None, ONE.checked_sub(MAX));
        assert_eq!(Some(ONE), ONE.checked_sub(ZERO));
        assert_eq!(Some(MAX), MAX.checked_sub(ZERO));
        assert_eq!(Some(MAX-ONE), MAX.checked_sub(ONE));
    }

    #[test]
    fn test_checked_mul() {
        assert_eq!(Some(ONE), ONE.checked_mul(ONE));
        assert_eq!(Some(MAX), MAX.checked_mul(ONE));
        assert_eq!(None, MAX.checked_mul(MAX));
        assert_eq!(None, MAX.checked_mul(u128::new(2)));
        assert_eq!(None, u128::from_parts(1, 0).checked_mul(u128::from_parts(1, 0)));
        assert_eq!(Some(u128::from_parts(u64::MAX-1, 1)),
                    u128::new(u64::MAX).checked_mul(u128::new(u64::MAX)));
    }

    #[test]
    fn test_checked_div() {
        assert_eq!(Some(ONE), ONE.checked_div(ONE));
        assert_eq!(Some(MAX), MAX.checked_div(ONE));
        assert_eq!(Some(ZERO), ONE.checked_div(MAX));
        assert_eq!(Some(ZERO), ZERO.checked_div(MAX));
        assert_eq!(None, ONE.checked_div(ZERO));
        assert_eq!(None, MAX.checked_div(ZERO));
    }
}

#[cfg(all(test, extprim_channel="unstable"))]
mod checked_add_sub_bench {
    use u128::u128;
    use test::{Bencher, black_box};

    const BENCH_CHECKED_ADD_SUB: &'static [u128] = &[
        u128 { lo: 8530639231766041497, hi: 1287710968871074399 },
        u128 { lo: 1203542656178406941, hi: 17699966409461566340 },
        u128 { lo: 718458371035876551, hi: 3606247509203879903 },
        u128 { lo: 9776046594219398139, hi: 11242044896228553946 },
        u128 { lo: 7902474877314354323, hi: 15571658655527718712 },
        u128 { lo: 12666717328207407901, hi: 18395053205720380381 },
        u128 { lo: 17339836091522731855, hi: 15731019889221707237 },
        u128 { lo: 8366128025082480321, hi: 13984191269538716594 },
        u128 { lo: 8593645006461074455, hi: 10189081980804969201 },
        u128 { lo: 8264027155501625330, hi: 6198464561866207623 },
        u128 { lo: 10849132074109635036, hi: 5777302818880052808 },
        u128 { lo: 8053806942953838280, hi: 4617639587817452744 },
        u128 { lo: 7575409236673560956, hi: 10773137480165156891 },
        u128 { lo: 4323210863932108621, hi: 16058751318664008901 },
        u128 { lo: 336314576898396552, hi: 8743495691718489785 },
        u128 { lo: 6527874161908570477, hi: 926686061690459595 },
        u128 { lo: 15442937728615642560, hi: 2666553580477360520 },
        u128 { lo: 11855805362816810591, hi: 17643219502201004064 },
        u128 { lo: 16313274500479459547, hi: 5436651574417345289 },
        u128 { lo: 15008613641935618684, hi: 12105224025714335156 },
    ];

    #[bench]
    fn bench_checked_add(bencher: &mut Bencher) {
        bencher.iter(|| {
            for a in BENCH_CHECKED_ADD_SUB {
                for b in BENCH_CHECKED_ADD_SUB {
                    black_box(a.checked_add(*b));
                }
            }
        })
    }

    #[bench]
    fn bench_checked_sub(bencher: &mut Bencher) {
        bencher.iter(|| {
            for a in BENCH_CHECKED_ADD_SUB {
                for b in BENCH_CHECKED_ADD_SUB {
                    black_box(a.checked_sub(*b));
                }
            }
        })
    }
}


//}}}

//{{{ FromStr, FromStrRadix

impl u128 {
    pub fn from_str_radix(src: &str, radix: u32) -> Result<u128, ParseIntError> {
        assert!(radix >= 2 && radix <= 36,
                "from_str_radix_int: must lie in the range `[2, 36]` - found {}",
                radix);

        if src.len() == 0 {
            return Err(error::EMPTY.clone());
        }

        let mut result = ZERO;
        let radix128 = u128::new(radix as u64);

        for c in src.chars() {
            let digit = try!(c.to_digit(radix).ok_or(error::INVALID_DIGIT.clone()));
            let int_result = try!(result.checked_mul(radix128).ok_or(error::OVERFLOW.clone()));
            let digit128 = u128::new(digit as u64);
            result = try!(int_result.checked_add(digit128).ok_or(error::OVERFLOW.clone()));
        }

        Ok(result)
    }
}

impl Num for u128 {
    type FromStrRadixErr = ParseIntError;

    fn from_str_radix(src: &str, radix: u32) -> Result<Self, ParseIntError> {
        Self::from_str_radix(src, radix)
    }
}

impl FromStr for u128 {
    type Err = ParseIntError;

    fn from_str(src: &str) -> Result<Self, ParseIntError> {
        Self::from_str_radix(src, 10)
    }
}

#[cfg(test)]
mod from_str_tests {
    use u128::{u128, MAX, ZERO};
    use error;

    #[test]
    fn test_from_str_radix() {
        const TEST_RESULTS: &'static [&'static str] = &[
            "10011011100100101101000110001011110001010011011101110001100111001000101000101101010100100100010100111001011101010000110001010101",
            "110120222012101010211220122102022000210010022000111102212102202222012022120111212",
            "2123210231012023301103131301213020220231110210110321131100301111",
            "3330311440012420033140113104304110413013304434422141400",
            "13113233024433543105511522325553410033343505511205",
            "1634565460422653144356213116334346545422433412",
            "2334455061361233561471050552444247135206125",
            "13528171124818368023108014385382865276455",
            "206792664785365372185662205006093552725",
            "67649064a7890404084060a25479431a98470",
            "360187787119a95bb767ba32bb0a5b642505",
            "29c058245bb23487574aca216c29577b882",
            "3184907b028135c9183b72cdac9c103109",
            "4bd69b73d8a16036ebec88cd6bb33d335",
            "9b92d18bc537719c8a2d524539750c55",
            "1840gefbd6g31a6ecgg7gc50bd70g1g7",
            "49dheg38e0608a9f4a9267e4g4aagg5",
            "h0h83ahe8172ah96d68dfe26e94124",
            "3h0ea36ada20a526i53ee31044e1g5",
            "jde641e697f962kkidc27ce2edcj2",
            "57bb2c3jgc5h08a1ga70l48l6hc3b",
            "1c8ma26907bj977e8j19da70g8h9e",
            "b547gj6f5egh808nmcnebbeji765",
            "3i36o07m0i9185n46481i4noc990",
            "17f9kaldpa569n4p5gagei47konf",
            "cfq5a3mohb80l380dbnbkq58fdn",
            "4oqm8ncn25iij172m7giopbaol9",
            "1rrq11r63qkjr4s06jq142klq23",
            "oc5mpkf55e6kpj97prm765q0o5",
            "an9nde76jttn4ifukgsdinhsc8",
            "4rib8onh9ne6e8kbai8ksna32l",
            "28b35bg89n93in6l8rfpijv92b",
            "12ajr3pwad0qofcfuk1wbutlp7",
            "i3svxg6wovmba6en6lp37x4cu",
            "97kl2slyj5vbekzxp0lmn5v85",
        ];

        let v = u128::from_parts(11210252820717990300, 9956704808456227925);

        for (base2, res) in TEST_RESULTS.iter().enumerate() {
            assert_eq!(Ok(v), u128::from_str_radix(*res, (base2+2) as u32));
        }

        assert_eq!(Ok(ZERO), u128::from_str_radix("0", 2));
        assert_eq!(Ok(ZERO), u128::from_str_radix("0000000000000000000000000000000000", 36));
        assert_eq!(Err(error::INVALID_DIGIT.clone()), u128::from_str_radix("123", 3));
        assert_eq!(Err(error::INVALID_DIGIT.clone()), u128::from_str_radix("-1", 10));
        assert_eq!(Err(error::EMPTY.clone()), u128::from_str_radix("", 10));
        assert_eq!(Ok(MAX), u128::from_str_radix("f5lxx1zz5pnorynqglhzmsp33", 36));
        assert_eq!(Err(error::OVERFLOW.clone()), u128::from_str_radix("f5lxx1zz5pnorynqglhzmsp34", 36));
        assert_eq!(Err(error::OVERFLOW.clone()), u128::from_str_radix("f5lxx1zz5pnorynqglhzmsp43", 36));
    }
}

//}}}

//{{{ Binary, LowerHex, UpperHex, Octal, String, Show

impl fmt::Display for u128 {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        if self.hi == 0 {
            self.lo.fmt(formatter)
        } else {
            const TEN19: u128 = u128 { lo: 10000000000000000000, hi: 0 };

            let (mid, lo) = div_rem(*self, TEN19);
            let core_string = if mid.hi == 0 {
                format!("{}{:019}", mid.lo, lo.lo)
            } else {
                let (hi, mid) = div_rem(mid, TEN19);
                format!("{}{:019}{:019}", hi.lo, mid.lo, lo.lo)
            };

            formatter.pad_integral(true, "", &*core_string)
        }
    }
}

impl fmt::Debug for u128 {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(formatter, "u128!({})", self)
    }
}

impl fmt::Binary for u128 {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        if self.hi == 0 {
            self.lo.fmt(formatter)
        } else {
            let core_string = format!("{:b}{:064b}", self.hi, self.lo);
            formatter.pad_integral(true, "0b", &core_string)
        }
    }
}

impl fmt::LowerHex for u128 {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        if self.hi == 0 {
            self.lo.fmt(formatter)
        } else {
            let core_string = format!("{:x}{:016x}", self.hi, self.lo);
            formatter.pad_integral(true, "0x", &core_string)
        }
    }
}

impl fmt::UpperHex for u128 {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        if self.hi == 0 {
            self.lo.fmt(formatter)
        } else {
            let core_string = format!("{:X}{:016X}", self.hi, self.lo);
            formatter.pad_integral(true, "0x", &core_string)
        }
    }
}

impl fmt::Octal for u128 {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        const MASK: u64 = (1 << 63) - 1;

        let lo = self.lo & MASK;
        let mid = (self.hi << 1 | self.lo >> 63) & MASK;
        let hi = self.hi >> 62;

        let core_string = if hi != 0 {
            format!("{:o}{:021o}{:021o}", hi, mid, lo)
        } else if mid != 0 {
            format!("{:o}{:021o}", mid, lo)
        } else {
            return lo.fmt(formatter);
        };

        formatter.pad_integral(true, "0o", &core_string)
    }
}

#[cfg(test)]
mod show_tests {
    use u128::{u128, MAX};

    #[test]
    fn test_display() {
        assert_eq!("0", format!("{}", u128::new(0)));
        assert_eq!("10578104835920319894",
                    format!("{}", u128::new(10578104835920319894)));
        assert_eq!("91484347284476727216111035283008240438",
                    format!("{}", u128::from_parts(4959376403712401289, 46322452157807414)));
        assert_eq!("221073131124184722582670274216994227164",
                    format!("{}", u128::from_parts(11984398452150693167, 12960002013829219292)));
        assert_eq!("340282366920938463463374607431768211455",
                    format!("{}", MAX));
        assert_eq!("100000000000000000000000000000000000000",
                    format!("{}", u128::from_parts(5421010862427522170, 687399551400673280)));
        assert_eq!("+00340282366920938463463374607431768211455",
                    format!("{:+042}", MAX));
    }

    #[test]
    fn test_binary() {
        assert_eq!("0", format!("{:b}", u128::new(0)));
        assert_eq!("111001011001111000111001100100010100111010010001110101100101011",
                    format!("{:b}", u128::new(8272862688628501291)));
        assert_eq!("10101011011101011000001011010101101001110110100010000100010001111001010100010010110000000000100100101001100100010110111010011011",
                    format!("{:b}", u128::from_parts(12354925006909113415, 10741859206816689819)));
        assert_eq!("10000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000",
                    format!("{:b}", u128::from_parts(9223372036854775808, 0)));
        assert_eq!("11111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111",
                    format!("{:b}", MAX));
    }

    #[test]
    fn test_hex() {
        assert_eq!("0", format!("{:x}", u128::new(0)));
        assert_eq!("25c22f8602efedb5",
                    format!("{:x}", u128::new(2720789377506602421)));
        assert_eq!("2c73d4b3d1a46f081a04e1ea9846faee",
                    format!("{:x}", u128::from_parts(3203137628772003592, 1874871742586354414)));
        assert_eq!("80000000000000000000000000000000",
                    format!("{:x}", u128::from_parts(9223372036854775808, 0)));
        assert_eq!(" 0xA", format!("{:#4X}", u128::new(10)));
        assert_eq!("25C22F8602EFEDB5",
                    format!("{:X}", u128::new(2720789377506602421)));
        assert_eq!("2C73D4B3D1A46F081A04E1EA9846FAEE",
                    format!("{:X}", u128::from_parts(3203137628772003592, 1874871742586354414)));
        assert_eq!("C000000000000000000000000000000",
                    format!("{:X}", u128::from_parts(864691128455135232, 0)));
    }

    #[test]
    fn test_octal() {
        assert_eq!("0", format!("{:o}", u128::new(0)));
        assert_eq!("351462366146756037170",
                    format!("{:o}", u128::new(4208138189379485304)));
        assert_eq!("7000263630010212417200",
                    format!("{:o}", u128::from_parts(3, 9229698078115241600)));
        assert_eq!("3465177151267706210351536216110755202064135",
                    format!("{:o}", u128::from_parts(16620520452737763444, 15533412710015854685)));
        assert_eq!("3777777777777777777777777777777777777777777",
                    format!("{:o}", u128::from_parts(18446744073709551615, 18446744073709551615)));
        assert_eq!("2000000000000000000000000000000000000000000",
                    format!("{:o}", u128::from_parts(9223372036854775808, 0)));
    }
}

//}}}

