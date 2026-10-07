//! Test-only subprocess boundary for the corpus runner. Never used by the engine.
use serde_json::{json, Value};
use starpdf::document::PdfDocument;
use starpdf::execution::{
    ClosureCompleteness, DocumentDelta, HashDomain, MutationEnvelope, SemanticHashInput,
    StructuralClosure, VisualClosure,
};
use starpdf::forms::FieldValue;
use starpdf::mutation::PdfChange;
use starpdf::syntax::object::ObjectRef;
use starpdf::writer::serializer::Serializer;
use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::time::Instant;

fn timed<T>(f: impl FnOnce() -> Result<T, String>) -> (Result<T, String>, u128) {
    let start = Instant::now();
    let result = f();
    (result, start.elapsed().as_micros())
}

fn snapshot(doc: &mut PdfDocument<'_>) -> BTreeMap<ObjectRef, Vec<u8>> {
    let refs: Vec<_> = doc
        .store()
        .xref()
        .entries
        .iter()
        .filter_map(|(&number, entry)| match entry {
            starpdf::xref::table::XrefEntry::InUse { generation, .. } => {
                Some(ObjectRef::new(number, *generation))
            }
            starpdf::xref::table::XrefEntry::Compressed { .. } => Some(ObjectRef::new(number, 0)),
            _ => None,
        })
        .collect();
    let mut out = BTreeMap::new();
    for reference in refs {
        if let Ok(object) = doc.store_mut().resolve(reference) {
            let mut bytes = Vec::new();
            if Serializer::write_object(&mut bytes, object).is_ok() {
                out.insert(reference, bytes);
            }
        }
    }
    out
}

fn digest(bytes: &[u8]) -> starpdf::execution::ContentHash {
    SemanticHashInput {
        domain: HashDomain::PdfObject,
        fields: BTreeMap::from([("serialized".into(), bytes.to_vec())]),
    }
    .content_hash()
    .expect("valid object digest")
}

