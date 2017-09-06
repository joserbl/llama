use std::cmp;
use std::collections::VecDeque;
use std::fmt;
use std::mem;

use extprim::u128::u128 as u128_t;
use openssl::symm;

bfdesc!(RegCnt: u32, {
    fifo_in_count: 0 => 4,
    fifo_out_count: 5 => 9,
    flush_fifo_in: 10 => 10,
    flush_fifo_out: 11 => 11,
    fifo_in_dma_size: 12 => 13,
    fifo_out_dma_size: 14 => 15,
    mac_size: 16 => 18,
    mac_source_reg: 20 => 20,
    mac_verified: 21 => 21,
    out_big_endian: 22 => 22,
    in_big_endian: 23 => 23,
    out_normal_order: 24 => 24,
    in_normal_order: 25 => 25,
    update_keyslot: 26 => 26,
    mode: 27 => 29,
    enable_irq: 30 => 30,
    busy: 31 => 31
});

bfdesc!(RegKeyCnt: u8, {
    keyslot: 0 => 5,
    use_dsi_keygen: 6 => 6,
    enable_fifo_flush: 7 => 7
});

#[derive(Clone, Copy)]
enum KeygenMode {
    THREEDS,
    DSi
}

#[derive(Clone, Copy, Default)]
struct Key {
    data: [u8; 0x10]
}

impl Key {
    fn from_keypair(keyx: &Key, keyy: &Key, mode: KeygenMode) -> Key {
        let keyx = keyx.to_u128();
        let keyy = keyy.to_u128();
        let common = match mode {
            KeygenMode::THREEDS => {
                let c = u128_t::from_str_radix("1FF9E9AAC5FE0408024591DC5D52768A", 16).unwrap();
                (keyx.rotate_left(2) ^ keyy).wrapping_add(c).rotate_right(41)
            }
            KeygenMode::DSi => unimplemented!()
        };
        Key::from_int(common)
    }

    fn from_int(mut num: u128_t) -> Key {
        let mut data = [0u8; 0x10];
        for b in data.iter_mut().rev() {
            *b = num.low64() as u8;
            num >>= 8;
        }
        Key { data: data }
    }

    fn to_u128(&self) -> u128_t {
        let mut new = u128_t::new(0);
        for b in self.data.iter() {
            new <<= 8;
            new |= u128_t::new(*b as u64);
        }
        new
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_tofrom128() {
        let key = Key { data: [0xd2, 0x2f, 0x5e, 0x15, 0xee, 0xfb, 0x12, 0x0d, 0x50, 0xf7, 0x6b, 0xbc, 0x76, 0x1a, 0x8f, 0x41] };
        let int = u128_t::from_str_radix("D22F5E15EEFB120D50F76BBC761A8F41", 16).unwrap();

        assert_eq!(key.data, Key::from_int(int).data);
        assert_eq!(key.to_u128(), int);
    }

    #[test]
    fn test_keygen() {
        let keyx: [u8; 0x10] = [0xd2, 0x2f, 0x5e, 0x15, 0xee, 0xfb, 0x12, 0x0d, 0x50, 0xf7, 0x6b, 0xbc, 0x76, 0x1a, 0x8f, 0x41];
        let keyy: [u8; 0x10] = [0xe7, 0x1c, 0x6c, 0x13, 0xe8, 0x0e, 0x40, 0x70, 0x1c, 0x1f, 0x03, 0x11, 0x14, 0x8b, 0x73, 0x8b];
        let norm: [u8; 0x10] = [0xde, 0x95, 0x19, 0xe2, 0x8b, 0x67, 0xcd, 0x7e, 0xf7, 0x8c, 0xf0, 0x06, 0x26, 0xb1, 0x04, 0x1f];
        assert_eq!(Key::from_keypair(&Key { data: keyx }, &Key { data: keyy }, KeygenMode::THREEDS).data,
            Key { data: norm }.data);
    }
}

#[derive(Default)]
struct KeyFifoState {
    pos: usize,
    buf: [u32; 4]
}

pub struct AesDeviceState {
    active_keyslot: usize,
    active_process: Option<symm::Crypter>,
    bytes_left: usize,

