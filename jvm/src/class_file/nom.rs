use cesu8::from_java_cesu8;
use nom::{IResult, Parser};
use nom::multi::length_count;
use nom::number::{be_u16, be_u8};
use nom_derive::NomBE;
use crate::class_file::constant_pool::ConstantPoolEntry;

#[derive(Clone, Debug, PartialEq, NomBE)]
pub struct RawClassFile{
    magic: u32,
    minor_version: u16,
    major_version: u16,
    constant_pool_count: u16,
    #[nom(Count="(constant_pool_count-1)")]
    constant_pool: Vec<ConstantPoolEntry>,
    access_flags: u16,
    this_class: u16,
    super_class: u16,
    interfaces_count: u16,
    #[nom(Count="interfaces_count")]
    interfaces: Vec<u16>,
    fields_count: u16,
    #[nom(Count="fields_count")]
    fields: Vec<RawAccessibleInfo>,
    methods_count: u16,
    #[nom(Count="methods_count")]
    methods: Vec<RawAccessibleInfo>,
    attributes_count: u16,
    #[nom(Count="attributes_count")]
    attributes: Vec<RawAttribute>,
}

#[derive(Clone, Debug, PartialEq, NomBE)]
struct RawAccessibleInfo{
    access_flags: u16,
    name_index: u16,
    descriptor_index: u16,
    attributes_count: u16,
    #[nom(Count="attributes_count")]
    attributes: Vec<RawAttribute>,
}

#[derive(Clone, Debug, PartialEq, NomBE)]
struct RawAttribute {
    attribute_name_index: u16,
    attribute_length: u32,
    #[nom(Count="attribute_length")]
    info: Vec<u8>,
}

fn parse_u16_length_bytes(input: &[u8]) -> IResult<&[u8], Vec<u8>> {
    length_count(be_u16(), be_u8()).parse(input)
}

pub fn parse_cesu_string(input: &[u8]) -> IResult<&[u8], String> {
    let (remaining, bytes) = parse_u16_length_bytes(input)?;
    let string = from_java_cesu8(&bytes).map_err(|e| nom::Err::Error(nom::error::FromExternalError::from_external_error(input, nom::error::ErrorKind::Alt, e)))?.to_string();
    Ok((remaining, string))
}

#[cfg(test)]
mod tests {
    use std::error::Error;
    use nom_derive::Parse;
    use crate::class_file::nom::{RawAccessibleInfo, RawAttribute, RawClassFile};

    const SINGLE_METHOD_ATTRIBUTE: &[u8] = &[0,10, 0,0,0,4, 0,11, 0,12];
    const METHOD_INFO: &[u8] = &[0,1, 0,20, 0,21, 0,2, 0,10, 0,0,0,4, 0,11, 0,12, 0,13, 0,0,0,4, 0,14, 0,15];
    const FULL_CLASS: &[u8] = include_bytes!("../../../resources/test/Simple.class");

    #[test]
    fn test_basic_attribute() -> Result<(), Box<dyn Error>>{
        let (remaining, parsed) = RawAttribute::parse(SINGLE_METHOD_ATTRIBUTE)?;
        assert_eq!(remaining, &vec![]);
        assert_eq!(parsed, RawAttribute{attribute_name_index: 10, attribute_length: 4, info: vec![0, 11, 0, 12]});
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
            attributes_count: 2,
            attributes: vec![
                RawAttribute{attribute_name_index: 10, attribute_length: 4, info: vec![0, 11, 0, 12]},
                RawAttribute{attribute_name_index: 13, attribute_length: 4, info: vec![0, 14, 0, 15]}
            ]}
        );
        Ok(())
    }

    #[test]
    fn test_full_class() -> Result<(), Box<dyn Error>>{
        let (remaining, class_file) = RawClassFile::parse(FULL_CLASS)?;
        assert_eq!(remaining, &vec![]);
        println!("result {:#?}", class_file);
        Ok(())
    }
}