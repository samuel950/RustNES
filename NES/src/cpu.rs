#[derive(Debug)]
#[allow(non_camel_case_types)]
pub enum AddressingMode {
    Immediate,
    ZeroPage,
    ZeroPage_X,
    ZeroPage_Y,
    Absolute,
    Absolute_X,
    Absolute_Y,
    Indirect_X,
    Indirect_Y,
    NotSupported,
}
pub struct CPU {
    pub register_a: u8,
    pub register_x: u8,
    pub register_y: u8,
    pub status: u8,
    pub program_counter: u16,
    memory: [u8; 0xFFFF],
}
/*    7             6               5              4          3         2        1        0
|  negative  |  overflow  |  unused always 1  |  break  |  decimal  |  IRQ  |  zero  |  Carry  | --> processor status flags
glossary
cycles - how long an instruction should roughly take.
bytes - tells you if there are additional paramaters for command ie 2 bytes is 1 extra param.
address - points to a 1 byte cell (8bits) ie think how that relates to getting 16bits of data.
program counter - the current memory address. 2 bytes increment once, 3 bytes increment twice etc.
 */
impl CPU {
    pub fn new() -> Self {
        CPU {
            register_a: 0,
            register_x: 0,
            register_y: 0,
            status: 0b0010_0000,
            program_counter: 0,
        }
    }