    key_slots: [Key; 0x40],
    keyx_slots: [Key; 0x40],
    keyfifo_state: KeyFifoState,
    keyxfifo_state: KeyFifoState,
    keyyfifo_state: KeyFifoState,

    fifo_in_buf: VecDeque<u32>,
    fifo_out_buf: VecDeque<u32>,
    reg_ctr: [u8; 0x10],
}

unsafe impl Send for AesDeviceState {} // TODO: Not good!

impl Default for AesDeviceState {
    fn default() -> AesDeviceState {
        AesDeviceState {
            active_keyslot: 0,
            active_process: None,
            bytes_left: 0,
            key_slots: [Default::default(); 0x40],
            keyx_slots: [Default::default(); 0x40],
            keyfifo_state: Default::default(),
            keyxfifo_state: Default::default(),
            keyyfifo_state: Default::default(),
            fifo_in_buf: VecDeque::new(),
            fifo_out_buf: VecDeque::new(),
            reg_ctr: [0; 0x10],
        }
    }
}

impl fmt::Debug for AesDeviceState {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "AesDeviceState {{ }}")
    }
}

fn reg_cnt_onread(dev: &mut AesDevice) {
    let mut cnt = dev.cnt.get();
    let in_count = cmp::min(16, dev._internal_state.fifo_in_buf.len());
    let out_count = cmp::min(16, dev._internal_state.fifo_out_buf.len());
    bf!(cnt @ RegCnt::fifo_in_count = in_count as u32);
    bf!(cnt @ RegCnt::fifo_out_count = out_count as u32);
    dev.cnt.set_unchecked(cnt);
}

fn reg_cnt_update(dev: &mut AesDevice) {
    let cnt = dev.cnt.get();
    warn!("STUBBED: Wrote 0x{:08X} to AES CNT register!", cnt);

    if bf!(cnt @ RegCnt::update_keyslot) == 1 {
        dev._internal_state.active_keyslot = dev.key_sel.get() as usize;
        trace!("Setting AES active keyslot to 0x{:X}", dev._internal_state.active_keyslot);
        // Remove update_keyslot bit
        dev.cnt.set_unchecked(bf!(cnt @ RegCnt::update_keyslot as 0));
    }

    if bf!(cnt @ RegCnt::busy) == 1 {
        let mode = bf!(cnt @ RegCnt::mode);
        let keyslot = dev._internal_state.active_keyslot;
        let key = dev._internal_state.key_slots[keyslot];
        let bytes = dev.blk_cnt.get() << 4;

        // Reverse word order for CTR
        let mut ctr: [u32; 4] = unsafe { mem::transmute(dev._internal_state.reg_ctr) };
        ctr.reverse();
        let ctr: [u8; 0x10] = unsafe { mem::transmute(ctr) };

        assert!(dev.mac_blk_cnt.get() == 0);
        assert!(bf!(cnt @ RegCnt::out_big_endian) == 1);
        assert!(bf!(cnt @ RegCnt::in_big_endian) == 1);
        assert!(bf!(cnt @ RegCnt::out_normal_order) == 1);
        assert!(bf!(cnt @ RegCnt::in_normal_order) == 1);

        let mut key_str = String::new();
        let mut iv_str = String::new();
        for b in key.data.iter() { key_str.push_str(&format!("{:02X}", b)); }
        for b in ctr.iter() { iv_str.push_str(&format!("{:02X}", b)); }

        trace!("Attempted to start AES crypto! mode: {}, keyslot: 0x{:X}, bytes: 0x{:X}, key: {}, iv: {}",
            mode, keyslot, bytes, key_str, iv_str);

        match mode {
            4 | 5 => {
                let symm_mode = if mode & 1 == 1 {
                    symm::Mode::Encrypt
                } else {
                    symm::Mode::Decrypt
                };
                let mut crypter = symm::Crypter::new(symm::Cipher::aes_128_cbc(), symm_mode,
                                                     &key.data[..], Some(&ctr[..])).unwrap();
                crypter.pad(false);
                dev._internal_state.active_process = Some(crypter);
            }
            _ => unimplemented!()
        }

        dev._internal_state.bytes_left = bytes as usize;
    }
}

