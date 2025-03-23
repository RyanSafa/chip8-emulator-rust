# Chip8 Emulator/interpeter/VM in C++

![Screenshot 2025-03-01 021422](https://github.com/user-attachments/assets/48bf553b-528f-46e1-b544-b2db4bc08fd7)

Requirements:
  * SDL2

```
Usage: chip8-emulator-rust [OPTIONS] <PATH_TO_ROM>

Arguments:
  <PATH_TO_ROM>  Path to a ROM

Options:
      --scale-factor <SCALE_FACTOR>
          Scale factor for the original 64 x 32 screen size [default: 24]
      --instructions-per-second <INSTRUCTIONS_PER_SECOND>
          The number of instructions that should be performed in one frame [default: 11]
      --primary-color <PRIMARY_COLOR>
          Primary color in rgba format Accepts hex values like "0xFF0000FF" [default: 0xFFFFFFFF]
      --secondary-color <SECONDARY_COLOR>
          Secondary color in rgba format Accepts hex values like "0x000000FF" [default: 0x000000FF]
  -h, --help
          Print help
  -V, --version
          Print version
```
