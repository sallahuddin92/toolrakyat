use starpdf::execution::{RenderNodeId, StarNodeId};
use starpdf::syntax::object::ObjectRef;
use starpdf_app_core::selection::{FontInfo, Rect, SelectionState};

#[test]
fn test_selection_state_transitions() {
    let mut state = SelectionState::None;
    assert!(state.is_none());
    assert_eq!(state.type_name(), "None");

    // Transition to Text selection
    let star_id = StarNodeId::new("doc-1", ObjectRef::new(4, 0), "p0_s0_i0_o0").unwrap();
    let render_id = RenderNodeId::new(&star_id, "prim-0").unwrap();

    state = SelectionState::Text {
        star_node_id: star_id.clone(),
        render_node_id: Some(render_id),
        text: "Sample text".to_string(),
        bounds: Rect::new(100.0, 700.0, 150.0, 24.0),
        font_info: Some(FontInfo::new("Helvetica", 24.0)),
        is_editable: true,
    };

    assert!(state.is_some());
    assert_eq!(state.type_name(), "Text");
    assert_eq!(state.star_node_id(), Some(&star_id));
    assert_eq!(state.bounds(), Some(Rect::new(100.0, 700.0, 150.0, 24.0)));

    // Transition to Image selection
    let img_star_id = StarNodeId::new("doc-1", ObjectRef::new(12, 0), "img_0").unwrap();
    state = SelectionState::Image {
        star_node_id: img_star_id.clone(),
        render_node_id: None,
        image_ref: "Im0".to_string(),
        bounds: Rect::new(50.0, 400.0, 200.0, 200.0),
        is_shared: false,
    };
    assert_eq!(state.type_name(), "Image");
    assert_eq!(state.star_node_id(), Some(&img_star_id));

    // Transition to Vector selection
    let vec_star_id = StarNodeId::new("doc-1", ObjectRef::new(15, 0), "vec_0").unwrap();
    state = SelectionState::Vector {
        star_node_id: vec_star_id,
        render_node_id: None,
        bounds: Rect::new(10.0, 10.0, 100.0, 50.0),
        stroke: Some("#000000".to_string()),
        fill: Some("#FFFFFF".to_string()),
    };
    assert_eq!(state.type_name(), "Vector");

    // Transition to Annotation selection
    state = SelectionState::Annotation {
        annot_ref: "annot_1".to_string(),
        subtype: "Highlight".to_string(),
        contents: Some("Note here".to_string()),
        bounds: Rect::new(100.0, 680.0, 120.0, 15.0),
    };
    assert_eq!(state.type_name(), "Annotation");

    // Transition to Page selection
    state = SelectionState::Page { page_index: 0 };
    assert_eq!(state.type_name(), "Page");

    // Clear back to None
    state = SelectionState::None;
    assert!(state.is_none());
}

#[test]
fn test_selection_serde_roundtrip() {
    let star_id = StarNodeId::new("doc-42", ObjectRef::new(5, 0), "p1_s0_i2_o1").unwrap();
    let render_id = RenderNodeId::new(&star_id, "glyph-3").unwrap();

    let original = SelectionState::Text {
        star_node_id: star_id,
        render_node_id: Some(render_id),
        text: "Serialized Text".to_string(),
        bounds: Rect::new(50.0, 500.0, 200.0, 18.0),
        font_info: Some(FontInfo::new("Times-Roman", 18.0)),
        is_editable: true,
    };

    let serialized = serde_json::to_string(&original).expect("Serialize selection");
    let deserialized: SelectionState =
        serde_json::from_str(&serialized).expect("Deserialize selection");

    assert_eq!(original, deserialized);
}
