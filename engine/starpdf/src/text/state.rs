use crate::text::matrix::Matrix2D;

#[derive(Debug, Clone)]
pub struct GraphicsState {
    pub ctm: Matrix2D,
    pub fill_color: [f64; 3],
}

impl Default for GraphicsState {
    fn default() -> Self {
        Self {
            ctm: Matrix2D::identity(),
            fill_color: [0.0, 0.0, 0.0],
        }
    }
}

#[derive(Debug, Clone)]
pub struct TextState {
    pub tm: Matrix2D,
    pub tlm: Matrix2D,
    pub font_name: Option<String>,
    pub font_size: f64,
    pub char_spacing: f64,
    pub word_spacing: f64,
    pub horizontal_scaling: f64,
    pub leading: f64,
    pub text_rise: f64,
    pub render_mode: i32,
    pub in_text_object: bool,
}

#[derive(Debug, Clone)]
pub struct TextStateParameters {
    font_name: Option<String>,
    font_size: f64,
    char_spacing: f64,
    word_spacing: f64,
    horizontal_scaling: f64,
    leading: f64,
    text_rise: f64,
    render_mode: i32,
}

impl Default for TextState {
    fn default() -> Self {
        Self {
            tm: Matrix2D::identity(),
            tlm: Matrix2D::identity(),
            font_name: None,
            font_size: 12.0,
            char_spacing: 0.0,
            word_spacing: 0.0,
            horizontal_scaling: 100.0,
            leading: 0.0,
            text_rise: 0.0,
            render_mode: 0,
            in_text_object: false,
        }
    }
}

impl TextState {
    pub fn save_parameters(&self) -> TextStateParameters {
        TextStateParameters {
            font_name: self.font_name.clone(),
            font_size: self.font_size,
            char_spacing: self.char_spacing,
            word_spacing: self.word_spacing,
            horizontal_scaling: self.horizontal_scaling,
            leading: self.leading,
            text_rise: self.text_rise,
            render_mode: self.render_mode,
        }
    }

    pub fn restore_parameters(&mut self, saved: TextStateParameters) {
        self.font_name = saved.font_name;
        self.font_size = saved.font_size;
        self.char_spacing = saved.char_spacing;
        self.word_spacing = saved.word_spacing;
        self.horizontal_scaling = saved.horizontal_scaling;
        self.leading = saved.leading;
        self.text_rise = saved.text_rise;
        self.render_mode = saved.render_mode;
    }

    pub fn begin_text(&mut self) {
        self.tm = Matrix2D::identity();
        self.tlm = Matrix2D::identity();
        self.in_text_object = true;
    }

    pub fn end_text(&mut self) {
        self.in_text_object = false;
    }

    pub fn set_font(&mut self, font_name: &str, size: f64) {
        self.font_name = Some(font_name.to_string());
        self.font_size = size;
    }

    pub fn set_matrix(&mut self, a: f64, b: f64, c: f64, d: f64, e: f64, f: f64) {
        self.tm = Matrix2D::new(a, b, c, d, e, f);
        self.tlm = self.tm;
    }

    pub fn move_text_position(&mut self, tx: f64, ty: f64) {
        let t_matrix = Matrix2D::translation(tx, ty);
        self.tlm = t_matrix.multiply(&self.tlm);
        self.tm = self.tlm;
    }

    pub fn move_to_next_line(&mut self) {
        self.move_text_position(0.0, -self.leading);
    }
}
