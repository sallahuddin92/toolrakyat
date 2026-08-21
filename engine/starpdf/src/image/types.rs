use crate::syntax::object::ObjectRef;

/// Detailed metadata describing a discovered Image XObject in a PDF content stream.
#[derive(Debug, Clone, PartialEq)]
pub struct ImageXObjectInfo {
    /// Unique structural identifier: `img_p{page}_s{stream}_i{instr}_{name}`
    pub image_id: String,
    /// 0-indexed page in document
    pub page_index: usize,
    /// 0-indexed content stream in page (0 for single stream)
    pub stream_index: usize,
    /// 0-indexed instruction index of the `Do` operator
    pub instruction_index: usize,
    /// Resource name in `/Resources /XObject` dictionary (e.g. "Im1")
    pub resource_name: String,
    /// Indirect object reference of the image stream
    pub object_ref: ObjectRef,
    /// Pixel width of image from `/Width`
    pub width: u32,
    /// Pixel height of image from `/Height`
    pub height: u32,
    /// Color space string (e.g. "DeviceRGB", "DeviceGray", "DeviceCMYK", "Indexed")
    pub color_space: String,
    /// Bits per component (typically 8, or 1, 2, 4, 16)
    pub bits_per_component: u32,
    /// Compression filter name (e.g. "DCTDecode", "FlateDecode", or None)
    pub filter: Option<String>,
    /// Effective transformation matrix at `Do` invocation `[a, b, c, d, e, f]`
    pub transform: [f64; 6],
    /// Computed bounding box on page `[x_min, y_min, x_max, y_max]` in points
    pub rect: [f64; 4],
    /// True if the image is nested inside a Form XObject (`/Subtype /Form`)
    pub is_nested_form: bool,
    /// True if this Image XObject is referenced by multiple pages or multiple resources
    pub is_shared: bool,
}

/// Encoding format for an image to be inserted or replaced.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImageFormat {
    /// Standard JPEG / JFIF stream (encoded with `/Filter /DCTDecode`)
    Jpeg,
    /// Raw uncompressed or flate-compressed raster pixels
    Flate {
        color_space: String,
        width: u32,
        height: u32,
        bits_per_component: u32,
    },
    /// Automatically determine format from magic bytes (JPEG `0xFF 0xD8 0xFF` or PNG)
    AutoDetect,
}

/// Parameters for replacing an existing image XObject.
#[derive(Debug, Clone, PartialEq)]
pub struct ReplaceImageSpec {
    pub page_index: usize,
    pub image_id: String,
    pub new_image_bytes: Vec<u8>,
    pub format: ImageFormat,
    /// If true and the image object is shared across pages, clones the object to prevent aliasing.
    pub clone_if_shared: bool,
}

/// Parameters for inserting a new image onto a page.
#[derive(Debug, Clone, PartialEq)]
pub struct AddImageSpec {
    pub page_index: usize,
    pub image_bytes: Vec<u8>,
    pub format: ImageFormat,
    /// X coordinate on page in points
    pub x: f64,
    /// Y coordinate on page in points
    pub y: f64,
    /// Display width in points
    pub width: f64,
    /// Display height in points
    pub height: f64,
}

/// Parameters for removing an existing image from a page.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoveImageSpec {
    pub page_index: usize,
    pub image_id: String,
}
