use starpdf::document::ObjectStore;
use starpdf::io::ByteSource;
use starpdf::syntax::object::{ObjectRef, PdfObject};
use starpdf::xref::XrefResolver;

#[test]
fn test_lazy_object_resolution_and_cache_metrics() {
    let pdf = b"%PDF-1.7\n1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n2 0 obj\n<< /Type /Pages /Count 1 /Kids [3 0 R] >>\nendobj\n3 0 obj\n<< /Type /Page /Parent 2 0 R >>\nendobj\nxref\n0 4\n0000000000 65535 f \n0000000009 00000 n \n0000000057 00000 n \n0000000114 00000 n \ntrailer\n<< /Size 4 /Root 1 0 R >>\nstartxref\n162\n%%EOF";
    let source = ByteSource::new(pdf);
    let table = XrefResolver::load_xref_and_trailer(source).unwrap();
    let mut store = ObjectStore::new(source, table);

    assert_eq!(store.metrics().objects_known, 4);
    assert_eq!(store.metrics().objects_resolved, 0);
    assert_eq!(store.metrics().cache_hits, 0);

    // Resolve object 1
    let obj1 = store.resolve(ObjectRef::new(1, 0)).unwrap().clone();
    assert_eq!(
        obj1.as_dict().unwrap().get("Type").unwrap().as_name(),
        Some("Catalog")
    );
    assert_eq!(store.metrics().objects_resolved, 1);
    assert_eq!(store.metrics().cache_hits, 0);

    // Re-resolve object 1 (should hit cache)
    let _ = store.resolve(ObjectRef::new(1, 0)).unwrap();
    assert_eq!(store.metrics().objects_resolved, 1);
    assert_eq!(store.metrics().cache_hits, 1);

    // Resolve object 2
    let obj2 = store.resolve(ObjectRef::new(2, 0)).unwrap().clone();
    assert_eq!(
        obj2.as_dict().unwrap().get("Type").unwrap().as_name(),
        Some("Pages")
    );
    assert_eq!(store.metrics().objects_resolved, 2);

    // Resolve object 3
    let obj3 = store.resolve(ObjectRef::new(3, 0)).unwrap().clone();
    assert_eq!(
        obj3.as_dict().unwrap().get("Type").unwrap().as_name(),
        Some("Page")
    );
    assert_eq!(store.metrics().objects_resolved, 3);
}

#[test]
fn test_resolve_direct_vs_indirect_object() {
    let pdf = b"%PDF-1.7\n1 0 obj\n42\nendobj\nxref\n0 2\n0000000000 65535 f \n0000000009 00000 n \ntrailer\n<< /Size 2 /Root 1 0 R >>\nstartxref\n23\n%%EOF";
    let source = ByteSource::new(pdf);
    let xref_offset = source.find_from(0, b"xref").unwrap();
    let table = starpdf::xref::XrefResolver::parse_xref_table(source, xref_offset as u64).unwrap();
    let mut store = ObjectStore::new(source, table);

    // Direct object passes through
    let direct = PdfObject::Integer(99);
    let res = store.resolve_object(&direct).unwrap();
    assert_eq!(res, PdfObject::Integer(99));

    // Reference resolves
    let reference = PdfObject::Reference(ObjectRef::new(1, 0));
    let res2 = store.resolve_object(&reference).unwrap();
    assert_eq!(res2, PdfObject::Integer(42));
}

#[test]
fn test_object_ref_display() {
    let r = ObjectRef::new(123, 45);
    assert_eq!(format!("{r}"), "123 45 R");
}
