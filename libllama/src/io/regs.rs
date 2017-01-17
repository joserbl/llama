use std::mem;
use std::ptr;

use std::ops::BitAnd;
use std::ops::BitAndAssign;
use std::ops::BitOrAssign;
use std::ops::Not;

#[derive(Debug)]
pub struct IoReg<T>
    where T: Copy + BitAnd<Output=T> + BitAndAssign
                  + BitOrAssign + Not<Output=T> {
    val: T,
    write_bits: T,
}
impl<T> IoReg<T>
    where T: Copy + BitAnd<Output=T> + BitAndAssign
                  + BitOrAssign + Not<Output=T> {

    pub fn new(val: T, write_bits: T) -> IoReg<T> {
        IoReg { val: val, write_bits: write_bits }
    }
    pub fn set(&mut self, new_val: T) {
        self.val &= !self.write_bits;
        self.val |= new_val & self.write_bits;
    }
    pub fn set_unchecked(&mut self, new_val: T) {
        self.val = new_val;
    }
    pub fn get(&self) -> T {
        self.val
    }

    pub unsafe fn mem_load<BUF: Copy>(&self, buf: *mut BUF, buf_size: usize) {
        assert!(mem::size_of::<T>() == buf_size);
        ptr::write(mem::transmute(buf), self.get());
    }

    pub unsafe fn mem_save<BUF: Copy>(&mut self, buf: *const BUF, buf_size: usize) {
        assert!(mem::size_of::<T>() == buf_size);
        self.set(ptr::read(mem::transmute(buf)));
    }
}

pub trait IoRegAccess {
    unsafe fn read_reg(&self, offset: usize, buf: *mut u8, buf_size: usize);
    unsafe fn write_reg(&mut self, offset: usize, buf: *const u8, buf_size: usize);
}


macro_rules! __iodevice__ {
    ($name:ident, {
        $(
            $reg_offs:expr => $reg_name:ident: $reg_ty:ty {
                default = $reg_default:expr;
                write_bits = $reg_wb:expr;
                read_effect = $reg_reff:expr;
                write_effect = $reg_weff:expr;
            }
        )*
    }) => (
        #[derive(Debug)]
        pub struct $name {
            $( $reg_name: $crate::io::regs::IoReg<$reg_ty>, )*
        }

        impl ::std::default::Default for $name {
            fn default() -> $name {
                $name {
                    $( $reg_name: $crate::io::regs::IoReg::new($reg_default, $reg_wb), )*
                }
            }
        }

        impl $name {
            #[allow(dead_code)]
            pub fn new() -> $name {
                ::std::default::Default::default()
            }
        }

        impl $crate::io::regs::IoRegAccess for $name {
            unsafe fn read_reg(&self, offset: usize, buf: *mut u8, buf_size: usize) {
                trace!("Reading from $name at +0x{:X}", offset);
                match offset {
                    $( $reg_offs => {
                        self.$reg_name.mem_load(buf, buf_size);
                        $reg_reff();
                    })*
                    _ => panic!("at the disco")
                }
            }

            unsafe fn write_reg(&mut self, offset: usize, buf: *const u8, buf_size: usize) {
                trace!("Writing to $name at +0x{:X}", offset);
                match offset {
                    $( $reg_offs => {
                        self.$reg_name.mem_save(buf, buf_size);
                        $reg_weff();
                    })*
                    _ => panic!("at the disco")
                }
            }
        }
    )
}

macro_rules! __iodevice_desc_default__ {
    ($val:expr) => ($val);
    () => (0);
}

macro_rules! __iodevice_desc_wb__ {
    ($val:expr) => ($val);
    () => (!0);
}

macro_rules! __iodevice_desc_reff__ {
    ($val:expr) => ($val);
    () => (||{});
}

macro_rules! __iodevice_desc_weff__ {
    ($val:expr) => ($val);
    () => (||{});
}

#[macro_export]
macro_rules! iodevice {
    ($name:ident, {
        $(
            $reg_offs:expr => $reg_name:ident: $reg_ty:ty {
                $(default = $reg_default:expr;)*
                $(write_bits = $reg_wb:expr;)*
                $(read_effect = $reg_reff:expr;)*
                $(write_effect = $reg_weff:expr;)*
            }
        )*
    }) => (
        __iodevice__!($name, {
            $(
                $reg_offs => $reg_name: $reg_ty {
                    default = __iodevice_desc_default__!($($reg_default),*);
                    write_bits = __iodevice_desc_wb__!($($reg_wb),*);
                    read_effect = __iodevice_desc_reff__!($($reg_reff),*);
                    write_effect = __iodevice_desc_weff__!($($reg_weff),*);
                }
            )*
        });
    );
}


#[cfg(test)]
mod test {
    use super::*;

    iodevice!(MMCRegs, {
        0x000 => reg0: u16 { }
        0x002 => reg2: u16 {
            write_effect = || { panic!("while writing") };
        }
        0x004 => reg4: u16 { write_bits = 0; }
    });

    #[test]
    fn read_reg() {
        let mmc_regs = MMCRegs::new();
        let mut buf = vec![0xFFu8; 2];
        unsafe { mmc_regs.read_reg(0x000, buf.as_mut_ptr(), buf.len()); }
        assert_eq!(buf, vec![0x00, 0x00]);
    }

    #[test]
    fn write_reg() {
        let mut mmc_regs = MMCRegs::new();
        assert_eq!(mmc_regs.cmd.get(), 0x0000);

        let mut buf = vec![0xFFu8; 2];
        unsafe { mmc_regs.write_reg(0x000, buf.as_ptr(), buf.len()); }
        assert_eq!(mmc_regs.cmd.get(), 0xFFFF);
    }

    #[test]
    #[should_panic]
    fn write_effect() {
        let mut mmc_regs = MMCRegs::new();
        let mut buf = vec![0xFFu8; 2];
        unsafe { mmc_regs.write_reg(0x002, buf.as_ptr(), buf.len()); }
    }
}