# renode_peripheral_testing

Demo STM32 SPI driver and mock C# device implementation. Proof-of-concept for debugging embedded Rust SPI peripheral drivers

# Renode
Renode is a tool that lets you test software targeting your favourite microcontrollers. 
- Compile targeting your MCU
- Mock custom peripherals using C# modules (or python but they don't work as well)

## Setup
(This is for mac)
1. Download renode tool: https://github.com/renode/renode/releases/tag/v1.16.0
2. Install mono .NET framework: `brew install mono`
3. Build this repo `cargo build --release`

# To run
`mono <path_to_Renode.exe> --console run.resc` 
- For me it's `mono /Applications/Renode.app/Contents/MacOS/bin/Renode.exe`
- I made a bash script called `renode` so you can just call `renode` from the cmd line 

```
#!/bin/bash
exec mono /Applications/Renode.app/Contents/MacOS/bin/Renode.exe "$@"
```
Then: `renode --console run.resc`

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

## Todos 
Currently this repo doesn't use the embedded_hal traits. That's why UART enabling is done via `write_volatile` and there's a custom implementation of `SPI` in `stm32_spi.rs`. For some reason there were some compatability issues. In theory there shouldn't be an issue so this just needs debugging. 
