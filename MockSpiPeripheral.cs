using System;
using System.Collections.Generic;
using Antmicro.Renode.Core;
using Antmicro.Renode.Logging;
using Antmicro.Renode.Peripherals.SPI;

namespace Antmicro.Renode.Peripherals.SPI
{
    public class MockSpiPeripheral : ISPIPeripheral
    {

        public MockSpiPeripheral(IMachine machine)
        {
            this.machine = machine;
            registers = new byte[RegisterFileSize];
            Reset();
        }

        public byte Transmit(byte data)
        {
            byte response;

            switch (state)
            {
                case State.Idle:
                    byteIndex = 0;
                    switch ((Command)data)
                    {
                        case Command.Echo:
                            currentCommand = Command.Echo;
                            echoBuffer.Clear();
                            state = State.EchoPayload;
                            break;
                        
                        case Command.WriteReg:
                            currentCommand = Command.WriteReg;
                            writeAddr = 0;
                            writeValue = 0;
                            state = State.WriteRegAddr;
                            break;

                        case Command.ReadReg:
                            currentCommand = Command.ReadReg;
                            readAddr = 0;
                            state = State.ReadRegAddr;
                            break;

                        default:
                            LogError($"Unknown command byte 0x{data:X2}");
                            state = State.Error;
                            break;
                    }
                    return 0x0;
                
                case State.EchoPayload:
                    if (echoBuffer.Count > 0)
                    {
                        response = echoBuffer[0];
                        echoBuffer.RemoveAt(0);
                    }
                    else
                    {
                        response = 0x0;
                    }

                    echoBuffer.Add(data);
                    LogDebug($"Echo: received 0x{data:X2}, returning 0x{response:X2}");
                    return response;

                case State.WriteRegAddr:
                    writeAddr = data;
                    state = State.WriteRegValue;
                    LogDebug($"WriteReg: address = 0x{writeAddr:X2}");
                    return 0x0;

                case State.WriteRegValue:
                    writeValue = data;
                    if (writeAddr < RegisterFileSize)
                    {
                        registers[writeAddr] = writeValue;
                        LogDebug($"WriteReg: registers[0x{writeAddr:X2}] = 0x{writeValue:X2}");
                    }
                    else 
                    {
                        LogError($"WriteReg: address 0x{writeAddr:X2} out of range (max 0x{RegisterFileSize - 1:X2})");
                    }
                    state = State.Idle;
                    return 0x0;

                case State.ReadRegAddr:
                    readAddr = data;
                    state = State.ReadRegValue;
                    LogDebug($"ReadReg: address = 0x{readAddr:X2}");
                    return 0x0;

                case State.ReadRegValue:
                    if (readAddr < RegisterFileSize)
                    {
                        response = registers[readAddr];
                        LogDebug($"ReadReg: returning registers[0x{readAddr:X2}] = 0x{response:X2}");
                    }
                    else
                    {
                        LogError($"ReadReg: address 0x{readAddr:X2} out of range");
                        response = 0xFF;
                    }
                    return response;

                case State.Error:
                    return 0xFF;

                default:
                    return 0x0;
            }
        }

        public void FinishTransmission()
        {
            LogDebug($"FinishTransmission() – was in state {state}, command {currentCommand}");
            state = State.Idle;
            currentCommand = Command.None;
            echoBuffer.Clear();
        }

        public void Reset()
        {
            state = State.Idle;
            currentCommand = Command.None;
            echoBuffer.Clear();
            Array.Clear(registers, 0, registers.Length);
            LogDebug("Peripheral reset");
        }

        private void LogDebug(string msg)
        {
            machine?.Log(LogLevel.Debug, "[MockSpiPeripheral] " + msg);
        }

        private void LogError(string msg)
        {
            machine?.Log(LogLevel.Error, "[MockSpiPeripheral] " + msg);
        }

        private enum Command : byte
        {
            None = 0x0,
            Echo = 0x1,
            WriteReg = 0x2,
            ReadReg = 0x3
        }

        private enum State 
        {
            Idle,
            EchoPayload,
            WriteRegAddr,
            WriteRegValue,
            ReadRegAddr,
            ReadRegValue,
            Error,
        }

        private const int RegisterFileSize = 16;

        private readonly IMachine machine;
        private readonly byte[] registers;
        private readonly List<byte> echoBuffer = new List<byte>();

        private State state;
        private Command currentCommand;
        private int byteIndex;
        private byte writeAddr;
        private byte writeValue;
        private byte readAddr;
    }
}