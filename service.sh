#!/bin/zsh

speak() {
    flite -voice slt -t "$1"
}

cd "${0:h}"

speak "Power on"
sleep 1
aplay sounds/beep-two.wav
speak "Initializing"
sleep 1
speak "Control systems online"
sleep 1
aplay sounds/quindar.wav
echo "This is control, reading all systems five by five. Cleared for launch." | festival --tts
aplay sounds/quindar.wav

RUST_LOG=info ./target/debug/gemini-panel /dev/i2c-1 /dev/i2c-0 testinputs.csv

