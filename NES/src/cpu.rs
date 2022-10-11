pub struct CPU {
    pub register_a: u8,
    pub register_x: u8,
    pub register_y: u8,
    pub status: u8,
    pub program_counter: u16,
}

impl CPU {
    pub fn new() -> Self {
        CPU {
            register_a: 0,
            register_x: 0,
            register_y: 0,
            status: 0,
            program_counter: 0,
        }
    }

    fn lda(&mut self, value: u8) {}

    pub fn interpret(&mut self, program: Vec<u8>) {
        self.program_counter = 0;
        loop {
            let opscode = program[self.program_counter as usize];
            self.program_counter += 1;
            match opscode {
                0xA9 => {}
            }
        }
    }
}
