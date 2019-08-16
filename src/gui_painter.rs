pub trait GUIPainter {
    fn start(&mut self);
    fn draw_rect(&mut self, color: [f32; 4], pos: [f32; 2], size: [f32; 2], filled: bool, thickness: f32);
    fn draw_text(&mut self, color: [f32; 4], pos: [f32; 2], size: f32, text: String);
    fn draw_lines(&mut self, color: [f32; 4], pos: [f32; 2], points: &[[f32; 2]], filled: bool, thickness: f32);
    fn show(&mut self);

    fn set_offs(&mut self, offs: (f32, f32));
    fn get_offs(&mut self) -> (f32, f32);
    fn set_area_size(&mut self, area: (f32, f32));
    fn get_area_size(&mut self) -> (f32, f32);
}
