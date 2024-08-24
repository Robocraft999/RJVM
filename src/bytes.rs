

pub fn read_byte<I>(bytes: &mut I) -> Result<u8, std::io::Error>
where I: Iterator<Item=Result<u8, std::io::Error>>
{
    return bytes.next().expect("Could not read byte, because of EOF")
}

pub fn parse_u1<I>(bytes: &mut I) -> Result<u8, std::io::Error>
where I: Iterator<Item=Result<u8, std::io::Error>>
{
    return Ok(read_byte(bytes)?)
}

pub fn parse_u2<I>(bytes: &mut I) -> Result<u16, std::io::Error>
where I: Iterator<Item=Result<u8, std::io::Error>>
{
    let b1 = read_byte(bytes)?;
    let b2 = read_byte(bytes)?;
    let val = u16::from_be_bytes([b1,b2]);
    return Ok(val)
}

pub fn parse_u4<I>(bytes: &mut I) -> Result<u32, std::io::Error>
where I: Iterator<Item=Result<u8, std::io::Error>>
{
    let b1 = read_byte(bytes)?;
    let b2 = read_byte(bytes)?;
    let b3 = read_byte(bytes)?;
    let b4 = read_byte(bytes)?;
    let val = u32::from_be_bytes([b1,b2,b3,b4]);
    return Ok(val)
}