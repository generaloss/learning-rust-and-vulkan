
pub trait AppAdapter {

    fn init(&mut self);
    fn render(&mut self);
    fn resize(&mut self, width: u32, height: u32);
    fn shutdown(&mut self);

}