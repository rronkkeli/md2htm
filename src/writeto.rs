pub trait WtiteTo {
    fn write(&mut self, bytes: &[u8]);
}

impl WtiteTo for Vec<u8> {
    fn write(&mut self, bytes: &[u8]) {
        for b in bytes {
            self.push(*b);
        }
    }
}