fn run(input: &str, output: Option<&str>, edit: bool) -> Value {
    let (bytes, read_us) = timed(|| std::fs::read(input).map_err(|e| e.to_string()));
    let Ok(bytes) = bytes else {
        return json!({"error": bytes.err(), "stage": "OPEN"});
    };
    let (doc, open_us) = timed(|| PdfDocument::from_bytes(&bytes).map_err(|e| e.to_string()));
    let Ok(mut doc) = doc else {
        return json!({"error": doc.err(), "stage": "OPEN", "duration_us": {"read": read_us, "open": open_us}});
    };
    let (pages, parse_us) = timed(|| doc.page_count().map_err(|e| e.to_string()));
    let Ok(pages) = pages else {
        return json!({"error": pages.err(), "stage": "PARSE", "duration_us": {"read": read_us, "open": open_us, "parse": parse_us}});
    };
    let (first_page, first_page_us) = timed(|| {
        if pages == 0 {
            Ok(())
        } else {
            doc.page_dict(0).map(|_| ()).map_err(|e| e.to_string())
        }
    });
    if let Err(error) = first_page {
        return json!({"error": error, "stage": "FIRST_PAGE", "page_count": pages, "duration_us": {"read": read_us, "open": open_us, "parse": parse_us, "first_page": first_page_us}});
    }
    let (text, text_us) = timed(|| {
        doc.extract_all_text()
            .map(|p| p.iter().map(|x| x.plain_text()).collect::<Vec<_>>())
            .map_err(|e| e.to_string())
    });
    let Ok(text) = text else {
        return json!({"error": text.err(), "stage": "TEXT_EXTRACT", "page_count": pages, "duration_us": {"read": read_us, "open": open_us, "parse": parse_us, "first_page": first_page_us, "text": text_us}});
    };
    let mut result = json!({"page_count": pages, "text": text, "xref_status": format!("{:?}", doc.xref_status()),
        "duration_us": {"read": read_us, "open": open_us, "parse": parse_us, "first_page": first_page_us, "text": text_us}});
    if !edit {
        return result;
    }
    let (fields, fields_us) = timed(|| doc.form_fields().map_err(|e| e.to_string()));
    result["duration_us"]["fields"] = json!(fields_us);
    let Ok(fields) = fields else {
        result["error"] = json!(fields.err());
        result["stage"] = json!("EDIT");
        return result;
    };
    let Some(field) = fields
        .iter()
        .find(|f| matches!(f.value, FieldValue::Text(_)) && !f.is_read_only)
    else {
        result["edit"] = json!({"status": "UNSUPPORTED", "reason": "no writable text field"});
        return result;
    };
    let original = match &field.value {
        FieldValue::Text(s) => s.clone(),
        _ => unreachable!(),
    };
    let replacement = format!("Qualification {}", field.object_ref.number);
    let before = snapshot(&mut doc);
    let (plan, plan_us) = timed(|| {
        doc.apply_mutation(&[PdfChange::SetTextField {
            field_ref: field.object_ref,
            value: replacement.clone(),
        }])
        .map_err(|e| e.to_string())
    });
    result["duration_us"]["mutation_planning"] = json!(plan_us);
    let Ok(plan) = plan else {
        result["edit"] = json!({"status": "UNSUPPORTED", "reason": plan.err()});
        return result;
    };
    let (exported, export_us) = timed(|| doc.export_incremental(&plan).map_err(|e| e.to_string()));
    result["duration_us"]["export"] = json!(export_us);
    let Ok(exported) = exported else {
        result["error"] = json!(exported.err());
        result["stage"] = json!("EXPORT");
        return result;
    };
    if let Some(path) = output {
        if let Err(error) = std::fs::write(path, &exported) {
            result["error"] = json!(error.to_string());
            result["stage"] = json!("EXPORT");
            return result;
        }
    }
    let (reopened, reopen_us) =
        timed(|| PdfDocument::from_bytes(&exported).map_err(|e| e.to_string()));
    result["duration_us"]["reopen"] = json!(reopen_us);
    let Ok(mut reopened) = reopened else {
        result["error"] = json!(reopened.err());
        result["stage"] = json!("REOPEN");
        return result;
    };
    let after = snapshot(&mut reopened);
    let changed: Vec<_> = after
        .iter()
        .filter(|(reference, bytes)| {
            before
                .get(reference)
                .map(|old| old != *bytes)
                .unwrap_or(true)
        })
        .map(|(reference, _)| format!("{} {}", reference.number, reference.generation))
        .collect();
    let mut delta = DocumentDelta::default();
    for (reference, bytes) in &after {
        if let Some(previous) = before.get(reference) {
            if previous != bytes {
                delta.modified_objects.insert(*reference);
                delta.before_hashes.insert(*reference, digest(previous));
                delta.after_hashes.insert(*reference, digest(bytes));
            }
        } else {
            delta.added_objects.insert(*reference);
            delta.after_hashes.insert(*reference, digest(bytes));
        }
    }
    let envelope = MutationEnvelope {
        allowed_pdf_objects: plan.modified_objects.keys().copied().collect(),
        ..MutationEnvelope::default()
    };
    let structural = StructuralClosure {
        objects: delta
            .added_objects
            .union(&delta.modified_objects)
            .copied()
            .collect(),
        completeness: ClosureCompleteness::Incomplete,
        ..StructuralClosure::default()
    };
    let visual = VisualClosure {
        completeness: ClosureCompleteness::Incomplete,
        ..VisualClosure::default()
    };
    let certificate = envelope.compare(&delta, &structural, &visual);
    let outside_plan: BTreeSet<_> = delta
        .added_objects
        .union(&delta.modified_objects)
        .filter(|r| !envelope.allowed_pdf_objects.contains(r))
        .map(ToString::to_string)
        .collect();
    let (new_text, retext_us) = timed(|| {
        reopened
            .extract_all_text()
            .map(|p| p.iter().map(|x| x.plain_text()).collect::<Vec<_>>())
            .map_err(|e| e.to_string())
    });
    result["duration_us"]["reextract"] = json!(retext_us);
    let (new_fields, refields_us) = timed(|| reopened.form_fields().map_err(|e| e.to_string()));
    result["duration_us"]["reinspect_fields"] = json!(refields_us);
    result["edit"] = json!({"status": "SUCCESS", "field_ref": format!("{} {}", field.object_ref.number, field.object_ref.generation),
        "before": original, "expected_after": replacement, "actual_after": new_fields.ok().and_then(|fs| fs.into_iter().find(|f| f.object_ref == field.object_ref)).map(|f| format!("{:?}", f.value)),
        "changed_objects": changed, "plan_objects": plan.modified_objects.keys().map(|r| format!("{} {}", r.number, r.generation)).collect::<Vec<_>>(),
        "actual_document_delta": {"added_objects": delta.added_objects.iter().map(ToString::to_string).collect::<Vec<_>>(), "modified_objects": delta.modified_objects.iter().map(ToString::to_string).collect::<Vec<_>>(), "before_hashes": delta.before_hashes.iter().map(|(r,h)| (r.to_string(), h.to_hex())).collect::<BTreeMap<_,_>>(), "after_hashes": delta.after_hashes.iter().map(|(r,h)| (r.to_string(), h.to_hex())).collect::<BTreeMap<_,_>>()},
        "mutation_certificate": {"classification": format!("{:?}", certificate.classification).to_uppercase(), "refusal": certificate.refusal.map(|r| format!("{:?}: {}", r.code, r.reason)), "unexplained_changes": certificate.unexplained_changes, "outside_plan": outside_plan},
        "reopened_page_count": reopened.page_count().ok(), "reopened_text": new_text.ok(), "output_bytes": exported.len()});
    result
}

fn main() {
    let args: Vec<_> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("usage: qualification_probe INPUT [OUTPUT]");
        std::process::exit(2);
    }
    let value = run(&args[1], args.get(2).map(String::as_str), args.len() > 2);
    println!("{}", value);
}
