# renode_peripheral_testing

Demo STM32 SPI driver and mock C# device implementation. Proof-of-concept for debugging embedded Rust SPI peripheral drivers

# Renode
Renode is a tool that lets you test software targeting your favourite microcontrollers. 
- Compile targeting your MCU
- Mock custom peripherals using C# modules (or python but they don't work as well)

## Download
https://github.com/renode/renode/releases/tag/v1.16.0

# To run
`cargo build --release`

`renode --console run.resc`

## Demo driver bug
Check out the `demo-debugging-driver` branch. There is a driver bug. Try and find it 

(don't look at the commits on `main`...)

# Repo Layout
`src/main.rs` - Sets up UART and calls SPI setup. Runs some basic SPI tests and prints output

`src/mock_spi.rs` - Contains MockSpiDriver which exposes some basic SPI operations (read/write register, and echo input)

`src/stm32_spi.rs` - Implements SPI for STM32. Ideally will be done by the `embedded-hal` crate in future. 

`MockSpiPeripheral.cs` - Logic for mocked peripheral. Responds over SPI, has a rw register file and echo functionality

`mock_spi_board.repl` - Elects the MCU for renode to emulate. Does some memory and SPI setup

`run.resc` - Script for renode to step through. Commands can also be interactively entered into the renode console

