#[derive(Debug)]
pub struct Attribute{
    pub name: String,
    pub info: Vec<u8>
}

#[derive(Debug)]
pub struct ConstantValue{

}

#[derive(Debug)]
pub struct Code{
    pub max_stack: u16,
    pub max_locals: u16,
    //TODO add proper struct
    pub code: Vec<u8>,
    //TODO add remaining fields (https://docs.oracle.com/javase/specs/jvms/se7/html/jvms-4.html#jvms-4.7.3)
    pub attributes: Vec<Attribute>,
}