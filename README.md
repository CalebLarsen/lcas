# LC-3 Assembler (`lcas`)

A two-pass assembler in Rust for the LC-3 (Little Computer 3) architecture.

### Features
- Lexes and parses LC-3 assembly directives (`.ORIG`, `.FILL`, `.BLKW`, `.STRINGZ`, `.END`) and instructions
- Two-pass symbol resolution to map labels to memory addresses
- Emits assembled machine code directly to binary object files (`out.obj`)

### Usage
Place your assembly source in `in.asm` and run:

```bash
cargo run
```
