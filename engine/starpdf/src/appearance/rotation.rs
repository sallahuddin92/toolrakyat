use crate::error::{PdfError, PdfResult};
use crate::syntax::object::{PdfObject, StreamObject};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WidgetRotation {
    Degrees0,
    Degrees90,
    Degrees180,
    Degrees270,
}

impl WidgetRotation {
    pub fn from_degrees(degrees: i64) -> PdfResult<Self> {
        match degrees.rem_euclid(360) {
            0 => Ok(Self::Degrees0),
            90 => Ok(Self::Degrees90),
            180 => Ok(Self::Degrees180),
            270 => Ok(Self::Degrees270),
            normalized => Err(PdfError::InvalidOperation(format!(
                "Unsupported widget rotation {normalized} degrees; expected 0, 90, 180, or 270"
            ))),
        }
    }

    pub const fn degrees(self) -> i64 {
        match self {
            Self::Degrees0 => 0,
            Self::Degrees90 => 90,
            Self::Degrees180 => 180,
            Self::Degrees270 => 270,
        }
    }

    pub fn layout_rect(self, rect: [f64; 4]) -> PdfResult<[f64; 4]> {
        let width = rect[2] - rect[0];
        let height = rect[3] - rect[1];
        if !width.is_finite() || !height.is_finite() || width <= 0.0 || height <= 0.0 {
            return Err(PdfError::InvalidOperation(
                "Rotated widget requires finite positive dimensions".into(),
            ));
        }
        Ok(match self {
            Self::Degrees0 | Self::Degrees180 => [0.0, 0.0, width, height],
            Self::Degrees90 | Self::Degrees270 => [0.0, 0.0, height, width],
        })
    }

    pub fn apply_to_stream(self, rect: [f64; 4], stream: &mut StreamObject) -> PdfResult<()> {
        let layout = self.layout_rect(rect)?;
        let logical_width = layout[2];
        let logical_height = layout[3];
        stream.dict.insert(
            "BBox".to_string(),
            PdfObject::Array(vec![
                PdfObject::Real(0.0),
                PdfObject::Real(0.0),
                PdfObject::Real(logical_width),
                PdfObject::Real(logical_height),
            ]),
        );
        let matrix = match self {
            Self::Degrees0 => [1.0, 0.0, 0.0, 1.0, 0.0, 0.0],
            Self::Degrees90 => [0.0, 1.0, -1.0, 0.0, logical_height, 0.0],
            Self::Degrees180 => [-1.0, 0.0, 0.0, -1.0, logical_width, logical_height],
            Self::Degrees270 => [0.0, -1.0, 1.0, 0.0, 0.0, logical_width],
        };
        stream.dict.insert(
            "Matrix".to_string(),
            PdfObject::Array(matrix.into_iter().map(PdfObject::Real).collect()),
        );
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;

    fn empty_stream() -> StreamObject {
        StreamObject {
            dict: BTreeMap::default(),
            stream_offset: 0,
            stream_length: 0,
            data: Vec::new(),
        }
    }

    #[test]
    fn common_widget_rotations_produce_exact_bbox_and_matrix() {
        let cases = [
            (
                0,
                vec![1.0, 0.0, 0.0, 1.0, 0.0, 0.0],
                vec![0.0, 0.0, 120.0, 30.0],
            ),
            (
                90,
                vec![0.0, 1.0, -1.0, 0.0, 120.0, 0.0],
                vec![0.0, 0.0, 30.0, 120.0],
            ),
            (
                180,
                vec![-1.0, 0.0, 0.0, -1.0, 120.0, 30.0],
                vec![0.0, 0.0, 120.0, 30.0],
            ),
            (
                270,
                vec![0.0, -1.0, 1.0, 0.0, 0.0, 30.0],
                vec![0.0, 0.0, 30.0, 120.0],
            ),
        ];
        for (degrees, expected_matrix, expected_bbox) in cases {
            let rotation = WidgetRotation::from_degrees(degrees)
                .unwrap_or_else(|error| panic!("rotation failed: {error}"));
            let mut stream = empty_stream();
            rotation
                .apply_to_stream([10.0, 20.0, 130.0, 50.0], &mut stream)
                .unwrap_or_else(|error| panic!("matrix construction failed: {error}"));
            let matrix = stream
                .dict
                .get("Matrix")
                .and_then(PdfObject::as_array)
                .unwrap_or_else(|| panic!("matrix missing"));
            let bbox = stream
                .dict
                .get("BBox")
                .and_then(PdfObject::as_array)
                .unwrap_or_else(|| panic!("bbox missing"));
            let numbers = |items: &[PdfObject]| {
                items
                    .iter()
                    .map(|item| match item {
                        PdfObject::Integer(value) => *value as f64,
                        PdfObject::Real(value) => *value,
                        _ => panic!("rotation matrix entry is not numeric"),
                    })
                    .collect::<Vec<_>>()
            };
            assert_eq!(numbers(matrix), expected_matrix);
            assert_eq!(numbers(bbox), expected_bbox);
        }
    }

    #[test]
    fn arbitrary_and_non_finite_widget_geometry_is_refused() {
        assert!(WidgetRotation::from_degrees(45).is_err());
        assert!(WidgetRotation::Degrees90
            .layout_rect([0.0, 0.0, f64::NAN, 10.0])
            .is_err());
    }
}