fn reg_key_cnt_update(dev: &mut AesDevice) {
    let key_cnt = dev.key_cnt.get();
    let flush_fifo = bf!(key_cnt @ RegKeyCnt::enable_fifo_flush) == 1;

    trace!("Wrote to AES KEYCNT register; keyslot: 0x{:X}, Mode: {}, FIFO flush: {}",
        bf!(key_cnt @ RegKeyCnt::keyslot),
        if bf!(key_cnt @ RegKeyCnt::use_dsi_keygen) == 1 { "DSi" } else { "3DS" },
        flush_fifo
    );

    if flush_fifo {
        warn!("STUBBED: Flushing AES key FIFOs");
        // TODO: verify?
        dev._internal_state.keyfifo_state.pos = 0;
        dev._internal_state.keyxfifo_state.pos = 0;
        dev._internal_state.keyyfifo_state.pos = 0;
    }
}

fn reg_fifo_in_update(dev: &mut AesDevice) {
    {
        let active_process = dev._internal_state.active_process.as_mut()
            .expect("Attempted to write to AES FIFO-IN when not started!");

        let word = dev.fifo_in.get();
        dev._internal_state.fifo_in_buf.push_back(word);

        if dev._internal_state.fifo_in_buf.len() == 4 {
            let words = [
                dev._internal_state.fifo_in_buf.pop_front().unwrap(),
                dev._internal_state.fifo_in_buf.pop_front().unwrap(),
                dev._internal_state.fifo_in_buf.pop_front().unwrap(),
                dev._internal_state.fifo_in_buf.pop_front().unwrap()
            ];
            let bytes: [u8; 0x10] = unsafe { mem::transmute(words) };

            let mut dec_bytes = [0u8; 0x20]; // Double size because of library silliness
            active_process.update(&bytes[..], &mut dec_bytes[..]);

            let dec_words: [u32; 8] = unsafe { mem::transmute(dec_bytes) };
            dev._internal_state.fifo_out_buf.push_back(dec_words[0]);
            dev._internal_state.fifo_out_buf.push_back(dec_words[1]);
            dev._internal_state.fifo_out_buf.push_back(dec_words[2]);
            dev._internal_state.fifo_out_buf.push_back(dec_words[3]);
        }
    }

    dev._internal_state.bytes_left -= 4;
    if dev._internal_state.bytes_left == 0 {
        dev._internal_state.active_process = None;
        let cnt = dev.cnt.get();
        dev.cnt.set_unchecked(bf!(cnt @ RegCnt::busy as 0));
    }
}

fn reg_fifo_out_onread(dev: &mut AesDevice) {
    if let Some(word) = dev._internal_state.fifo_out_buf.pop_front() {
        dev.fifo_out.set_unchecked(word);
    }
}

#[derive(Clone, Copy)]
enum KeyType {
    CommonKey,
    KeyX,
    KeyY
}

