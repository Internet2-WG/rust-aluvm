// AluRE: AluVM runtime environment.
// This is rust implementation of AluVM (arithmetic logic unit virtual machine).
//
// Designed & written in 2021 by
//     Dr. Maxim Orlovsky <orlovsky@pandoracore.com>
//
// This software is licensed under the terms of MIT License.
// You should have received a copy of the MIT License along with this software.
// If not, see <https://opensource.org/licenses/MIT>.

use core::fmt::{self, Display, Formatter, Write};
use core::hash::{Hash, Hasher};
use core::ops::{Deref, Index, IndexMut};
#[cfg(feature = "std")]
use core::str::FromStr;

use amplify_num::{u1024, u256, u512};

/// Register value, which may be `None`
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug, Default, From)]
pub struct RegVal(
    Option<Value>, // TODO: Keep arithmetics type
);

impl RegVal {
    /// Creates [`RegVal`] without assigning a value to it
    pub fn none() -> RegVal { RegVal(None) }

    /// Creates [`RegVal`] assigning a value to it
    pub fn some(val: Value) -> RegVal { RegVal(Some(val)) }
}

impl From<Value> for RegVal {
    fn from(val: Value) -> Self { RegVal(Some(val)) }
}

impl From<&Value> for RegVal {
    fn from(val: &Value) -> Self { RegVal(Some(*val)) }
}

impl From<&Option<Value>> for RegVal {
    fn from(val: &Option<Value>) -> Self { RegVal(*val) }
}

impl From<Option<&Value>> for RegVal {
    fn from(val: Option<&Value>) -> Self { RegVal(val.copied()) }
}

impl From<RegVal> for Option<Value> {
    fn from(val: RegVal) -> Self { val.0 }
}

impl Deref for RegVal {
    type Target = Option<Value>;

    fn deref(&self) -> &Self::Target { &self.0 }
}

impl Display for RegVal {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self.0 {
            None => f.write_str("~"),
            Some(ref val) => Display::fmt(val, f),
        }
    }
}

/// Copy'able variable length slice
#[derive(Copy, Clone, Debug)]
pub struct Value {
    /// Slice length
    pub len: u16,

    /// Slice bytes
    pub bytes: [u8; 1024],
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool { self.to_clean().eq(&other.to_clean()) }
}

impl Eq for Value {}

impl Hash for Value {
    fn hash<H: Hasher>(&self, state: &mut H) {
        let clean = self.to_clean();
        state.write_u16(clean.len);
        state.write(&clean.bytes);
    }
}

impl Default for Value {
    fn default() -> Value { Value { len: 0, bytes: [0u8; 1024] } }
}

impl AsRef<[u8]> for Value {
    fn as_ref(&self) -> &[u8] { &self.bytes[..self.len as usize] }
}

impl AsMut<[u8]> for Value {
    fn as_mut(&mut self) -> &mut [u8] { &mut self.bytes[..self.len as usize] }
}

impl Index<u16> for Value {
    type Output = u8;

    fn index(&self, index: u16) -> &Self::Output {
        assert!(index < self.len);
        &self.bytes[index as usize]
    }
}

impl IndexMut<u16> for Value {
    fn index_mut(&mut self, index: u16) -> &mut Self::Output {
        assert!(index < self.len);
        &mut self.bytes[index as usize]
    }
}

impl Value {
    /// Creates zero value of a given dimension
    #[inline]
    pub fn zero(len: u16) -> Value { Value { len, bytes: [0u8; 1024] } }

    /// Constructs value from slice of bytes.
    ///
    /// Panics if the length of the slice is greater than 1024 bytes.
    pub fn with(slice: impl AsRef<[u8]>) -> Value {
        let len = slice.as_ref().len();
        let mut bytes = [0u8; 1024];
        bytes[0..len].copy_from_slice(slice.as_ref());
        Value { len: len as u16, bytes }
    }

