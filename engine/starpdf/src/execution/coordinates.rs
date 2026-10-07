use super::{ContractRefusal, ContractResult, RefusalCode};

/// [left, bottom, right, top] in unrotated PDF default user space.
pub type PdfBox = [f64; 4];

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PageBounds {
    pub page_index: u32,
    pub rect: PdfBox,
}

impl PageBounds {
    pub fn is_valid(self) -> bool {
        valid_box(self.rect)
    }
    pub fn contains(self, other: Self) -> bool {
        self.is_valid()
            && other.is_valid()
            && self.page_index == other.page_index
            && self.rect[0] <= other.rect[0]
            && self.rect[1] <= other.rect[1]
            && self.rect[2] >= other.rect[2]
            && self.rect[3] >= other.rect[3]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PageRotation {
    R0,
    R90,
    R180,
    R270,
}

/// CSS viewport coordinates have top-left origin. Device pixels are a separate scale.
#[derive(Debug, Clone, PartialEq)]
pub struct CoordinateTransform {
    view_box: PdfBox,
    rotation: PageRotation,
    css_scale: f64,
    device_pixel_ratio: f64,
}

impl CoordinateTransform {
    pub fn new(
        media_box: PdfBox,
        crop_box: Option<PdfBox>,
        rotation: PageRotation,
        scale: f64,
        user_unit: f64,
        device_pixel_ratio: f64,
    ) -> ContractResult<Self> {
        let crop = crop_box.unwrap_or(media_box);
        let view_box = [
            media_box[0].max(crop[0]),
            media_box[1].max(crop[1]),
            media_box[2].min(crop[2]),
            media_box[3].min(crop[3]),
        ];
        let css_scale = scale * user_unit;
        if !valid_box(media_box)
            || !valid_box(crop)
            || !valid_box(view_box)
            || view_box[0] >= view_box[2]
            || view_box[1] >= view_box[3]
            || [
                scale,
                user_unit,
                device_pixel_ratio,
                css_scale,
                css_scale * device_pixel_ratio,
            ]
            .iter()
            .any(|v| !v.is_finite() || *v <= 0.0)
        {
            return Err(ContractRefusal::new(
                RefusalCode::InvalidCoordinates,
                "invalid page boxes or scale",
            ));
        }
        Ok(Self {
            view_box,
            rotation,
            css_scale,
            device_pixel_ratio,
        })
    }

    pub fn pdf_to_viewport(&self, point: [f64; 2]) -> ContractResult<[f64; 2]> {
        let [left, bottom, right, top] = self.view_box;
        let [x, y] = point;
        let result = match self.rotation {
            PageRotation::R0 => [x - left, top - y],
            PageRotation::R90 => [y - bottom, x - left],
            PageRotation::R180 => [right - x, y - bottom],
            PageRotation::R270 => [top - y, right - x],
        }
        .map(|v| v * self.css_scale);
        finite_point(point)?;
        finite_point(result)
    }

    pub fn viewport_to_pdf(&self, point: [f64; 2]) -> ContractResult<[f64; 2]> {
        finite_point(point)?;
        let [left, bottom, right, top] = self.view_box;
        let [x, y] = point.map(|v| v / self.css_scale);
        finite_point(match self.rotation {
            PageRotation::R0 => [left + x, top - y],
            PageRotation::R90 => [left + y, bottom + x],
            PageRotation::R180 => [right - x, bottom + y],
            PageRotation::R270 => [right - y, top - x],
        })
    }

    pub fn viewport_to_device(&self, point: [f64; 2]) -> ContractResult<[f64; 2]> {
        finite_point(point)?;
        finite_point(point.map(|v| v * self.device_pixel_ratio))
    }

    pub fn device_to_viewport(&self, point: [f64; 2]) -> ContractResult<[f64; 2]> {
        finite_point(point)?;
        finite_point(point.map(|v| v / self.device_pixel_ratio))
    }

    pub fn pdf_box_to_viewport(&self, rect: PdfBox) -> ContractResult<PdfBox> {
        if !valid_box(rect) {
            return Err(ContractRefusal::new(
                RefusalCode::InvalidCoordinates,
                "invalid bounds",
            ));
        }
        let points = [
            [rect[0], rect[1]],
            [rect[0], rect[3]],
            [rect[2], rect[1]],
            [rect[2], rect[3]],
        ]
        .map(|p| self.pdf_to_viewport(p));
        let mut result = [
            f64::INFINITY,
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::NEG_INFINITY,
        ];
        for point in points {
            let [x, y] = point?;
            result = [
                result[0].min(x),
                result[1].min(y),
                result[2].max(x),
                result[3].max(y),
            ];
        }
        Ok(result)
    }
}

pub(super) fn valid_box(rect: PdfBox) -> bool {
    rect.iter().all(|v| v.is_finite()) && rect[0] <= rect[2] && rect[1] <= rect[3]
}

fn finite_point(point: [f64; 2]) -> ContractResult<[f64; 2]> {
    if point.iter().all(|v| v.is_finite()) {
        Ok(point)
    } else {
        Err(ContractRefusal::new(
            RefusalCode::InvalidCoordinates,
            "nonfinite point",
        ))
    }
}