    fn mem_read_u16(&mut self, pos: u16) -> u16 {
        let lo = self.mem_read(pos) as u16;
        let hi = self.mem_read(pos + 1) as u16; //visit the next cell to grab the last 8 bits of data.
        (hi << 8) | lo
    }
    fn mem_write_u16(&mut self, pos: u16, data: u16) {
        let hi = (data >> 8) as u8; //basically shifts hi 8 bits to the right to allow truncating
        let lo = (data & 0xff) as u8; //zero out 8 hi bits to allow truncating
        self.mem_write(pos, lo);
        self.mem_write(pos + 1, hi);
    }
    fn mem_read(&self, addr: u16) -> u8 {
        self.memory[addr as usize]
    }
    fn mem_write(&mut self, addr: u16, data: u8) {
        self.memory[addr as usize] = data;
    }
    pub fn reset(&mut self) {
        self.register_a = 0;
        self.register_x = 0;
        self.register_y = 0;
        self.status = 0b0010_0000;
        self.program_counter = self.mem_read_u16(0xFFFC);
    }
    pub fn load(&mut self, program: Vec<u8>) {
        self.memory[0x8000..(0x8000 + program.len())].copy_from_slice(&program[..]);
        self.mem_write_u16(0xFFFC, 0x8000);
    }
    pub fn load_and_run(&mut self, program: Vec<u8>) {
        self.load(program);
        self.reset();
        self.run();
    }
    fn get_operand_addressing_mode(&self, mode: &AddressingMode) -> u16 {
        match mode {
            AddressingMode::Immediate => self.program_counter,
            AddressingMode::ZeroPage => self.mem_read(self.program_counter) as u16,
            AddressingMode::ZeroPage_X => {
                let zp_addr = self.mem_read(self.program_counter);
                zp_addr.wrapping_add(self.register_x) as u16
            }
            AddressingMode::ZeroPage_Y => {
                let zp_addr = self.mem_read(self.program_counter);
                zp_addr.wrapping_add(self.register_y) as u16
            }
            AddressingMode::Absolute => self.mem_read_u16(self.program_counter),
            AddressingMode::Absolute_X => {
                let abs_addr: u16 = self.mem_read_u16(self.program_counter);
                abs_addr.wrapping_add(self.register_x as u16)
            }
            AddressingMode::Absolute_Y => {
                let abs_addr: u16 = self.mem_read_u16(self.program_counter);
                abs_addr.wrapping_add(self.register_y as u16)
            }
            AddressingMode::Indirect_X => {
                let indr_addr: u8 = self.mem_read(self.program_counter);
                let ptr: u8 = indr_addr.wrapping_add(self.register_x);
                let lo = self.mem_read(ptr as u16) as u16;
                let hi = self.mem_read(ptr.wrapping_add(1) as u16) as u16;
                (hi << 8) | lo //dont need to 0 out first 8 bits because lo is originally 8 bits anyways.
            }
            AddressingMode::Indirect_Y => {
                let indr_addr: u8 = self.mem_read(self.program_counter); //starting point of a 16bit address.
                let lo = self.mem_read(indr_addr as u16) as u16;
                let hi = self.mem_read(indr_addr.wrapping_add(1) as u16) as u16;
                let ptr: u16 = (hi << 8) | lo;
                ptr.wrapping_add(self.register_y as u16)
            }
            AddressingMode::NotSupported => {
                panic!("Addressing mode {:?} is not supported!", mode);
            }
        }
    }
    pub fn run(&mut self) {
        loop {
            let opscode = self.mem_read(self.program_counter);
            self.program_counter += 1;
            match opscode {
                0xE8 => {
                    //INX
                    self.inx();
                }
                0xC8 => {
                    //INY
                    self.iny();
                }
                0xA9 => {
                    //LDA-I
                    self.lda(&AddressingMode::Immediate);
                    self.program_counter += 1;
                }
                0xA2 => {
                    //LDX-I
                    self.ldx(&AddressingMode::Immediate);
                    self.program_counter += 1;
                }
                0xA0 => {
                    //LDY-I
                    self.ldy(&AddressingMode::Immediate);
                    self.program_counter += 1;
                }
                0xAA => {
                    //TAX
                    self.tax();
                }
                0xA8 => {
                    //TAY
                    self.tay();
                }
                0x00 => {
                    //brk
                    return;
                }
                _ => todo!(),
            }
        }
    }
    /*
    Cpu instruction functions start
     */
    fn inx(&mut self) {
        self.register_x = self.register_x.wrapping_add(1);
        self.set_zn_flags_v1(self.register_x);
    }
    fn iny(&mut self) {
        self.register_y = self.register_y.wrapping_add(1);
        self.set_zn_flags_v1(self.register_y);
    }
    fn lda(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_addressing_mode(mode);
        self.register_a = self.mem_read(addr);
        self.set_zn_flags_v1(self.register_a);
    }
    fn ldx(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_addressing_mode(mode);
        self.register_x = self.mem_read(addr);
        self.set_zn_flags_v1(self.register_x);
    }
    fn ldy(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_addressing_mode(mode);
        self.register_y = self.mem_read(addr);
        self.set_zn_flags_v1(self.register_y);
    }
    fn tax(&mut self) {
        self.register_x = self.register_a;
        self.set_zn_flags_v1(self.register_x);
    }
    fn tay(&mut self) {
        self.register_y = self.register_a;
        self.set_zn_flags_v1(self.register_y);
    }
    fn set_zn_flags_v1(&mut self, reg: u8) {
        //z->set if ? = 0|n->set if bit 7 of ? is set
        if reg == 0 {
            //then set zero flag
            self.status = self.status | 0b0000_0010;
        } else {
            self.status = self.status & 0b1111_1101; //sets z    bit to 0 and preserves rest of bits
        }
        if reg & 0b1000_0000 != 0 {
            //check if bit in 7th pos is set, ie if bit in pos 7 is 1 than calculation should not equal 0
            self.status = self.status | 0b1000_0000;
        } else {
            self.status = self.status & 0b0111_1111; //sets n flag to 0 and preserves rest of bits
        }
    }

    /*pub fn interpret(&mut self, program: Vec<u8>) {
        self.program_counter = 0;
        loop {
            let opscode = self.mem_read(self.program_counter);
            //let opscode = program[self.program_counter as usize];
            self.program_counter += 1;
            match opscode {
                0xE8 => {
                    //INX
                    self.inx();
                }
                0xC8 => {
                    //INY
                    self.iny();
                }
                0xA9 => {
                    //LDA-I
                    let param = program[self.program_counter as usize];
                    self.program_counter += 1;
                    self.lda(param);
                }
                0xA2 => {
                    //LDX-I
                    let param = program[self.program_counter as usize];
                    self.program_counter += 1;
                    self.ldx(param);
                }
                0xA0 => {
                    //LDY-I
                    let param = program[self.program_counter as usize];
                    self.program_counter += 1;
                    self.ldy(param);
                }
                0xAA => {
                    //TAX
                    self.tax();
                }
                0xA8 => {
                    //TAY
                    self.tay();
                }
                0x00 => {
                    //brk
                    return;
                }
                _ => todo!(),
            }
        }
    }*/
}