fn reg_key_fifo_update(dev: &mut AesDevice, key_ty: KeyType) {
    let cnt = dev.cnt.get();
    let (word, state) = match key_ty {
        KeyType::CommonKey => (dev.key_fifo.get(), &mut dev._internal_state.keyfifo_state),
        KeyType::KeyX => (dev.keyx_fifo.get(), &mut dev._internal_state.keyxfifo_state),
        KeyType::KeyY => (dev.keyy_fifo.get(), &mut dev._internal_state.keyyfifo_state),
    };

    trace!("Wrote 0x{:08X} to AES {} register!", word.to_be(), match key_ty {
        KeyType::CommonKey => "KEYFIFO", KeyType::KeyX => "KEYXFIFO", KeyType::KeyY => "KEYYFIFO"
    });

    state.buf[state.pos / 4] = word;
    state.pos += 4;
    if state.pos >= 0x10 {
        // Done updating the key
        let key_cnt = dev.key_cnt.get();
        assert!(bf!(key_cnt @ RegKeyCnt::use_dsi_keygen) == 0);

        let keyslot = bf!(key_cnt @ RegKeyCnt::keyslot) as usize;
        let key = Key {
            data: unsafe { mem::transmute(state.buf) }
        };
        match key_ty {
            KeyType::CommonKey => dev._internal_state.key_slots[keyslot] = key,
            KeyType::KeyX => dev._internal_state.keyx_slots[keyslot] = key,
            KeyType::KeyY => {
                let keyx = &dev._internal_state.keyx_slots[keyslot];
                let keyy = &key;
                dev._internal_state.key_slots[keyslot] = Key::from_keypair(keyx, keyy, KeygenMode::THREEDS);
            }
        }
    }
}

fn reg_ctr_write(dev: &mut AesDevice, buf_pos: usize, src: &[u8]) {
    trace!("Writing {} bytes to AES CTR at +0x{:X}", src.len(), buf_pos);
    let dst_slice = &mut dev._internal_state.reg_ctr[buf_pos .. buf_pos + src.len()];
    dst_slice.clone_from_slice(src);
}

iodevice!(AesDevice, {
    internal_state: AesDeviceState;
    regs: {
        0x000 => cnt: u32 {
            write_bits = 0b11111111_11011111_11111100_00000000;
            read_effect = reg_cnt_onread;
            write_effect = reg_cnt_update;
        }
        0x004 => mac_blk_cnt: u16 { }
        0x006 => blk_cnt: u16 { }
        0x008 => fifo_in: u32 { write_effect = reg_fifo_in_update; }
        0x00C => fifo_out: u32 { read_effect = reg_fifo_out_onread; }
        0x010 => key_sel: u8 { }
        0x011 => key_cnt: u8 { write_effect = reg_key_cnt_update; }
        0x100 => key_fifo: u32 {
            read_effect = |_| unimplemented!();
            write_effect = |dev: &mut AesDevice| reg_key_fifo_update(dev, KeyType::CommonKey);
        }
        0x104 => keyx_fifo: u32 {
            read_effect = |_| unimplemented!();
            write_effect = |dev: &mut AesDevice| reg_key_fifo_update(dev, KeyType::KeyX);
        }
        0x108 => keyy_fifo: u32 {
            read_effect = |_| unimplemented!();
            write_effect = |dev: &mut AesDevice| reg_key_fifo_update(dev, KeyType::KeyY);
        }
    }
    ranges: {
        0x020;0x10 => {  // CTR
            read_effect = |_, _, _| unimplemented!();
            write_effect = reg_ctr_write;
        }
        0x030;0x10 => {  // MAC
            read_effect = |_, _, _| unimplemented!();
            write_effect = |_, _, _| unimplemented!();
        }
        0x040;0x30 => {  // KEY0
            read_effect = |_, _, _| unimplemented!();
            write_effect = |_, _, _| unimplemented!();
        }
        0x070;0x30 => {  // KEY1
            read_effect = |_, _, _| unimplemented!();
            write_effect = |_, _, _| unimplemented!();
        }
        0x0A0;0x30 => {  // KEY2
            read_effect = |_, _, _| unimplemented!();
            write_effect = |_, _, _| unimplemented!();
        }
        0x0D0;0x30 => {  // KEY3
            read_effect = |_, _, _| unimplemented!();
            write_effect = |_, _, _| unimplemented!();
        }
    }
});