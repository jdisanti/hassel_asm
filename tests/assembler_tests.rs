extern crate hassel_asm;

use hassel_asm::Assembler;

#[test]
fn org_and_pad_test() {
    let program = String::from_utf8(include_bytes!("./org_and_pad.s").to_vec()).unwrap();
    let mut assembler = Assembler::new();
    assembler.parse_unit("org_and_pad.s", &program).unwrap();
    let result = assembler.assemble().unwrap();

    let expected_bytes = include_bytes!("./org_and_pad.rom").to_vec();
    assert_eq!(expected_bytes, result.bytes.unwrap());
}