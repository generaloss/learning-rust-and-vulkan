pub struct Sync {
    last_time: u64
}

impl Sync {
    pub fn new() -> Self {
        Self {
            last_time: 0
        }
    }

    pub fn sync(&mut self) {
        std::thread::sleep(std::time::Duration::from_millis(16));
    }
}