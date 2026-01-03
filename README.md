# Pico Toy Elevator

A toy elevator simulator running on a Raspberry Pi Pico.

## Features

- 8 floors (B2, B1, 1-6) with LED buttons
- Door open/close buttons
- SSD1306 OLED display (128x64) showing elevator status
- ATP3012 speech synthesizer for floor announcements

## Hardware

Hardware design and schematics: https://github.com/tkhshmsy/pico-toy-elevator

### 3D Printed Case

STL files for the enclosure are in the `case/` directory:
- `PicoToy ELEVATOR case top.stl`
- `PicoToy ELEVATOR case bottom.stl`

### Components

- Raspberry Pi Pico
- SSD1306 OLED display (128x64, I2C)
- ATP3012 speech synthesizer (UART)
- 10 LED buttons (8 floors + door open/close)

### Pin Assignment

| Function | GPIO |
|----------|------|
| **I2C (Display)** | |
| SDA | GP16 |
| SCL | GP17 |
| **UART (Speech)** | |
| TX | GP0 |
| RX | GP1 |
| **Door Buttons** | |
| Close LED / Button | GP4 / GP2 |
| Open LED / Button | GP5 / GP3 |
| **Floor Buttons** | |
| B2 LED / Button | GP22 / GP27 |
| B1 LED / Button | GP26 / GP28 |
| 1F LED / Button | GP19 / GP21 |
| 2F LED / Button | GP18 / GP20 |
| 3F LED / Button | GP10 / GP8 |
| 4F LED / Button | GP11 / GP9 |
| 5F LED / Button | GP13 / GP14 |
| 6F LED / Button | GP12 / GP15 |

## Building

```bash
cargo build --release
```

## Flashing

### Via USB (BOOTSEL mode)

Hold BOOTSEL button while connecting USB, then:

```bash
cargo run --release
```

### Via picotool

```bash
picotool load -x target/thumbv6m-none-eabi/release/pico-toy-elevator.elf
```

### Via debug probe

```bash
cargo embed --release
```

## License

MIT OR Apache-2.0
