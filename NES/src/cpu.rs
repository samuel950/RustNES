use crate::bus::Bus;
use crate::bus::Memory;
use crate::opcodes;
use std::collections::HashMap;
#[allow(non_camel_case_types)]
#[derive(Debug)]
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
    Break2,
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
    pub bus: Bus,
}
const STACK_OFFSET: u16 = 0x100;
const STACK_RESET: u8 = 0xfd;
/*    7             6               5              4          3         2        1        0
|  negative  |  overflow  |  unused always 1  |  break  |  decimal  |  IRQ  |  zero  |  Carry  | --> processor status flags
glossary
cycles - how long an instruction should roughly take.
bytes - tells you if there are additional paramaters for command ie 2 bytes is 1 extra param.
address - points to a 1 byte cell (8bits) ie think how that relates to getting 16bits of data.
program counter - the current memory address. 2 bytes increment once, 3 bytes increment twice etc.
-In 2's complement, to make positive number negative, invert bits and add 1.
 */
impl Memory for CPU {
    fn mem_read(&self, addr: u16) -> u8 {
        self.bus.mem_read(addr)
    }
    fn mem_write(&mut self, addr: u16, data: u8) {
        self.bus.mem_write(addr, data);
    }
    fn mem_read_u16(&self, pos: u16) -> u16 {
        self.bus.mem_read_u16(pos)
    }
    fn mem_write_u16(&mut self, pos: u16, data: u16) {
        self.bus.mem_write_u16(pos, data);
    }
}
impl CPU {
    pub fn new(bus: Bus) -> Self {
        CPU {
            register_a: 0,
            register_x: 0,
            register_y: 0,
            status: 0b0010_0100,
            stack_ptr: STACK_RESET, //starts at 1fd per hardware specification
            program_counter: 0,
            bus: bus,
        }
    }

