use std::fmt::{Binary, Debug, Display, Formatter, LowerHex};
use crate::bytes::ByteType::{U1, U2, U4};

pub fn read_byte<I>(bytes: &mut I) -> Result<u8, std::io::Error>
where I: Iterator<Item=Result<u8, std::io::Error>>
{
    return bytes.next().expect("Could not read byte, because of EOF")
}

pub fn parse_u1<I>(bytes: &mut I) -> Result<ByteType, std::io::Error>
where I: Iterator<Item=Result<u8, std::io::Error>>
{
    return Ok(U1(read_byte(bytes)?))
}

pub fn parse_u2<I>(bytes: &mut I) -> Result<ByteType, std::io::Error>
where I: Iterator<Item=Result<u8, std::io::Error>>
{
    let b1 = read_byte(bytes)?;
    let b2 = read_byte(bytes)?;
    let val = u16::from_be_bytes([b1,b2]);
    return Ok(U2(val))
}

pub fn parse_u4<I>(bytes: &mut I) -> Result<ByteType, std::io::Error>
where I: Iterator<Item=Result<u8, std::io::Error>>
{
    let b1 = read_byte(bytes)?;
    let b2 = read_byte(bytes)?;
    let b3 = read_byte(bytes)?;
    let b4 = read_byte(bytes)?;
    let val = u32::from_be_bytes([b1,b2,b3,b4]);
    return Ok(U4(val))
}

#[derive(Clone, Copy)]
pub enum ByteType{
    U1(u8),
    U2(u16),
    U4(u32)
}

impl LowerHex for ByteType{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            U1(val) => {write!(f, "{:x}", val)}
            U2(val) => {write!(f, "{:x}", val)}
            U4(val) => {write!(f, "{:x}", val)}
        }
    }
}

impl Binary for ByteType{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            U1(val) => {write!(f, "{:b}", val)}
            U2(val) => {write!(f, "{:b}", val)}
            U4(val) => {write!(f, "{:b}", val)}
        }
    }
}

impl Into<u32> for ByteType{
    fn into(self) -> u32 {
        match self {
            U1(val) => {val as u32}
            U2(val) => {val as u32}
            U4(val) => {val}
        }
    }
}

impl Display for ByteType{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            U1(val) => {write!(f, "{val}")}
            U2(val) => {write!(f, "{val}")}
            U4(val) => {write!(f, "{val}")}
        }
    }
}

impl Debug for ByteType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self)
    }
}