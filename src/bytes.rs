use crate::error::ClassParseError;

pub fn read_byte<I>(bytes: &mut I) -> Result<u8, ClassParseError>
where I: Iterator<Item=u8>
{
    match bytes.next() {
        None => {Err(ClassParseError::ReadError) }
        Some(e) => {Ok(e)}
    }
}

pub fn parse_u1<I>(bytes: &mut I) -> Result<u8, ClassParseError>
where I: Iterator<Item=u8>
{
    return Ok(read_byte(bytes)?)
}

pub fn parse_u2<I>(bytes: &mut I) -> Result<u16, ClassParseError>
where I: Iterator<Item=u8>
{
    let b1 = read_byte(bytes)?;
    let b2 = read_byte(bytes)?;
    let val = u16::from_be_bytes([b1,b2]);
    return Ok(val)
}

pub fn parse_u4<I>(bytes: &mut I) -> Result<u32, ClassParseError>
where I: Iterator<Item=u8>
{
    let b1 = read_byte(bytes)?;
    let b2 = read_byte(bytes)?;
    let b3 = read_byte(bytes)?;
    let b4 = read_byte(bytes)?;
    let val = u32::from_be_bytes([b1,b2,b3,b4]);
    return Ok(val)
}

pub fn parse_u8<I>(bytes: &mut I) -> Result<u64, ClassParseError>
where I: Iterator<Item=u8>
{
    /*let b1 = read_byte(bytes)?;
    let b2 = read_byte(bytes)?;
    let b3 = read_byte(bytes)?;
    let b4 = read_byte(bytes)?;
    let b5 = read_byte(bytes)?;
    let b6 = read_byte(bytes)?;
    let b7 = read_byte(bytes)?;
    let b8 = read_byte(bytes)?;
    let val = u64::from_be_bytes([b1,b2,b3,b4,b5,b6,b7,b8]);*/
    let t = [parse_u4(bytes)?.to_be_bytes(), parse_u4(bytes)?.to_be_bytes()].concat();
    let val = u64::from_be_bytes(t.as_slice().try_into().unwrap());

    return Ok(val)
}