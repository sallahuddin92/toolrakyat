use starpdf::font::sfnt::{
    HeadTable, HheaTable, HmtxTable, MaxpTable, SfntCmapTable, SfntFont, TableDirectory,
};

#[test]
fn test_sfnt_table_directory_and_head() {
    // Construct minimal SFNT header + head table
    let mut sfnt_data = Vec::new();
    // Offset table: sfnt version (0x00010000), numTables: 1, searchRange: 16, entrySelector: 0, rangeShift: 0
    sfnt_data.extend_from_slice(&[0x00, 0x01, 0x00, 0x00]);
    sfnt_data.extend_from_slice(&[0x00, 0x01]); // 1 table
    sfnt_data.extend_from_slice(&[0x00, 0x10, 0x00, 0x00, 0x00, 0x00]);

    // Table record for 'head'
    let head_offset = 12 + 16;
    let head_length = 54;
    sfnt_data.extend_from_slice(b"head");
    sfnt_data.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]); // checksum
    sfnt_data.extend_from_slice(&(head_offset as u32).to_be_bytes());
    sfnt_data.extend_from_slice(&(head_length as u32).to_be_bytes());

    // Head table content (54 bytes)
    let mut head_bytes = vec![0u8; 54];
    // unitsPerEm at byte 18: 2048 (0x0800)
    head_bytes[18] = 0x08;
    head_bytes[19] = 0x00;
    // indexToLocFormat at byte 50: 1 (long)
    head_bytes[50] = 0x00;
    head_bytes[51] = 0x01;
    sfnt_data.extend_from_slice(&head_bytes);

    let dir = TableDirectory::parse(&sfnt_data).unwrap();
    assert!(dir.tables.contains_key(b"head"));

    let head_slice = dir.get_table(b"head", &sfnt_data).unwrap();
    let head = HeadTable::parse(head_slice).unwrap();
    assert_eq!(head.units_per_em, 2048);
    assert_eq!(head.index_to_loc_format, 1);
}

#[test]
fn test_sfnt_maxp_and_hhea_and_hmtx() {
    // Maxp table: version 1.0 (0x00010000), numGlyphs: 10
    let mut maxp_bytes = vec![0u8; 32];
    maxp_bytes[4] = 0x00;
    maxp_bytes[5] = 0x0A; // 10 glyphs
    let maxp = MaxpTable::parse(&maxp_bytes).unwrap();
    assert_eq!(maxp.num_glyphs, 10);

    // Hhea table: ascender: 800, descender: -200, lineGap: 50, numberOfHMetrics: 2
    let mut hhea_bytes = vec![0u8; 36];
    hhea_bytes[4] = 0x03;
    hhea_bytes[5] = 0x20; // 800
    hhea_bytes[6] = 0xFF;
    hhea_bytes[7] = 0x38; // -200
    hhea_bytes[34] = 0x00;
    hhea_bytes[35] = 0x02; // 2 metrics
    let hhea = HheaTable::parse(&hhea_bytes).unwrap();
    assert_eq!(hhea.ascender, 800);
    assert_eq!(hhea.descender, -200);
    assert_eq!(hhea.number_of_h_metrics, 2);

    // Hmtx table: 2 metrics: (adv 600, lsb 50), (adv 800, lsb 20)
    let mut hmtx_bytes = Vec::new();
    hmtx_bytes.extend_from_slice(&600u16.to_be_bytes());
    hmtx_bytes.extend_from_slice(&50i16.to_be_bytes());
    hmtx_bytes.extend_from_slice(&800u16.to_be_bytes());
    hmtx_bytes.extend_from_slice(&20i16.to_be_bytes());

    let hmtx = HmtxTable::parse(&hmtx_bytes, 2, 10).unwrap();
    assert_eq!(hmtx.get_advance_width(0), 600);
    assert_eq!(hmtx.get_advance_width(1), 800);
    assert_eq!(hmtx.get_advance_width(5), 800); // defaults to last
}

#[test]
fn test_sfnt_cmap_format_4_parsing() {
    // Format 4 cmap table:
    // Header: version 0, numTables: 1, platform: 3 (Windows), encoding: 1 (Unicode BMP), offset: 12
    let mut cmap_data = Vec::new();
    cmap_data.extend_from_slice(&[0x00, 0x00]); // version 0
    cmap_data.extend_from_slice(&[0x00, 0x01]); // 1 table
    cmap_data.extend_from_slice(&[0x00, 0x03]); // platform 3
    cmap_data.extend_from_slice(&[0x00, 0x01]); // encoding 1
    cmap_data.extend_from_slice(&12u32.to_be_bytes()); // subtable offset = 12

    // Subtable Format 4:
    // segCount: 2 (segCountX2 = 4)
    // Seg 0: 'A' (65) -> Glyph ID 1 (delta = -64)
    // Seg 1: 0xFFFF -> 0xFFFF
    let mut subtable = Vec::new();
    subtable.extend_from_slice(&4u16.to_be_bytes()); // format 4
    subtable.extend_from_slice(&32u16.to_be_bytes()); // length
    subtable.extend_from_slice(&0u16.to_be_bytes()); // language
    subtable.extend_from_slice(&4u16.to_be_bytes()); // segCountX2 = 4
    subtable.extend_from_slice(&[0x00, 0x04, 0x00, 0x01, 0x00, 0x00]); // searchRange, entrySelector, rangeShift

    // endCode: [65, 0xFFFF]
    subtable.extend_from_slice(&65u16.to_be_bytes());
    subtable.extend_from_slice(&0xFFFFu16.to_be_bytes());
    // reservedPad: 0
    subtable.extend_from_slice(&0u16.to_be_bytes());
    // startCode: [65, 0xFFFF]
    subtable.extend_from_slice(&65u16.to_be_bytes());
    subtable.extend_from_slice(&0xFFFFu16.to_be_bytes());
    // idDelta: [-64 (0xFFC0), 1]
    subtable.extend_from_slice(&(-64i16).to_be_bytes());
    subtable.extend_from_slice(&1i16.to_be_bytes());
    // idRangeOffset: [0, 0]
    subtable.extend_from_slice(&0u16.to_be_bytes());
    subtable.extend_from_slice(&0u16.to_be_bytes());

    cmap_data.extend_from_slice(&subtable);

    let cmap = SfntCmapTable::parse(&cmap_data).unwrap();
    assert_eq!(cmap.map_char_to_glyph(65), Some(1));
    assert_eq!(cmap.map_glyph_to_char(1), Some('A'));
}

#[test]
fn test_sfnt_malformed_bytes_no_panic() {
    let junk: &[&[u8]] = &[
        b"",
        b"\x00\x01\x00\x00\x00\x05",
        b"head\x00\x00\x00\x00\xFF\xFF\xFF\xFF",
        &[0u8; 100],
    ];

    for j in junk {
        let _ = SfntFont::parse(j);
    }
}
