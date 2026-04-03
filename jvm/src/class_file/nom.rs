use cesu8::from_java_cesu8;
use nom::{IResult, Parser};
use nom::combinator::map;
use nom::multi::length_count;
use nom::number::{be_f32, be_f64, be_i32, be_i64, be_u16, be_u32, be_u8};
use crate::class_file::constant_pool::ConstantPoolEntry;

#[derive(Clone, Debug, PartialEq)]
pub struct RawClassFile{
    magic: u32,
    minor_version: u16,
    major_version: u16,
    constant_pool: Vec<ConstantPoolEntry>,
    access_flags: u16,
    this_class: u16,
    super_class: u16,
    interfaces: Vec<u16>,
    fields: Vec<RawAccessibleInfo>,
    methods: Vec<RawAccessibleInfo>,
    attributes: Vec<RawAttribute>,
}

#[derive(Clone, Debug, PartialEq)]
struct RawAccessibleInfo{
    access_flags: u16,
    name_index: u16,
    descriptor_index: u16,
    attributes: Vec<RawAttribute>,
}

#[derive(Clone, Debug, PartialEq)]
struct RawAttribute {
    attribute_name_index: u16,
    info: Vec<u8>,
}

fn parse_u16_vec_u8(input: &[u8]) -> IResult<&[u8], Vec<u8>> {
    length_count(be_u16(), be_u8()).parse(input)
}

fn parse_u16_vec_u16(input: &[u8]) -> IResult<&[u8], Vec<u16>> {
    length_count(be_u16(), be_u16()).parse(input)
}

fn parse_u32_vec_u8(input: &[u8]) -> IResult<&[u8], Vec<u8>> {
    length_count(be_u32(), be_u8()).parse(input)
}

fn parse_attributes_vec(input: &[u8]) -> IResult<&[u8], Vec<RawAttribute>> {
    length_count(be_u16(), parse_attribute).parse(input)
}

fn parse_accessible_vec(input: &[u8]) -> IResult<&[u8], Vec<RawAccessibleInfo>> {
    length_count(be_u16(), parse_accessible_info).parse(input)
}

fn parse_constant_pool(input: &[u8]) -> IResult<&[u8], Vec<ConstantPoolEntry>> {
    length_count(map(be_u16(), |c| c - 1), parse_constant_pool_entry).parse(input)
}

// FIXME use length_value when we switch to parsing the actual variants (but has to be in the raw -> intermediary stage)
fn parse_attribute(input: &[u8]) -> IResult<&[u8], RawAttribute> {
    let (remaining, (attribute_name_index, info)) = (be_u16(), parse_u32_vec_u8).parse(input)?;
    Ok((remaining, RawAttribute{attribute_name_index, info}))
}

fn parse_accessible_info(input: &[u8]) -> IResult<&[u8], RawAccessibleInfo> {
    let (remaining, (access_flags, name_index, descriptor_index, attributes)) = (be_u16(), be_u16(), be_u16(), parse_attributes_vec).parse(input)?;
    Ok((remaining, RawAccessibleInfo{access_flags, name_index, descriptor_index, attributes}))
}

fn parse_cesu_string(input: &[u8]) -> IResult<&[u8], String> {
    let (remaining, bytes) = parse_u16_vec_u8(input)?;
    let string = from_java_cesu8(&bytes).map_err(|e| nom::Err::Error(nom::error::FromExternalError::from_external_error(input, nom::error::ErrorKind::Alt, e)))?.to_string();
    Ok((remaining, string))
}

fn parse_constant_pool_entry(input: &[u8]) -> IResult<&[u8], ConstantPoolEntry> {
    let (remaining, tag) = be_u8().parse(input)?;
    match tag {
        1  => map(parse_cesu_string, ConstantPoolEntry::Utf8).parse(remaining),
        3  => map(be_i32(), ConstantPoolEntry::Integer).parse(remaining),
        4  => map(be_f32(), ConstantPoolEntry::Float).parse(remaining),
        5  => map(be_i64(), ConstantPoolEntry::Long).parse(remaining),
        6  => map(be_f64(), ConstantPoolEntry::Double).parse(remaining),
        7  => map(be_u16(), ConstantPoolEntry::Class).parse(remaining),
        8  => map(be_u16(), ConstantPoolEntry::String).parse(remaining),
        9  => map((be_u16(), be_u16()), |(a, b)| ConstantPoolEntry::Fieldref(a, b)).parse(remaining),
        10 => map((be_u16(), be_u16()), |(a, b)| ConstantPoolEntry::Methodref(a, b)).parse(remaining),
        11 => map((be_u16(), be_u16()), |(a, b)| ConstantPoolEntry::InterfaceMethodref(a, b)).parse(remaining),
        12 => map((be_u16(), be_u16()), |(a, b)| ConstantPoolEntry::NameAndType(a, b)).parse(remaining),
        15 => map((be_u8() , be_u16()), |(a, b)| ConstantPoolEntry::MethodHandle(a, b)).parse(remaining),
        16 => map(be_u16(), ConstantPoolEntry::MethodType).parse(remaining),
        18 => map((be_u16(), be_u16()), |(a, b)| ConstantPoolEntry::InvokeDynamic(a, b)).parse(remaining),
        _ => Err(nom::Err::Error(nom::error::Error::new(input, nom::error::ErrorKind::Alt))),
    }
}

fn parse_class_file(input: &[u8]) -> IResult<&[u8], RawClassFile> {
    let (remaining, (
        magic,
        minor_version,
        major_version,
        constant_pool,
        access_flags,
        this_class,
        super_class,
        interfaces,
        fields,
        methods,
        attributes,
    )) = (be_u32(), be_u16(), be_u16(), parse_constant_pool, be_u16(), be_u16(), be_u16(), parse_u16_vec_u16, parse_accessible_vec, parse_accessible_vec, parse_attributes_vec).parse(input)?;
    Ok((remaining, RawClassFile{magic, minor_version, major_version, constant_pool, access_flags, this_class, super_class, interfaces, fields, methods, attributes}))
}

#[cfg(test)]
mod tests{
    use std::error::Error;
    use crate::class_file::nom::{parse_attribute, parse_accessible_info, RawAttribute, RawAccessibleInfo, parse_class_file};

    const SINGLE_METHOD_ATTRIBUTE: &[u8] = &[0,10, 0,0,0,4, 0,11, 0,12];
    const METHOD_INFO: &[u8] = &[0,1, 0,20, 0,21, 0,2, 0,10, 0,0,0,4, 0,11, 0,12, 0,13, 0,0,0,4, 0,14, 0,15];
    const FULL_CLASS: &[u8] = include_bytes!("../../../resources/test/Simple.class");

    #[test]
    fn test_basic_attribute() -> Result<(), Box<dyn Error>>{
        let (remaining, parsed) = parse_attribute(SINGLE_METHOD_ATTRIBUTE)?;
        assert_eq!(remaining, &vec![]);
        assert_eq!(parsed, RawAttribute{attribute_name_index: 10, info: vec![0, 11, 0, 12]});
        Ok(())
    }

    #[test]
    fn test_basic_method_info() -> Result<(), Box<dyn Error>>{
        let (remaining, parsed) = parse_accessible_info(METHOD_INFO)?;
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
        let (remaining, parsed) = parse_class_file(FULL_CLASS)?;
        assert_eq!(remaining, &vec![]);
        println!("result {:#?}", parsed);
        Ok(())
    }
}