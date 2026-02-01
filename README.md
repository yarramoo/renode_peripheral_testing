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