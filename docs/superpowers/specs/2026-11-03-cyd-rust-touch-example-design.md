# ESP32-2432S028R Rust touch example

## Goal

Add a minimal Rust firmware example under `firmware/esp2` for the ESP32 Cheap Yellow Display (ESP32-2432S028R). It shows a text prompt on the 320×240 ILI9341 display and changes the text after an XPT2046 touch.

## Approaches

- PlatformIO with Arduino/C++: best match for existing PlatformIO installation, but rejected because the requested example is Rust.
- Bare-metal `esp-hal`: Rust-native and lean, but peripheral setup and board support need more manual work.
- ESP-IDF Rust (`esp-idf-hal`): recommended; uses the ESP-IDF toolchain, supports the classic ESP32 and provides SPI/GPIO access for both peripherals.

## Design

Create an independent Cargo project in `firmware/esp2` using Rust with `esp-idf-hal`, `mipidsi`/`embedded-graphics` for text rendering, and `xpt2046` for touch input. Configure the board's documented display pins (SCLK 14, MOSI 13, MISO 12, CS 15, DC 2, backlight 21) and touch pins (SCLK 25, MOSI 32, MISO 39, CS 33, IRQ 36). Initialize the display, draw a short prompt, poll touch, and replace the prompt after a touch. Keep calibration out of scope; document that reported coordinates may need calibration if board revisions differ.

## Verification

Run `cargo check`/the ESP-IDF build for the project where the installed target toolchain allows it. Hardware validation requires a connected board and serial port; do not flash automatically.
