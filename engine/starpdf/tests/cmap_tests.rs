use starpdf::font::UnicodeCMap;

#[test]
fn test_cmap_bfchar_parsing() {
    let cmap_data = b"
/CIDInit /ProcSet findresource begin
12 dict begin
begincmap
/CIDSystemInfo << /Registry (Adobe) /Ordering (UCS) /Supplement 0 >> def
/CMapName /Custom-ToUnicode def
/CMapType 2 def
1 begincodespacerange
<0001> <FFFF>
endcodespacerange
3 beginbfchar
<0001> <0041>
<0002> <0042>
<0003> <00660069>
endbfchar
endcmap
CMapName currentdict /CMap defineresource pop
end
end
";
    let cmap = UnicodeCMap::parse(cmap_data).unwrap();
    assert_eq!(cmap.lookup(1), Some("A"));
    assert_eq!(cmap.lookup(2), Some("B"));
    assert_eq!(cmap.lookup(3), Some("fi")); // Ligature
    assert_eq!(cmap.lookup(4), None);
}

#[test]
fn test_cmap_bfrange_sequential_parsing() {
    let cmap_data = b"
1 beginbfrange
<0001> <0003> <0041>
endbfrange
";
    let cmap = UnicodeCMap::parse(cmap_data).unwrap();
    assert_eq!(cmap.lookup(1), Some("A"));
    assert_eq!(cmap.lookup(2), Some("B"));
    assert_eq!(cmap.lookup(3), Some("C"));
}

#[test]
fn test_cmap_bfrange_array_parsing() {
    let cmap_data = b"
1 beginbfrange
<0001> <0003> [ <0058> <0059> <005A> ]
endbfrange
";
    let cmap = UnicodeCMap::parse(cmap_data).unwrap();
    assert_eq!(cmap.lookup(1), Some("X"));
    assert_eq!(cmap.lookup(2), Some("Y"));
    assert_eq!(cmap.lookup(3), Some("Z"));
}

#[test]
fn test_cmap_malformed_input_no_panic() {
    let junk_cmaps: &[&[u8]] = &[
        b"",
        b"beginbfchar <0001>",
        b"beginbfrange <0005> <0001> <0041> endbfrange",
        b"beginbfchar <ZZZZ> <0041> endbfchar",
    ];

    for data in junk_cmaps {
        let _ = UnicodeCMap::parse(data);
    }
}
