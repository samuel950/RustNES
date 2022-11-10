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
pub enum Flag {
    Carry,
    Zero,
    IRQ,
    Dec,
    Break,
    Overflow,
    Negative,
}
pub struct CPU {
    pub register_a: u8,
    pub register_x: u8,
    pub register_y: u8,
    pub status: u8,
    pub stack_ptr: u8,
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
-In 2's complement, to make positive number negative, invert bits and add 1.
 */
impl CPU {
    pub fn new() -> Self {
        CPU {
            register_a: 0,
            register_x: 0,
            register_y: 0,
            status: 0b0010_0000,
            stack_ptr: 0xff,
            program_counter: 0,
            memory: [0; 0xFFFF],
        }
    }

    fn mem_read_u16(&self, pos: u16) -> u16 {
        let lo = self.mem_read(pos) as u16;
        //let hi = self.mem_read(pos + 1) as u16; //visit the next cell to grab the last 8 bits of data.
        let hi = self.mem_read(pos.wrapping_add(1)) as u16;
        (hi << 8) | lo
    }
    fn mem_write_u16(&mut self, pos: u16, data: u16) {
        let hi = (data >> 8) as u8; //basically shifts hi 8 bits to the right to allow truncating
        let lo = (data & 0xff) as u8; //zero out 8 hi bits to allow truncating
        self.mem_write(pos, lo);
        //self.mem_write(pos + 1, hi);
        self.mem_write(pos.wrapping_add(1), hi);
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
    fn is_negative(&self, target: u8) -> bool {
        target & 0b1000_0000 != 0
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
                /*let lo = self.mem_read(ptr as u16) as u16;
                let hi = self.mem_read(ptr.wrapping_add(1) as u16) as u16;
                (hi << 8) | lo //dont need to 0 out first 8 bits because lo is originally 8 bits anyways.*/
                self.mem_read_u16(ptr as u16)
            }
            AddressingMode::Indirect_Y => {
                let indr_addr: u8 = self.mem_read(self.program_counter); //starting point of a 16bit address.

                /*let lo = self.mem_read(indr_addr as u16) as u16;
                let hi = self.mem_read(indr_addr.wrapping_add(1) as u16) as u16;
                let ptr: u16 = (hi << 8) | lo;*/
                let ptr = self.mem_read_u16(indr_addr as u16);
                ptr.wrapping_add(self.register_y as u16)
            }
            AddressingMode::NotSupported => {
                panic!("Addressing mode {:?} is not supported!", mode);
            }
        }
    }
    /*
     * * * * * * * * * * Flag functions start here * * * * * * * * * *
     */
    fn enable_flag(&mut self, flag: &Flag) {
        match flag {
            Flag::Carry => self.status = self.status | 0b0000_0001,
            Flag::Zero => self.status = self.status | 0b0000_0010,
            Flag::IRQ => self.status = self.status | 0b0000_0100,
            Flag::Dec => self.status = self.status | 0b0000_1000,
            Flag::Break => self.status = self.status | 0b0001_0000,
            Flag::Overflow => self.status = self.status | 0b0100_0000,
            Flag::Negative => self.status = self.status | 0b1000_0000,
        }
    }
    fn disable_flag(&mut self, flag: &Flag) {
        match flag {
            Flag::Carry => self.status = self.status & 0b1111_1110,
            Flag::Zero => self.status = self.status & 0b1111_1101,
            Flag::IRQ => self.status = self.status & 0b1111_1011,
            Flag::Dec => self.status = self.status & 0b1111_0111,
            Flag::Break => self.status = self.status & 0b1110_1111,
            Flag::Overflow => self.status = self.status & 0b1011_1111,
            Flag::Negative => self.status = self.status & 0b0111_1111,
        }
    }
    fn get_flag_status(&self, flag: &Flag) -> bool {
        match flag {
            Flag::Carry => self.status & 0b0000_0001 != 0,
            Flag::Zero => self.status & 0b0000_0010 != 0,
            Flag::IRQ => self.status & 0b0000_0100 != 0,
            Flag::Dec => self.status & 0b0000_1000 != 0,
            Flag::Break => self.status & 0b0001_0000 != 0,
            Flag::Overflow => self.status & 0b0100_0000 != 0,
            Flag::Negative => self.status & 0b1000_0000 != 0,
        }
    }
    fn set_zn_flags_v1(&mut self, reg: u8) {
        //z->set if ? = 0 and n->set if bit 7 of ? is set
        if reg == 0 {
            //then set zero flag
            self.enable_flag(&Flag::Zero);
        } else {
            self.disable_flag(&Flag::Zero); //sets zero bit to 0 and preserves rest of bits
        }
        if self.is_negative(reg) {
            //check if bit in 7th pos is set, ie if bit in pos 7 is 1 than calculation should not equal 0
            self.enable_flag(&Flag::Negative);
        } else {
            self.disable_flag(&Flag::Negative); //sets n flag to 0 and preserves rest of bits
        }
    }
    /*
     * * * * * * * * * * Cpu instruction functions start here * * * * * * * * * *
     */
    fn adc(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_addressing_mode(mode);
        self.add(self.mem_read(addr));
        self.set_zn_flags_v1(self.register_a);
    }
    fn add(&mut self, addend: u8) {
        let status_carry: u16 = if self.get_flag_status(&Flag::Carry) {
            1
        } else {
            0
        };
        let sum: u16 = self.register_a as u16 + addend as u16 + status_carry;
        //let carry = sum > 0b1111_1111; //cant be represented as unsigned ie > 255 then carry
        if sum > 0b1111_1111 {
            self.enable_flag(&Flag::Carry);
        } else {
            self.disable_flag(&Flag::Carry);
        }
        let result = sum as u8;
        /*
         * With XOR were really just trying to check if the most significant
         * bit of the original addends and the result are different. If say
         * both of them are different from the result we would for example
         * get 1 & 1 & 1 (with leftmost bit) which means overflow happened.
         * If the the signs of both addends are different from the result
         * then overflow occurs. If one or both of the addends has a
         * different sign from the result then overflow did not occur.
         * IE 1 & 0 & 1 or 0 & 0 & 1 etc should terminate to 0.
         */
        if (self.register_a ^ result) & (addend ^ result) & 0b1000_0000 == 0 {
            //short hand for checking signs of addends and results
            self.disable_flag(&Flag::Overflow);
        } else {
            self.enable_flag(&Flag::Overflow);
        }
        self.register_a = result;
    }
    fn and(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_addressing_mode(mode);
        self.register_a = self.register_a & self.mem_read(addr);
        self.set_zn_flags_v1(self.register_a);
    }
    fn asl_accumulator(&mut self) {
        let operand = self.register_a;
        if operand > 0b0111_1111 {
            //carry threshold is greater than 255. so if operand is strictly greater than 127(times 2), then need to set carry.
            self.enable_flag(&Flag::Carry);
        } else {
            self.disable_flag(&Flag::Carry);
        }
        self.register_a = operand << 1;
        self.set_zn_flags_v1(self.register_a);
    }
    fn asl(&mut self, mode: &AddressingMode) {
        //same effect as multiplying by 2
        let addr = self.get_operand_addressing_mode(mode);
        let mut operand = self.mem_read(addr);
        if operand > 0b0111_1111 {
            //carry threshold is greater than 255. so if operand is strictly greater than 127(times 2), then need to set carry.
            self.enable_flag(&Flag::Carry);
        } else {
            self.disable_flag(&Flag::Carry);
        }
        operand = operand << 1;
        self.mem_write(addr, operand);
        self.set_zn_flags_v1(operand);
    }
    fn bit(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_addressing_mode(mode);
        let operand = self.register_a & self.mem_read(addr);
        if operand == 0 {
            self.enable_flag(&Flag::Zero);
        } else {
            self.disable_flag(&Flag::Zero);
        }
        if operand & 0b1000_0000 != 0 {
            self.enable_flag(&Flag::Negative);
        } else {
            self.disable_flag(&Flag::Negative);
        }
        if operand & 0b0100_0000 != 0 {
            self.enable_flag(&Flag::Overflow);
        } else {
            self.disable_flag(&Flag::Overflow);
        }
    }
    fn branch_set(&mut self, flag: &Flag) {
        if self.get_flag_status(flag) {
            let displacement = self.mem_read(self.program_counter) as i8;
            self.program_counter = self
                .program_counter
                .wrapping_add(1)
                .wrapping_add(displacement as u16);
        }
    }
    fn branch_clear(&mut self, flag: &Flag) {
        if !self.get_flag_status(flag) {
            let displacement = self.mem_read(self.program_counter) as i8;
            self.program_counter = self
                .program_counter
                .wrapping_add(1)
                .wrapping_add(displacement as u16);
        }
    }
    fn cmp(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_addressing_mode(mode);
        let operand = self.mem_read(addr);
        if self.register_a >= operand {
            self.enable_flag(&Flag::Carry);
        } else {
            self.disable_flag(&Flag::Carry);
        }
        self.set_zn_flags_v1(self.register_a.wrapping_sub(operand));
    }
    fn cpx(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_addressing_mode(mode);
        let operand = self.mem_read(addr);
        if self.register_x >= operand {
            self.enable_flag(&Flag::Carry);
        } else {
            self.disable_flag(&Flag::Carry);
        }
        self.set_zn_flags_v1(self.register_x.wrapping_sub(operand));
    }
    fn cpy(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_addressing_mode(mode);
        let operand = self.mem_read(addr);
        if self.register_y >= operand {
            self.enable_flag(&Flag::Carry);
        } else {
            self.disable_flag(&Flag::Carry);
        }
        self.set_zn_flags_v1(self.register_y.wrapping_sub(operand));
    }
    fn dec(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_addressing_mode(mode);
        let mut operand = self.mem_read(addr);
        operand = operand.wrapping_sub(1);
        self.mem_write(addr, operand);
        self.set_zn_flags_v1(operand);
    }
    fn dex(&mut self) {
        self.register_x = self.register_x.wrapping_sub(1);
        self.set_zn_flags_v1(self.register_x);
    }
    fn dey(&mut self) {
        self.register_y = self.register_y.wrapping_sub(1);
        self.set_zn_flags_v1(self.register_y);
    }
    fn eor(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_addressing_mode(mode);
        self.register_a = self.register_a ^ self.mem_read(addr);
        self.set_zn_flags_v1(self.register_a);
    }
    fn inc(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_addressing_mode(mode);
        let mut operand = self.mem_read(addr);
        operand = operand.wrapping_add(1);
        self.mem_write(addr, operand);
        self.set_zn_flags_v1(operand);
    }
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
    pub fn run(&mut self) {
        loop {
            let opscode = self.mem_read(self.program_counter);
            self.program_counter += 1;
            match opscode {
                /*
                 * * * * * * * * * * ADC OPCODES * * * * * * * * * *
                 */
                0x69 => {
                    //ADC-I
                    self.adc(&AddressingMode::Immediate);
                    self.program_counter += 1;
                }
                0x65 => {
                    //ADC-ZP
                    self.adc(&AddressingMode::ZeroPage);
                    self.program_counter += 1;
                }
                0x75 => {
                    //ADC-ZPX
                    self.adc(&AddressingMode::ZeroPage_X);
                    self.program_counter += 1;
                }
                0x6D => {
                    //ADC-ABS
                    self.adc(&AddressingMode::Absolute);
                    self.program_counter += 2;
                }
                0x7D => {
                    //ADC-ABSX
                    self.adc(&AddressingMode::Absolute_X);
                    self.program_counter += 2;
                }
                0x79 => {
                    //ADC-ABSY
                    self.adc(&AddressingMode::Absolute_Y);
                    self.program_counter += 2;
                }
                0x61 => {
                    //ADC-INDX
                    self.adc(&AddressingMode::Indirect_X);
                    self.program_counter += 1;
                }
                0x71 => {
                    //ADC-INDY
                    self.adc(&AddressingMode::Indirect_Y);
                    self.program_counter += 1;
                }
                /*
                 * * * * * * * * * * AND OPCODES * * * * * * * * * *
                 */
                0x29 => {
                    //AND-I
                    self.and(&AddressingMode::Immediate);
                    self.program_counter += 1;
                }
                0x25 => {
                    //AND-ZP
                    self.and(&AddressingMode::ZeroPage);
                    self.program_counter += 1;
                }
                0x35 => {
                    //AND-ZPX
                    self.and(&AddressingMode::ZeroPage_X);
                    self.program_counter += 1;
                }
                0x2D => {
                    //AND-ABS
                    self.and(&AddressingMode::Absolute);
                    self.program_counter += 2;
                }
                0x3D => {
                    //AND-ABSX
                    self.and(&AddressingMode::Absolute_X);
                    self.program_counter += 2;
                }
                0x39 => {
                    //AND-ABSY
                    self.and(&AddressingMode::Absolute_Y);
                    self.program_counter += 2;
                }
                0x21 => {
                    //AND-INDX
                    self.and(&AddressingMode::Indirect_X);
                    self.program_counter += 1;
                }
                0x31 => {
                    //AND-INDY
                    self.and(&AddressingMode::Indirect_Y);
                    self.program_counter += 1;
                }
                /*
                 * * * * * * * * * * ASL OPCODES * * * * * * * * * *
                 */
                0x0A => {
                    //ASL-ACC
                    self.asl_accumulator();
                }
                0x06 => {
                    //ASL-ZP
                    self.asl(&AddressingMode::ZeroPage);
                    self.program_counter += 1;
                }
                0x16 => {
                    //ASL-ZPX
                    self.asl(&AddressingMode::ZeroPage_X);
                    self.program_counter += 1;
                }
                0x0E => {
                    //ASL-ABS
                    self.asl(&AddressingMode::Absolute);
                    self.program_counter += 2;
                }
                0x1E => {
                    //ASL-ABSX
                    self.asl(&AddressingMode::Absolute_X);
                    self.program_counter += 2;
                }
                /*
                 * * * * * * * * * * Bit Test OPCODES * * * * * * * * * *
                 */
                0x24 => {
                    //BIT-ZP
                    self.bit(&AddressingMode::ZeroPage);
                    self.program_counter += 1;
                }
                0x2C => {
                    //BIT-ABS
                    self.bit(&AddressingMode::Absolute);
                    self.program_counter += 2;
                }
                /*
                 * * * * * * * * * * Branch OPCODES * * * * * * * * * *
                 */
                0x90 => {
                    //BCC-Clear
                    self.branch_clear(&Flag::Carry);
                }
                0xB0 => {
                    //BCS-Set
                    self.branch_set(&Flag::Carry);
                }
                0xF0 => {
                    //BEQ-Set
                    self.branch_set(&Flag::Zero);
                }
                0xD0 => {
                    //BNE-Clear
                    self.branch_clear(&Flag::Zero);
                }
                0x30 => {
                    //BMI-Set
                    self.branch_set(&Flag::Negative);
                }
                0x10 => {
                    //BPL-Clear
                    self.branch_clear(&Flag::Negative);
                }
                0x50 => {
                    //BVC-Clear
                    self.branch_clear(&Flag::Overflow);
                }
                0x70 => {
                    //BVS-Set
                    self.branch_set(&Flag::Overflow);
                }
                /*
                 * * * * * * * * * * Clear OPCODES * * * * * * * * * *
                 */
                0x18 => {
                    //CLC
                    self.disable_flag(&Flag::Carry);
                }
                0xD8 => {
                    //CLD
                    self.disable_flag(&Flag::Dec);
                }
                0x58 => {
                    //CLI
                    self.disable_flag(&Flag::IRQ);
                }
                0xB8 => {
                    //CLV
                    self.disable_flag(&Flag::Overflow);
                }
                /*
                 * * * * * * * * * * CMP OPCODES * * * * * * * * * *
                 */
                0xC9 => {
                    //CMP-I
                    self.cmp(&AddressingMode::Immediate);
                    self.program_counter += 1;
                }
                0xC5 => {
                    //CMP-ZP
                    self.cmp(&AddressingMode::ZeroPage);
                    self.program_counter += 1;
                }
                0xD5 => {
                    //CMP-ZPX
                    self.cmp(&AddressingMode::ZeroPage_X);
                    self.program_counter += 1;
                }
                0xCD => {
                    //CMP-ABS
                    self.cmp(&AddressingMode::Absolute);
                    self.program_counter += 2;
                }
                0xDD => {
                    //CMP-ABSX
                    self.cmp(&AddressingMode::Absolute_X);
                    self.program_counter += 2;
                }
                0xD9 => {
                    //CMP-ABSY
                    self.cmp(&AddressingMode::Absolute_Y);
                    self.program_counter += 2;
                }
                0xC1 => {
                    //CMP-INDX
                    self.cmp(&AddressingMode::Indirect_X);
                    self.program_counter += 1;
                }
                0xD1 => {
                    //CMP-INDY
                    self.cmp(&AddressingMode::Indirect_Y);
                    self.program_counter += 1;
                }
                /*
                 * * * * * * * * * * CPX OPCODES * * * * * * * * * *
                 */
                0xE0 => {
                    //CPX-I
                    self.cpx(&AddressingMode::Immediate);
                    self.program_counter += 1;
                }
                0xE4 => {
                    //CPX-ZP
                    self.cpx(&AddressingMode::ZeroPage);
                    self.program_counter += 1;
                }
                0xEC => {
                    //CPX-ABS
                    self.cpx(&AddressingMode::Absolute);
                    self.program_counter += 2;
                }
                /*
                 * * * * * * * * * * CPY OPCODES * * * * * * * * * *
                 */
                0xC0 => {
                    //CPY-I
                    self.cpy(&AddressingMode::Immediate);
                    self.program_counter += 1;
                }
                0xC4 => {
                    //CPY-ZP
                    self.cpy(&AddressingMode::ZeroPage);
                    self.program_counter += 1;
                }
                0xCC => {
                    //CPY-ABS
                    self.cpy(&AddressingMode::Absolute);
                    self.program_counter += 2;
                }
                /*
                 * * * * * * * * * * DEC/DEX/DEY OPCODES * * * * * * * * * *
                 */
                0xC6 => {
                    //DEC-ZP
                    self.dec(&AddressingMode::ZeroPage);
                    self.program_counter += 1;
                }
                0xD6 => {
                    //DEC-ZPX
                    self.dec(&AddressingMode::ZeroPage_X);
                    self.program_counter += 1;
                }
                0xCE => {
                    //DEC-ABS
                    self.dec(&AddressingMode::Absolute);
                    self.program_counter += 2;
                }
                0xDE => {
                    //DEC-ABSX
                    self.dec(&AddressingMode::Absolute_X);
                    self.program_counter += 2;
                }
                0xCA => {
                    //DEX
                    self.dex();
                }
                0x88 => {
                    //DEY
                    self.dey();
                }
                /*
                 * * * * * * * * * * EOR OPCODES * * * * * * * * * *
                 */
                0x49 => {
                    //EOR-I
                    self.eor(&AddressingMode::Immediate);
                    self.program_counter += 1;
                }
                0x45 => {
                    //EOR-ZP
                    self.eor(&AddressingMode::ZeroPage);
                    self.program_counter += 1;
                }
                0x55 => {
                    //EOR-ZPX
                    self.eor(&AddressingMode::ZeroPage_X);
                    self.program_counter += 1;
                }
                0x4D => {
                    //EOR-ABS
                    self.eor(&AddressingMode::Absolute);
                    self.program_counter += 2;
                }
                0x5D => {
                    //EOR-ABSX
                    self.eor(&AddressingMode::Absolute_X);
                    self.program_counter += 2;
                }
                0x59 => {
                    //EOR-ABSY
                    self.eor(&AddressingMode::Absolute_Y);
                    self.program_counter += 2;
                }
                0x41 => {
                    //EOR-INDX
                    self.eor(&AddressingMode::Indirect_X);
                    self.program_counter += 1;
                }
                0x51 => {
                    //EOR-INDY
                    self.eor(&AddressingMode::Indirect_Y);
                    self.program_counter += 1;
                }
                /*
                 * * * * * * * * * * INC OPCODES * * * * * * * * * *
                 */
                0xE6 => {
                    //INC-ZP
                    self.inc(&AddressingMode::ZeroPage);
                    self.program_counter += 1;
                }
                0xF6 => {
                    //INC-ZPX
                    self.inc(&AddressingMode::ZeroPage_X);
                    self.program_counter += 1;
                }
                0xEE => {
                    //INC-ABS
                    self.inc(&AddressingMode::Absolute);
                    self.program_counter += 2;
                }
                0xFE => {
                    //INC-ABSX
                    self.inc(&AddressingMode::Absolute_X);
                    self.program_counter += 2;
                }
                0xE8 => {
                    //INX
                    self.inx();
                }
                0xC8 => {
                    //INY
                    self.iny();
                }
                /*
                 * * * * * * * * * * LDA OPCODES * * * * * * * * * *
                 */
                0xA9 => {
                    //LDA-I
                    self.lda(&AddressingMode::Immediate);
                    self.program_counter += 1;
                }
                0xA5 => {
                    //LDA-ZP
                    self.lda(&AddressingMode::ZeroPage);
                    self.program_counter += 1;
                }
                0xB5 => {
                    //LDA-ZPX
                    self.lda(&AddressingMode::ZeroPage_X);
                    self.program_counter += 1;
                }
                0xAD => {
                    //LDA-ABS
                    self.lda(&AddressingMode::Absolute);
                    self.program_counter += 2;
                }
                0xBD => {
                    //LDA-ABSX
                    self.lda(&AddressingMode::Absolute_X);
                    self.program_counter += 2;
                }
                0xB9 => {
                    //LDA-ABSY
                    self.lda(&AddressingMode::Absolute_Y);
                    self.program_counter += 2;
                }
                0xA1 => {
                    //LDA-INDX
                    self.lda(&AddressingMode::Indirect_X);
                    self.program_counter += 1;
                }
                0xB1 => {
                    //LDA-INDY
                    self.lda(&AddressingMode::Indirect_Y);
                    self.program_counter += 1;
                }
                /*
                 * * * * * * * * * * LDX OPCODES * * * * * * * * * *
                 */
                0xA2 => {
                    //LDX-I
                    self.ldx(&AddressingMode::Immediate);
                    self.program_counter += 1;
                }
                0xA6 => {
                    //LDX-ZP
                    self.ldx(&AddressingMode::ZeroPage);
                    self.program_counter += 1;
                }
                0xB6 => {
                    //LDX-ZPY
                    self.ldx(&AddressingMode::ZeroPage_Y);
                    self.program_counter += 1;
                }
                0xAE => {
                    //LDX-ABS
                    self.ldx(&AddressingMode::Absolute);
                    self.program_counter += 2;
                }
                0xBE => {
                    //LDX-ABSY
                    self.ldx(&AddressingMode::Absolute_Y);
                    self.program_counter += 2;
                }
                /*
                 * * * * * * * * * * LDY OPCODES * * * * * * * * * *
                 */
                0xA0 => {
                    //LDY-I
                    self.ldy(&AddressingMode::Immediate);
                    self.program_counter += 1;
                }
                0xA4 => {
                    //LDY-ZP
                    self.ldy(&AddressingMode::ZeroPage);
                    self.program_counter += 1;
                }
                0xB4 => {
                    //LDY-ZPX
                    self.ldy(&AddressingMode::ZeroPage_X);
                    self.program_counter += 1;
                }
                0xAC => {
                    //LDY-ABS
                    self.ldy(&AddressingMode::Absolute);
                    self.program_counter += 2;
                }
                0xBC => {
                    //LDY-ABSX
                    self.ldy(&AddressingMode::Absolute_X);
                    self.program_counter += 2;
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
                0xEA => {
                    //nop
                    continue;
                }
                _ => todo!(),
            }
        }
    }
}
