use crate::class_file::constant_pool::ConstantPoolEntry;
use crate::vm::result::VMResult;
use crate::vm::VmError;
use cesu8::from_java_cesu8;
use nom::error::ParseError;
use nom::multi::length_count;
use nom::number::{be_u16, be_u32, be_u8};
use nom::{IResult, Parser};
use nom_derive::{NomBE, Parse};
use std::mem;

#[derive(Clone, Debug, NomBE)]
#[nom(DebugDerive)]
pub struct RawClassFile<'a>{
    magic: u32,
    minor_version: u16,
    major_version: u16,
    #[nom(Parse="(parse_constant_pool)")]
    pub constant_pool: Vec<ConstantPoolEntry<'a>>,
    pub access_flags: u16,
    pub this_class: u16,
    pub super_class: u16,
    #[nom(LengthCount = "be_u16()")]
    pub interfaces: Vec<u16>,
    #[nom(LengthCount = "be_u16()")]
    pub fields: Vec<RawAccessibleInfo>,
    #[nom(LengthCount = "be_u16()")]
    pub methods: Vec<RawAccessibleInfo>,
    #[nom(LengthCount = "be_u16()")]
    pub attributes: Vec<RawAttribute>,
}

fn parse_constant_pool<'a, 'nom>(orig_i: &'nom [u8]) -> IResult<&'nom [u8], Vec<ConstantPoolEntry<'a>>>
where
    'nom: 'a
{
    let (mut i, constant_pool_count) = be_u16().parse(orig_i)?;
    let mut constant_pool = Vec::with_capacity(constant_pool_count as usize -1);
    let mut remaining = constant_pool_count-1;
    while remaining > 0 {
        let (j, entry) = ConstantPoolEntry::parse(i)?;
        i = j;
        let is_double = if let ConstantPoolEntry::Double(_) | ConstantPoolEntry::Long(_) = entry {true} else {false};
        constant_pool.push(entry);

        remaining -= if is_double {
            constant_pool.push(ConstantPoolEntry::Dummy);
            2
        } else {
            1
        }
    }
    Ok((i, constant_pool))
}

#[derive(Clone, Debug, PartialEq, NomBE)]
pub struct RawAccessibleInfo{
    pub access_flags: u16,
    pub name_index: u16,
    pub descriptor_index: u16,
    #[nom(LengthCount = "be_u16()")]
    pub attributes: Vec<RawAttribute>,
}

#[derive(Clone, Debug, PartialEq, NomBE)]
pub struct RawAttribute {
    pub attribute_name_index: u16,
    #[nom(LengthCount = "be_u32()")]
    pub info: Vec<u8>,
}

fn parse_u16_length_bytes(input: &[u8]) -> IResult<&[u8], Vec<u8>> {
    length_count(be_u16(), be_u8()).parse(input)
}

pub fn parse_cesu_string(input: &[u8]) -> IResult<&[u8], String> {
    let (remaining, bytes) = parse_u16_length_bytes(input)?;
    let string = from_java_cesu8(&bytes).map_err(|e| nom::Err::Error(nom::error::FromExternalError::from_external_error(input, nom::error::ErrorKind::Alt, e)))?.to_string();
    Ok((remaining, string))
}

pub fn parse_class_file<'a>(bytes: Vec<u8>) -> VMResult<RawClassFile<'a>>{
    let bytes = unsafe { mem::transmute::<_, &'static [u8]>(bytes.as_slice()) };
    let (_, class_file) = RawClassFile::parse(&bytes).map_err(|e| VmError::Unspecified(format!("Error when parsing raw class {}", e.to_string())))?;
    assert_eq!(class_file.magic, 0xCAFEBABE);
    Ok(class_file)
}

#[cfg(test)]
mod tests {
    use crate::class_file::nom::{RawAccessibleInfo, RawAttribute, RawClassFile};
    use nom_derive::Parse;
    use std::error::Error;

    const SINGLE_METHOD_ATTRIBUTE: &[u8] = &[0,10, 0,0,0,4, 0,11, 0,12];
    const METHOD_INFO: &[u8] = &[0,1, 0,20, 0,21, 0,2, 0,10, 0,0,0,4, 0,11, 0,12, 0,13, 0,0,0,4, 0,14, 0,15];
    const FULL_CLASS: &[u8] = include_bytes!("../../../resources/test/Simple.class");

    #[test]
    fn test_basic_attribute() -> Result<(), Box<dyn Error>>{
        let (remaining, parsed) = RawAttribute::parse(SINGLE_METHOD_ATTRIBUTE)?;
        assert_eq!(remaining, &vec![]);
        assert_eq!(parsed, RawAttribute{attribute_name_index: 10, info: vec![0, 11, 0, 12]});
        Ok(())
    }

    #[test]
    fn test_basic_method_info() -> Result<(), Box<dyn Error>>{
        let (remaining, parsed) = RawAccessibleInfo::parse(METHOD_INFO)?;
        assert_eq!(remaining, &vec![]);
        assert_eq!(parsed, RawAccessibleInfo{
            access_flags: 1,
            name_index: 20,
            descriptor_index: 21,
            attributes: vec![
                RawAttribute{attribute_name_index: 10, info: vec![0, 11, 0, 12]},
                RawAttribute{attribute_name_index: 13, info: vec![0, 14, 0, 15]}
            ]}
        );
        Ok(())
    }

    #[test]
    fn test_full_class() -> Result<(), Box<dyn Error>>{
        println!("{:?}", &FULL_CLASS[..10]);
        let (remaining, class_file) = RawClassFile::parse(FULL_CLASS)?;
        assert_eq!(remaining, &vec![]);
        println!("result {:#?}", class_file);
        Ok(())
    }
}