    /*fn mem_read_u16(&self, pos: u16) -> u16 {
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
    pub fn mem_read(&self, addr: u16) -> u8 {
        self.memory[addr as usize]
    }
    pub fn mem_write(&mut self, addr: u16, data: u8) {
        self.memory[addr as usize] = data;
    }*/
    fn stack_pop_u16(&mut self) -> u16 {
        let lo = self.stack_pop() as u16;
        let hi = self.stack_pop() as u16;
        (hi << 8) | lo
    }
    fn stack_push_u16(&mut self, data: u16) {
        let hi = (data >> 8) as u8;
        let lo = (data & 0xff) as u8;
        self.stack_push(hi);
        self.stack_push(lo);
    }
    fn stack_pop(&mut self) -> u8 {
        self.stack_ptr = self.stack_ptr.wrapping_add(1);
        self.mem_read(STACK_OFFSET + self.stack_ptr as u16)
    }
    fn stack_push(&mut self, data: u8) {
        self.mem_write(STACK_OFFSET + self.stack_ptr as u16, data);
        self.stack_ptr = self.stack_ptr.wrapping_sub(1);
    }
    pub fn reset(&mut self) {
        self.register_a = 0;
        self.register_x = 0;
        self.register_y = 0;
        self.status = 0b0010_0100;
        self.stack_ptr = STACK_RESET;
        self.program_counter = self.mem_read_u16(0xFFFC);
    }
    pub fn load(&mut self, program: Vec<u8>) {
        //self.bus.memory[0x0600..(0x0600 + program.len())].copy_from_slice(&program[..]);
        for i in 0..(program.len() as u16) {
            self.mem_write(0x0600 + i, program[i as usize]);
        }
        //self.mem_write_u16(0xFFFC, 0x8600);
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
                let lo = self.mem_read(ptr as u16) as u16;
                let hi = self.mem_read(ptr.wrapping_add(1) as u16) as u16;
                (hi << 8) | lo //dont need to 0 out first 8 bits because lo is originally 8 bits anyways.
                               //self.mem_read_u16(ptr as u16)
            }
            AddressingMode::Indirect_Y => {
                let indr_addr: u8 = self.mem_read(self.program_counter); //starting point of a 16bit address.

                let lo = self.mem_read(indr_addr as u16) as u16;
                let hi = self.mem_read(indr_addr.wrapping_add(1) as u16) as u16;
                let ptr: u16 = (hi << 8) | lo;
                //let ptr = self.mem_read_u16(indr_addr as u16);
                ptr.wrapping_add(self.register_y as u16)
            }
            AddressingMode::NotSupported => {
                panic!("Addressing mode {:?} is not supported!", mode);
            }
        }
    }
    pub fn get_operand_addressing_mode_trace(&self, mode: &AddressingMode, pc: u16) -> u16 {
        match mode {
            AddressingMode::Immediate => self.program_counter,
            AddressingMode::ZeroPage => self.mem_read(pc) as u16,
            AddressingMode::ZeroPage_X => {
                let zp_addr = self.mem_read(pc);
                zp_addr.wrapping_add(self.register_x) as u16
            }
            AddressingMode::ZeroPage_Y => {
                let zp_addr = self.mem_read(pc);
                zp_addr.wrapping_add(self.register_y) as u16
            }
            AddressingMode::Absolute => self.mem_read_u16(pc),
            AddressingMode::Absolute_X => {
                let abs_addr: u16 = self.mem_read_u16(pc);
                abs_addr.wrapping_add(self.register_x as u16)
            }
            AddressingMode::Absolute_Y => {
                let abs_addr: u16 = self.mem_read_u16(pc);
                abs_addr.wrapping_add(self.register_y as u16)
            }
            AddressingMode::Indirect_X => {
                let indr_addr: u8 = self.mem_read(pc);
                let ptr: u8 = indr_addr.wrapping_add(self.register_x);
                let lo = self.mem_read(ptr as u16) as u16;
                let hi = self.mem_read(ptr.wrapping_add(1) as u16) as u16;
                (hi << 8) | lo //dont need to 0 out first 8 bits because lo is originally 8 bits anyways.
                               //self.mem_read_u16(ptr as u16)
            }
            AddressingMode::Indirect_Y => {
                let indr_addr: u8 = self.mem_read(pc); //starting point of a 16bit address.

                let lo = self.mem_read(indr_addr as u16) as u16;
                let hi = self.mem_read(indr_addr.wrapping_add(1) as u16) as u16;
                let ptr: u16 = (hi << 8) | lo;
                //let ptr = self.mem_read_u16(indr_addr as u16);
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
            Flag::Break2 => self.status = self.status | 0b0010_0000,
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
            Flag::Break2 => self.status = self.status & 0b1101_1111,
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
            Flag::Break2 => self.status & 0b0010_0000 != 0,
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
         * then overflow occurs. If one or both of the addends has the
         * same sign as the result then overflow did not occur.
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
        if self.register_a > 0b0111_1111 {
            self.enable_flag(&Flag::Carry);
        } else {
            self.disable_flag(&Flag::Carry);
        }
        self.register_a = self.register_a << 1;
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
        let data = self.mem_read(addr);
        let operand = self.register_a & data;
        if operand == 0 {
            self.enable_flag(&Flag::Zero);
        } else {
            self.disable_flag(&Flag::Zero);
        }
        if data & 0b1000_0000 != 0 {
            self.enable_flag(&Flag::Negative);
        } else {
            self.disable_flag(&Flag::Negative);
        }
        if data & 0b0100_0000 != 0 {
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
    fn lsr_accumulator(&mut self) {
        if self.register_a & 0b0000_0001 == 1 {
            self.enable_flag(&Flag::Carry);
        } else {
            self.disable_flag(&Flag::Carry);
        }
        self.register_a = self.register_a >> 1;
        self.set_zn_flags_v1(self.register_a);
    }
    fn lsr(&mut self, mode: &AddressingMode) {
        //same effect as dividing by 2
        let addr = self.get_operand_addressing_mode(mode);
        let mut operand = self.mem_read(addr);
        if operand & 0b0000_0001 == 1 {
            self.enable_flag(&Flag::Carry);
        } else {
            self.disable_flag(&Flag::Carry);
        }
        operand = operand >> 1;
        self.mem_write(addr, operand);
        self.set_zn_flags_v1(operand);
    }
    fn ora(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_addressing_mode(mode);
        self.register_a = self.register_a | self.mem_read(addr);
        self.set_zn_flags_v1(self.register_a);
    }
    fn rol_accumulator(&mut self) {
        //isolating the carry flag
        let carry_isolate = self.status & 0b0000_0001;
        if self.register_a > 0b0111_1111 {
            self.enable_flag(&Flag::Carry);
        } else {
            self.disable_flag(&Flag::Carry);
        }
        self.register_a = self.register_a << 1;
        self.register_a = if carry_isolate == 1 {
            self.register_a | carry_isolate
        } else {
            self.register_a & 0b1111_1110
        };
        self.set_zn_flags_v1(self.register_a);
    }
    fn rol(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_addressing_mode(mode);
        let mut operand = self.mem_read(addr);
        let carry_isolate = self.status & 0b0000_0001;
        if operand > 0b0111_1111 {
            //carry threshold is greater than 255. so if operand is strictly greater than 127(times 2), then need to set carry.
            self.enable_flag(&Flag::Carry);
        } else {
            self.disable_flag(&Flag::Carry);
        }
        operand = operand << 1;
        operand = if carry_isolate == 1 {
            operand | carry_isolate
        } else {
            operand
        };
        self.mem_write(addr, operand);
        self.set_zn_flags_v1(operand);
    }
    fn ror_accumulator(&mut self) {
        let carry_isolate = self.status & 0b0000_0001;
        if self.register_a & 0b0000_0001 == 1 {
            self.enable_flag(&Flag::Carry);
        } else {
            self.disable_flag(&Flag::Carry);
        }
        self.register_a = self.register_a >> 1;
        self.register_a = if carry_isolate != 0 {
            self.register_a | 0b1000_0000
        } else {
            self.register_a
        };
        self.set_zn_flags_v1(self.register_a);
    }
    fn ror(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_addressing_mode(mode);
        let mut operand = self.mem_read(addr);
        let carry_isolate = self.status & 0b0000_0001;
        if operand & 0b0000_0001 == 1 {
            self.enable_flag(&Flag::Carry);
        } else {
            self.disable_flag(&Flag::Carry);
        }
        operand = operand >> 1;
        operand = if carry_isolate != 0 {
            operand | 0b1000_0000
        } else {
            operand
        };
        self.mem_write(addr, operand);
        self.set_zn_flags_v1(operand);
    }
    fn sbc(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_addressing_mode(mode);
        let operand = self.mem_read(addr);
        self.add(operand.wrapping_neg().wrapping_sub(1));
        self.set_zn_flags_v1(self.register_a);
    }
    fn sta(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_addressing_mode(mode);
        self.mem_write(addr, self.register_a);
    }
    fn stx(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_addressing_mode(mode);
        self.mem_write(addr, self.register_x);
    }
    fn sty(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_addressing_mode(mode);
        self.mem_write(addr, self.register_y);
    }
    /*
     * * * * * * * * * * Unoffical instruction functions start here * * * * * * * * * *
     */
    fn sax(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_addressing_mode(mode);
        let result = self.register_x & self.register_a;
        //self.set_zn_flags_v1(result); no status changes according to wiki
        self.mem_write(addr as u16, result)
    }
    pub fn run(&mut self) {
        self.run_with_callback(|_| {});
    }
    pub fn run_with_callback<F>(&mut self, mut callback: F)
    where
        F: FnMut(&mut CPU),
    {
        let ref opcodes: HashMap<u8, &'static opcodes::Opcode> = *opcodes::OPCODES_MAP;
        loop {
            callback(self);
            let opscode = self.mem_read(self.program_counter);
            let opscode_data = opcodes.get(&opscode).unwrap();
            self.program_counter += 1;
            let pccopy = self.program_counter;
            //println!("{:#04x}", opscode);
            match opscode {
                /*
                 * * * * * * * * * * ADC OPCODES * * * * * * * * * *
                 */
                0x69 => {
                    //ADC-I
                    self.adc(&AddressingMode::Immediate);
                }
                0x65 => {
                    //ADC-ZP
                    self.adc(&AddressingMode::ZeroPage);
                }
                0x75 => {
                    //ADC-ZPX
                    self.adc(&AddressingMode::ZeroPage_X);
                }
                0x6D => {
                    //ADC-ABS
                    self.adc(&AddressingMode::Absolute);
                }
                0x7D => {
                    //ADC-ABSX
                    self.adc(&AddressingMode::Absolute_X);
                }
                0x79 => {
                    //ADC-ABSY
                    self.adc(&AddressingMode::Absolute_Y);
                }
                0x61 => {
                    //ADC-INDX
                    self.adc(&AddressingMode::Indirect_X);
                }
                0x71 => {
                    //ADC-INDY
                    self.adc(&AddressingMode::Indirect_Y);
                }
                /*
                 * * * * * * * * * * AND OPCODES * * * * * * * * * *
                 */
                0x29 => {
                    //AND-I
                    self.and(&AddressingMode::Immediate);
                }
                0x25 => {
                    //AND-ZP
                    self.and(&AddressingMode::ZeroPage);
                }
                0x35 => {
                    //AND-ZPX
                    self.and(&AddressingMode::ZeroPage_X);
                }
                0x2D => {
                    //AND-ABS
                    self.and(&AddressingMode::Absolute);
                }
                0x3D => {
                    //AND-ABSX
                    self.and(&AddressingMode::Absolute_X);
                }
                0x39 => {
                    //AND-ABSY
                    self.and(&AddressingMode::Absolute_Y);
                }
                0x21 => {
                    //AND-INDX
                    self.and(&AddressingMode::Indirect_X);
                }
                0x31 => {
                    //AND-INDY
                    self.and(&AddressingMode::Indirect_Y);
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
                }
                0x16 => {
                    //ASL-ZPX
                    self.asl(&AddressingMode::ZeroPage_X);
                }
                0x0E => {
                    //ASL-ABS
                    self.asl(&AddressingMode::Absolute);
                }
                0x1E => {
                    //ASL-ABSX
                    self.asl(&AddressingMode::Absolute_X);
                }
                /*
                 * * * * * * * * * * Bit Test OPCODES * * * * * * * * * *
                 */
                0x24 => {
                    //BIT-ZP
                    self.bit(&AddressingMode::ZeroPage);
                }
                0x2C => {
                    //BIT-ABS
                    self.bit(&AddressingMode::Absolute);
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
                }
                0xC5 => {
                    //CMP-ZP
                    self.cmp(&AddressingMode::ZeroPage);
                }
                0xD5 => {
                    //CMP-ZPX
                    self.cmp(&AddressingMode::ZeroPage_X);
                }
                0xCD => {
                    //CMP-ABS
                    self.cmp(&AddressingMode::Absolute);
                }
                0xDD => {
                    //CMP-ABSX
                    self.cmp(&AddressingMode::Absolute_X);
                }
                0xD9 => {
                    //CMP-ABSY
                    self.cmp(&AddressingMode::Absolute_Y);
                }
                0xC1 => {
                    //CMP-INDX
                    self.cmp(&AddressingMode::Indirect_X);
                }
                0xD1 => {
                    //CMP-INDY
                    self.cmp(&AddressingMode::Indirect_Y);
                }
                /*
                 * * * * * * * * * * CPX OPCODES * * * * * * * * * *
                 */
                0xE0 => {
                    //CPX-I
                    self.cpx(&AddressingMode::Immediate);
                }
                0xE4 => {
                    //CPX-ZP
                    self.cpx(&AddressingMode::ZeroPage);
                }
                0xEC => {
                    //CPX-ABS
                    self.cpx(&AddressingMode::Absolute);
                }
                /*
                 * * * * * * * * * * CPY OPCODES * * * * * * * * * *
                 */
                0xC0 => {
                    //CPY-I
                    self.cpy(&AddressingMode::Immediate);
                }
                0xC4 => {
                    //CPY-ZP
                    self.cpy(&AddressingMode::ZeroPage);
                }
                0xCC => {
                    //CPY-ABS
                    self.cpy(&AddressingMode::Absolute);
                }
                /*
                 * * * * * * * * * * DEC/DEX/DEY OPCODES * * * * * * * * * *
                 */
                0xC6 => {
                    //DEC-ZP
                    self.dec(&AddressingMode::ZeroPage);
                }
                0xD6 => {
                    //DEC-ZPX
                    self.dec(&AddressingMode::ZeroPage_X);
                }
                0xCE => {
                    //DEC-ABS
                    self.dec(&AddressingMode::Absolute);
                }
                0xDE => {
                    //DEC-ABSX
                    self.dec(&AddressingMode::Absolute_X);
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
                }
                0x45 => {
                    //EOR-ZP
                    self.eor(&AddressingMode::ZeroPage);
                }
                0x55 => {
                    //EOR-ZPX
                    self.eor(&AddressingMode::ZeroPage_X);
                }
                0x4D => {
                    //EOR-ABS
                    self.eor(&AddressingMode::Absolute);
                }
                0x5D => {
                    //EOR-ABSX
                    self.eor(&AddressingMode::Absolute_X);
                }
                0x59 => {
                    //EOR-ABSY
                    self.eor(&AddressingMode::Absolute_Y);
                }
                0x41 => {
                    //EOR-INDX
                    self.eor(&AddressingMode::Indirect_X);
                }
                0x51 => {
                    //EOR-INDY
                    self.eor(&AddressingMode::Indirect_Y);
                }
                /*
                 * * * * * * * * * * INC OPCODES * * * * * * * * * *
                 */
                0xE6 => {
                    //INC-ZP
                    self.inc(&AddressingMode::ZeroPage);
                }
                0xF6 => {
                    //INC-ZPX
                    self.inc(&AddressingMode::ZeroPage_X);
                }
                0xEE => {
                    //INC-ABS
                    self.inc(&AddressingMode::Absolute);
                }
                0xFE => {
                    //INC-ABSX
                    self.inc(&AddressingMode::Absolute_X);
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
                 * * * * * * * * * * JMP/RTS OPCODES * * * * * * * * * *
                 */
                0x4C => {
                    //JMP-ABS
                    let addr = self.get_operand_addressing_mode(&AddressingMode::Absolute);
                    self.program_counter = addr;
                }
                0x6C => {
                    //JMP-IND
                    let addr = self.mem_read_u16(self.program_counter);
                    let indirect_addr = if addr & 0x00FF == 0x00FF {
                        let lo = self.mem_read(addr) as u16;
                        let hi = self.mem_read(addr & 0xFF00) as u16;
                        (hi << 8) | lo
                    } else {
                        self.mem_read_u16(addr)
                    };
                    self.program_counter = indirect_addr;
                }
                0x20 => {
                    //JSR-ABS
                    let addr = self.get_operand_addressing_mode(&AddressingMode::Absolute);
                    self.stack_push_u16(self.program_counter + 1); //+ 2 - 1
                    self.program_counter = addr;
                }
                0x60 => {
                    //RTS
                    self.program_counter = self.stack_pop_u16() + 1;
                }
                /*
                 * * * * * * * * * * LDA OPCODES * * * * * * * * * *
                 */
                0xA9 => {
                    //LDA-I
                    self.lda(&AddressingMode::Immediate);
                }
                0xA5 => {
                    //LDA-ZP
                    self.lda(&AddressingMode::ZeroPage);
                }
                0xB5 => {
                    //LDA-ZPX
                    self.lda(&AddressingMode::ZeroPage_X);
                }
                0xAD => {
                    //LDA-ABS
                    self.lda(&AddressingMode::Absolute);
                }
                0xBD => {
                    //LDA-ABSX
                    self.lda(&AddressingMode::Absolute_X);
                }
                0xB9 => {
                    //LDA-ABSY
                    self.lda(&AddressingMode::Absolute_Y);
                }
                0xA1 => {
                    //LDA-INDX
                    self.lda(&AddressingMode::Indirect_X);
                }
                0xB1 => {
                    //LDA-INDY
                    self.lda(&AddressingMode::Indirect_Y);
                }
                /*
                 * * * * * * * * * * LDX OPCODES * * * * * * * * * *
                 */
                0xA2 => {
                    //LDX-I
                    self.ldx(&AddressingMode::Immediate);
                }
                0xA6 => {
                    //LDX-ZP
                    self.ldx(&AddressingMode::ZeroPage);
                }
                0xB6 => {
                    //LDX-ZPY
                    self.ldx(&AddressingMode::ZeroPage_Y);
                }
                0xAE => {
                    //LDX-ABS
                    self.ldx(&AddressingMode::Absolute);
                }
                0xBE => {
                    //LDX-ABSY
                    self.ldx(&AddressingMode::Absolute_Y);
                }
                /*
                 * * * * * * * * * * LDY OPCODES * * * * * * * * * *
                 */
                0xA0 => {
                    //LDY-I
                    self.ldy(&AddressingMode::Immediate);
                }
                0xA4 => {
                    //LDY-ZP
                    self.ldy(&AddressingMode::ZeroPage);
                }
                0xB4 => {
                    //LDY-ZPX
                    self.ldy(&AddressingMode::ZeroPage_X);
                }
                0xAC => {
                    //LDY-ABS
                    self.ldy(&AddressingMode::Absolute);
                }
                0xBC => {
                    //LDY-ABSX
                    self.ldy(&AddressingMode::Absolute_X);
                }
                /*
                 * * * * * * * * * * LSR OPCODES * * * * * * * * * *
                 */
                0x4A => {
                    //LSR-ACC
                    self.lsr_accumulator();
                }
                0x46 => {
                    //LSR-ZP
                    self.lsr(&AddressingMode::ZeroPage);
                }
                0x56 => {
                    //LSR-ZPX
                    self.lsr(&AddressingMode::ZeroPage_X);
                }
                0x4E => {
                    //LSR-ABS
                    self.lsr(&AddressingMode::Absolute);
                }
                0x5E => {
                    //LSR-ABSX
                    self.lsr(&AddressingMode::Absolute_X);
                }
                /*
                 * * * * * * * * * * ORA OPCODES * * * * * * * * * *
                 */
                0x09 => {
                    //ORA-I
                    self.ora(&AddressingMode::Immediate);
                }
                0x05 => {
                    //ORA-ZP
                    self.ora(&AddressingMode::ZeroPage);
                }
                0x15 => {
                    //ORA-ZPX
                    self.ora(&AddressingMode::ZeroPage_X);
                }
                0x0D => {
                    //ORA-ABS
                    self.ora(&AddressingMode::Absolute);
                }
                0x1D => {
                    //ORA-ABSX
                    self.ora(&AddressingMode::Absolute_X);
                }
                0x19 => {
                    //ORA-ABSY
                    self.ora(&AddressingMode::Absolute_Y);
                }
                0x01 => {
                    //ORA-INDX
                    self.ora(&AddressingMode::Indirect_X);
                }
                0x11 => {
                    //ORA-INDY
                    self.ora(&AddressingMode::Indirect_Y);
                }
                /*
                 * * * * * * * * * * PUSH/PULL OPCODES * * * * * * * * * *
                 */
                0x48 => {
                    //PHA
                    self.stack_push(self.register_a);
                }
                0x08 => {
                    //PHP https://www.nesdev.org/wiki/Status_flags#The_B_flag
                    let mut flag = self.status;
                    flag = flag | 0b0011_0000; //enable "B" flag as per wiki
                    self.stack_push(flag);
                }
                0x68 => {
                    //PLA
                    self.register_a = self.stack_pop();
                    self.set_zn_flags_v1(self.register_a);
                }
                0x28 => {
                    //PLP
                    self.status = self.stack_pop();
                    self.disable_flag(&Flag::Break);
                    self.enable_flag(&Flag::Break2)
                }
                /*
                 * * * * * * * * * * ROL OPCODES * * * * * * * * * *
                 */
                0x2A => {
                    //ROL-ACC
                    self.rol_accumulator();
                }
                0x26 => {
                    //ROL-ZP
                    self.rol(&AddressingMode::ZeroPage);
                }
                0x36 => {
                    //ROL-ZPX
                    self.rol(&AddressingMode::ZeroPage_X);
                }
                0x2E => {
                    //ROL-ABS
                    self.rol(&AddressingMode::Absolute);
                }
                0x3E => {
                    //ROL-ABSX
                    self.rol(&AddressingMode::Absolute_X);
                }
                /*
                 * * * * * * * * * * ROR OPCODES * * * * * * * * * *
                 */
                0x6A => {
                    //ROR-ACC
                    self.ror_accumulator();
                }
                0x66 => {
                    //ROR-ZP
                    self.ror(&AddressingMode::ZeroPage);
                }
                0x76 => {
                    //ROR-ZPX
                    self.ror(&AddressingMode::ZeroPage_X);
                }
                0x6E => {
                    //ROR-ABS
                    self.ror(&AddressingMode::Absolute);
                }
                0x7E => {
                    //ROR-ABSX
                    self.ror(&AddressingMode::Absolute_X);
                }
                /*
                 * * * * * * * * * * RTI/BRK OPCODES * * * * * * * * * *
                 */
                0x00 => {
                    //brk
                    /*let mut flag = self.status;
                    flag = flag | 0b0011_0000; //enable "B" flag as per wiki
                    self.stack_push_u16(self.program_counter);
                    self.stack_push(flag);
                    self.program_counter = self.mem_read_u16(0xFFFE);
                    self.enable_flag(&Flag::Break);*/
                    return;
                }
                0x40 => {
                    //RTI
                    self.status = self.stack_pop();
                    self.disable_flag(&Flag::Break);
                    self.enable_flag(&Flag::Break2);
                    self.program_counter = self.stack_pop_u16();
                }
                /*
                 * * * * * * * * * * SBC OPCODES * * * * * * * * * *
                 */
                0xE9 => {
                    //SBC-I
                    self.sbc(&AddressingMode::Immediate);
                }
                0xE5 => {
                    //SBC-ZP
                    self.sbc(&AddressingMode::ZeroPage);
                }
                0xF5 => {
                    //SBC-ZPX
                    self.sbc(&AddressingMode::ZeroPage_X);
                }
                0xED => {
                    //SBC-ABS
                    self.sbc(&AddressingMode::Absolute);
                }
                0xFD => {
                    //SBC-ABSX
                    self.sbc(&AddressingMode::Absolute_X);
                }
                0xF9 => {
                    //SBC-ABSY
                    self.sbc(&AddressingMode::Absolute_Y);
                }
                0xE1 => {
                    //SBC-INDX
                    self.sbc(&AddressingMode::Indirect_X);
                }
                0xF1 => {
                    //SBC-INDY
                    self.sbc(&AddressingMode::Indirect_Y);
                }
                /*
                 * * * * * * * * * * SET OPCODES * * * * * * * * * *
                 */
                0x38 => {
                    //SEC
                    self.enable_flag(&Flag::Carry);
                }
                0xF8 => {
                    //SED
                    self.enable_flag(&Flag::Dec);
                }
                0x78 => {
                    //SEI
                    self.enable_flag(&Flag::IRQ);
                }
                /*
                 * * * * * * * * * * STA OPCODES * * * * * * * * * *
                 */
                0x85 => {
                    //STA-ZP
                    self.sta(&AddressingMode::ZeroPage);
                }
                0x95 => {
                    //STA-ZPX
                    self.sta(&AddressingMode::ZeroPage_X);
                }
                0x8D => {
                    //STA-ABS
                    self.sta(&AddressingMode::Absolute);
                }
                0x9D => {
                    //STA-ABSX
                    self.sta(&AddressingMode::Absolute_X);
                }
                0x99 => {
                    //STA-ABSY
                    self.sta(&AddressingMode::Absolute_Y);
                }
                0x81 => {
                    //STA-INDX
                    self.sta(&AddressingMode::Indirect_X);
                }
                0x91 => {
                    //STA-INDY
                    self.sta(&AddressingMode::Indirect_Y);
                }
                /*
                 * * * * * * * * * * STX OPCODES * * * * * * * * * *
                 */
                0x86 => {
                    //STX-ZP
                    self.stx(&AddressingMode::ZeroPage);
                }
                0x96 => {
                    //STX-ZPY
                    self.stx(&AddressingMode::ZeroPage_Y);
                }
                0x8E => {
                    //STX-ABS
                    self.stx(&AddressingMode::Absolute);
                }
                /*
                 * * * * * * * * * * STY OPCODES * * * * * * * * * *
                 */
                0x84 => {
                    //STY-ZP
                    self.sty(&AddressingMode::ZeroPage);
                }
                0x94 => {
                    //STY-ZPX
                    self.sty(&AddressingMode::ZeroPage_X);
                }
                0x8C => {
                    //STY-ABS
                    self.sty(&AddressingMode::Absolute);
                }
                /*
                 * * * * * * * * * * Transfer OPCODES * * * * * * * * * *
                 */
                0xAA => {
                    //TAX
                    self.tax();
                }
                0xA8 => {
                    //TAY
                    self.register_y = self.register_a;
                    self.set_zn_flags_v1(self.register_y);
                }
                0xBA => {
                    //TSX
                    self.register_x = self.stack_ptr;
                    self.set_zn_flags_v1(self.register_x);
                }
                0x8A => {
                    //TXA
                    self.register_a = self.register_x;
                    self.set_zn_flags_v1(self.register_a);
                }
                0x9A => {
                    //TXS
                    self.stack_ptr = self.register_x;
                }
                0x98 => {
                    //TYA
                    self.register_a = self.register_y;
                    self.set_zn_flags_v1(self.register_a);
                }
                0xEA | 0x1A | 0x3A | 0x5A | 0x7A | 0xDA | 0xFA => {
                    //nop
                    continue;
                }
                /*
                 * * * * * * * * * * Unofficial OPCODES * * * * * * * * * *
                 */
                0x4B => {
                    //ALR
                    self.register_a = self.register_a & self.mem_read(self.program_counter);
                    self.lsr_accumulator();
                }
                0x0B | 0x2B => {
                    //ANC
                    let result = self.register_a & self.mem_read(self.program_counter);
                    if self.is_negative(result) {
                        self.enable_flag(&Flag::Carry);
                    } else {
                        self.disable_flag(&Flag::Carry);
                    }
                    self.register_a = result;
                    self.set_zn_flags_v1(self.register_a);
                }
                0x6B => {
                    //ARR
                    self.register_a = self.register_a & self.mem_read(self.program_counter);
                    self.ror_accumulator();
                    let b5 = self.register_a & 0b0010_0000 != 0;
                    let b6 = self.register_a & 0b0100_0000 != 0;
                    if b6 {
                        self.enable_flag(&Flag::Carry);
                    } else {
                        self.disable_flag(&Flag::Carry);
                    }
                    if b6 ^ b5 {
                        self.enable_flag(&Flag::Overflow);
                    } else {
                        self.disable_flag(&Flag::Overflow);
                    }
                }
                0xCB => {
                    //AXS
                    let operand = self.mem_read(self.program_counter);
                    let andresult = self.register_a & self.register_x;
                    let result = andresult.wrapping_sub(operand);
                    if operand <= andresult {
                        self.enable_flag(&Flag::Carry);
                    }
                    self.set_zn_flags_v1(result);
                    self.register_x = result;
                }
                0xC3 | 0xC7 | 0xCF | 0xD3 | 0xD7 | 0xDB | 0xDF => {
                    let mode = &opscode_data.mode;
                    self.dec(mode);
                    self.cmp(mode);
                }
                0x0C | 0x1C | 0x3C | 0x5C | 0x7C | 0xDC | 0xFC | 0x04 | 0x44 | 0x64 | 0x14
                | 0x34 | 0x54 | 0x74 | 0xD4 | 0xF4 => {
                    //IGN
                    let addr = self.get_operand_addressing_mode(&opscode_data.mode);
                    self.mem_read(addr);
                }
                0xE3 | 0xE7 | 0xEF | 0xF3 | 0xF7 | 0xFB | 0xFF => {
                    //ISC/ISB
                    let mode = &opscode_data.mode;
                    self.inc(mode);
                    self.sbc(mode);
                }
                0xA3 | 0xA7 | 0xAF | 0xB3 | 0xB7 | 0xBF => {
                    //LAX
                    self.lda(&opscode_data.mode);
                    self.tax();
                }
                0xAB => {
                    //LXA
                    self.register_a = self.register_a & self.mem_read(self.program_counter);
                    self.set_zn_flags_v1(self.register_a);
                    self.tax();
                }
                0x87 | 0x97 | 0x83 | 0x8F => {
                    //SAX
                    self.sax(&opscode_data.mode);
                }
                0xEB => {
                    //SBC
                    let operand = self.mem_read(self.program_counter);
                    self.add(((operand as i8).wrapping_neg().wrapping_sub(1)) as u8);
                    self.set_zn_flags_v1(self.register_a);
                }
                0x9F | 0x93 => {
                    //SHA
                }
                0x80 | 0x82 | 0x89 | 0xC2 | 0xE2 => {
                    self.mem_read(self.program_counter);
                }
                _ => panic!(),
            }
            if pccopy == self.program_counter {
                self.program_counter += (opscode_data.len - 1) as u16;
            }
        }
    }
    fn tax(&mut self) {
        self.register_x = self.register_a;
        self.set_zn_flags_v1(self.register_x);
    }
}
