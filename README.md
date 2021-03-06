## llama

### the Low Level ARM Machine Amulator (of course)

---

### What is llama?

Llama is a ***very*** incomplete and experimental emulator for the Nintendo 3DS's ARM9. While you may find some use in it for debugging or reverse engineering, it still needs a lot of work to reach general usefulness.

Llama most certainly cannot run any 3DS games. Don't even try.

### What does it look like?

See for yourself!

![Llama's GUI, running Hourglass9](https://i.imgur.com/dl5YOH1.png)

> Source-level debugging in a GDB TUI? That's a lot of buzzwords!

Much like the emulator itself, llama's GUI is currently incomplete and probably not super nice to use.

### How do I use it?

First, you have to build llama from source. See below.

#### "ctr9 packages"

Llama loads binaries from what I call "ctr9" packages. These packages comprise a directory, named `[dirname].ctr9`, with the following structure:

```
foo.ctr9:
|- desc.json
|- ...
```

#### "desc.json"

The `desc.json` file describes how llama will load your ARM9 binaries. `desc.json` files look like this:

```
{
    "entryPoint": "0x0801B01C",
    "binFiles": [
        { "bin": "firm_2_08006800.bin", "vAddr": "0x08006800" }
    ],
    "arm11State": "kernelSync"
}
```

- `entryPoint`: Address at which llama will begin executing.
- `binFiles`: Array of binaries found within the ctr9 package.
  - `bin`: The binary filename.
  - `vAddr`: Address where llama will copy the binary.
- `arm11State` (defaults to `none`): String that describes how the ARM11 will be emulated. Possible values are:
  - `kernelSync`: Performs ARM9<=>ARM11 handshake that allows kernel to run
  - `bootSync`: Performs ARM9<=>ARM11 handshake that allows bootloader to run
  - `none`: Does not attempt any ARM11 emulation

#### Debugger

Llama will not automatically begin running the ctr9 package upon opening. To run, press the play/pause button or use the `run` debugger command.

Llama has a semi-useful built-in debugger controlled with textual commands.

- `run`: Unpauses the loaded program.
- `asm [address hex]`: Prints disassembly for the current instruction.
- `brk <address hex>`: Adds a CPU breakpoint at the specified address.
- `irq <type>`: Triggers an interrupt request of the specified type.
- `keydmp`: Dump AES keys.
- `mem <start address hex> [# bytes hex]`: Prints n bytes of memory from the specified address.
- `reg [register name]`: Prints specified register, or all registers if none specified.
- `step`: Runs one CPU instruction.

### What can I use it with?

My [crossbar9](https://github.com/archshift/crossbar9) repository can be used as a template for Rust programs that should run on both llama and the actual 3DS.

### How do I build it?

Llama is written in Rust and C++, which means you need a compiler for both languages installed on your system.

The GUI uses Qt5, which must be installed as well. Make sure you also have QtQuick/Qt-declarative.

#### Miscellaneous dependencies:

- Capstone disassembler

#### Actually building

Once all dependencies are installed, building should be as easy as running:

```
cargo build --release
```