    /// Constructs value from hex string
    #[cfg(feature = "std")]
    pub fn from_hex(s: &str) -> Result<Value, amplify_num::hex::Error> {
        use amplify_num::hex::FromHex;
        let s = s.trim_start_matches("0x");
        let len = s.len() / 2;
        if len > 1024 {
            return Err(amplify_num::hex::Error::InvalidLength(1024, len));
        }
        let mut bytes = [0u8; 1024];
        let hex = Vec::<u8>::from_hex(&s)?;
        bytes[0..len].copy_from_slice(&hex);
        Ok(Value { len: hex.len() as u16, bytes })
    }

    /// Serializes value in hexadecimal format to a string
    #[cfg(feature = "std")]
    pub fn to_hex(self) -> String {
        let mut ret = String::with_capacity(2usize * self.len as usize + 2);
        write!(ret, "0x").expect("writing to string");
        for ch in &self.bytes {
            write!(ret, "{:02x}", ch).expect("writing to string");
        }
        ret
    }

    /// Returns the number of ones in the binary representation of `self`.
    pub fn count_ones(&self) -> u16 {
        let mut count = 0u16;
        for byte in &self.bytes[..self.len as usize] {
            count += byte.count_ones() as u16;
        }
        count
    }

    /// Ensures that all non-value bits are set to zero
    #[inline]
    pub fn clean(&mut self) { self.bytes[self.len as usize..].fill(0); }

    /// Returns a copy where all non-value bits are set to zero
    #[inline]
    pub fn to_clean(self) -> Self {
        let mut copy = self;
        copy.bytes[self.len as usize..].fill(0);
        copy
    }

    /// Converts the value into `u1024` integer
    #[inline]
    pub fn to_u1024(self) -> u1024 { self.to_clean().into() }
}

/// Errors parsing literal values in AluVM assembly code
#[cfg(feature = "std")]
#[derive(Clone, Eq, PartialEq, Debug, Display, From)]
#[cfg_attr(feature = "std", derive(Error))]
#[display(inner)]
#[allow(clippy::branches_sharing_code)]
pub enum LiteralParseError {
    /// Error parsing hexadecimal literal
    #[from]
    Hex(amplify_num::hex::Error),

    /// Error parsing decimal literal
    #[from]
    Int(core::num::ParseIntError),

    /// Unknown literal
    #[display("unknown token `{0}` while parsing AluVM assembly literal")]
    UnknownLiteral(String),
}

#[cfg(feature = "std")]
impl FromStr for Value {
    type Err = LiteralParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.starts_with("0x") {
            Value::from_hex(s).map_err(LiteralParseError::from)
        } else if s.starts_with('-') {
            // TODO: use arbitrary-precision type `FromStr`
            Ok(Value::from(i128::from_str(s)?))
        } else {
            // TODO: use arbitrary-precision type `FromStr`
            let val = u128::from_str(s)?;
            Ok(match val {
                0..=0xFF => Value::from(val as u8),
                0x100..=0xFFFF => Value::from(val as u16),
                0x10000..=0xFFFFFFFF => Value::from(val as u32),
                0x100000000..=0xFFFFFFFFFFFFFFFF => Value::from(val as u64),
                _ => Value::from(val),
            })
        }
    }
}

#[cfg(feature = "std")]
impl Display for Value {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        use amplify_num::hex::ToHex;
        f.write_str("0x")?;
        if f.alternate() && self.len > 4 {
            write!(
                f,
                "{}..{}",
                self.bytes[..4].to_hex(),
                self.bytes[(self.len as usize - 4)..].to_hex()
            )
        } else {
            f.write_str(&self.bytes[0usize..(self.len as usize)].to_hex())
        }
    }
}

#[cfg(not(feature = "std"))]
impl Display for Value {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str("0x")?;
        write!(f, "{:#04X?}..{:#04X?}", &self.bytes[..4], &self.bytes[(self.len as usize - 4)..])
    }
}

