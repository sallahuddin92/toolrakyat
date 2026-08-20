#![no_main]
use libfuzzer_sys::fuzz_target;
use starpdf::font::sfnt::{
    HeadTable, HheaTable, HmtxTable, MaxpTable, SfntCmapTable, SfntFont, TableDirectory,
};

fuzz_target!(|data: &[u8]| {
    let _ = TableDirectory::parse(data);
    let _ = HeadTable::parse(data);
    let _ = MaxpTable::parse(data);
    let _ = HheaTable::parse(data);
    let _ = HmtxTable::parse(data, 10, 20);
    let _ = SfntCmapTable::parse(data);
    let _ = SfntFont::parse(data);
});