macro_rules! impl_value_bytes_conv {
    ($len:literal) => {
        impl From<Value> for [u8; $len] {
            fn from(mut val: Value) -> Self {
                let mut bytes = [0u8; $len];
                let clean = Value::default();
                val.bytes[$len..].copy_from_slice(&clean.bytes[$len..]);
                bytes.copy_from_slice(&val.bytes[0..$len]);
                bytes
            }
        }

        impl From<[u8; $len]> for Value {
            fn from(val: [u8; $len]) -> Value {
                let mut bytes = [0u8; 1024];
                bytes[0..$len].copy_from_slice(&val[..]);
                Value { len: $len, bytes }
            }
        }

        impl From<[u8; $len]> for RegVal {
            fn from(val: [u8; $len]) -> RegVal { RegVal::from(Value::from(val)) }
        }

        impl From<Option<[u8; $len]>> for RegVal {
            fn from(val: Option<[u8; $len]>) -> RegVal { RegVal::from(val.map(Value::from)) }
        }

        impl From<&Option<[u8; $len]>> for RegVal {
            fn from(val: &Option<[u8; $len]>) -> RegVal { RegVal::from(val.map(Value::from)) }
        }
    };
}

macro_rules! impl_value_ty_conv {
    ($ty:ident, $len:literal) => {
        impl From<Value> for $ty {
            fn from(val: Value) -> Self { $ty::from_le_bytes(<[u8; $len]>::from(val)) }
        }

        impl From<$ty> for Value {
            fn from(val: $ty) -> Self { Value::from(&val) }
        }
        impl From<&$ty> for Value {
            fn from(val: &$ty) -> Self {
                let mut bytes = [0u8; 1024];
                let le = val.to_le_bytes();
                bytes[0..le.len()].copy_from_slice(&le[..]);
                Value { len: le.len() as u16, bytes }
            }
        }

        impl From<$ty> for RegVal {
            fn from(val: $ty) -> Self { RegVal::some(Value::from(val)) }
        }
        impl From<&$ty> for RegVal {
            fn from(val: &$ty) -> Self { RegVal::some(Value::from(*val)) }
        }
        impl From<Option<$ty>> for RegVal {
            fn from(val: Option<$ty>) -> Self { RegVal::from(val.map(Value::from)) }
        }
        impl From<Option<&$ty>> for RegVal {
            fn from(val: Option<&$ty>) -> Self { RegVal::from(val.copied().map(Value::from)) }
        }
        impl From<&Option<$ty>> for RegVal {
            fn from(val: &Option<$ty>) -> Self { RegVal::from((*val).map(Value::from)) }
        }
    };
}

impl_value_bytes_conv!(1);
impl_value_bytes_conv!(2);
impl_value_bytes_conv!(4);
impl_value_bytes_conv!(8);
impl_value_bytes_conv!(16);
impl_value_bytes_conv!(20);
impl_value_bytes_conv!(32);
impl_value_bytes_conv!(64);
impl_value_bytes_conv!(128);
impl_value_bytes_conv!(256);
impl_value_bytes_conv!(512);
impl_value_bytes_conv!(1024);

impl_value_ty_conv!(u8, 1);
impl_value_ty_conv!(u16, 2);
impl_value_ty_conv!(u32, 4);
impl_value_ty_conv!(u64, 8);
impl_value_ty_conv!(u128, 16);
impl_value_ty_conv!(u256, 32);
impl_value_ty_conv!(u512, 64);
impl_value_ty_conv!(u1024, 128);

impl_value_ty_conv!(i8, 1);
impl_value_ty_conv!(i16, 2);
impl_value_ty_conv!(i32, 4);
impl_value_ty_conv!(i64, 8);
impl_value_ty_conv!(i128, 16);

/// Value for step instructions which can be displayed as a part of operation mnemonic
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug, Default, From)]
pub struct Step(#[from] Value);

impl Display for Step {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            let val = i64::from(self.0);
            match val {
                1 => f.write_str("inc"),
                -1 => f.write_str("dec"),
                x if x < 0 => f.write_str("sub"),
                x if x >= 0 => f.write_str("add"),
            }
        } else {
            f.write_char(',')?;
            Display::fmt(&self.0, f)
        }
    }
}